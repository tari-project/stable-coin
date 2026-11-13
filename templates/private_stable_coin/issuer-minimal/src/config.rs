// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::types::Amount;

#[derive(Clone, Default, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct StableCoinConfig {
    /// Caps the outstanding supply. `None` leaves supply uncapped.
    #[n(0)]
    pub max_supply: Option<Amount>,
}
