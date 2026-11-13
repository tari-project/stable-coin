//   Copyright 2023. The Tari Project
//
//   Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//   following conditions are met:
//
//   1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//   disclaimer.
//
//   2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//   following disclaimer in the documentation and/or other materials provided with the distribution.
//
//   3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//   products derived from this software without specific prior written permission.
//
//   THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//   INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//   DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//   SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//   SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//   WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//   USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

#![no_std]

extern crate alloc;

pub mod config;
mod roles;
use alloc::vec::Vec;
use tari_template_lib::prelude::*;

#[cfg(all(not(target_feature = "atomics"), target_family = "wasm"))]
#[global_allocator]
static TALC: talc::wasm::WasmArenaTalc = {
    use core::mem::MaybeUninit;
    static mut MEMORY: [MaybeUninit<u8>; 0x80000] = [MaybeUninit::uninit(); 0x80000];
    // SAFETY: the memory for MEMORY is never modified externally. It's the allocator's.
    unsafe { talc::wasm::new_wasm_arena_allocator(&raw mut MEMORY) }
};

#[template]
mod template {
    use tari_template_lib::types::crypto::CommitmentValueProof;

    use tari_template_lib::types::SubstateOwnerRule;

    use super::*;
    use crate::config::StableCoinConfig;
    use crate::roles::{PendingGovernor, Role, RoleConfig, Roles, assert_valid_role_rule};

    /// The role that may call each component method. A method not listed here can only be called by the governor,
    /// which owns the component.
    const METHOD_ROLES: &[(&str, Role)] = &[
        ("increase_supply", Role::Minter),
        ("decrease_supply", Role::Burner),
        ("burn_utxo", Role::Burner),
        ("withdraw", Role::Treasurer),
        ("deposit", Role::Treasurer),
        ("recall_revealed_tokens", Role::Compliance),
        ("freeze_utxos", Role::Compliance),
        ("unfreeze_utxos", Role::Compliance),
        ("pause", Role::Pauser),
    ];

    /// The most UTXOs `freeze_utxos` and `unfreeze_utxos` take in one call.
    const MAX_FREEZE_BATCH: usize = 100;

    pub struct TariStableCoin {
        config: StableCoinConfig,
        token_vault: Vault,
        is_paused: bool,
        roles: Roles,
        pending_governor: Option<PendingGovernor>,
    }

