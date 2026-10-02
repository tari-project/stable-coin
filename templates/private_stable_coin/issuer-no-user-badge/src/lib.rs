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

mod config;
mod roles;
mod user_data;
mod wrapped_exchange_token;
use alloc::format;
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
    use crate::user_data::{UserData, UserId, UserMutableData};
    use tari_template_lib::component::ComponentManager;
    use tari_template_lib::types::SubstateOwnerRule;
    use tari_template_lib::types::crypto::CommitmentValueProof;

    use super::*;
    use crate::config::FeeSpec;
    use crate::roles::{Role, RoleConfig, Roles};
    use crate::{config::StableCoinConfig, wrapped_exchange_token::WrappedExchangeToken};

    /// The role that may call each component method. A method not listed here can only be called by the governor,
    /// which owns the component.
    const METHOD_ROLES: &[(&str, Role)] = &[
        ("increase_supply", Role::Minter),
        ("decrease_supply", Role::Burner),
        ("burn_utxo", Role::Burner),
        ("withdraw", Role::Treasurer),
        ("deposit", Role::Treasurer),
        ("exchange_stable_for_wrapped_tokens", Role::Treasurer),
        ("exchange_wrapped_for_stable_tokens", Role::Treasurer),
        ("recall_revealed_tokens", Role::Compliance),
        ("blacklist_user", Role::Compliance),
        ("remove_from_blacklist", Role::Compliance),
        ("freeze_utxos", Role::Compliance),
        ("unfreeze_utxos", Role::Compliance),
        ("create_new_user", Role::UserManager),
        ("set_user_exchange_limit", Role::UserManager),
        ("set_user_wrapped_exchange_limit", Role::UserManager),
        ("pause", Role::Pauser),
    ];

    pub struct TariStableCoin {
        config: StableCoinConfig,
        token_vault: Vault,
        user_auth_manager: ResourceManager,
        admin_auth_manager: ResourceManager,
        blacklisted_users: Vault,
        wrapped_token: Option<WrappedExchangeToken>,
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
            enable_wrapped_token: bool,
            roles: Option<RoleConfig>,
        ) -> Bucket {
            let provider_name = token_metadata.get_str("provider_name").unwrap_or_default();

            let config = StableCoinConfig::default();

            // Privileged resource actions are reserved for this component. A proof handed to a
            // component at a call boundary is revoked once the callee's own access rule has been
            // checked, so a workspace badge proof is not in scope inside a method body and cannot satisfy
            // a `resource(..)` rule there. Role checks happen at the component method boundary.
            let component_address = address_alloc.get_address();
            let require_component = rule!(component(component_address));

            // Admin badges are issued and revoked only through the component, so no key or badge
            // can mint one directly and a lost badge can be recalled and burnt by the governor.
            let admin_badge = ResourceBuilder::non_fungible()
                .with_metadata(metadata!(
                    "name" => "Stable Coin Admin Badge",
                    "provider_name" => provider_name,
                    "description" => format!("Admin badge for the {provider_name} stable coin"),
                    "admin_badge" => "true",
                ))
                .mintable(require_component.clone(), LOCKED)
                .burnable(require_component.clone(), LOCKED)
                .recallable(require_component.clone(), LOCKED)
                .update_non_fungible_data(rule!(deny_all), LOCKED)
                .with_owner_rule(OwnerRule::None)
                .initial_supply(Some(NonFungibleId::from_u64(0)));

            let admin_resource = admin_badge.resource_address();
            let roles =
                Roles::from_config(roles.unwrap_or_default(), rule!(resource(admin_resource)));

            // Create user badge resource
            let user_auth_resource = ResourceBuilder::non_fungible()
                .with_metadata(metadata!(
                    "name" => "Stable Coin User Badge",
                    "provider_name" => provider_name,
                    "description" => format!("User authentication badge for the {provider_name} stable coin")
                ))
                .mintable(require_component.clone(), LOCKED)
                // A deposit is authorized inside the recipient account's call frame, where no
                // proof is in scope, so a badge-gated deposit rule could never be satisfied.
                // Restricting who may hold a badge would need an authorization hook (see the
                // `issuer` variant); here the badge is a registry entry rather than an access
                // requirement, so deposits are open and the issuer controls badges through recall.
                .depositable(rule!(allow_all), LOCKED)
                .recallable(require_component.clone(), LOCKED)
                .update_non_fungible_data(require_component.clone(), LOCKED)
                .with_owner_rule(OwnerRule::None)
                .build();

            // Create tokens resource with initial supply
            let initial_tokens = ResourceBuilder::stealth()
                .with_metadata(token_metadata.clone())
                .with_token_symbol(token_symbol.as_ref())
                // Access rules
                .mintable(require_component.clone(), LOCKED)
                .burnable(require_component.clone(), LOCKED)
                .recallable(require_component.clone(), LOCKED)
                .freezable(require_component.clone(), LOCKED)
                .with_view_key(view_key)
                .with_divisibility(divisibility)
                .with_owner_rule(OwnerRule::None)
                .initial_supply(initial_token_supply);

            let wrapped_token = if enable_wrapped_token {
                let wrapped_resource = ResourceBuilder::public_fungible()
                    .with_metadata(token_metadata)
                    .with_token_symbol(format!("w{token_symbol}"))
                    // Access rules
                    .mintable(require_component.clone(), LOCKED)
                    .burnable(require_component, LOCKED)
                    .with_owner_rule(OwnerRule::None)
                    .build();

                Some(WrappedExchangeToken::new(wrapped_resource))
            } else {
                None
            };

            let component_access_rules = Self::component_access_rules(&roles);
            let governor = roles.get(Role::Governor).clone();

            Component::new(Self {
                config,
                token_vault: Vault::from_bucket(initial_tokens),
                user_auth_manager: user_auth_resource.into(),
                admin_auth_manager: admin_resource.into(),
                blacklisted_users: Vault::new_empty(user_auth_resource),
                wrapped_token,
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
            assert!(amount.is_positive(), "Amount must be positive");
            let new_tokens = self.token_vault_manager().mint_stealth(amount);
            self.token_vault.deposit(new_tokens);

            emit_event("increase_supply", metadata!("amount" => amount));
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, amount: Amount) {
            self.assert_not_paused();
            assert!(amount.is_positive(), "Amount must be positive");
            let tokens = self.token_vault.withdraw(amount);
            tokens.burn();

            emit_event(
                "decrease_supply",
                metadata!("revealed_burn_amount" => amount),
            );
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            self.assert_not_paused();
            assert!(amount.is_positive(), "Amount must be positive");
            let bucket = self.token_vault.withdraw(amount);
            emit_event("withdraw", metadata!("amount_withdrawn" => bucket.amount()));
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            self.assert_not_paused();
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", metadata!("amount" => amount));
        }

        /// Allow the user to exchange their tokens for wrapped tokens
        pub fn exchange_stable_for_wrapped_tokens(
            &mut self,
            proof: Proof,
            mut bucket: Bucket,
        ) -> Bucket {
            self.assert_not_paused();
            assert_eq!(
                bucket.resource_address(),
                self.token_vault.resource_address(),
                "The bucket must contain the same resource as the token vault"
            );

            assert!(
                bucket.amount().is_positive(),
                "The bucket must contain some tokens"
            );

            proof.assert_resource(self.user_auth_manager.resource_address());
            let badges = proof.get_non_fungibles();
            assert_eq!(badges.len(), 1, "The proof must contain exactly one badge");
            let badge = badges.into_iter().next().unwrap();
            let badge = self.user_auth_manager.get_non_fungible(&badge);
            let user = badge.get_data::<UserData>();
            let user_data = badge.get_mutable_data::<UserMutableData>();

            let amount = bucket.amount();
            assert!(
                amount <= user_data.wrapped_exchange_limit,
                "Exchange limit exceeded"
            );

            self.set_user_wrapped_exchange_limit(
                user.user_id,
                user_data.wrapped_exchange_limit - amount,
            );

            let fee = self.config.wrapped_exchange_fee.calculate_fee(amount);
            let new_amount = amount
                .checked_sub(fee)
                .expect("Insufficient funds to pay exchange fee");
            let fee_bucket = bucket.take(fee);
            bucket.burn();

            self.token_vault.deposit(fee_bucket);

            let wrapped_tokens = self.wrapped_token().manager().mint_fungible(new_amount);

            emit_event(
                "exchange_stable_for_wrapped_tokens",
                metadata!(
                    "user_id" => user.user_id,
                    "amount" => amount,
                    "fee" => fee,
                ),
            );

            wrapped_tokens
        }

        /// Allow the user to exchange their wrapped tokens for stable coin tokens
        pub fn exchange_wrapped_for_stable_tokens(
            &mut self,
            proof: Proof,
            wrapped_bucket: Bucket,
        ) -> Bucket {
            self.assert_not_paused();
            assert!(
                !wrapped_bucket.amount().is_zero(),
                "The bucket must contain some tokens"
            );

            proof.assert_resource(self.user_auth_manager.resource_address());

            assert_eq!(
                wrapped_bucket.resource_address(),
                self.wrapped_token().resource_address(),
                "The bucket must contain the same resource as the wrapped token vault"
            );

            let badges = proof.get_non_fungibles();
            assert_eq!(badges.len(), 1, "The proof must contain exactly one badge");
            let badge = badges.into_iter().next().unwrap();
            let badge = self.user_auth_manager.get_non_fungible(&badge);
            let user = badge.get_data::<UserData>();

            let amount = wrapped_bucket.amount();

            // Burn the wrapped tokens
            wrapped_bucket.burn();

            // Mint tokens
            let tokens = self.token_vault.get_resource_manager().mint_stealth(amount);

            emit_event(
                "exchange_wrapped_for_stable_tokens",
                metadata!(
                        "user_id" => user.user_id,
                        "amount" => amount,
                        "fee" => 0u64,
                ),
            );

            tokens
        }

        pub fn recall_revealed_tokens(&mut self, user_id: UserId, amount: Amount) {
            assert!(amount.is_positive(), "Amount must be positive");
            // Fetch the user badge
            let badge = self.user_auth_manager.get_non_fungible(&user_id.into());
            let user = badge.get_data::<UserData>();

            let account = user.user_account.get_state::<Account>();

            let vault = account
                .get_vault_by_resource(&self.token_vault.resource_address())
                .expect("The user's account does not have a vault for the stable coin resource");
            let vault_id = vault.vault_id();

            let bucket = self
                .token_vault_manager()
                .recall_fungible_amount(vault_id, amount);
            self.token_vault.deposit(bucket);

            emit_event(
                "recall_tokens",
                metadata!(
                        "user_id" => user_id,
                        "revealed_amount" => amount,
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

        pub fn create_new_user(
            &mut self,
            user_id: UserId,
            user_account: ComponentAddress,
        ) -> Bucket {
            self.assert_not_paused();
            let epoch = Consensus::current_epoch();
            let badge = self.user_auth_manager.mint_non_fungible(
                user_id.into(),
                &UserData {
                    user_id,
                    user_account: ComponentManager::get(user_account),
                    created_at_epoch: epoch,
                },
                &UserMutableData {
                    is_blacklisted: false,
                    wrapped_exchange_limit: self.config.default_exchange_limit,
                },
            );
            emit_event("create_new_user", metadata!("user_id" => user_id));
            badge
        }

        pub fn set_user_exchange_limit(&mut self, user_id: UserId, limit: Amount) {
            assert!(limit.is_positive(), "Exchange limit must be positive");
            let non_fungible_id: NonFungibleId = user_id.into();

            let user_badge = self.user_auth_manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            self.user_auth_manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    wrapped_exchange_limit: limit,
                    ..user_data
                },
            );

            let admin = CallerContext::transaction_signer_public_key();
            emit_event(
                "set_user_exchange_limit",
                metadata!(
                        "user_id" => user_id,
                        "limit" => limit,
                        "admin" => admin,
                ),
            );
        }

        pub fn blacklist_user(&mut self, vault_id: VaultId, user_id: UserId) {
            let non_fungible_id: NonFungibleId = user_id.into();

            let recalled = self
                .user_auth_manager
                .recall_non_fungible(vault_id, non_fungible_id.clone());
            let user_badge = self.user_auth_manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            self.user_auth_manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    is_blacklisted: true,
                    ..user_data
                },
            );

            self.blacklisted_users.deposit(recalled);
            emit_event("blacklist_user", metadata!("user_id" => user_id));
        }

        pub fn remove_from_blacklist(&mut self, user_id: UserId) -> Bucket {
            let non_fungible_id: NonFungibleId = user_id.into();
            let user_badge_bucket = self
                .blacklisted_users
                .withdraw_non_fungible(non_fungible_id.clone());
            let user_badge = self.user_auth_manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            self.user_auth_manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    is_blacklisted: false,
                    ..user_data
                },
            );
            emit_event("remove_from_blacklist", metadata!("user_id" => user_id));
            user_badge_bucket
        }

        pub fn set_user_wrapped_exchange_limit(&mut self, user_id: UserId, new_limit: Amount) {
            let mut badge = self.user_auth_manager.get_non_fungible(&user_id.into());
            let mut user_data = badge.get_mutable_data::<UserMutableData>();
            user_data.set_wrapped_exchange_limit(new_limit);
            badge.set_mutable_data(&user_data);
            emit_event(
                "set_user_wrapped_exchange_limit",
                metadata!(
                    "user_id" => user_id,
                    "limit" => new_limit,
                ),
            );
        }

        pub fn set_config_transfer_fee_fixed(&mut self, new_fee: Amount) {
            emit_event(
                "config.set_transfer_fee_fixed",
                metadata!(
                    "old_transfer_fee" => self.config.transfer_fee,
                    "new_transfer_fee" => FeeSpec::Fixed(new_fee),
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
                        "old_transfer_fee" => self.config.transfer_fee,
                        "new_transfer_fee" => FeeSpec::Percentage(new_fee_perc),
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

        fn assert_not_paused(&self) {
            assert!(!self.is_paused, "Component is paused");
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

        fn token_vault_manager(&self) -> ResourceManager {
            self.token_vault.get_resource_manager()
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

        fn wrapped_token(&self) -> &WrappedExchangeToken {
            self.wrapped_token
                .as_ref()
                .expect("Wrapped token is not enabled")
        }
    }
}
