//! # Reentrancy Guard Module
//!
//! Provides protection against reentrancy attacks in the Bounty Escrow contract.
//!
//! ## Overview
//!
//! Reentrancy occurs when an external contract call (e.g. a token transfer)
//! triggers a callback that re-enters a protected function before the first
//! invocation completes.  This module uses a boolean flag in instance storage
//! to detect and block such re-entry.
//!
//! ## Usage
//!
//! ```rust
//! use crate::reentrancy_guard;
//!
//! pub fn sensitive_function(env: Env) {
//!     reentrancy_guard::acquire(&env);   // panics if already entered
//!     // ... perform checks, state updates, external calls ...
//!     reentrancy_guard::release(&env);
//! }
//! ```
//!
//! ## Security Notes
//!
//! - On `panic!` Soroban rolls back all state changes, so the guard cannot
//!   get stuck in a locked state.
//! - Returning `Err(..)` from a `#[contractimpl]` function also reverts state,
//!   so early error returns after `acquire` are safe.
//! - The same guard key is shared across all protected functions, providing
//!   cross-function reentrancy protection.

use super::DataKey;
use soroban_sdk::Env;

/// Acquire the reentrancy guard.
///
/// # Panics
/// Panics with `"Reentrancy detected"` if the guard is already held.
pub fn acquire(env: &Env) {
    if env.storage().instance().has(&DataKey::ReentrancyGuard) {
        panic!("Reentrancy detected");
    }
    env.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &true);
}

/// Release the reentrancy guard.
///
/// Must be called before returning from every protected function on the
/// success path.
pub fn release(env: &Env) {
    env.storage().instance().remove(&DataKey::ReentrancyGuard);
}

/// Check whether the guard is currently held (useful in tests).
#[cfg(test)]
pub fn is_active(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::ReentrancyGuard)
}