    impl TariStableCoin {
        /// Instantiates a new stable coin component.
        ///
        /// `roles` assigns each privileged role an access rule over signer keys. The governor is required, and every
        /// unset role follows the governor. The component owns the coin resource, so all privileged actions go
        /// through its methods and are subject to pause and to the role rules.
        #[allow(clippy::too_many_arguments)]
        pub fn instantiate(
            address_alloc: ComponentAddressAllocation,
            initial_token_supply: Amount,
            token_symbol: MaxString<8>,
            token_metadata: Metadata,
            divisibility: u8,
            view_key: RistrettoPublicKeyBytes,
            config: Option<StableCoinConfig>,
            roles: RoleConfig,
        ) -> Component<Self> {
            let config = config.unwrap_or_default();
            if let Some(max_supply) = config.max_supply {
                assert!(
                    initial_token_supply <= max_supply,
                    "The initial supply exceeds the max supply"
                );
            }
            // Minting, burning, recalling and freezing the coin are reserved for this component, so
            // every privileged action goes through a component method, where the role rules and
            // pause apply.
            let component_address = address_alloc.get_address();
            let require_component = rule!(component(component_address));

            // Create tokens resource with initial supply
            let initial_tokens = ResourceBuilder::stealth()
                .with_metadata(token_metadata)
                .with_token_symbol(token_symbol.as_ref())
                // Access rules
                .mintable(require_component.clone(), LOCKED)
                .burnable(require_component.clone(), LOCKED)
                .recallable(require_component.clone(), LOCKED)
                .freezable(require_component, LOCKED)
                .with_view_key(view_key)
                .with_divisibility(divisibility)
                .with_owner_rule(OwnerRule::None)
                .initial_supply(initial_token_supply);

            let roles = Roles::from_config(roles);
            let component_access_rules = Self::component_access_rules(&roles, None);
            let governor = roles.get(Role::Governor).clone();

            Component::new(Self {
                config,
                token_vault: Vault::from_bucket(initial_tokens),
                is_paused: false,
                roles,
                pending_governor: None,
            })
            .with_address_allocation(address_alloc)
            .with_access_rules(component_access_rules)
            // The governor owns the component: it can call every method and is the only role
            // permitted to change the access rules.
            .with_owner_rule(OwnerRule::ByAccessRule(governor))
            .create()
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, amount: Amount) {
            self.assert_not_paused();
            assert!(!amount.is_zero());
            self.assert_within_max_supply(amount);
            let new_tokens = self.token_vault_manager().mint_stealth(amount);
            self.token_vault.deposit(new_tokens);
            emit_event("increase_supply", metadata!("amount" => amount));
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, amount: Amount) {
            self.assert_not_paused();
            assert!(!amount.is_zero());
            let tokens = self.token_vault.withdraw(amount);
            tokens.burn();
            emit_event("decrease_supply", metadata!("amount" => amount));
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            self.assert_not_paused();
            assert!(amount.is_positive(), "Amount must be positive");
            let bucket = self.token_vault.withdraw(amount);
            emit_event("withdraw", metadata!("amount" => bucket.amount()));
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            self.assert_not_paused();
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", metadata!("amount" => amount));
        }

        pub fn recall_revealed_tokens(&mut self, vault_id: VaultId, amount: Amount) {
            assert!(!amount.is_zero());
            let bucket = self
                .token_vault_manager()
                .recall_fungible_amount(vault_id, amount);
            self.token_vault.deposit(bucket);

            emit_event(
                "recall_revealed_tokens",
                metadata!(
                    "vault_id" => vault_id,
                    "amount" => amount,
                ),
            );
        }

        pub fn burn_utxo(&mut self, utxo: UtxoId, value_proof: CommitmentValueProof) {
            self.assert_not_paused();
            self.token_vault_manager().burn_utxo(utxo, value_proof);
            emit_event("burn_utxo", metadata!("utxo_id" => utxo));
        }

        /// Gives `role` to whoever satisfies `rule`.
        ///
        /// Changing the governor needs a proposal for `rule` that its holder has confirmed: see `propose_governor`.
        pub fn set_role(&mut self, role: Role, rule: AccessRule) {
            if role == Role::Governor {
                let pending = self
                    .pending_governor
                    .take()
                    .expect("No governor proposal is pending");
                assert!(
                    pending.rule == rule,
                    "The rule does not match the pending governor proposal"
                );
                assert!(pending.confirmed, "The proposed governor has not confirmed");
            }
            self.roles.set(role, rule.clone());
            let component = self.update_access_rules();
            emit_event("set_role", metadata!("role" => role, "rule" => rule));
            // Ownership is checked against the current governor, so the owner rule changes last.
            if role == Role::Governor {
                component.set_owner_rule(SubstateOwnerRule::ByAccessRule(rule));
            }
        }

        /// Proposes `rule` as the next governor, replacing any earlier proposal. Whoever satisfies `rule` must call
        /// `confirm_governor` before the governor can hand over with `set_role`, which guards against a rule no one
        /// holds (a mistyped key).
        ///
        /// The governor owns the component and can call `confirm_governor` itself, so this guards against mistakes,
        /// not against a governor that means to hand over to an unheld rule.
        pub fn propose_governor(&mut self, rule: AccessRule) {
            assert_valid_role_rule(Role::Governor, &rule);
            emit_event("propose_governor", metadata!("rule" => rule));
            self.pending_governor = Some(PendingGovernor {
                rule,
                confirmed: false,
            });
            self.update_access_rules();
        }

