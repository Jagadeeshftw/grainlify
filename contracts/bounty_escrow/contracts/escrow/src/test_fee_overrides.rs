//! Tests for per-escrow fee override functionality
//!
//! This module tests the fee override feature that allows setting custom
//! fee rates for specific escrows, enabling partnerships and promotions.

#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient, Escrow, EscrowStatus, Error};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Helper to create a test escrow with overrides
fn create_test_escrow_with_overrides(
    env: &Env,
    client: &BountyEscrowContractClient,
    admin: &Address,
    depositor: &Address,
    bounty_id: u64,
    amount: i128,
) {
    // Lock funds to create escrow
    client.lock_funds(depositor, &bounty_id, &amount, &(env.ledger().timestamp() + 1000));
}

#[test]
fn test_set_escrow_fee_override_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin, &token);

    // Create escrow
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // Set fee overrides
    let lock_override = Some(100i128); // 1%
    let release_override = Some(200i128); // 2%

    let result = client.try_set_escrow_fee_override(&bounty_id, &lock_override, &release_override);
    assert!(result.is_ok());

    // Verify overrides were set
    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, lock_override);
    assert_eq!(escrow.release_fee_override, release_override);
}

#[test]
fn test_set_escrow_fee_override_zero_fees() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // Set zero fee overrides (promotional/free)
    let result = client.try_set_escrow_fee_override(&bounty_id, &Some(0), &Some(0));
    assert!(result.is_ok());

    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, Some(0));
    assert_eq!(escrow.release_fee_override, Some(0));
}

#[test]
fn test_set_escrow_fee_override_remove_override() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // First set overrides
    client.set_escrow_fee_override(&bounty_id, &Some(100), &Some(200));

    // Then remove them (revert to global)
    let result = client.try_set_escrow_fee_override(&bounty_id, &None, &None);
    assert!(result.is_ok());

    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, None);
    assert_eq!(escrow.release_fee_override, None);
}

#[test]
fn test_set_escrow_fee_override_max_rate() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // Set maximum allowed fee rate (5000 = 50%)
    let result = client.try_set_escrow_fee_override(&bounty_id, &Some(5000), &Some(5000));
    assert!(result.is_ok());

    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, Some(5000));
    assert_eq!(escrow.release_fee_override, Some(5000));
}

#[test]
fn test_set_escrow_fee_override_invalid_rate_too_high() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // Try to set rate above maximum (5001 > 5000)
    let result = client.try_set_escrow_fee_override(&bounty_id, &Some(5001), &None);
    assert_eq!(result, Err(Ok(Error::InvalidFeeRate)));
}

#[test]
fn test_set_escrow_fee_override_invalid_rate_negative() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // Try to set negative rate
    let result = client.try_set_escrow_fee_override(&bounty_id, &Some(-1), &None);
    assert_eq!(result, Err(Ok(Error::InvalidFeeRate)));
}

#[test]
fn test_set_escrow_fee_override_bounty_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let non_existent_bounty_id = 999u64;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);

    // Try to set override for non-existent bounty
    let result = client.try_set_escrow_fee_override(&non_existent_bounty_id, &Some(100), &None);
    assert_eq!(result, Err(Ok(Error::BountyNotFound)));
}

#[test]
fn test_set_escrow_fee_override_partial_override() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // Override only lock fee, leave release fee as global
    let result = client.try_set_escrow_fee_override(&bounty_id, &Some(150), &None);
    assert!(result.is_ok());

    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, Some(150));
    assert_eq!(escrow.release_fee_override, None);
}

#[test]
fn test_set_escrow_fee_override_multiple_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // First override
    client.set_escrow_fee_override(&bounty_id, &Some(100), &Some(200));
    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, Some(100));
    assert_eq!(escrow.release_fee_override, Some(200));

    // Second override (change values)
    client.set_escrow_fee_override(&bounty_id, &Some(300), &Some(400));
    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, Some(300));
    assert_eq!(escrow.release_fee_override, Some(400));

    // Third override (remove)
    client.set_escrow_fee_override(&bounty_id, &None, &None);
    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, None);
    assert_eq!(escrow.release_fee_override, None);
}

#[test]
fn test_new_escrow_has_no_overrides_by_default() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000_0000000i128;

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    client.init(&admin, &token);
    create_test_escrow_with_overrides(&env, &client, &admin, &depositor, bounty_id, amount);

    // Verify new escrow has no overrides
    let escrow = client.get_escrow_info(&bounty_id).unwrap();
    assert_eq!(escrow.lock_fee_override, None);
    assert_eq!(escrow.release_fee_override, None);
}
