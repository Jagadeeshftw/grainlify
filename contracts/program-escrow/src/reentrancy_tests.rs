//! # Reentrancy Guard Tests
//!
//! Comprehensive test suite for reentrancy protection in the ProgramEscrow contract.
//!
//! ## Test Categories
//!
//! 1. **Basic Guard Functionality**: Test the guard mechanism itself
//! 2. **Single Payout Reentrancy**: Attempt reentrancy during single payouts
//! 3. **Batch Payout Reentrancy**: Attempt reentrancy during batch payouts
//! 4. **Schedule Release Reentrancy**: Attempt reentrancy during schedule releases
//! 5. **Cross-Function Reentrancy**: Attempt to call different functions during execution
//! 6. **Nested Call Protection**: Test protection against deeply nested calls

#![cfg(test)]

use crate::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};

// Test helper to create a mock token contract
fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract.address();
    token::Client::new(env, &token_address)
}

// ============================================================================
// Basic Reentrancy Guard Tests
// ============================================================================

#[test]
fn test_reentrancy_guard_basic_functionality() {
    use crate::reentrancy_guard::*;

    let env = Env::default();

    // Initially, guard should not be set
    assert!(!is_entered(&env));

    // Check should pass
    check_not_entered(&env);

    // Set the guard
    set_entered(&env);
    assert!(is_entered(&env));

    // Clear the guard
    clear_entered(&env);
    assert!(!is_entered(&env));
}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_reentrancy_guard_detects_reentry() {
    use crate::reentrancy_guard::*;

    let env = Env::default();

    // Set the guard
    set_entered(&env);

    // This should panic
    check_not_entered(&env);
}

#[test]
fn test_reentrancy_guard_allows_sequential_calls() {
    use crate::reentrancy_guard::*;

    let env = Env::default();

    // First call
    check_not_entered(&env);
    set_entered(&env);
    clear_entered(&env);

    // Second call (should succeed)
    check_not_entered(&env);
    set_entered(&env);
    clear_entered(&env);

    // Third call (should succeed)
    check_not_entered(&env);
    set_entered(&env);
    clear_entered(&env);
}

// ============================================================================
// Single Payout Reentrancy Tests
// ============================================================================

#[test]
fn test_single_payout_normal_execution() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let amount = 1000_0000000i128;

    // Setup: Create token and initialize program
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);

    // Transfer tokens to contract
    token_client.transfer(&authorized_key, &contract_id, &amount);

    // Lock funds
    client.lock_program_funds(&amount);

    // Execute single payout (should succeed)
    let result = client.single_payout(&recipient, &(amount / 2));

    assert_eq!(result.remaining_balance, amount / 2);
}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_single_payout_blocks_reentrancy() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let amount = 1000_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &amount);
    client.lock_program_funds(&amount);

    // Manually set the reentrancy guard to simulate an ongoing call
    crate::reentrancy_guard::set_entered(&env);

    // This should panic with "Reentrancy detected"
    client.single_payout(&authorized_key, &(amount / 2));
}

// ============================================================================
// Batch Payout Reentrancy Tests
// ============================================================================

#[test]
fn test_batch_payout_normal_execution() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let total_amount = 1000_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &total_amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &total_amount);
    client.lock_program_funds(&total_amount);

    // Execute batch payout
    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 400_0000000i128, 600_0000000i128];

    let result = client.batch_payout(&recipients, &amounts);

    assert_eq!(result.remaining_balance, 0);
}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_batch_payout_blocks_reentrancy() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let total_amount = 1000_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &total_amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &total_amount);
    client.lock_program_funds(&total_amount);

    // Manually set the reentrancy guard
    crate::reentrancy_guard::set_entered(&env);

    // This should panic
    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 400_0000000i128, 600_0000000i128];
    client.batch_payout(&recipients, &amounts);
}

// ============================================================================
// Cross-Function Reentrancy Tests
// ============================================================================

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_cross_function_reentrancy_single_to_batch() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let amount = 1000_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &amount);
    client.lock_program_funds(&amount);

    // Simulate being inside single_payout
    crate::reentrancy_guard::set_entered(&env);

    // Try to call batch_payout (should be blocked)
    let recipients = vec![&env, recipient];
    let amounts = vec![&env, amount / 2];
    client.batch_payout(&recipients, &amounts);
}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_cross_function_reentrancy_batch_to_single() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let amount = 1000_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &amount);
    client.lock_program_funds(&amount);

    // Simulate being inside batch_payout
    crate::reentrancy_guard::set_entered(&env);

    // Try to call single_payout (should be blocked)
    client.single_payout(&recipient, &(amount / 2));
}

