//! Role-Based Access Control (RBAC) Module for Program Escrow
//!
//! Provides hierarchical role management for the Program Escrow contract.

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
    pub fn as_str(&self) -> &str {
        match self {
            Role::Admin => "admin",
            Role::Operator => "operator",
            Role::Pauser => "pauser",
            Role::Viewer => "viewer",
        }
    }

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

pub const ROLE_STORAGE_KEY: &str = "rbac_roles";

pub fn grant_role(env: &Env, address: Address, role: Role, _granted_by: Address) {
    let mut roles: Map<Address, Map<Symbol, bool>> = env
        .storage()
        .instance()
        .get(&Symbol::new(env, ROLE_STORAGE_KEY))
        .unwrap_or_else(|| Map::new(env));

    let mut address_roles = roles.get(address.clone()).unwrap_or_else(|| Map::new(env));
    let role_symbol = Symbol::new(env, role.as_str());
    address_roles.set(role_symbol, true);

    roles.set(address, address_roles);
    env.storage()
        .instance()
        .set(&Symbol::new(env, ROLE_STORAGE_KEY), &roles);
}

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

pub fn require_admin(env: &Env, caller: Address) {
    if !has_role(env, caller, Role::Admin) {
        panic!("Unauthorized: Admin role required");
    }
}

pub fn require_role(env: &Env, caller: Address, role: Role) {
    if !has_role(env, caller, role) {
        panic!("Unauthorized: role required");
    }
}

pub fn is_operator(env: &Env, address: Address) -> bool {
    has_role(env, address, Role::Admin) || has_role(env, address, Role::Operator)
}

pub fn can_pause(env: &Env, address: Address) -> bool {
    has_role(env, address, Role::Admin) || has_role(env, address, Role::Pauser)
}
