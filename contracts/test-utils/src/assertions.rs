//! Assertion utilities for common test scenarios.
//!
//! Provides helper functions for common assertions in contract tests.

use soroban_sdk::{token, Address};

#[cfg(test)]
use bounty_escrow::{BountyEscrowContractClient, EscrowStatus};

#[cfg(test)]
/// Asserts that an escrow has the expected status.
///
/// # Arguments
/// * `escrow_client` - The escrow contract client
/// * `bounty_id` - The bounty ID
/// * `expected_status` - The expected escrow status
///
/// # Panics
/// Panics if the escrow status doesn't match the expected status.
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use bounty_escrow::{BountyEscrowContractClient, EscrowStatus};
/// # use test_utils::assertions::assert_escrow_status;
/// # let env = Env::default();
/// # let escrow = BountyEscrowContractClient::new(&env, &Address::generate(&env));
/// assert_escrow_status(&escrow, 1, EscrowStatus::Locked);
/// ```
pub fn assert_escrow_status(
    escrow_client: &BountyEscrowContractClient,
    bounty_id: u64,
    expected_status: EscrowStatus,
) {
    let escrow = escrow_client.get_escrow_info(&bounty_id);
    assert_eq!(
        escrow.status, expected_status,
        "Expected escrow {} to have status {:?}, but got {:?}",
        bounty_id, expected_status, escrow.status
    );
}

#[cfg(test)]
/// Asserts that an escrow has the expected amount.
///
/// # Arguments
/// * `escrow_client` - The escrow contract client
/// * `bounty_id` - The bounty ID
/// * `expected_amount` - The expected amount
///
/// # Panics
/// Panics if the escrow amount doesn't match the expected amount.
pub fn assert_escrow_amount(
    escrow_client: &BountyEscrowContractClient,
    bounty_id: u64,
    expected_amount: i128,
) {
    let escrow = escrow_client.get_escrow_info(&bounty_id);
    assert_eq!(
        escrow.amount, expected_amount,
        "Expected escrow {} to have amount {}, but got {}",
        bounty_id, expected_amount, escrow.amount
    );
}

/// Asserts that a token balance matches the expected value.
///
/// # Arguments
/// * `token_client` - The token client
/// * `address` - The address to check
/// * `expected_balance` - The expected balance
///
/// # Panics
/// Panics if the balance doesn't match the expected value.
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::{token, Address, Env};
/// # use test_utils::assertions::assert_balance;
/// # let env = Env::default();
/// # let token = token::Client::new(&env, &Address::generate(&env));
/// # let addr = Address::generate(&env);
/// assert_balance(&token, &addr, 1000);
/// ```
pub fn assert_balance(token_client: &token::Client, address: &Address, expected_balance: i128) {
    let balance = token_client.balance(address);
    assert_eq!(
        balance, expected_balance,
        "Expected address {:?} to have balance {}, but got {}",
        address, expected_balance, balance
    );
}

#[cfg(test)]
/// Asserts that an escrow exists.
///
/// # Arguments
/// * `escrow_client` - The escrow contract client
/// * `bounty_id` - The bounty ID
///
/// # Panics
/// Panics if the escrow doesn't exist (will panic on get_escrow_info).
pub fn assert_escrow_exists(escrow_client: &BountyEscrowContractClient, bounty_id: u64) {
    let _escrow = escrow_client.get_escrow_info(&bounty_id);
    // If we get here, the escrow exists
}

/// Asserts that balances match expected values after a transaction.
///
/// # Arguments
/// * `token_client` - The token client
/// * `expected_balances` - A slice of (address, expected_balance) tuples
///
/// # Panics
/// Panics if any balance doesn't match the expected value.
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::{token, Address, Env};
/// # use test_utils::assertions::assert_balances;
/// # let env = Env::default();
/// # let token = token::Client::new(&env, &Address::generate(&env));
/// # let addr1 = Address::generate(&env);
/// # let addr2 = Address::generate(&env);
/// assert_balances(&token, &[(&addr1, 1000), (&addr2, 500)]);
/// ```
pub fn assert_balances(
    token_client: &token::Client,
    expected_balances: &[(&Address, i128)],
) {
    for (address, expected_balance) in expected_balances {
        assert_balance(token_client, address, *expected_balance);
    }
}

#[cfg(test)]
/// Asserts that an escrow has the expected depositor.
///
/// # Arguments
/// * `escrow_client` - The escrow contract client
/// * `bounty_id` - The bounty ID
/// * `expected_depositor` - The expected depositor address
pub fn assert_escrow_depositor(
    escrow_client: &BountyEscrowContractClient,
    bounty_id: u64,
    expected_depositor: &Address,
) {
    let escrow = escrow_client.get_escrow_info(&bounty_id);
    assert_eq!(
        escrow.depositor, *expected_depositor,
        "Expected escrow {} to have depositor {:?}, but got {:?}",
        bounty_id, expected_depositor, escrow.depositor
    );
}

#[cfg(test)]
/// Asserts that an escrow has the expected deadline.
///
/// # Arguments
/// * `escrow_client` - The escrow contract client
/// * `bounty_id` - The bounty ID
/// * `expected_deadline` - The expected deadline timestamp
pub fn assert_escrow_deadline(
    escrow_client: &BountyEscrowContractClient,
    bounty_id: u64,
    expected_deadline: u64,
) {
    let escrow = escrow_client.get_escrow_info(&bounty_id);
    assert_eq!(
        escrow.deadline, expected_deadline,
        "Expected escrow {} to have deadline {}, but got {}",
        bounty_id, expected_deadline, escrow.deadline
    );
}
