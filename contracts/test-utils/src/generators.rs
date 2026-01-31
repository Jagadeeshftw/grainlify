//! Test data generators.
//!
//! Provides functions to generate common test data values.

use soroban_sdk::{Address, Env};

/// Generates a test bounty ID.
///
/// # Arguments
/// * `index` - Optional index for generating different IDs (defaults to 1)
///
/// # Returns
/// A bounty ID (u64)
///
/// # Example
/// ```rust,no_run
/// # use test_utils::generators::generate_bounty_id;
/// let bounty_id = generate_bounty_id(None);
/// ```
pub fn generate_bounty_id(index: Option<u64>) -> u64 {
    index.unwrap_or(1)
}

/// Generates a test amount.
///
/// # Arguments
/// * `base` - Base amount (defaults to 1000)
/// * `multiplier` - Optional multiplier
///
/// # Returns
/// An amount (i128)
///
/// # Example
/// ```rust,no_run
/// # use test_utils::generators::generate_amount;
/// let amount = generate_amount(1000, Some(10)); // Returns 10000
/// ```
pub fn generate_amount(base: i128, multiplier: Option<i128>) -> i128 {
    base * multiplier.unwrap_or(1)
}

/// Generates a deadline timestamp.
///
/// # Arguments
/// * `env` - The contract environment
/// * `offset_seconds` - Offset in seconds from current time (defaults to 1000)
///
/// # Returns
/// A deadline timestamp (u64)
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::generators::generate_deadline;
/// # let env = Env::default();
/// let deadline = generate_deadline(&env, Some(3600)); // 1 hour from now
/// ```
pub fn generate_deadline(env: &Env, offset_seconds: Option<u64>) -> u64 {
    env.ledger().timestamp() + offset_seconds.unwrap_or(1000)
}

/// Generates multiple addresses.
///
/// # Arguments
/// * `env` - The contract environment
/// * `count` - Number of addresses to generate
///
/// # Returns
/// A vector of addresses
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::generators::generate_addresses;
/// # let env = Env::default();
/// let addresses = generate_addresses(&env, 5);
/// ```
pub fn generate_addresses(env: &Env, count: u32) -> Vec<Address> {
    let mut addresses = Vec::new(env);
    for _ in 0..count {
        addresses.push_back(Address::generate(env));
    }
    addresses.to_array()
}

/// Generates a standard test amount (1000).
///
/// # Returns
/// A standard test amount (i128)
pub fn standard_amount() -> i128 {
    1000
}

/// Generates a large test amount (1,000,000).
///
/// # Returns
/// A large test amount (i128)
pub fn large_amount() -> i128 {
    1_000_000
}

/// Generates a small test amount (100).
///
/// # Returns
/// A small test amount (i128)
pub fn small_amount() -> i128 {
    100
}
