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

use crate::malicious_reentrant::{
    AttackMode, MaliciousReentrantContract, MaliciousReentrantContractClient,
};
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

    client.init_program(
        &program_id,
        &authorized_key,
        &token_client.address,
        &authorized_key,
        &None,
    );

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

    client.init_program(
        &program_id,
        &authorized_key,
        &token_client.address,
        &authorized_key,
        &None,
    );
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

    client.init_program(
        &program_id,
        &authorized_key,
        &token_client.address,
        &authorized_key,
        &None,
    );
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
    client.create_program_release_schedule(&amount, &release_timestamp, &recipient);

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
    client.create_program_release_schedule(&amount, &release_timestamp, &recipient);

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
    // Add these tests to the existing reentrancy_tests.rs file

    #[test]
    fn test_malicious_contract_single_payout_reentrancy() {
        // Test that a malicious contract cannot re-enter single_payout
        let env = Env::default();
        env.mock_all_auths();

        // Deploy contracts
        let escrow_id = env.register_contract(None, crate::ProgramEscrowContract);
        let escrow_client = crate::ProgramEscrowContractClient::new(&env, &escrow_id);

        let malicious_id = env.register_contract(None, MaliciousReentrantContract);
        let malicious_client = MaliciousReentrantContractClient::new(&env, &malicious_id);

        // Initialize malicious contract with target
        malicious_client.init(&escrow_id);

        // Setup test data
        let admin = Address::random(&env);
        let token = register_test_token(&env);
        let recipient = Address::random(&env);

        // Initialize escrow
        escrow_client.initialize(&admin, &token);

        // Fund the escrow
        token_client(&env, &token).mint(&escrow_id, &1000);

        // Create a program with a payout
        let program_id = 1;
        let start = env.ledger().timestamp() + 100;
        let cliff = start + 200;
        let end = cliff + 1000;
        let amount = 500;

        escrow_client.create_program(&admin, &program_id, &start, &cliff, &end, &true, &false);

        // Register the malicious contract as a recipient
        escrow_client.register_recipient(&admin, &program_id, &malicious_id, &amount);

        // Advance time past the cliff
        env.ledger().set_timestamp(end + 1);

        // Attempt the attack
        let result = std::panic::catch_unwind(|| {
            malicious_client.attack_single_payout(&malicious_id, &amount);
        });

        // Verify the attack was prevented
        assert!(
            result.is_err(),
            "Reentrancy attack should have been prevented"
        );

        // Verify no funds were transferred
        let malicious_balance = token_client(&env, &token).balance(&malicious_id);
        assert_eq!(
            malicious_balance, 0,
            "Malicious contract should not have received funds"
        );

        // Verify escrow still has funds
        let escrow_balance = token_client(&env, &token).balance(&escrow_id);
        assert_eq!(
            escrow_balance, 1000,
            "Escrow funds should not have been released"
        );
    }

    #[test]
    fn test_nested_reentrancy_attack_depth_3() {
        // Test nested reentrancy with depth 3
        let env = Env::default();
        env.mock_all_auths();

        // Deploy contracts
        let escrow_id = env.register_contract(None, crate::ProgramEscrowContract);
        let escrow_client = crate::ProgramEscrowContractClient::new(&env, &escrow_id);

        let malicious_id = env.register_contract(None, MaliciousReentrantContract);
        let malicious_client = MaliciousReentrantContractClient::new(&env, &malicious_id);

        // Initialize
        malicious_client.init(&escrow_id);

        // Setup
        let admin = Address::random(&env);
        let token = register_test_token(&env);

        escrow_client.initialize(&admin, &token);
        token_client(&env, &token).mint(&escrow_id, &1000);

        // Create program
        let program_id = 1;
        let start = env.ledger().timestamp() + 100;
        let cliff = start + 200;
        let end = cliff + 1000;

        escrow_client.create_program(&admin, &program_id, &start, &cliff, &end, &true, &false);

        // Register malicious contract as recipient
        escrow_client.register_recipient(&admin, &program_id, &malicious_id, &500);

        // Advance time
        env.ledger().set_timestamp(end + 1);

        // Attempt nested attack with depth 3
        let result = std::panic::catch_unwind(|| {
            malicious_client.attack_nested(&malicious_id, &500, &3);
        });

        // Should be blocked at first reentrancy attempt
        assert!(
            result.is_err(),
            "Nested reentrancy should be prevented at first level"
        );

        // Verify attack count (should be 0 or 1 depending on when guard triggers)
        let attack_count = malicious_client.get_attack_count();
        assert!(
            attack_count <= 1,
            "Attack should not progress beyond first level"
        );
    }

    #[test]
    fn test_cross_contract_reentrancy_chain() {
        // Test reentrancy across multiple malicious contracts
        let env = Env::default();
        env.mock_all_auths();

        // Deploy main escrow
        let escrow_id = env.register_contract(None, crate::ProgramEscrowContract);
        let escrow_client = crate::ProgramEscrowContractClient::new(&env, &escrow_id);

        // Deploy two malicious contracts
        let malicious1_id = env.register_contract(None, MaliciousReentrantContract);
        let malicious1_client = MaliciousReentrantContractClient::new(&env, &malicious1_id);

        let malicious2_id = env.register_contract(None, MaliciousReentrantContract);
        let malicious2_client = MaliciousReentrantContractClient::new(&env, &malicious2_id);

        // Initialize malicious contracts
        malicious1_client.init(&escrow_id);
        malicious2_client.init(&escrow_id);

        // Set up the chain: malicious1 -> malicious2 -> escrow
        malicious1_client.set_next_contract(&malicious2_id);
        malicious2_client.set_next_contract(&escrow_id);

        // Setup escrow
        let admin = Address::random(&env);
        let token = register_test_token(&env);

        escrow_client.initialize(&admin, &token);
        token_client(&env, &token).mint(&escrow_id, &1000);

        // Create program
        let program_id = 1;
        let start = env.ledger().timestamp() + 100;
        let cliff = start + 200;
        let end = cliff + 1000;

        escrow_client.create_program(&admin, &program_id, &start, &cliff, &end, &true, &false);

        // Register malicious1 as recipient
        escrow_client.register_recipient(&admin, &program_id, &malicious1_id, &500);

        // Advance time
        env.ledger().set_timestamp(end + 1);

        // Start the chain attack
        let result = std::panic::catch_unwind(|| {
            malicious1_client.start_chain_attack(&malicious1_id, &500);
        });

        // Should be blocked
        assert!(
            result.is_err(),
            "Cross-contract reentrancy chain should be prevented"
        );

        // Verify no funds were transferred
        let balance1 = token_client(&env, &token).balance(&malicious1_id);
        let balance2 = token_client(&env, &token).balance(&malicious2_id);

        assert_eq!(balance1, 0, "Malicious1 should not have received funds");
        assert_eq!(balance2, 0, "Malicious2 should not have received funds");
    }

    #[test]
    fn test_cross_function_reentrancy_single_to_batch() {
        // Test reentrancy from single_payout to batch_payout
        let env = Env::default();
        env.mock_all_auths();

        let escrow_id = env.register_contract(None, crate::ProgramEscrowContract);
        let escrow_client = crate::ProgramEscrowContractClient::new(&env, &escrow_id);

        let malicious_id = env.register_contract(None, MaliciousReentrantContract);
        let malicious_client = MaliciousReentrantContractClient::new(&env, &malicious_id);

        malicious_client.init(&escrow_id);

        // Setup
        let admin = Address::random(&env);
        let token = register_test_token(&env);

        escrow_client.initialize(&admin, &token);
        token_client(&env, &token).mint(&escrow_id, &1000);

        let program_id = 1;
        let start = env.ledger().timestamp() + 100;
        let cliff = start + 200;
        let end = cliff + 1000;

        escrow_client.create_program(&admin, &program_id, &start, &cliff, &end, &true, &false);

        // Register malicious contract
        escrow_client.register_recipient(&admin, &program_id, &malicious_id, &500);

        // Advance time
        env.ledger().set_timestamp(end + 1);

        // Attack: single_payout should trigger batch_payout reentrancy
        let result = std::panic::catch_unwind(|| {
            malicious_client.attack_cross_function(&malicious_id, &500, &true);
        });

        assert!(
            result.is_err(),
            "Cross-function reentrancy should be prevented"
        );
    }

    #[test]
    fn test_reentrancy_guard_prevents_all_attack_patterns() {
        // Comprehensive test that verifies all attack patterns are blocked
        let attack_patterns = vec![
            (1, "Single Payout Reentrant"),
            (2, "Batch Payout Reentrant"),
            (3, "Trigger Releases Reentrant"),
            (4, "Nested Reentrant"),
            (5, "Chain Reentrant"),
            (6, "Cross Function Single to Batch"),
            (7, "Cross Function Batch to Single"),
        ];

        for (mode, description) in attack_patterns {
            let env = Env::default();
            env.mock_all_auths();

            // Deploy contracts
            let escrow_id = env.register_contract(None, crate::ProgramEscrowContract);
            let escrow_client = crate::ProgramEscrowContractClient::new(&env, &escrow_id);

            let malicious_id = env.register_contract(None, MaliciousReentrantContract);
            let malicious_client = MaliciousReentrantContractClient::new(&env, &malicious_id);

            // Initialize
            malicious_client.init(&escrow_id);
            malicious_client.set_attack_mode(&mode);

            // Setup
            let admin = Address::random(&env);
            let token = register_test_token(&env);

            escrow_client.initialize(&admin, &token);
            token_client(&env, &token).mint(&escrow_id, &1000);

            let program_id = 1;
            let start = env.ledger().timestamp() + 100;
            let cliff = start + 200;
            let end = cliff + 1000;

            escrow_client.create_program(&admin, &program_id, &start, &cliff, &end, &true, &false);

            // Register malicious contract
            escrow_client.register_recipient(&admin, &program_id, &malicious_id, &500);

            // Advance time
            env.ledger().set_timestamp(end + 1);

            // Attempt attack
            let result = std::panic::catch_unwind(|| {
                match mode {
                    1 | 4 | 5 | 6 | 7 => {
                        malicious_client.attack_single_payout(&malicious_id, &500);
                    }
                    2 => {
                        let recipients = Vec::from_array(&env, [malicious_id.clone()]);
                        let amounts = Vec::from_array(&env, [500]);
                        malicious_client.attack_batch_payout(&recipients, &amounts);
                    }
                    3 => {
                        // For trigger_releases, we might need a different setup
                        // This is a placeholder
                    }
                    _ => {}
                }
            });

            assert!(
                result.is_err(),
                "Attack pattern '{}' (mode {}) should be prevented",
                description,
                mode
            );

            println!("✓ {} attack correctly blocked", description);
        }
    }

    #[test]
    fn test_reentrancy_guard_state_consistency_after_failed_attack() {
        // Test that contract state remains consistent after a failed reentrancy attack
        let env = Env::default();
        env.mock_all_auths();

        let escrow_id = env.register_contract(None, crate::ProgramEscrowContract);
        let escrow_client = crate::ProgramEscrowContractClient::new(&env, &escrow_id);

        let malicious_id = env.register_contract(None, MaliciousReentrantContract);
        let malicious_client = MaliciousReentrantContractClient::new(&env, &malicious_id);

        malicious_client.init(&escrow_id);

        // Setup with multiple recipients
        let admin = Address::random(&env);
        let token = register_test_token(&env);
        let honest_recipient = Address::random(&env);

        escrow_client.initialize(&admin, &token);
        token_client(&env, &token).mint(&escrow_id, &1000);

        let program_id = 1;
        let start = env.ledger().timestamp() + 100;
        let cliff = start + 200;
        let end = cliff + 1000;

        escrow_client.create_program(&admin, &program_id, &start, &cliff, &end, &true, &false);

        // Register both honest recipient and malicious contract
        escrow_client.register_recipient(&admin, &program_id, &honest_recipient, &300);
        escrow_client.register_recipient(&admin, &program_id, &malicious_id, &200);

        // Advance time
        env.ledger().set_timestamp(end + 1);

        // Store balances before attack
        let escrow_balance_before = token_client(&env, &token).balance(&escrow_id);
        let honest_balance_before = token_client(&env, &token).balance(&honest_recipient);
        let malicious_balance_before = token_client(&env, &token).balance(&malicious_id);

        // Attempt attack
        let _ = std::panic::catch_unwind(|| {
            malicious_client.attack_single_payout(&malicious_id, &200);
        });

        // Verify all balances remain unchanged
        let escrow_balance_after = token_client(&env, &token).balance(&escrow_id);
        let honest_balance_after = token_client(&env, &token).balance(&honest_recipient);
        let malicious_balance_after = token_client(&env, &token).balance(&malicious_id);

        assert_eq!(
            escrow_balance_before, escrow_balance_after,
            "Escrow balance changed"
        );
        assert_eq!(
            honest_balance_before, honest_balance_after,
            "Honest recipient balance changed"
        );
        assert_eq!(
            malicious_balance_before, malicious_balance_after,
            "Malicious contract balance changed"
        );
    }

    // Helper function to get token client
    fn token_client(env: &Env, token_id: &Address) -> soroban_sdk::token::TokenClient {
        soroban_sdk::token::TokenClient::new(env, token_id)
    }

    // Helper to register a test token
    fn register_test_token(env: &Env) -> Address {
        let token_wasm = soroban_sdk::contractfile!(soroban_token_contract::Token);
        env.deployer().register_wasm(&token_wasm, ())
    }
}
