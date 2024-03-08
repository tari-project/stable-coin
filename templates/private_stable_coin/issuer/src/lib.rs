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

use std::fmt::Display;
use tari_template_lib::prelude::*;

#[derive(Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct UserId(u64);

impl From<UserId> for NonFungibleId {
    fn from(value: UserId) -> Self {
        Self::from_u64(value.0)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0>19}", self.0)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserData {
    pub user_id: UserId,
    pub user_account: ComponentAddress,
    pub created_at: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserMutableData {
    pub is_blacklisted: bool,
    pub wrapped_exchange_limit: Amount,
}

#[template]
mod template {
    use super::*;

    pub struct TariStableCoin {
        token_vault: Vault,
        user_auth_resource: ResourceAddress,
        admin_auth_resource: ResourceAddress,
        blacklisted_users: Vault,
        wrapped_vault: Option<Vault>,
    }

    impl TariStableCoin {
        /// Instantiates a new stable coin component, returning the component and an bucket containing an admin badge
        pub fn instantiate(
            initial_token_supply: Amount,
            token_symbol: String,
            token_metadata: Metadata,
            enable_wrapped_token: bool,
        ) -> Bucket {
            let provider_name = token_metadata
                .get("provider_name")
                .expect("provider_name metadata entry is required");

            // Create admin badge resource
            let admin_badge = ResourceBuilder::non_fungible()
                .with_non_fungible(NonFungibleId::from_u64(0), &(), &())
                .build_bucket();

            // Create admin access rules
            let admin_resource = admin_badge.resource_address();
            let require_admin =
                AccessRule::Restricted(Require(RequireRule::Require(admin_resource.into())));

            // Create user badge resource
            let user_auth_resource = ResourceBuilder::non_fungible()
                .add_metadata("provider_name", provider_name)
                .depositable(require_admin.clone())
                .recallable(require_admin.clone())
                .update_non_fungible_data(require_admin.clone())
                .build();

            // Create user access rules
            let require_user = AccessRule::Restricted(Require(RequireRule::AnyOf(vec![
                admin_resource.into(),
                user_auth_resource.into(),
            ])));

            // Create tokens resource with initial supply
            let initial_supply_proof = ConfidentialOutputProof::mint_revealed(initial_token_supply);
            let initial_tokens = ResourceBuilder::confidential()
                .initial_supply(initial_supply_proof)
                .with_token_symbol(&token_symbol)
                .with_metadata(token_metadata.clone())
                // Access rules
                .mintable(require_admin.clone())
                .burnable(require_admin.clone())
                .depositable(require_user.clone())
                .withdrawable(require_user.clone())
                .recallable(require_admin.clone())
                .build_bucket();

            // Create tokens resource with initial supply
            let initial_wrapped_tokens = if enable_wrapped_token {
                Some(
                    ResourceBuilder::fungible()
                        .initial_supply(initial_token_supply)
                        .with_token_symbol(token_symbol)
                        .with_metadata(token_metadata)
                        // Access rules
                        .mintable(require_admin.clone())
                        .burnable(require_admin.clone())
                        .build_bucket(),
                )
            } else {
                None
            };

            // Create component access rules
            let component_access_rules = AccessRules::new()
                .add_method_rule("total_supply", AccessRule::AllowAll)
                .default(require_admin);

            // Create component
            let _component = Component::new(Self {
                token_vault: Vault::from_bucket(initial_tokens),
                user_auth_resource,
                admin_auth_resource: admin_badge.resource_address(),
                blacklisted_users: Vault::new_empty(user_auth_resource),
                wrapped_vault: initial_wrapped_tokens.map(Vault::from_bucket),
            })
            .with_access_rules(component_access_rules)
            // Access is entirely controlled by anyone with an admin badge
            .with_owner_rule(OwnerRule::None)
            .create();

            admin_badge
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, amount: Amount) {
            let proof = ConfidentialOutputProof::mint_revealed(amount);
            let new_tokens =
                ResourceManager::get(self.token_vault.resource_address()).mint_confidential(proof);
            self.token_vault.deposit(new_tokens);

            if let Some(ref mut wrapped_vault) = self.wrapped_vault {
                let new_tokens =
                    ResourceManager::get(wrapped_vault.resource_address()).mint_fungible(amount);
                wrapped_vault.deposit(new_tokens);
            }

            emit_event("increase_supply", [("amount", amount.to_string())]);
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, amount: Amount) {
            let proof = ConfidentialWithdrawProof::revealed_withdraw(amount);

            let tokens = self.token_vault.withdraw_confidential(proof);
            tokens.burn();

            if let Some(ref mut wrapped_vault) = self.wrapped_vault {
                let wrapped_tokens = wrapped_vault.withdraw(amount);
                wrapped_tokens.burn();
            }

            emit_event(
                "decrease_supply",
                [("revealed_burn_amount", amount.to_string())],
            );
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.token_vault.resource_address()).total_supply()
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            let proof = ConfidentialWithdrawProof::revealed_withdraw(amount);
            let bucket = self.token_vault.withdraw_confidential(proof);
            emit_event(
                "withdraw",
                [("amount_withdrawn", bucket.amount().to_string())],
            );
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", [("amount", amount.to_string())]);
        }

