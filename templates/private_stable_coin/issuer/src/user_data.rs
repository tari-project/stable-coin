// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use core::fmt;
use tari_template_lib::component::ComponentManager;
use tari_template_lib::{types::Amount, types::NonFungibleId};

#[derive(Clone, Copy, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct UserId(#[n(0)] u64);

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

#[derive(Clone, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct UserData {
    #[n(0)]
    pub user_id: UserId,
    #[n(1)]
    pub user_account: ComponentManager,
    #[n(2)]
    pub created_at_epoch: u64,
}

#[derive(Clone, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct UserMutableData {
    #[n(0)]
    pub is_blacklisted: bool,
    #[n(1)]
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
