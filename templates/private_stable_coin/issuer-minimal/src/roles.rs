// Copyright 2026 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::prelude::*;

/// A privileged capability of the stable coin component: a named group of methods.
///
/// A role is held by whoever satisfies the access rule on its methods. The governor is not a role: it is the
/// component's owner, which may call every method and replace the rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Mints new supply into the treasury.
    Minter,
    /// Burns supply from the treasury and burns UTXOs.
    Burner,
    /// Moves funds into and out of the treasury.
    Treasurer,
    /// Recalls and freezes.
    Compliance,
    /// Pauses the component. Unpausing is reserved for the governor.
    Pauser,
}

impl Role {
    pub const ALL: [Role; 5] = [
        Role::Minter,
        Role::Burner,
        Role::Treasurer,
        Role::Compliance,
        Role::Pauser,
    ];

    /// The component methods the role may call. Every other method is reserved for the governor.
    pub const fn methods(self) -> &'static [&'static str] {
        match self {
            Role::Minter => &["increase_supply"],
            Role::Burner => &["decrease_supply", "burn_utxo"],
            Role::Treasurer => &["withdraw", "deposit"],
            Role::Compliance => &["recall_revealed_tokens", "freeze_utxos", "unfreeze_utxos"],
            Role::Pauser => &["pause"],
        }
    }
}

/// Who governs the coin and who holds each role.
///
/// The governor rule becomes the component's owner rule. A role left unset has no rule of its own, so only the
/// governor can call its methods, whoever the governor is at the time.
#[derive(Clone, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct RoleConfig {
    #[n(0)]
    pub governor: AccessRule,
    #[n(1)]
    pub minter: Option<AccessRule>,
    #[n(2)]
    pub burner: Option<AccessRule>,
    #[n(3)]
    pub treasurer: Option<AccessRule>,
    #[n(4)]
    pub compliance: Option<AccessRule>,
    #[n(6)]
    pub pauser: Option<AccessRule>,
}

impl RoleConfig {
    pub fn get(&self, role: Role) -> Option<&AccessRule> {
        match role {
            Role::Minter => self.minter.as_ref(),
            Role::Burner => self.burner.as_ref(),
            Role::Treasurer => self.treasurer.as_ref(),
            Role::Compliance => self.compliance.as_ref(),
            Role::Pauser => self.pauser.as_ref(),
        }
    }

    /// Rejects the rules that are only ever a mistake at instantiation: a governor or role that everyone holds, and
    /// a governor that no one holds, which would leave the coin without an owner for good.
    // The tests include this file as a module and never call this.
    #[allow(dead_code)]
    pub fn assert_valid(&self) {
        assert!(
            !matches!(self.governor, AccessRule::AllowAll),
            "The governor cannot be open to everyone"
        );
        assert!(
            !matches!(self.governor, AccessRule::DenyAll),
            "The governor must be held by someone"
        );
        for role in Role::ALL {
            assert!(
                !matches!(self.get(role), Some(AccessRule::AllowAll)),
                "A role cannot be open to everyone"
            );
        }
    }

    /// The component access rules that give each role its methods. Everything else is denied to all but the owner.
    pub fn access_rules(&self) -> AccessRules {
        let mut rules = AccessRules::new();
        for role in Role::ALL {
            if let Some(rule) = self.get(role) {
                for method in role.methods() {
                    rules = rules.add_method_rule(*method, rule.clone());
                }
            }
        }
        rules
    }
}
