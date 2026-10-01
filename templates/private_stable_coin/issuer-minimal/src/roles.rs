// Copyright 2026 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use core::fmt;
use tari_template_lib::prelude::*;

/// A privileged capability of the stable coin component.
///
/// Each role is held by whoever satisfies its access rule, and the component's method access rules are derived from
/// the role rules (see `TariStableCoin::component_access_rules`). The governor owns the component, so it can call
/// every method and is the only role that can change who holds a role.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, minicbor::Encode, minicbor::Decode, minicbor::CborLen,
)]
#[cbor(index_only)]
pub enum Role {
    /// Grants and revokes roles, issues and revokes admin badges, changes config and unpauses.
    #[n(0)]
    Governor,
    /// Mints new supply into the treasury.
    #[n(1)]
    Minter,
    /// Burns supply from the treasury and burns UTXOs.
    #[n(2)]
    Burner,
    /// Moves funds into and out of the treasury.
    #[n(3)]
    Treasurer,
    /// Recalls and freezes.
    #[n(4)]
    Compliance,
    /// Pauses the component. Unpausing is reserved for the governor.
    #[n(6)]
    Pauser,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Governor => "governor",
            Self::Minter => "minter",
            Self::Burner => "burner",
            Self::Treasurer => "treasurer",
            Self::Compliance => "compliance",
            Self::Pauser => "pauser",
        };
        f.write_str(name)
    }
}

/// The access rule for each role, chosen at instantiation. A role left as `None` is held by any admin badge holder.
///
/// A rule may name public keys (satisfied by the transaction's signers), specific admin badges
/// (`non_fungible(..)`) or a threshold of either (`m_of_n(..)`).
#[derive(Debug, Clone, Default, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct RoleConfig {
    #[n(0)]
    pub governor: Option<AccessRule>,
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

/// The access rule currently held by each role.
#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct Roles {
    #[n(0)]
    governor: AccessRule,
    #[n(1)]
    minter: AccessRule,
    #[n(2)]
    burner: AccessRule,
    #[n(3)]
    treasurer: AccessRule,
    #[n(4)]
    compliance: AccessRule,
    #[n(6)]
    pauser: AccessRule,
}

impl Roles {
    /// Resolves `config`, giving every unset role to `default_rule`.
    pub fn from_config(config: RoleConfig, default_rule: AccessRule) -> Self {
        let roles = Self {
            governor: config.governor.unwrap_or_else(|| default_rule.clone()),
            minter: config.minter.unwrap_or_else(|| default_rule.clone()),
            burner: config.burner.unwrap_or_else(|| default_rule.clone()),
            treasurer: config.treasurer.unwrap_or_else(|| default_rule.clone()),
            compliance: config.compliance.unwrap_or_else(|| default_rule.clone()),
            pauser: config.pauser.unwrap_or(default_rule),
        };
        for role in Role::ALL {
            assert_valid_role_rule(role, roles.get(role));
        }
        roles
    }

    pub fn get(&self, role: Role) -> &AccessRule {
        match role {
            Role::Governor => &self.governor,
            Role::Minter => &self.minter,
            Role::Burner => &self.burner,
            Role::Treasurer => &self.treasurer,
            Role::Compliance => &self.compliance,
            Role::Pauser => &self.pauser,
        }
    }

    pub fn set(&mut self, role: Role, rule: AccessRule) {
        assert_valid_role_rule(role, &rule);
        let slot = match role {
            Role::Governor => &mut self.governor,
            Role::Minter => &mut self.minter,
            Role::Burner => &mut self.burner,
            Role::Treasurer => &mut self.treasurer,
            Role::Compliance => &mut self.compliance,
            Role::Pauser => &mut self.pauser,
        };
        *slot = rule;
    }
}

impl Role {
    pub const ALL: [Role; 6] = [
        Role::Governor,
        Role::Minter,
        Role::Burner,
        Role::Treasurer,
        Role::Compliance,
        Role::Pauser,
    ];
}

/// A role open to everyone would make its privileged methods public, e.g. permissionless minting. A governor rule that
/// no one satisfies would leave every role permanently fixed, including a compromised one.
fn assert_valid_role_rule(role: Role, rule: &AccessRule) {
    assert!(
        !matches!(rule, AccessRule::AllowAll),
        "The {role} role cannot be open to everyone"
    );
    assert!(
        role != Role::Governor || !matches!(rule, AccessRule::DenyAll),
        "The governor role must be held by someone"
    );
}
