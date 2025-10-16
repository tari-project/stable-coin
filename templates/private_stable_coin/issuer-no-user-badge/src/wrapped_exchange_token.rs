// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::models::ResourceAddress;
use tari_template_lib::resource::ResourceManager;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WrappedExchangeToken {
    pub manager: ResourceManager,
}

impl WrappedExchangeToken {
    pub fn new(resource_address: ResourceAddress) -> Self {
        Self {
            manager: ResourceManager::get(resource_address),
        }
    }

    pub fn resource_address(&self) -> ResourceAddress {
        self.manager.resource_address()
    }

    pub fn manager(&self) -> &ResourceManager {
        &self.manager
    }
}
