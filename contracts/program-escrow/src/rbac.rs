//! # Role-Based Access Control (RBAC) Module for Program Escrow
//!
//! This module implements a flexible role-based access control system for the Program Escrow contract.
//!
//! ## Roles
//!
//! - **Admin**: Full control - can manage roles, pause/unpause, configure fees, emergency withdrawal
//! - **Operator**: Day-to-day operations - can release funds, lock funds
//! - **Pauser**: Can pause/unpause the contract in emergency situations
//! - **Viewer**: Read-only access - can query contract state
//!
//! ## Access Control Matrix
//!
//! | Operation | Admin | Operator | Pauser | Viewer |
//! |-----------|-------|----------|--------|--------|
//! | initialize_program | ✓ | ✗ | ✗ | ✗ |
//! | grant_role | ✓ | ✗ | ✗ | ✗ |
//! | revoke_role | ✓ | ✗ | ✗ | ✗ |
//! | lock_funds | ✓ | ✓ | ✗ | ✗ |
//! | batch_payout | ✓ | ✓ | ✗ | ✗ |
//! | single_payout | ✓ | ✓ | ✗ | ✗ |
//! | pause | ✓ | ✗ | ✓ | ✗ |
//! | unpause | ✓ | ✗ | ✗ | ✗ |
//! | emergency_withdraw | ✓ | ✗ | ✗ | ✗ |
//! | update_fee_config | ✓ | ✗ | ✗ | ✗ |
//! | get_roles | ✓ | ✓ | ✓ | ✓ |

use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

/// Enumeration of all available roles in the program escrow contract.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Role {
    /// Admin: Full control (manages roles, config, emergency operations)
    Admin = 0,
    /// Operator: Day-to-day operations (releases funds, locks funds)
    Operator = 1,
    /// Pauser: Can pause/unpause contract in emergencies
    Pauser = 2,
    /// Viewer: Read-only access
    Viewer = 3,
}

/// Storage key for role assignments: (address, role) → bool
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RBACKey {
    /// Maps (Address, Role) to bool
    RoleAssignment(Address, Role),
}

/// Event emitted when a role is granted to an address.
#[contracttype]
#[derive(Clone, Debug)]
pub struct RoleGranted {
    pub address: Address,
    pub role: Role,
    pub granted_by: Address,
    pub timestamp: u64,
}

/// Event emitted when a role is revoked from an address.
#[contracttype]
#[derive(Clone, Debug)]
pub struct RoleRevoked {
    pub address: Address,
    pub role: Role,
    pub revoked_by: Address,
    pub timestamp: u64,
}

/// Check if an address has a specific role.
pub fn has_role(env: &Env, address: Address, role: Role) -> bool {
    let key = RBACKey::RoleAssignment(address, role);
    env.storage()
        .persistent()
        .get::<RBACKey, bool>(&key)
        .unwrap_or(false)
}

/// Grant a role to an address.
pub fn grant_role(env: &Env, address: Address, role: Role, admin: Address) {
    // Only Admin can grant roles
    if !has_role(env, admin.clone(), Role::Admin) {
        panic!("Only Admin can grant roles");
    }

    let key = RBACKey::RoleAssignment(address.clone(), role.clone());
    env.storage()
        .persistent()
        .set::<RBACKey, bool>(&key, &true);

    // Extend TTL for the role assignment (30 days)
    env.storage().persistent().extend_ttl(&key, 2592000, 2592000);

    // Emit role granted event
    env.events().publish(
        (symbol_short!("RoleGr"),),
        RoleGranted {
            address,
            role,
            granted_by: admin,
            timestamp: env.ledger().timestamp(),
        },
    );
}

/// Revoke a role from an address.
pub fn revoke_role(env: &Env, address: Address, role: Role, admin: Address) {
    // Only Admin can revoke roles
    if !has_role(env, admin.clone(), Role::Admin) {
        panic!("Only Admin can revoke roles");
    }

    let key = RBACKey::RoleAssignment(address.clone(), role.clone());
    if env.storage().persistent().has(&key) {
        env.storage().persistent().remove(&key);
    }

    // Emit role revoked event
    env.events().publish(
        (symbol_short!("RoleRv"),),
        RoleRevoked {
            address,
            role,
            revoked_by: admin,
            timestamp: env.ledger().timestamp(),
        },
    );
}

/// Get all roles for an address.
pub fn get_roles(env: &Env, address: Address) -> Vec<Role> {
    let mut roles = Vec::new(env);

    for role in &[Role::Admin, Role::Operator, Role::Pauser, Role::Viewer] {
        if has_role(env, address.clone(), role.clone()) {
            roles.push_back(role.clone());
        }
    }

    roles
}

/// Check if an address has any of the required roles.
pub fn has_any_role(env: &Env, address: Address, required_roles: &Vec<Role>) -> bool {
    for role in required_roles.iter() {
        if has_role(env, address.clone(), role.clone()) {
            return true;
        }
    }
    false
}

/// Require that an address has a specific role, panicking if not.
#[inline]
pub fn require_role(env: &Env, address: Address, required_role: Role) {
    if !has_role(env, address, required_role) {
        panic!("Insufficient permissions: required role not found");
    }
}

/// Require that an address has any of the specified roles, panicking if not.
#[inline]
pub fn require_any_role(env: &Env, address: Address, required_roles: &Vec<Role>) {
    if !has_any_role(env, address, required_roles) {
        panic!("Insufficient permissions: none of the required roles found");
    }
}

/// Require that an address is Admin, panicking if not.
#[inline]
pub fn require_admin(env: &Env, address: Address) {
    require_role(env, address, Role::Admin);
}

/// Require that an address is Admin or Operator, panicking if not.
#[inline]
pub fn require_admin_or_operator(env: &Env, address: Address) {
    let mut roles = Vec::new(env);
    roles.push_back(Role::Admin);
    roles.push_back(Role::Operator);
    require_any_role(env, address, &roles);
}

/// Emit RoleGranted event.
pub fn emit_role_granted(env: &Env, event: RoleGranted) {
    env.events().publish((symbol_short!("RoleGr"),), event);
}

/// Emit RoleRevoked event.
pub fn emit_role_revoked(env: &Env, event: RoleRevoked) {
    env.events().publish((symbol_short!("RoleRv"),), event);
}
