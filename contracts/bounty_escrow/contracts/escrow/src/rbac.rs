//! Role-Based Access Control (RBAC) Module
//!
//! Provides role definitions and enforcement for the Bounty Escrow contract.
//! Supports multiple roles: Admin, Operator, Pauser, and Viewer.

use soroban_sdk::{symbol_short, Address, Env, Map, Symbol, contracttype};

/// Role definitions for RBAC
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,    // Full control: init, config, emergency controls
    Operator, // Day-to-day operations: release, refund
    Pauser,   // Emergency pause capability
    Viewer,   // Read-only access
}

impl Role {
    /// Convert role to symbol for storage
    pub fn as_symbol(self) -> Symbol {
        match self {
            Role::Admin => symbol_short!("admin"),
            Role::Operator => symbol_short!("operat"),
            Role::Pauser => symbol_short!("pauser"),
            Role::Viewer => symbol_short!("viewer"),
        }
    }

    /// Convert role to string representation
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::Operator => "Operator",
            Role::Pauser => "Pauser",
            Role::Viewer => "Viewer",
        }
    }

    /// Parse role from string
    pub fn from_str(_env: &Env, s: &str) -> Option<Self> {
        match s {
            "Admin" => Some(Role::Admin),
            "Operator" => Some(Role::Operator),
            "Pauser" => Some(Role::Pauser),
            "Viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
}

/// Storage key for RBAC roles mapping
const RBAC_ROLES: Symbol = symbol_short!("rbac");

/// Grant a role to an address
pub fn grant_role(env: &Env, address: &Address, role: Role) {
    let mut roles: Map<Address, Symbol> = env
        .storage()
        .instance()
        .get(&RBAC_ROLES)
        .unwrap_or(Map::new(env));

    roles.set(address.clone(), role.as_symbol());
    env.storage().instance().set(&RBAC_ROLES, &roles);

    // Emit event
    env.events().publish(
        (symbol_short!("rbac"),),
        (symbol_short!("grant"), address.clone(), role.as_str()),
    );
}

/// Revoke a role from an address
pub fn revoke_role(env: &Env, address: &Address) {
    let mut roles: Map<Address, Symbol> = env
        .storage()
        .instance()
        .get(&RBAC_ROLES)
        .unwrap_or(Map::new(env));

    if roles.contains_key(address.clone()) {
        roles.remove(address.clone());
        env.storage().instance().set(&RBAC_ROLES, &roles);

        // Emit event
        env.events().publish(
            (symbol_short!("rbac"),),
            (symbol_short!("revoke"), address.clone()),
        );
    }
}

/// Check if an address has a specific role
pub fn has_role(env: &Env, address: &Address, role: Role) -> bool {
    let roles: Map<Address, Symbol> = env
        .storage()
        .instance()
        .get(&RBAC_ROLES)
        .unwrap_or(Map::new(env));

    if let Some(user_role) = roles.get(address.clone()) {
        user_role == role.as_symbol()
    } else {
        false
    }
}

/// Get the role of an address (if any)
pub fn get_role(env: &Env, address: &Address) -> Option<Role> {
    let roles: Map<Address, Symbol> = env
        .storage()
        .instance()
        .get(&RBAC_ROLES)
        .unwrap_or(Map::new(env));

    if let Some(user_role) = roles.get(address.clone()) {
        if user_role == Role::Admin.as_symbol() {
            Some(Role::Admin)
        } else if user_role == Role::Operator.as_symbol() {
            Some(Role::Operator)
        } else if user_role == Role::Pauser.as_symbol() {
            Some(Role::Pauser)
        } else if user_role == Role::Viewer.as_symbol() {
            Some(Role::Viewer)
        } else {
            None
        }
    } else {
        None
    }
}

/// Require a specific role (panics if not authorized)
pub fn require_role(env: &Env, address: &Address, role: Role) {
    if !has_role(env, address, role) {
        panic!("Unauthorized: caller does not have required role");
    }
}

/// Require Admin role
pub fn require_admin(env: &Env, address: &Address) {
    require_role(env, address, Role::Admin);
}

/// Require Operator role (can also fulfill admin roles in some contexts)
pub fn require_operator(env: &Env, address: &Address) {
    let has_perm = has_role(env, address, Role::Operator) || has_role(env, address, Role::Admin);
    if !has_perm {
        panic!("Unauthorized: caller does not have Operator or Admin role");
    }
}

/// Require Pauser role (can also fulfill admin roles in some contexts)
pub fn require_pauser(env: &Env, address: &Address) {
    let has_perm = has_role(env, address, Role::Pauser) || has_role(env, address, Role::Admin);
    if !has_perm {
        panic!("Unauthorized: caller does not have Pauser or Admin role");
    }
}

/// Check if address is an operator (has Operator or Admin role)
pub fn is_operator(env: &Env, address: &Address) -> bool {
    has_role(env, address, Role::Operator) || has_role(env, address, Role::Admin)
}

/// Check if address is a pauser (has Pauser or Admin role)
pub fn can_pause(env: &Env, address: &Address) -> bool {
    has_role(env, address, Role::Pauser) || has_role(env, address, Role::Admin)
}

/// Check if address is an admin
pub fn is_admin(env: &Env, address: &Address) -> bool {
    has_role(env, address, Role::Admin)
}
