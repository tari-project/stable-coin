// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::models::{ResourceAddress, Vault};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WrappedExchangeToken {
    pub vault: Vault,
}

impl WrappedExchangeToken {
    pub(crate) fn resource_address(&self) -> ResourceAddress {
        self.vault.resource_address()
    }

    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    pub fn vault_mut(&mut self) -> &mut Vault {
        &mut self.vault
    }
}
