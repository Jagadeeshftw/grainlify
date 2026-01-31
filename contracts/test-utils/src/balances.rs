//! Balance verification helpers.
//!
//! Provides functions to verify token balances in tests.

use soroban_sdk::{token, Address};

/// Verifies that a balance change occurred.
///
/// # Arguments
/// * `token_client` - The token client
/// * `address` - The address to check
/// * `initial_balance` - The initial balance
/// * `expected_change` - The expected change (positive or negative)
///
/// # Returns
/// The new balance (i128)
///
/// # Panics
/// Panics if the balance change doesn't match the expected change.
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::{token, Address, Env};
/// # use test_utils::balances::verify_balance_change;
/// # let env = Env::default();
/// # let token = token::Client::new(&env, &Address::generate(&env));
/// # let addr = Address::generate(&env);
/// let initial = token.balance(&addr);
/// // ... perform transaction ...
/// let new_balance = verify_balance_change(&token, &addr, initial, 1000);
/// ```
pub fn verify_balance_change(
    token_client: &token::Client,
    address: &Address,
    initial_balance: i128,
    expected_change: i128,
) -> i128 {
    let new_balance = token_client.balance(address);
    let actual_change = new_balance - initial_balance;
    
    assert_eq!(
        actual_change, expected_change,
        "Expected balance change of {} for address {:?}, but got {} (initial: {}, new: {})",
        expected_change, address, actual_change, initial_balance, new_balance
    );
    
    new_balance
}

/// Gets the initial balance before a transaction.
///
/// # Arguments
/// * `token_client` - The token client
/// * `address` - The address to check
///
/// # Returns
/// The initial balance (i128)
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::{token, Address, Env};
/// # use test_utils::balances::get_initial_balance;
/// # let env = Env::default();
/// # let token = token::Client::new(&env, &Address::generate(&env));
/// # let addr = Address::generate(&env);
/// let initial = get_initial_balance(&token, &addr);
/// // ... perform transaction ...
/// let new = token.balance(&addr);
/// assert_eq!(new - initial, 1000);
/// ```
pub fn get_initial_balance(token_client: &token::Client, address: &Address) -> i128 {
    token_client.balance(address)
}

/// Verifies that balances are zero for multiple addresses.
///
/// # Arguments
/// * `token_client` - The token client
/// * `addresses` - A slice of addresses to check
///
/// # Panics
/// Panics if any address has a non-zero balance.
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::{token, Address, Env};
/// # use test_utils::balances::verify_all_zero;
/// # let env = Env::default();
/// # let token = token::Client::new(&env, &Address::generate(&env));
/// # let addr1 = Address::generate(&env);
/// # let addr2 = Address::generate(&env);
/// verify_all_zero(&token, &[&addr1, &addr2]);
/// ```
pub fn verify_all_zero(token_client: &token::Client, addresses: &[&Address]) {
    for address in addresses {
        let balance = token_client.balance(address);
        assert_eq!(
            balance, 0,
            "Expected address {:?} to have zero balance, but got {}",
            address, balance
        );
    }
}
