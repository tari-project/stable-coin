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

mod config;
mod user_data;
mod wrapped_exchange_token;

use user_data::{UserData, UserId, UserMutableData};

use tari_template_lib::prelude::*;

#[template]
mod template {
    use super::*;
    use crate::config::FeeSpec;
    use crate::{config::StableCoinConfig, wrapped_exchange_token::WrappedExchangeToken};
    use tari_template_lib::engine;
    use tari_template_lib::prelude::crypto::StealthValueProof;

    pub struct TariStableCoin {
        config: StableCoinConfig,
        token_vault: Vault,
        user_auth_manager: ResourceManager,
        admin_auth_manager: ResourceManager,
        blacklisted_users: Vault,
        wrapped_token: Option<WrappedExchangeToken>,
        is_paused: bool,
    }

    impl TariStableCoin {
        /// Instantiates a new stable coin component, returning the component and an bucket containing an admin badge
        pub fn instantiate(
            initial_token_supply: Amount,
            token_symbol: String,
            token_metadata: Metadata,
            view_key: RistrettoPublicKeyBytes,
            enable_wrapped_token: bool,
        ) -> Bucket {
            let provider_name = token_metadata
                .get("provider_name")
                .filter(|v| !v.trim().is_empty())
                .expect("provider_name metadata entry is required");

            let config = StableCoinConfig::default();

            // Create admin badge resource
            let admin_badge =
                ResourceBuilder::non_fungible().initial_supply(Some(NonFungibleId::from_u64(0)));

            // Create admin access rules
            let admin_resource = admin_badge.resource_address();
            let require_admin = rule!(resource(admin_resource));

            // Create user badge resource
            let user_auth_resource = ResourceBuilder::non_fungible()
                .add_metadata("provider_name", provider_name.trim())
                .depositable(require_admin.clone())
                .recallable(require_admin.clone())
                .update_non_fungible_data(require_admin.clone())
                .build();

            // Create user access rules
            let require_user_or_admin = rule!(any_of(
                resource(admin_resource),
                resource(user_auth_resource)
            ));

            let component_alloc = CallerContext::allocate_component_address(None);
            // Create tokens resource with initial supply
            let initial_tokens = ResourceBuilder::stealth()
                .with_metadata(token_metadata.clone())
                .with_token_symbol(&token_symbol)
                // Access rules
                .mintable(require_admin.clone())
                .burnable(require_admin.clone())
                .depositable(require_user_or_admin.clone())
                .withdrawable(require_user_or_admin.clone())
                .recallable(require_admin.clone())
                .with_authorization_hook(component_alloc.get_address(), "authorize_user_deposit")
                .with_view_key(view_key)
                .initial_supply(initial_token_supply);

            // Create tokens resource with initial supply
            let wrapped_token = if enable_wrapped_token {
                let wrapped_resource = ResourceBuilder::fungible()
                    .with_metadata(token_metadata)
                    .with_token_symbol(format!("w{token_symbol}"))
                    // Access rules
                    .mintable(require_admin.clone())
                    .burnable(require_admin.clone())
                    .initial_supply(initial_token_supply);

                Some(WrappedExchangeToken {
                    vault: Vault::from_bucket(wrapped_resource),
                })
            } else {
                None
            };

            // Create component access rules
            let component_access_rules = AccessRules::new()
                .add_method_rule("total_supply", AccessRule::AllowAll)
                .add_method_rule(
                    "exchange_stable_for_wrapped_tokens",
                    require_user_or_admin.clone(),
                )
                .add_method_rule(
                    "exchange_wrapped_for_stable_tokens",
                    require_user_or_admin.clone(),
                )
                // authorize_user_deposit is an auth hook, so needs to be callable by any user/admin (TODO: currently needs allow_all)
                .add_method_rule("authorize_user_deposit", rule!(allow_all))
                .default(require_admin);

            // Create component
            let _component = Component::new(Self {
                config,
                token_vault: Vault::from_bucket(initial_tokens),
                user_auth_manager: user_auth_resource.into(),
                admin_auth_manager: admin_badge.resource_address().into(),
                blacklisted_users: Vault::new_empty(user_auth_resource),
                wrapped_token,
                is_paused: false,
            })
            .with_address_allocation(component_alloc)
            .with_access_rules(component_access_rules)
            // Access is controlled by anyone with an admin badge
            .with_owner_rule(OwnerRule::ByAccessRule(rule!(resource(admin_resource))))
            .create();

            admin_badge
        }

