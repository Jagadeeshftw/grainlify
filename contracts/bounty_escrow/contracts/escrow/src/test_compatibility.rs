#![cfg(test)]

//! Backwards Compatibility Tests
//!
//! These tests ensure that contract changes maintain backward compatibility
//! according to the policy defined in BACKWARDS_COMPATIBILITY_POLICY.md

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

/// Test that all error codes remain stable
#[test]
fn test_error_codes_are_stable() {
    // This test documents the expected error code numbers.
    // If this test fails, you've changed error codes which is BREAKING.
    
    assert_eq!(Error::AlreadyInitialized as u32, 1);
    assert_eq!(Error::NotInitialized as u32, 2);
    assert_eq!(Error::BountyExists as u32, 3);
    assert_eq!(Error::BountyNotFound as u32, 4);
    assert_eq!(Error::FundsNotLocked as u32, 5);
    assert_eq!(Error::DeadlineNotPassed as u32, 6);
    assert_eq!(Error::Unauthorized as u32, 7);
    assert_eq!(Error::InvalidFeeRate as u32, 8);
    assert_eq!(Error::FeeRecipientNotSet as u32, 9);
    assert_eq!(Error::InvalidBatchSize as u32, 10);
    assert_eq!(Error::BatchSizeMismatch as u32, 11);
    assert_eq!(Error::DuplicateBountyId as u32, 12);
    assert_eq!(Error::InvalidAmount as u32, 13);
    assert_eq!(Error::InvalidDeadline as u32, 14);
    assert_eq!(Error::InsufficientFunds as u32, 16);
    assert_eq!(Error::RefundNotApproved as u32, 17);
    assert_eq!(Error::FundsPaused as u32, 18);
    assert_eq!(Error::AmountBelowMinimum as u32, 19);
    assert_eq!(Error::AmountAboveMaximum as u32, 20);
    assert_eq!(Error::NotPaused as u32, 21);
}

/// Test that core function signatures haven't changed
#[test]
fn test_core_function_signatures_stable() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    
    env.mock_all_auths();
    
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    
    // These function calls should compile without errors.
    // If signatures change, this test will fail to compile.
    
    // Core functions
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1000);
    client.lock_funds(&depositor, &1, &1000, &100);
    client.release_funds(&1, &contributor);
    
    // Query functions
    let _ = client.get_escrow_info(&1);
    let _ = client.get_balance();
    let _ = client.get_fee_config();
}

/// Test that storage keys remain accessible
#[test]
fn test_storage_keys_accessible() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    
    env.mock_all_auths();
    
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    
    // Initialize and create escrow
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1000);
    client.lock_funds(&depositor, &1, &1000, &100);
    
    // Verify we can still access stored data
    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.amount, 1000);
    
    let balance = client.get_balance();
    assert_eq!(balance, 1000);
}

/// Test backward compatibility with old client code
#[test]
fn test_old_client_compatibility() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    
    env.mock_all_auths();
    
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    
    // Simulate old client code that doesn't use new features
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1000);
    client.lock_funds(&depositor, &1, &1000, &100);
    client.release_funds(&1, &contributor);
    
    // Old client code should work without errors
    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, crate::EscrowStatus::Released);
}

/// Test that optional parameters work with defaults
#[test]
fn test_optional_parameters_backward_compatible() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    env.mock_all_auths();
    
    let token_admin = Address::generate(&env);
    let (token, _token_client, _token_admin_client) = create_token_contract(&env, &token_admin);
    
    client.init(&admin, &token);
    
    // Test that update_fee_config works with None values (optional params)
    client.update_fee_config(&None, &None, &None, &None);
    
    // Config should remain unchanged
    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 0);
    assert_eq!(config.release_fee_rate, 0);
}

/// Test that new features don't break existing workflows
#[test]
fn test_new_features_dont_break_existing_workflows() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    
    env.mock_all_auths();
    
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    
    // Standard workflow without using new features
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1000);
    client.lock_funds(&depositor, &1, &1000, &100);
    
    // New feature: amount policy (should not affect existing escrows)
    client.set_amount_policy(&admin, &100, &10000);
    
    // Old escrow should still be releasable
    client.release_funds(&1, &contributor);
    
    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, crate::EscrowStatus::Released);
}

/// Test that deprecated features still work
#[test]
fn test_deprecated_features_still_functional() {
    // This test ensures that deprecated features continue to work
    // until they are removed in a MAJOR version bump.
    
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    env.mock_all_auths();
    
    let token_admin = Address::generate(&env);
    let (token, _token_client, _token_admin_client) = create_token_contract(&env, &token_admin);
    
    client.init(&admin, &token);
    
    // If any functions are deprecated, test them here
    // Example: client.old_function_name(...)
    
    // Currently no deprecated functions, but this test
    // serves as a template for future deprecations
}
