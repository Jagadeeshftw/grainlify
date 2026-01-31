//! Time manipulation helpers.
//!
//! Provides functions to manipulate time in tests.

use soroban_sdk::Env;

/// Gets the current ledger timestamp.
///
/// # Arguments
/// * `env` - The contract environment
///
/// # Returns
/// The current timestamp (u64)
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::time::current_time;
/// # let env = Env::default();
/// let now = current_time(&env);
/// ```
pub fn current_time(env: &Env) -> u64 {
    env.ledger().timestamp()
}

/// Advances the ledger timestamp by the specified number of seconds.
///
/// # Arguments
/// * `env` - The contract environment
/// * `seconds` - Number of seconds to advance
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::time::advance_time;
/// # let env = Env::default();
/// advance_time(&env, 3600); // Advance by 1 hour
/// ```
pub fn advance_time(env: &Env, seconds: u64) {
    let current = env.ledger().timestamp();
    env.ledger().set_timestamp(current + seconds);
}

/// Sets the ledger timestamp to a specific value.
///
/// # Arguments
/// * `env` - The contract environment
/// * `timestamp` - The timestamp to set
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::time::set_time;
/// # let env = Env::default();
/// set_time(&env, 1000000);
/// ```
pub fn set_time(env: &Env, timestamp: u64) {
    env.ledger().set_timestamp(timestamp);
}

/// Creates a deadline that is in the past.
///
/// # Arguments
/// * `env` - The contract environment
/// * `seconds_ago` - How many seconds in the past (defaults to 100)
///
/// # Returns
/// A timestamp in the past (u64)
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::time::past_deadline;
/// # let env = Env::default();
/// let deadline = past_deadline(&env, Some(3600)); // 1 hour ago
/// ```
pub fn past_deadline(env: &Env, seconds_ago: Option<u64>) -> u64 {
    let current = env.ledger().timestamp();
    current.saturating_sub(seconds_ago.unwrap_or(100))
}

/// Creates a deadline that is in the future.
///
/// # Arguments
/// * `env` - The contract environment
/// * `seconds_from_now` - How many seconds in the future (defaults to 1000)
///
/// # Returns
/// A timestamp in the future (u64)
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::time::future_deadline;
/// # let env = Env::default();
/// let deadline = future_deadline(&env, Some(86400)); // 1 day from now
/// ```
pub fn future_deadline(env: &Env, seconds_from_now: Option<u64>) -> u64 {
    env.ledger().timestamp() + seconds_from_now.unwrap_or(1000)
}
