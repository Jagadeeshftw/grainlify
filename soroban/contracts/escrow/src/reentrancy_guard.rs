//! Reentrancy guard for the soroban escrow contract.
//!
//! Uses the `DataKey::ReentrancyGuard` variant stored in instance storage.
//! Soroban rolls back all state on panic or `Err` return, so the flag
//! cannot get permanently stuck.

use crate::DataKey;
use soroban_sdk::Env;

/// Acquire the reentrancy guard.
///
/// # Panics
/// Panics with `"Reentrancy detected"` if the guard is already held.
pub fn acquire(env: &Env) {
    let entered: bool = env
        .storage()
        .instance()
        .get(&DataKey::ReentrancyGuard)
        .unwrap_or(false);
    if entered {
        panic!("Reentrancy detected");
    }
    env.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &true);
}

/// Release the reentrancy guard.
pub fn release(env: &Env) {
    env.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &false);
}