        /// Allow the user to exchange their tokens for wrapped tokens
        pub fn exchange_for_wrapped_tokens(
            &mut self,
            proof: Proof,
            confidential_bucket: Bucket,
        ) -> Bucket {
            if self.wrapped_vault.is_none() {
                panic!("Wrapped token is not enabled");
            }

            assert_eq!(
                confidential_bucket.resource_address(),
                self.token_vault.resource_address(),
                "The bucket must contain the same resource as the token vault"
            );

            // Check the bucket does not contain any non-revealed confidential tokens
            assert_eq!(
                confidential_bucket.count_confidential_commitments(),
                0,
                "No confidential outputs allowed when exchanging for wrapped tokens"
            );

            assert!(
                !confidential_bucket.amount().is_zero(),
                "The bucket must contain some tokens"
            );

            proof.assert_resource(self.user_auth_resource);
            let badges = proof.get_non_fungibles();
            assert_eq!(badges.len(), 1, "The proof must contain exactly one badge");
            let badge = badges.into_iter().next().unwrap();
            let badge = self.user_badge_manager().get_non_fungible(&badge);
            let user_data = badge.get_mutable_data::<UserMutableData>();

            let amount = confidential_bucket.amount();
            assert!(
                amount <= user_data.wrapped_exchange_limit,
                "Exchange limit exceeded"
            );

            let wrapped_vault_mut = self.wrapped_vault.as_mut().unwrap();
            let wrapped_tokens = wrapped_vault_mut.withdraw(amount);

            confidential_bucket.burn();

            wrapped_tokens
        }

        pub fn create_new_admin(&mut self, employee_id: String) -> Bucket {
            let id = NonFungibleId::random();
            emit_event("create_new_admin", [("admin_id", id.to_string())]);
            let mut metadata = Metadata::new();
            metadata.insert("employee_id", employee_id);
            let badge = ResourceManager::get(self.admin_auth_resource).mint_non_fungible(
                id,
                &metadata,
                &(),
            );
            badge
        }

        pub fn create_new_user(
            &mut self,
            user_id: UserId,
            user_account: ComponentAddress,
        ) -> Bucket {
            // TODO: configurable?
            const DEFAULT_EXCHANGE_LIMIT: Amount = Amount::new(1_000);

            let badge = self.user_badge_manager().mint_non_fungible(
                user_id.into(),
                &UserData {
                    user_id,
                    user_account,
                    // TODO: real time not implemented
                    created_at: 0,
                },
                &UserMutableData {
                    is_blacklisted: false,
                    wrapped_exchange_limit: DEFAULT_EXCHANGE_LIMIT,
                },
            );
            emit_event("create_new_user", [("user_id", user_id.to_string())]);
            badge
        }

        pub fn set_user_exchange_limit(&mut self, user_id: UserId, limit: Amount) {
            assert!(limit.is_positive(), "Exchange limit must be positive");
            let non_fungible_id: NonFungibleId = user_id.into();

            let manager = self.user_badge_manager();
            let user_badge = manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    wrapped_exchange_limit: limit,
                    ..user_data
                },
            );

            let admin = CallerContext::transaction_signer_public_key();
            emit_event(
                "set_user_exchange_limit",
                [
                    ("user_id", user_id.to_string()),
                    ("limit", limit.to_string()),
                    ("admin", admin.to_string()),
                ],
            );
        }

        pub fn blacklist_user(&mut self, vault_id: VaultId, user_id: UserId) {
            let non_fungible_id: NonFungibleId = user_id.into();

            let manager = self.user_badge_manager();
            let recalled = manager.recall_non_fungible(vault_id, non_fungible_id.clone());
            let user_badge = manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    is_blacklisted: true,
                    ..user_data
                },
            );

            self.blacklisted_users.deposit(recalled);
            emit_event("blacklist_user", [("user_id", user_id.to_string())]);
        }

        pub fn remove_from_blacklist(&mut self, user_id: UserId) -> Bucket {
            let non_fungible_id: NonFungibleId = user_id.into();
            let user_badge_bucket = self
                .blacklisted_users
                .withdraw_non_fungible(non_fungible_id.clone());
            let manager = self.user_badge_manager();
            let user_badge = manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            manager.update_non_fungible_data(
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
            let badge = self.user_badge_manager().get_non_fungible(&user_id.into());
            badge.get_data()
        }

        pub fn set_user_data(&mut self, user_id: UserId, data: UserMutableData) {
            self.user_badge_manager()
                .update_non_fungible_data(user_id.into(), &data);
            emit_event("set_user_data", [("user_id", user_id.to_string())]);
        }

        fn user_badge_manager(&self) -> ResourceManager {
            ResourceManager::get(self.user_auth_resource)
        }
    }
}