        pub fn authorize_user_deposit(&self, action: ResourceAuthAction, caller: AuthHookCaller) {
            match action {
                // Non-stealth deposits
                ResourceAuthAction::Deposit => {
                    if self.is_paused {
                        panic!("Token is paused");
                    }
                    let Some(component_state) = caller.component_state() else {
                        panic!("deposit not permitted from static template function")
                    };
                    info!(
                        "Authorizing deposit for user with component {}",
                        caller.component().unwrap()
                    );
                    let user_account = Account::from_value(component_state)
                        .expect("Deposit must be to an account");
                    let vault = user_account
                        .get_vault_by_resource(&self.user_auth_manager.resource_address())
                        .expect("Caller account does not have a vault for the resource");

                    // User must own a badge of this user auth resource. The badge may be locked when sending to self.
                    if vault.balance().is_zero() && vault.locked_balance().is_zero() {
                        panic!("This account does not have permission to deposit");
                    }
                }
                _ => {
                    // Withdraws etc are permitted as per normal resource access rules
                }
            }
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, amount: Amount) {
            let new_tokens = self.token_vault_manager().mint_stealth(amount);
            self.token_vault.deposit(new_tokens);

            if let Some(ref mut wrapped_token) = self.wrapped_token {
                let new_tokens =
                    ResourceManager::get(wrapped_token.resource_address()).mint_fungible(amount);
                wrapped_token.vault_mut().deposit(new_tokens);
            }

            emit_event("increase_supply", metadata!("amount" => amount.to_string()));
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, amount: Amount) {
            let tokens = self.token_vault.withdraw(amount);
            tokens.burn();

            if let Some(ref mut wrapped_token) = self.wrapped_token {
                let wrapped_tokens = wrapped_token.vault_mut().withdraw(amount);
                wrapped_tokens.burn();
            }

            emit_event(
                "decrease_supply",
                metadata!("revealed_burn_amount" => amount.to_string()),
            );
        }

        pub fn total_supply(&self) -> Amount {
            self.token_vault_manager().total_supply()
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            let bucket = self.token_vault.withdraw(amount);
            emit_event(
                "withdraw",
                metadata!("amount_withdrawn" => bucket.amount().to_string()),
            );
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", metadata!("amount" => amount.to_string()));
        }

        /// Allow the user to exchange their tokens for wrapped tokens
        pub fn exchange_stable_for_wrapped_tokens(
            &mut self,
            proof: Proof,
            bucket: Bucket,
        ) -> Bucket {
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

            self.token_vault.deposit(bucket);

            let wrapped_tokens = self.wrapped_token_mut().vault_mut().withdraw(new_amount);

            emit_event(
                "exchange_stable_for_wrapped_tokens",
                metadata!(
                        "user_id" => user.user_id.to_string(),
                        "amount" => amount.to_string(),
                        "fee" => fee.to_string(),
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
            proof.assert_resource(self.user_auth_manager.resource_address());

            assert_eq!(
                wrapped_bucket.resource_address(),
                self.wrapped_token_mut().vault().resource_address(),
                "The bucket must contain the same resource as the wrapped token vault"
            );

            assert!(
                !wrapped_bucket.amount().is_zero(),
                "The bucket must contain some tokens"
            );

            let badges = proof.get_non_fungibles();
            assert_eq!(badges.len(), 1, "The proof must contain exactly one badge");
            let badge = badges.into_iter().next().unwrap();
            let badge = self.user_auth_manager.get_non_fungible(&badge);
            let user = badge.get_data::<UserData>();

            let amount = wrapped_bucket.amount();

            self.wrapped_token_mut().vault_mut().deposit(wrapped_bucket);

            let tokens = self.token_vault.withdraw(amount);

            emit_event(
                "exchange_wrapped_for_stable_tokens",
                metadata!(
                        "user_id" => user.user_id.to_string(),
                        "amount" => amount.to_string(),
                        "fee" => 0.to_string(),
                ),
            );

            tokens
        }

        pub fn recall_revealed_tokens(&mut self, user_id: UserId, amount: Amount) {
            // Fetch the user badge
            let badge = self.user_auth_manager.get_non_fungible(&user_id.into());
            let user = badge.get_data::<UserData>();

            let component_manager = engine().component_manager(user.user_account);
            let account = component_manager.get_state::<Account>();

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
                        "user_id" => user_id.to_string(),
                        "revealed_amount" => amount.to_string(),
                ),
            );
        }

