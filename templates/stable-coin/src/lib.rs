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
    pub created_at: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserMutableData {
    pub is_blacklisted: bool,
}

#[template]
mod stable_coin {
    use super::*;

    pub struct TariStableCoin {
        token_vault: Vault,
        user_auth_resource: ResourceAddress,
        admin_auth_resource: ResourceAddress,
        blacklisted_users: Vault,
    }

    impl TariStableCoin {
        /// Instantiates a new stable coin component, returning the component and an bucket containing an admin badge
        pub fn instantiate(
            initial_token_supply: Amount,
            token_symbol: String,
            mut token_metadata: Metadata,
        ) -> (Component<Self>, Bucket) {
            let provider_name = token_metadata
                .get("provider_name")
                .expect("provider_name metadata entry is required");

            // Create admin badge resource
            let admin_badge = ResourceBuilder::non_fungible()
                .with_non_fungible(NonFungibleId::from_u64(0), &(), &())
                .build_bucket();

            // Create admin access rules
            let admin_resource = admin_badge.resource_address();
            let require_admin = AccessRule::Restricted(RestrictedAccessRule::Require(
                RequireRule::Require(admin_resource.into()),
            ));

            // Create user badge resource
            let user_auth_resource = ResourceBuilder::non_fungible()
                .add_metadata("provider_name", provider_name)
                .depositable(require_admin.clone())
                .update_non_fungible_data(require_admin.clone())
                .build();

            // Create user access rules
            let require_user =
                AccessRule::Restricted(RestrictedAccessRule::Require(RequireRule::AnyOf(vec![
                    admin_resource.into(),
                    user_auth_resource.into(),
                ])));

            // Create tokens resource with initial supply
            token_metadata.insert("provider", provider_name.clone());
            let initial_tokens = ResourceBuilder::fungible()
                .initial_supply(initial_token_supply)
                .with_token_symbol(token_symbol)
                .with_metadata(token_metadata)
                // Access rules
                .mintable(require_admin.clone())
                .burnable(require_admin.clone())
                // .recallable(require_admin.clone())
                .depositable(require_user.clone())
                .withdrawable(require_user.clone())
                .build_bucket();

            // Create component access rules
            let component_access_rules = AccessRules::new()
                .add_method_rule("total_supply", AccessRule::AllowAll)
                .default(require_admin);

            // Create component
            let component = Component::new(Self {
                token_vault: Vault::from_bucket(initial_tokens),
                user_auth_resource,
                admin_auth_resource: admin_badge.resource_address(),
                blacklisted_users: Vault::new_empty(user_auth_resource),
            })
            .with_access_rules(component_access_rules)
            // Access is entirely controlled by anyone with an admin badge
            .with_owner_rule(OwnerRule::None)
            .create();
            (component, admin_badge)
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, amount: Amount) {
            let new_tokens =
                ResourceManager::get(self.token_vault.resource_address()).mint_fungible(amount);
            self.token_vault.deposit(new_tokens);
            emit_event("increase_supply", [("amount", amount.to_string())]);
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, amount: Amount) {
            let tokens = self.token_vault.withdraw(amount);
            tokens.burn();
            emit_event("decrease_supply", [("amount", amount.to_string())]);
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.token_vault.resource_address()).total_supply()
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            let bucket = self.token_vault.withdraw(amount);
            emit_event("withdraw", [("amount", amount.to_string())]);
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", [("amount", amount.to_string())]);
        }

        pub fn create_new_admin(&mut self) -> Bucket {
            let badge = ResourceManager::get(self.admin_auth_resource).mint_non_fungible(
                NonFungibleId::random(),
                &(),
                &(),
            );
            emit_event("create_new_admin", [] as [(&str, String); 0]);
            badge
        }

        pub fn create_new_user(&mut self, user_id: UserId) -> Bucket {
            let badge = ResourceManager::get(self.user_auth_resource).mint_non_fungible(
                user_id.into(),
                &UserData {
                    user_id,
                    // TODO: real time not implemented
                    created_at: 0,
                },
                &UserMutableData {
                    is_blacklisted: false,
                },
            );
            emit_event("create_new_user", [("user_id", user_id.to_string())]);
            badge
        }

        // TODO: Implement recall
        // pub fn recall_user_tokens(&mut self, vault_id: VaultId) {
        //     let manager = ResourceManager::get(self.user_auth_resource);
        //     let recalled = manager.recall_all(vault_id);
        //     self.token_vault.deposit(recalled);
        //     emit_event("recall_user_tokens", [("vault_id", vault_id.to_string())]);
        // }
        //
        // pub fn blacklist_user(&mut self, vault_id: VaultId, user_id: UserId) {
        //     let non_fungible_id: NonFungibleId = user_id.into();
        //
        //     let manager = ResourceManager::get(self.user_auth_resource);
        //     let recalled = manager.recall_non_fungibles(vault_id, [non_fungible_id]);
        //     manager.update_non_fungible_data(
        //         non_fungible_id,
        //         &UserMutableData {
        //             is_blacklisted: true,
        //         },
        //     );
        //
        //     self.blacklisted_users.deposit(recalled);
        //     emit_event("blacklist_user", [("user_id", user_id.to_string())]);
        // }

        pub fn remove_from_blacklist(&mut self, user_id: UserId) -> Bucket {
            let non_fungible_id: NonFungibleId = user_id.into();
            let user_badge = self
                .blacklisted_users
                .withdraw_non_fungible(non_fungible_id.clone());
            ResourceManager::get(self.user_auth_resource).update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    is_blacklisted: false,
                },
            );
            emit_event("remove_from_blacklist", [("user_id", user_id.to_string())]);
            user_badge
        }

        pub fn get_user_data(&self, user_id: UserId) -> UserData {
            let badge =
                ResourceManager::get(self.user_auth_resource).get_non_fungible(&user_id.into());
            badge.get_data()
        }

        pub fn set_user_data(&mut self, user_id: UserId, data: UserMutableData) {
            ResourceManager::get(self.user_auth_resource)
                .update_non_fungible_data(user_id.into(), &data);
            emit_event("set_user_data", [("user_id", user_id.to_string())]);
        }
    }
}
