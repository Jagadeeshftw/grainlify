//! Role-Based Access Control (RBAC) Module
//!
//! Provides hierarchical role management for the Bounty Escrow contract.
//! Supports four roles with specific permissions:
//!
//! # Role Hierarchy
//! - **Admin**: Full control including role management, configuration, and emergency operations
//! - **Operator**: Can execute standard operations like releasing funds
//! - **Pauser**: Can pause/unpause contract operations
//! - **Viewer**: Read-only access to contract state
//!
//! # RBAC Matrix
//! | Operation | Admin | Operator | Pauser | Viewer |
//! |-----------|-------|----------|--------|--------|
//! | Initialize | ✓ | ✗ | ✗ | ✗ |
//! | Lock Funds | ✓ | ✓ | ✗ | ✗ |
//! | Release Funds | ✓ | ✓ | ✗ | ✗ |
//! | Refund | ✓ | ✓ | ✗ | ✗ |
//! | Pause | ✓ | ✗ | ✓ | ✗ |
//! | Unpause | ✓ | ✗ | ✓ | ✗ |
//! | Grant Role | ✓ | ✗ | ✗ | ✗ |
//! | Revoke Role | ✓ | ✗ | ✗ | ✗ |
//! | Query State | ✓ | ✓ | ✓ | ✓ |

use soroban_sdk::{Address, Env, Map, Symbol};

/// Role types in the RBAC system
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Admin,
    Operator,
    Pauser,
    Viewer,
}

impl Role {
    /// Convert role to string representation for storage
    pub fn as_str(&self) -> &str {
        match self {
            Role::Admin => "admin",
            Role::Operator => "operator",
            Role::Pauser => "pauser",
            Role::Viewer => "viewer",
        }
    }

    /// Convert string to role
    pub fn from_str(s: &str) -> Option<Role> {
        match s {
            "admin" => Some(Role::Admin),
            "operator" => Some(Role::Operator),
            "pauser" => Some(Role::Pauser),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
}

/// Storage key for role mappings
pub const ROLE_STORAGE_KEY: &str = "rbac_roles";

/// Grant a role to an address
///
/// # Arguments
/// * `env` - Contract environment
/// * `address` - Address to grant role to
/// * `role` - Role to grant
/// * `granted_by` - Address granting the role (for audit trail)
pub fn grant_role(env: &Env, address: Address, role: Role, _granted_by: Address) {
    let mut roles: Map<Address, Map<Symbol, bool>> = env
        .storage()
        .instance()
        .get(&Symbol::new(env, ROLE_STORAGE_KEY))
        .unwrap_or_else(|| Map::new(env));

    let mut address_roles = roles.get(address.clone()).unwrap_or_else(|| Map::new(env));

    // Store the role as a boolean flag (true = has role)
    let role_symbol = Symbol::new(env, role.as_str());
    address_roles.set(role_symbol, true);

    roles.set(address, address_roles);
    env.storage()
        .instance()
        .set(&Symbol::new(env, ROLE_STORAGE_KEY), &roles);
}

/// Revoke a role from an address
///
/// # Arguments
/// * `env` - Contract environment
/// * `address` - Address to revoke role from
/// * `role` - Role to revoke
pub fn revoke_role(env: &Env, address: Address, role: Role) {
    let mut roles: Map<Address, Map<Symbol, bool>> = env
        .storage()
        .instance()
        .get(&Symbol::new(env, ROLE_STORAGE_KEY))
        .unwrap_or_else(|| Map::new(env));

    if let Some(mut address_roles) = roles.get(address.clone()) {
        let role_symbol = Symbol::new(env, role.as_str());
        address_roles.remove(role_symbol);
        
        if address_roles.len() > 0 {
            roles.set(address, address_roles);
        } else {
            roles.remove(address);
        }
        
        env.storage()
            .instance()
            .set(&Symbol::new(env, ROLE_STORAGE_KEY), &roles);
    }
}

/// Check if an address has a specific role
///
/// # Arguments
/// * `env` - Contract environment
/// * `address` - Address to check
/// * `role` - Role to check for
///
/// # Returns
/// `true` if address has the role, `false` otherwise
pub fn has_role(env: &Env, address: Address, role: Role) -> bool {
    let roles: Map<Address, Map<Symbol, bool>> = env
        .storage()
        .instance()
        .get(&Symbol::new(env, ROLE_STORAGE_KEY))
        .unwrap_or_else(|| Map::new(env));

    if let Some(address_roles) = roles.get(address) {
        let role_symbol = Symbol::new(env, role.as_str());
        address_roles.get(role_symbol).unwrap_or(false)
    } else {
        false
    }
}

/// Require that caller has Admin role
///
/// # Panics
/// If caller does not have Admin role
pub fn require_admin(env: &Env, caller: Address) {
    if !has_role(env, caller, Role::Admin) {
        panic!("Unauthorized: Admin role required");
    }
}

/// Require that caller has specific role
///
/// # Panics
/// If caller does not have the required role
pub fn require_role(env: &Env, caller: Address, role: Role) {
    if !has_role(env, caller, role) {
        panic!("Unauthorized: {:?} role required", role);
    }
}

/// Check if address has Admin or Operator role
pub fn is_operator(env: &Env, address: Address) -> bool {
    has_role(env, address, Role::Admin) || has_role(env, address, Role::Operator)
}

/// Check if address has Admin or Pauser role
pub fn can_pause(env: &Env, address: Address) -> bool {
    has_role(env, address, Role::Admin) || has_role(env, address, Role::Pauser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as TestAddress;

    #[test]
    fn test_role_as_str() {
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::Operator.as_str(), "operator");
        assert_eq!(Role::Pauser.as_str(), "pauser");
        assert_eq!(Role::Viewer.as_str(), "viewer");
    }

    #[test]
    fn test_role_from_str() {
        assert_eq!(Role::from_str("admin"), Some(Role::Admin));
        assert_eq!(Role::from_str("operator"), Some(Role::Operator));
        assert_eq!(Role::from_str("pauser"), Some(Role::Pauser));
        assert_eq!(Role::from_str("viewer"), Some(Role::Viewer));
        assert_eq!(Role::from_str("invalid"), None);
    }
}
