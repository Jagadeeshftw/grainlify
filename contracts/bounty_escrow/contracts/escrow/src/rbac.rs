//! Role-Based Access Control (RBAC) helpers.
//!
//! # Role Matrix
//!
//! | Action                  | Admin | Operator | Participant |
//! |-------------------------|-------|----------|-------------|
//! | `init`                  | ✓     | ✗        | ✗           |
//! | `lock_funds`            | ✗     | ✗        | ✓ (self)    |
//! | `refund`                | ✓+✓   | ✗        | ✓ (co-sign) |
use soroban_sdk::{Address, Env};

use crate::DataKey;

/// Returns the stored admin address, panicking if not initialized.
pub fn require_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
        .expect("contract not initialized")
}

/// Asserts that `caller` is the stored admin. Panics otherwise.
pub fn assert_admin(env: &Env, caller: &Address) {
    let admin = require_admin(env);
    assert_eq!(&admin, caller, "caller is not admin");
    caller.require_auth();
}

/// Returns `true` if `addr` is the stored admin.
pub fn is_admin(env: &Env, addr: &Address) -> bool {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
        .map(|a| &a == addr)
        .unwrap_or(false)
}

/// Returns `true` if `addr` is the stored anti-abuse (operator) admin.
pub fn is_operator(env: &Env, addr: &Address) -> bool {
    use crate::anti_abuse;
    anti_abuse::get_admin(env)
        .map(|a| &a == addr)
        .unwrap_or(false)
}