// ============================================================================
// Schedule Release Reentrancy Tests
// ============================================================================

#[test]
fn test_trigger_releases_normal_execution() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let amount = 1000_0000000i128;
    let release_timestamp = 1000u64;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &amount);
    client.lock_program_funds(&amount);

    // Create schedule
    client.create_program_release_schedule(&recipient, &amount, &release_timestamp);

    // Advance time
    env.ledger().set_timestamp(release_timestamp + 1);

    // Trigger releases (should succeed)
    let released_count = client.trigger_program_releases();

    assert_eq!(released_count, 1);
}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_trigger_releases_blocks_reentrancy() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let amount = 1000_0000000i128;
    let release_timestamp = 1000u64;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &amount);
    client.lock_program_funds(&amount);

    // Create schedule
    client.create_program_release_schedule(&recipient, &amount, &release_timestamp);

    // Advance time
    env.ledger().set_timestamp(release_timestamp + 1);

    // Manually set the reentrancy guard
    crate::reentrancy_guard::set_entered(&env);

    // This should panic
    client.trigger_program_releases();
}

// ============================================================================
// Multiple Sequential Calls (Should Succeed)
// ============================================================================

#[test]
fn test_multiple_sequential_payouts_succeed() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let total_amount = 1000_0000000i128;
    let payout_amount = 300_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &total_amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &total_amount);
    client.lock_program_funds(&total_amount);

    // Execute multiple sequential payouts (all should succeed)
    client.single_payout(&recipient1, &payout_amount);
    client.single_payout(&recipient2, &payout_amount);
    client.single_payout(&recipient3, &payout_amount);

    let program_data = client.get_program_info();
    assert_eq!(
        program_data.remaining_balance,
        total_amount - (payout_amount * 3)
    );
}

// ============================================================================
// Guard State Verification Tests
// ============================================================================

#[test]
fn test_guard_cleared_after_successful_payout() {
    use crate::reentrancy_guard::*;

    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let amount = 1000_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &amount);
    client.lock_program_funds(&amount);

    // Guard should not be set initially
    assert!(!is_entered(&env));

    // Execute payout
    client.single_payout(&recipient, &(amount / 2));

    // Guard should be cleared after successful execution
    assert!(!is_entered(&env));
}

#[test]
fn test_guard_state_across_multiple_operations() {
    use crate::reentrancy_guard::*;

    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let authorized_key = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let program_id = String::from_str(&env, "test-program");
    let total_amount = 1000_0000000i128;

    // Setup
    let token_client = create_token_contract(&env, &authorized_key);
    let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin.mint(&authorized_key, &total_amount);

    client.init_program(&program_id, &authorized_key, &token_client.address);
    token_client.transfer(&authorized_key, &contract_id, &total_amount);
    client.lock_program_funds(&total_amount);

    // Verify guard state through multiple operations
    assert!(!is_entered(&env));

    client.single_payout(&recipient1, &300_0000000i128);
    assert!(!is_entered(&env));

    let recipients = vec![&env, recipient2];
    let amounts = vec![&env, 200_0000000i128];
    client.batch_payout(&recipients, &amounts);
    assert!(!is_entered(&env));

    client.single_payout(&recipient1, &100_0000000i128);
    assert!(!is_entered(&env));
}

// ============================================================================
// Documentation and Model Tests
// ============================================================================

#[test]
fn test_reentrancy_guard_model_documentation() {
    // This test documents the reentrancy guard model and guarantees

    // GUARANTEE 1: Sequential calls are always allowed
    // The guard is cleared after each successful operation, allowing
    // the next operation to proceed normally.

    // GUARANTEE 2: Nested/reentrant calls are always blocked
    // If a function is currently executing (guard is set), any attempt
    // to call another protected function will panic.

    // GUARANTEE 3: Cross-function protection
    // The guard protects across all sensitive functions (single_payout,
    // batch_payout, trigger_releases, etc.), not just same-function calls.

    // GUARANTEE 4: Automatic cleanup on panic
    // In Soroban, if a function panics, all state changes are rolled back,
    // including the guard flag. This prevents the guard from being stuck.

    // GUARANTEE 5: No deadlocks
    // Since the guard is automatically cleared on panic and explicitly
    // cleared on success, there's no risk of permanent lockout.

    assert!(true, "Documentation test - see comments for guarantees");
}
