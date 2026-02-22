//! # Reentrancy Guard Module
//!
//! Provides protection against reentrancy attacks in Soroban smart contracts.
//!
//! ## Overview
//!
//! Reentrancy occurs when an external contract call is made during the execution
//! of a function, and that external contract calls back into the original contract
//! before the first invocation has completed. This can lead to unexpected state
//! changes and potential exploits.
//!
//! ## Implementation
//!
//! This guard uses a simple boolean flag stored in contract storage to track
//! whether a protected function is currently executing. The guard:
//! 1. Checks if the function is already executing (flag is true)
//! 2. If yes, panics to prevent reentry
//! 3. If no, sets the flag to true
//! 4. Executes the protected code
//! 5. Resets the flag to false when done
//!
//! ## Usage
//!
//! ```rust
//! use crate::reentrancy_guard::{check_not_entered, set_entered, clear_entered};
//!
//! pub fn sensitive_function(env: Env) {
//!     // Check and set guard
//!     check_not_entered(&env);
//!     set_entered(&env);
//!     
//!     // ... protected code that makes external calls ...
//!     
//!     // Clear guard before returning
//!     clear_entered(&env);
//! }
//! ```
//!
//! ## Security Considerations
//!
//! - The guard MUST be cleared before the function returns
//! - If a panic occurs, Soroban will roll back all state changes including the guard
//! - The guard protects against same-contract reentrancy
//! - Cross-contract reentrancy requires additional considerations

use soroban_sdk::{symbol_short, Env, Symbol};

/// Storage key for the reentrancy guard flag
const REENTRANCY_GUARD: Symbol = symbol_short!("ReentGrd");

/// Check if a protected function is currently executing.
/// Panics if reentrancy is detected.
///
/// # Panics
/// * If the guard flag is already set (reentrancy detected)
pub fn check_not_entered(env: &Env) {
    let entered: bool = env
        .storage()
        .instance()
        .get(&REENTRANCY_GUARD)
        .unwrap_or(false);

    if entered {
        panic!("Reentrancy detected");
    }
}

/// Set the reentrancy guard flag to indicate a protected function is executing.
pub fn set_entered(env: &Env) {
    env.storage().instance().set(&REENTRANCY_GUARD, &true);
}

/// Clear the reentrancy guard flag to indicate the protected function has completed.
pub fn clear_entered(env: &Env) {
    env.storage().instance().set(&REENTRANCY_GUARD, &false);
}

/// Check if the guard is currently set (for testing purposes).
///
/// # Returns
/// * `true` if a protected function is currently executing
/// * `false` otherwise
pub fn is_entered(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&REENTRANCY_GUARD)
        .unwrap_or(false)
}

/// Macro to wrap a function with reentrancy protection.
///
/// This ensures the guard is properly set and cleared even if the function panics.
/// Note: In Soroban, panics roll back all state changes, so the guard will be
/// automatically cleared on panic.
#[macro_export]
macro_rules! with_reentrancy_guard {
    ($env:expr, $body:block) => {{
        $crate::reentrancy_guard::check_not_entered(&$env);
        $crate::reentrancy_guard::set_entered(&$env);

        let result = $body;

        $crate::reentrancy_guard::clear_entered(&$env);
        result
    }};
}
