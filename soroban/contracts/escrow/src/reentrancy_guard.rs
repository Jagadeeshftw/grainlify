//! Reentrancy guard for the soroban escrow contract.
//!
//! Uses the `DataKey::ReentrancyGuard` variant stored in instance storage.
//! Soroban rolls back all state on panic or `Err` return, so the flag
//! cannot get permanently stuck.

use crate::DataKey;
use soroban_sdk::Env;

/// State constants for the reentrancy guard.
/// Using non-zero values prevents default-zero value confusion.
const NOT_ENTERED: u32 = 1;
const ENTERED: u32 = 2;

/// Acquire the reentrancy guard.
///
/// Sets a u32 flag (ENTERED) in instance storage. If the flag is already set to ENTERED,
/// this function panics — indicating a re-entrant call.
///
/// # Panics
/// Panics with `"Reentrancy detected"` if the guard is already held.
pub fn acquire(env: &Env) {
    let status: u32 = env
        .storage()
        .instance()
        .get(&DataKey::ReentrancyGuard)
        .unwrap_or(NOT_ENTERED);

    if status != NOT_ENTERED {
        panic!("Reentrancy detected");
    }

    env.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &ENTERED);
}

/// Release the reentrancy guard.
///
/// Resets the guard flag to NOT_ENTERED in instance storage.
/// Note: On error/panic paths Soroban's automatic state rollback clears the
/// guard automatically, so manual release is only needed on success.
pub fn release(env: &Env) {
    env.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &NOT_ENTERED);
}

