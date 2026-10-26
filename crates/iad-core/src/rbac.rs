//! Role-Based Access Control for IAD API (Phase 4.11).
//!
//! Roles: Admin (full), Operator (agents+tasks), Reader (read-only)
//! Tokens are mapped to roles in config.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full access — everything.
    Admin,
    /// Can manage agents and tasks but not config/reload.
    Operator,
    /// Read-only — list/get only.
    Reader,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    AgentRead, AgentWrite,
    TaskRead,  TaskWrite,
    MetricsRead,
    SystemReload, SystemAdmin,
}

impl Role {
    pub fn permissions(&self) -> &[Permission] {
        match self {
            Role::Admin    => &[
                Permission::AgentRead, Permission::AgentWrite,
                Permission::TaskRead,  Permission::TaskWrite,
                Permission::MetricsRead, Permission::SystemReload, Permission::SystemAdmin,
            ],
            Role::Operator => &[
                Permission::AgentRead, Permission::AgentWrite,
                Permission::TaskRead,  Permission::TaskWrite,
                Permission::MetricsRead,
            ],
            Role::Reader   => &[
                Permission::AgentRead, Permission::TaskRead, Permission::MetricsRead,
            ],
        }
    }

    pub fn can(&self, perm: &Permission) -> bool {
        self.permissions().contains(perm)
    }
}

/// Token → Role mapping.
#[derive(Debug, Clone, Default)]
pub struct RbacPolicy {
    tokens: HashMap<String, Role>,
}

impl RbacPolicy {
    pub fn new() -> Self { Self::default() }

    pub fn add_token(&mut self, token: &str, role: Role) {
        self.tokens.insert(token.into(), role);
    }

    /// Resolve role from bearer token. None if unknown token.
    pub fn resolve(&self, token: &str) -> Option<&Role> {
        self.tokens.get(token)
    }

    pub fn check(&self, token: &str, perm: &Permission) -> bool {
        self.resolve(token).map(|r| r.can(perm)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_all() {
        let r = Role::Admin;
        assert!(r.can(&Permission::SystemAdmin));
        assert!(r.can(&Permission::AgentWrite));
    }

    #[test]
    fn reader_limited() {
        let r = Role::Reader;
        assert!(r.can(&Permission::AgentRead));
        assert!(!r.can(&Permission::AgentWrite));
        assert!(!r.can(&Permission::SystemAdmin));
    }

    #[test]
    fn policy_check() {
        let mut p = RbacPolicy::new();
        p.add_token("admin-tok", Role::Admin);
        p.add_token("reader-tok", Role::Reader);
        assert!(p.check("admin-tok", &Permission::SystemAdmin));
        assert!(!p.check("reader-tok", &Permission::AgentWrite));
        assert!(!p.check("unknown", &Permission::AgentRead));
    }
}
