// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use core::fmt;
use tari_template_lib::component::ComponentManager;
use tari_template_lib::{types::Amount, types::NonFungibleId};

#[derive(Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct UserId(u64);

impl From<UserId> for NonFungibleId {
    fn from(value: UserId) -> Self {
        Self::from_u64(value.0)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:0>19}", self.0)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct UserData {
    pub user_id: UserId,
    pub user_account: ComponentManager,
    pub created_at_epoch: u64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct UserMutableData {
    pub is_blacklisted: bool,
    pub wrapped_exchange_limit: Amount,
}

impl UserMutableData {
    pub fn set_wrapped_exchange_limit(&mut self, limit: Amount) -> &mut Self {
        self.wrapped_exchange_limit = limit;
        self
    }
}

impl Default for UserMutableData {
    fn default() -> Self {
        Self {
            is_blacklisted: false,
            wrapped_exchange_limit: 1000u64.into(),
        }
    }
}
