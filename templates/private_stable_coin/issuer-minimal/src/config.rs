// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::types::Amount;

#[derive(Clone, Default, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct StableCoinConfig {
    /// Caps the outstanding supply. `None` leaves supply uncapped.
    #[n(0)]
    pub max_supply: Option<Amount>,
    /// Routes every change of role or governor through the template's `set_roles` and `set_governor`, which check
    /// the new rule and emit an event. The `SetComponentAccessRules` and `SetComponentOwnerRule` transaction
    /// instructions are then rejected. Fixed at instantiation. Off by default.
    #[n(1)]
    #[cbor(default)]
    pub template_managed_rules: bool,
}
