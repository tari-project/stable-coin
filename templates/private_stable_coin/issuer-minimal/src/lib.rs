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
use alloc::string::String;
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
    use crate::config::FeeSpec;
    use crate::config::StableCoinConfig;
    use crate::roles::{Role, RoleConfig, Roles};

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

    pub struct TariStableCoin {
        config: StableCoinConfig,
        token_vault: Vault,
        admin_auth_manager: ResourceManager,
        is_paused: bool,
        roles: Roles,
    }

    impl TariStableCoin {
        /// Instantiates a new stable coin component, returning a bucket containing an admin badge.
        ///
        /// `roles` assigns each privileged role an access rule; any role left unset is held by the admin badge. The
        /// component owns every resource it creates, so all privileged actions go through its methods and are
        /// subject to pause and to the role rules.
        #[allow(clippy::too_many_arguments)]
        pub fn instantiate(
            address_alloc: ComponentAddressAllocation,
            initial_token_supply: Amount,
            token_symbol: MaxString<8>,
            token_metadata: Metadata,
            divisibility: u8,
            view_key: RistrettoPublicKeyBytes,
            config: Option<StableCoinConfig>,
            roles: Option<RoleConfig>,
        ) -> Bucket {
            let config = config.unwrap_or_default();
            // Minting, burning, recalling and freezing the coin are reserved for this component.
            // A proof handed to a component at a call boundary is revoked once the callee's own
            // access rule has been checked, so a workspace badge proof is not in scope inside a
            // method body and cannot satisfy a `resource(..)` rule there. Role checks happen at the
            // component method boundary.
            let component_address = address_alloc.get_address();
            let require_component = rule!(component(component_address));

            // Admin badges are issued and revoked only through the component, so no key or badge
            // can mint one directly and a lost badge can be recalled and burnt by the governor.
            let admin_badge = ResourceBuilder::non_fungible()
                .with_token_symbol("ADM")
                .mintable(require_component.clone(), LOCKED)
                .burnable(require_component.clone(), LOCKED)
                .recallable(require_component.clone(), LOCKED)
                .update_non_fungible_data(rule!(deny_all), LOCKED)
                .with_owner_rule(OwnerRule::None)
                .initial_supply(Some(NonFungibleId::from_u64(0)));

            let admin_resource = admin_badge.resource_address();
            let roles =
                Roles::from_config(roles.unwrap_or_default(), rule!(resource(admin_resource)));

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

            let component_access_rules = Self::component_access_rules(&roles);
            let governor = roles.get(Role::Governor).clone();

            Component::new(Self {
                config,
                token_vault: Vault::from_bucket(initial_tokens),
                admin_auth_manager: admin_resource.into(),
                is_paused: false,
                roles,
            })
            .with_address_allocation(address_alloc)
            .with_access_rules(component_access_rules)
            // The governor owns the component: it can call every method and is the only role
            // permitted to change the access rules.
            .with_owner_rule(OwnerRule::ByAccessRule(governor))
            .create();

            admin_badge
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, amount: Amount) {
            self.assert_not_paused();
            assert!(!amount.is_zero());
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
            emit_event("decrease_supply", metadata!("amt" => amount));
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            self.assert_not_paused();
            assert!(amount.is_positive(), "Amount must be positive");
            let bucket = self.token_vault.withdraw(amount);
            emit_event("withdraw", metadata!("amt" => bucket.amount()));
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            self.assert_not_paused();
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", metadata!("amt" => amount));
        }

        pub fn recall_revealed_tokens(&mut self, vault_id: VaultId, amount: Amount) {
            assert!(!amount.is_zero());
            let bucket = self
                .token_vault_manager()
                .recall_fungible_amount(vault_id, amount);
            self.token_vault.deposit(bucket);

            emit_event(
                "recall_tokens",
                metadata!(
                    "vault_id" => vault_id,
                    "amt" => amount,
                ),
            );
        }

        pub fn burn_utxo(&mut self, utxo: UtxoId, value_proof: CommitmentValueProof) {
            self.assert_not_paused();
            self.token_vault_manager()
                .burn_utxo(utxo, Some(value_proof));
            emit_event(
                "burn_utxo",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key(),
                    "utxo_id" => utxo
                ),
            );
        }

        pub fn create_new_admin(&mut self, employee_id: String) -> Bucket {
            let id = NonFungibleId::random();
            emit_event("create_new_admin", metadata!("admin_id" => id));
            let mut metadata = Metadata::new();
            metadata.insert("employee_id", &employee_id);
            self.admin_auth_manager
                .mint_non_fungible(id, &metadata, &())
        }

        /// Recalls the admin badge `badge_id` from `vault_id` and burns it.
        ///
        /// A role rule naming `resource(admin_badge)` is satisfied by any admin badge, so revoking
        /// a lost badge removes its holder from every such role.
        pub fn revoke_admin(&mut self, vault_id: VaultId, badge_id: NonFungibleId) {
            let badge = self
                .admin_auth_manager
                .recall_non_fungible(vault_id, badge_id.clone());
            badge.burn();
            emit_event("revoke_admin", metadata!("admin_id" => badge_id));
        }

        /// Gives `role` to whoever satisfies `rule`, for a governor that the transaction's signers satisfy.
        pub fn set_role(&mut self, role: Role, rule: AccessRule) {
            self.apply_role(role, rule);
        }

        /// Gives `role` to whoever satisfies `rule`, for a governor that requires a badge.
        ///
        /// Changing the component's access rules needs the governor's authority inside the method
        /// body, where a proof is in scope only if it is passed as an argument.
        pub fn set_role_with_proof(
            &mut self,
            role: Role,
            rule: AccessRule,
            _governor_proof: Proof,
        ) {
            self.apply_role(role, rule);
        }

        pub fn set_config_transfer_fee_fixed(&mut self, new_fee: Amount) {
            emit_event(
                "config.set_transfer_fee_fixed",
                metadata!(
                    "prev" => self.config.transfer_fee,
                    "new" => FeeSpec::Fixed(new_fee),
                ),
            );
            self.config.transfer_fee = FeeSpec::Fixed(new_fee);
        }

        pub fn set_config_transfer_fee_percentage(&mut self, new_fee_perc: u8) {
            assert!(
                new_fee_perc <= 100,
                "Percentage fee must be between 0 and 100"
            );
            emit_event(
                "config.set_transfer_fee_percentage",
                metadata!(
                    "prev" => self.config.transfer_fee,
                    "new" => FeeSpec::Percentage(new_fee_perc)
                ),
            );
            self.config.transfer_fee = FeeSpec::Percentage(new_fee_perc);
        }

        pub fn pause(&mut self) {
            self.is_paused = true;
            emit_event(
                "admin.paused",
                metadata!("tx_signer" => CallerContext::transaction_signer_public_key()),
            );
        }

        pub fn unpause(&mut self) {
            self.is_paused = false;
            emit_event(
                "admin.unpaused",
                metadata!("tx_signer" => CallerContext::transaction_signer_public_key()),
            );
        }

        pub fn freeze_utxos(&self, utxos: Vec<UtxoId>) {
            emit_event(
                "admin.freeze_utxos",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key(),
                    "num_utxos" => utxos.len(),
                ),
            );
            self.token_vault_manager().freeze_utxos(utxos);
        }

        pub fn unfreeze_utxos(&self, utxos: Vec<UtxoId>) {
            emit_event(
                "admin.unfreeze_utxos",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key(),
                    "num_utxos" => utxos.len(),
                ),
            );
            self.token_vault_manager().unfreeze_utxos(utxos);
        }

        fn component_access_rules(roles: &Roles) -> AccessRules {
            METHOD_ROLES
                .iter()
                .fold(AccessRules::new(), |rules, (method, role)| {
                    rules.add_method_rule(*method, roles.get(*role).clone())
                })
                .default(roles.get(Role::Governor).clone())
        }

        fn apply_role(&mut self, role: Role, rule: AccessRule) {
            self.roles.set(role, rule.clone());
            let component = ComponentManager::current();
            component.set_access_rules(Self::component_access_rules(&self.roles));
            emit_event("set_role", metadata!("role" => role, "rule" => rule));
            // Ownership is checked against the current governor, so the owner rule changes last.
            if role == Role::Governor {
                component.set_owner_rule(SubstateOwnerRule::ByAccessRule(rule));
            }
        }

        fn token_vault_manager(&self) -> ResourceManager {
            self.token_vault.get_resource_manager()
        }

        fn assert_not_paused(&self) {
            assert!(!self.is_paused, "Stable coin is paused");
        }
    }
}