        pub fn burn_utxos(&mut self, utxo: UtxoId, value_proof: StealthValueProof) {
            self.token_vault_manager()
                .burn_utxo(utxo, Some(value_proof));
            emit_event(
                "burn_utxos",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key().to_string(),
                    "utxo_id" => utxo.to_string()
                ),
            );
        }

        pub fn create_new_admin(&mut self, employee_id: String) -> Bucket {
            let id = NonFungibleId::random();
            emit_event("create_new_admin", metadata!("admin_id" => id.to_string()));
            let mut metadata = Metadata::new();
            metadata.insert("employee_id", employee_id);
            let badge = self
                .admin_auth_manager
                .mint_non_fungible(id, &metadata, &());
            badge
        }

        pub fn create_new_user(
            &mut self,
            user_id: UserId,
            user_account: ComponentAddress,
        ) -> Bucket {
            let epoch = Consensus::current_epoch();
            let badge = self.user_auth_manager.mint_non_fungible(
                user_id.into(),
                &UserData {
                    user_id,
                    user_account,
                    created_at_epoch: epoch,
                },
                &UserMutableData {
                    is_blacklisted: false,
                    wrapped_exchange_limit: self.config.default_exchange_limit,
                },
            );
            emit_event(
                "create_new_user",
                metadata!("user_id" => user_id.to_string()),
            );
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
                        "user_id" => user_id.to_string(),
                        "limit" => limit.to_string(),
                        "admin" => admin.to_string(),
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
            emit_event(
                "blacklist_user",
                metadata!("user_id" => user_id.to_string()),
            );
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
            emit_event("remove_from_blacklist", [("user_id", user_id.to_string())]);
            user_badge_bucket
        }

        pub fn get_user_data(&self, user_id: UserId) -> UserData {
            let badge = self.user_auth_manager.get_non_fungible(&user_id.into());
            badge.get_data()
        }

        pub fn set_user_wrapped_exchange_limit(&mut self, user_id: UserId, new_limit: Amount) {
            let mut badge = self.user_auth_manager.get_non_fungible(&user_id.into());
            let mut user_data = badge.get_mutable_data::<UserMutableData>();
            user_data.set_wrapped_exchange_limit(new_limit);
            badge.set_mutable_data(&user_data);
            emit_event(
                "set_user_wrapped_exchange_limit",
                [
                    ("user_id", user_id.to_string()),
                    ("limit", new_limit.to_string()),
                ],
            );
        }

        pub fn set_config_transfer_fee_fixed(&mut self, new_fee: Amount) {
            emit_event(
                "config.set_transfer_fee_fixed",
                metadata!(
                    "old_transfer_fee" => self.config.transfer_fee.to_string(),
                    "new_transfer_fee" => new_fee.to_string(),
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
                        "old_transfer_fee" => self.config.transfer_fee.to_string(),
                        "new_transfer_fee" => format!("{new_fee_perc}%"),
                ),
            );
            self.config.transfer_fee = FeeSpec::Percentage(new_fee_perc);
        }

        pub fn pause(&mut self, proof: Proof) {
            proof.assert_resource(self.admin_auth_manager.resource_address());
            // Could also add an check for a specific admin badge ID if desired
            let badge = proof
                .get_non_fungibles()
                .first()
                .expect("Proof must contain an admin badge")
                .to_string();
            self.is_paused = true;
            emit_event(
                "admin.paused",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key().to_string(),
                    "admin_badge" => badge
                ),
            );
        }

        pub fn freeze_utxos(&self, utxos: Vec<UtxoId>) {
            emit_event(
                "admin.freeze_utxos",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key().to_string(),
                    "num_utxos" => utxos.len().to_string(),
                ),
            );
            self.token_vault_manager().freeze_utxos(utxos);
        }

        pub fn unfreeze_utxos(&self, utxos: Vec<UtxoId>) {
            emit_event(
                "admin.unfreeze_utxos",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key().to_string(),
                    "num_utxos" => utxos.len().to_string(),
                ),
            );
            self.token_vault_manager().unfreeze_utxos(utxos);
        }

        fn token_vault_manager(&self) -> ResourceManager {
            ResourceManager::get(self.token_vault.resource_address())
        }

        fn wrapped_token_mut(&mut self) -> &mut WrappedExchangeToken {
            self.wrapped_token
                .as_mut()
                .expect("Wrapped token is not enabled")
        }
    }
}
