// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::resource::ResourceManager;
use tari_template_lib::types::ResourceAddress;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WrappedExchangeToken {
    manager: ResourceManager,
}

impl WrappedExchangeToken {
    pub fn new<T: Into<ResourceManager>>(resource: T) -> Self {
        Self {
            manager: resource.into(),
        }
    }

    pub fn resource_address(&self) -> ResourceAddress {
        self.manager.resource_address()
    }

    pub fn manager(&self) -> &ResourceManager {
        &self.manager
    }
}