        /// Confirms the pending governor proposal. Callable by whoever satisfies the proposed rule.
        pub fn confirm_governor(&mut self) {
            let pending = self
                .pending_governor
                .as_mut()
                .expect("No governor proposal is pending");
            pending.confirmed = true;
            emit_event("confirm_governor", metadata!("rule" => pending.rule));
        }

        /// Withdraws the pending governor proposal.
        pub fn cancel_governor_proposal(&mut self) {
            let pending = self
                .pending_governor
                .take()
                .expect("No governor proposal is pending");
            emit_event(
                "cancel_governor_proposal",
                metadata!("rule" => pending.rule),
            );
            self.update_access_rules();
        }

        /// Sets the cap on outstanding supply, or removes it with `None`. The cap cannot be set below the current
        /// supply.
        pub fn set_config_max_supply(&mut self, max_supply: Option<Amount>) {
            if let Some(max_supply) = max_supply {
                assert!(
                    self.outstanding_supply() <= max_supply,
                    "The max supply is below the current supply"
                );
            }
            emit_event(
                "set_config_max_supply",
                metadata!("prev" => self.config.max_supply, "new" => max_supply),
            );
            self.config.max_supply = max_supply;
        }

        pub fn pause(&mut self) {
            self.is_paused = true;
            emit_event("pause", Metadata::new());
        }

        pub fn unpause(&mut self) {
            self.is_paused = false;
            emit_event("unpause", Metadata::new());
        }

        pub fn freeze_utxos(&self, utxos: Vec<UtxoId>) {
            assert_freeze_batch(&utxos);
            emit_event("freeze_utxos", metadata!("utxos" => utxos));
            self.token_vault_manager().freeze_utxos(utxos);
        }

        pub fn unfreeze_utxos(&self, utxos: Vec<UtxoId>) {
            assert_freeze_batch(&utxos);
            emit_event("unfreeze_utxos", metadata!("utxos" => utxos));
            self.token_vault_manager().unfreeze_utxos(utxos);
        }

        /// `confirm_governor` is open to the proposed governor, and to no one but the owner while nothing is
        /// proposed.
        fn component_access_rules(roles: &Roles, pending: Option<&PendingGovernor>) -> AccessRules {
            let confirm_rule = pending.map_or(AccessRule::DenyAll, |p| p.rule.clone());
            METHOD_ROLES
                .iter()
                .fold(AccessRules::new(), |rules, (method, role)| {
                    rules.add_method_rule(*method, roles.get(*role).clone())
                })
                .add_method_rule("confirm_governor", confirm_rule)
                .default(roles.get(Role::Governor).clone())
        }

        fn update_access_rules(&self) -> ComponentManager {
            let component = ComponentManager::current();
            component.set_access_rules(Self::component_access_rules(
                &self.roles,
                self.pending_governor.as_ref(),
            ));
            component
        }

        /// The supply `max_supply` caps.
        fn outstanding_supply(&self) -> Amount {
            self.token_vault_manager().total_supply()
        }

        fn assert_within_max_supply(&self, amount: Amount) {
            if let Some(max_supply) = self.config.max_supply {
                let new_supply = self
                    .outstanding_supply()
                    .checked_add(amount)
                    .expect("Supply overflow");
                assert!(
                    new_supply <= max_supply,
                    "Minting would exceed the max supply"
                );
            }
        }

        fn token_vault_manager(&self) -> ResourceManager {
            self.token_vault.get_resource_manager()
        }

        fn assert_not_paused(&self) {
            assert!(!self.is_paused, "Stable coin is paused");
        }
    }

    fn assert_freeze_batch(utxos: &[UtxoId]) {
        assert!(
            utxos.len() <= MAX_FREEZE_BATCH,
            "Too many UTXOs in one batch"
        );
    }
}
