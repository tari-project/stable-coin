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
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
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
    use tari_template_lib::types::crypto::StealthValueProof;

    use super::*;
    use crate::config::FeeSpec;
    use crate::config::StableCoinConfig;

    pub struct TariStableCoin {
        config: StableCoinConfig,
        token_vault: Vault,
        admin_auth_manager: ResourceManager,
        is_paused: bool,
    }

    impl TariStableCoin {
        /// Instantiates a new stable coin component, returning a bucket containing an admin badge
        pub fn instantiate(
            address_alloc: ComponentAddressAllocation,
            initial_token_supply: Amount,
            token_symbol: MaxString<8>,
            token_metadata: Metadata,
            divisibility: u8,
            view_key: RistrettoPublicKeyBytes,
            config: Option<StableCoinConfig>,
        ) -> Bucket {
            let config = config.unwrap_or_default();
            // Create admin badge resource
            let admin_badge = ResourceBuilder::non_fungible()
                .with_metadata(metadata!("name" => "Admin"))
                .with_token_symbol("ADM")
                .recallable(rule![deny_all], OWNER)
                .initial_supply(Some(NonFungibleId::from_u64(0)));

            // Create admin access rules
            let admin_resource = admin_badge.resource_address();
            let require_admin = rule!(resource(admin_resource));
            // Allow other admins to recall an admin badge
            ResourceManager::get(admin_resource)
                .update_access_rule(ResourceAuthAction::Recall, require_admin.clone());

            // Create tokens resource with initial supply
            let initial_tokens = ResourceBuilder::stealth()
                .with_metadata(token_metadata)
                .with_token_symbol(token_symbol.as_ref())
                // Access rules
                .mintable(require_admin.clone(), OWNER)
                .burnable(require_admin.clone(), OWNER)
                .recallable(require_admin.clone(), LOCKED)
                .freezable(require_admin.clone(), LOCKED)
                .with_view_key(view_key)
                .with_divisibility(divisibility)
                .with_owner_rule(OwnerRule::ByAccessRule(require_admin.clone()))
                .initial_supply(initial_token_supply);

            // Create component access rules
            let component_access_rules = AccessRules::new().default(require_admin);

            // Vault deposits below are gated on the admin owner rule of their resources, so we
            // hold a proof of the freshly-minted admin badge for the duration of component
            // construction. Creating a proof on a bucket implicitly adds it to the auth scope;
            // dropping it removes the auth and releases the bucket lock so the badge can be
            // returned to the caller.
            let admin_proof = admin_badge.create_proof();
            Component::new(Self {
                config,
                token_vault: Vault::from_bucket(initial_tokens),
                admin_auth_manager: admin_badge.resource_address().into(),
                is_paused: false,
            })
            .with_address_allocation(address_alloc)
            .with_access_rules(component_access_rules)
            // Access is controlled by anyone with an admin badge
            .with_owner_rule(OwnerRule::ByAccessRule(rule!(resource(admin_resource))))
            .create();
            admin_proof.drop();

            admin_badge
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, amount: Amount) {
            self.assert_not_paused();
            assert!(amount.is_positive(), "Amount must be positive");
            let new_tokens = self.token_vault_manager().mint_stealth(amount);
            self.token_vault.deposit(new_tokens);
            emit_event("increase_supply", metadata!("amount" => amount.to_string()));
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, amount: Amount) {
            self.assert_not_paused();
            assert!(amount.is_positive(), "Amount must be positive");
            let tokens = self.token_vault.withdraw(amount);
            tokens.burn();
            emit_event(
                "decrease_supply",
                metadata!("revealed_burn_amount" => amount.to_string()),
            );
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            self.assert_not_paused();
            assert!(amount.is_positive(), "Amount must be positive");
            let bucket = self.token_vault.withdraw(amount);
            emit_event(
                "withdraw",
                metadata!("amount_withdrawn" => bucket.amount().to_string()),
            );
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            self.assert_not_paused();
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", metadata!("amount" => amount.to_string()));
        }

        pub fn recall_revealed_tokens(&mut self, vault_id: VaultId, amount: Amount) {
            assert!(amount.is_positive(), "Amount must be positive");
            let bucket = self
                .token_vault_manager()
                .recall_fungible_amount(vault_id, amount);
            self.token_vault.deposit(bucket);

            emit_event(
                "recall_tokens",
                metadata!(
                        "vault_id" => vault_id.to_string(),
                        "revealed_amount" => amount.to_string(),
                ),
            );
        }

        pub fn burn_utxo(&mut self, utxo: UtxoId, value_proof: StealthValueProof) {
            self.token_vault_manager()
                .burn_utxo(utxo, Some(value_proof));
            emit_event(
                "burn_utxo",
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
            self.admin_auth_manager
                .mint_non_fungible(id, &metadata, &())
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
            // Could also add a check for a specific admin badge ID if desired
            let badges = proof.get_non_fungibles();
            self.is_paused = true;
            emit_event(
                "admin.paused",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key().to_string(),
                    "admin" => badges.first().expect("Proof must contain an admin badge").to_string()
                ),
            );
        }

        pub fn unpause(&mut self, proof: Proof) {
            proof.assert_resource(self.admin_auth_manager.resource_address());
            let badges = proof.get_non_fungibles();
            self.is_paused = false;
            emit_event(
                "admin.unpaused",
                metadata!(
                    "tx_signer" => CallerContext::transaction_signer_public_key().to_string(),
                    "admin" => badges.first().expect("Proof must contain an admin badge").to_string()
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
            self.token_vault.get_resource_manager()
        }

        fn assert_not_paused(&self) {
            assert!(!self.is_paused, "Component is paused");
        }
    }
}
