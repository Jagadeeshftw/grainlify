//! # Role-Based Access Control (RBAC) Module
//!
//! This module implements a flexible role-based access control system for the Bounty Escrow contract.
//! It allows fine-grained permission management by defining roles and assigning them to addresses.
//!
//! ## Roles
//!
//! - **Admin**: Full control - can manage roles, pause/unpause, configure fees, emergency withdrawal
//! - **Operator**: Day-to-day operations - can release funds, approve refunds, lock bounties
//! - **Pauser**: Can pause/unpause the contract in emergency situations
//! - **Viewer**: Read-only access - can query contract state
//!
//! ## Role Hierarchy (Advisory)
//!
//! ```text
//! Admin
//!   ├─ Operator
//!   ├─ Pauser
//!   └─ Viewer
//! ```
//!
//! ## Access Control Matrix
//!
//! | Operation | Admin | Operator | Pauser | Viewer | Notes |
//! |-----------|-------|----------|--------|--------|-------|
//! | init | ✓ | ✗ | ✗ | ✗ | Single initialization |
//! | grant_role | ✓ | ✗ | ✗ | ✗ | Role management |
//! | revoke_role | ✓ | ✗ | ✗ | ✗ | Role revocation |
//! | lock_funds | ✓ | ✓ | ✗ | ✗ | Create bounties |
//! | release_funds | ✓ | ✓ | ✗ | ✗ | Release to winners |
//! | approve_refund | ✓ | ✓ | ✗ | ✗ | Approve refunds |
//! | refund | ✓ | ✓ | ✗ | ✗ | Execute refunds |
//! | pause | ✓ | ✗ | ✓ | ✗ | Emergency pause |
//! | unpause | ✓ | ✗ | ✗ | ✗ | Only admin can unpause |
//! | update_fee_config | ✓ | ✗ | ✗ | ✗ | Configure fees |
//! | get_roles | ✓ | ✓ | ✓ | ✓ | Query roles (any role) |

use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

/// Enumeration of all available roles in the contract.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Role {
    /// Admin: Full control (manages roles, config, emergency operations)
    Admin = 0,
    /// Operator: Day-to-day operations (releases funds, approves refunds)
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
    /// Stores all addresses with any role (for enumeration)
    RoleHolders,
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
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to check
/// * `role` - The role to verify
///
/// # Returns
/// * `bool` - True if the address has the role, false otherwise
pub fn has_role(env: &Env, address: Address, role: Role) -> bool {
    let key = RBACKey::RoleAssignment(address, role);
    env.storage()
        .persistent()
        .get::<RBACKey, bool>(&key)
        .unwrap_or(false)
}

/// Grant a role to an address.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to grant the role to
/// * `role` - The role to grant
/// * `admin` - The address granting the role (must be Admin)
///
/// # Panics
/// * If the caller is not an Admin
///
/// # Events
/// Emits: `RoleGranted { address, role, granted_by: admin, timestamp }`
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
            address: address.clone(),
            role: role.clone(),
            granted_by: admin,
            timestamp: env.ledger().timestamp(),
        },
    );
}

/// Revoke a role from an address.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to revoke the role from
/// * `role` - The role to revoke
/// * `admin` - The address revoking the role (must be Admin)
///
/// # Panics
/// * If the caller is not an Admin
///
/// # Events
/// Emits: `RoleRevoked { address, role, revoked_by: admin, timestamp }`
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
            address: address.clone(),
            role: role.clone(),
            revoked_by: admin,
            timestamp: env.ledger().timestamp(),
        },
    );
}

/// Get all roles for an address.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to query
///
/// # Returns
/// * `Vec<Role>` - Vector of all roles assigned to the address
pub fn get_roles(env: &Env, address: Address) -> Vec<Role> {
    let mut roles = Vec::new(&env);

    for role in &[Role::Admin, Role::Operator, Role::Pauser, Role::Viewer] {
        if has_role(env, address.clone(), role.clone()) {
            roles.push_back(role.clone());
        }
    }

    roles
}

/// Check if an address has any of the required roles.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to check
/// * `required_roles` - Vector of roles, at least one of which must be held
///
/// # Returns
/// * `bool` - True if the address has at least one of the required roles
pub fn has_any_role(env: &Env, address: Address, required_roles: &Vec<Role>) -> bool {
    for role in required_roles.iter() {
        if has_role(env, address.clone(), role.clone()) {
            return true;
        }
    }
    false
}

/// Check if an address has all of the required roles.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to check
/// * `required_roles` - Vector of roles, all of which must be held
///
/// # Returns
/// * `bool` - True if the address has all required roles
pub fn has_all_roles(env: &Env, address: Address, required_roles: &Vec<Role>) -> bool {
    for role in required_roles.iter() {
        if !has_role(env, address.clone(), role.clone()) {
            return false;
        }
    }
    true
}

/// Require that an address has a specific role, panicking if not.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to verify
/// * `required_role` - The required role
///
/// # Panics
/// * If the address does not have the required role
#[inline]
pub fn require_role(env: &Env, address: Address, required_role: Role) {
    if !has_role(env, address, required_role) {
        panic!("Insufficient permissions: required role not found");
    }
}

/// Require that an address has any of the specified roles, panicking if not.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to verify
/// * `required_roles` - At least one of these roles must be held
///
/// # Panics
/// * If the address does not have any of the required roles
#[inline]
pub fn require_any_role(env: &Env, address: Address, required_roles: &Vec<Role>) {
    if !has_any_role(env, address, required_roles) {
        panic!("Insufficient permissions: none of the required roles found");
    }
}

/// Require that an address is Admin, panicking if not.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to verify
///
/// # Panics
/// * If the address is not an Admin
#[inline]
pub fn require_admin(env: &Env, address: Address) {
    require_role(env, address, Role::Admin);
}

/// Require that an address is Admin or Operator, panicking if not.
///
/// # Arguments
/// * `env` - The contract environment
/// * `address` - The address to verify
///
/// # Panics
/// * If the address is neither Admin nor Operator
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
