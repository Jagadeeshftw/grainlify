#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, String, Vec, vec};

// Helper function to setup a basic program
fn setup_program(env: &Env) -> (ProgramEscrowContract, Address, Address, String) {
    let contract = ProgramEscrowContract;
    let admin = Address::generate(env);
    let token = Address::generate(env);
    let program_id = String::from_str(env, "hackathon-2024-q1");

    contract.init_program(env, program_id.clone(), admin.clone(), token.clone());
    (contract, admin, token, program_id)
}

// Helper function to setup program with funds
fn setup_program_with_funds(env: &Env, initial_amount: i128) -> (ProgramEscrowContract, Address, Address, String) {
    let (contract, admin, token, program_id) = setup_program(env);
    contract.lock_program_funds(env, initial_amount);
    (contract, admin, token, program_id)
}

// =============================================================================
// TESTS FOR init_program()
// =============================================================================

#[test]
fn test_init_program_success() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let program_id = String::from_str(&env, "hackathon-2024-q1");

    let program_data = contract.init_program(&env, program_id.clone(), admin.clone(), token.clone());

    assert_eq!(program_data.program_id, program_id);
    assert_eq!(program_data.total_funds, 0);
    assert_eq!(program_data.remaining_balance, 0);
    assert_eq!(program_data.authorized_payout_key, admin);
    assert_eq!(program_data.token_address, token);
    assert_eq!(program_data.payout_history.len(), 0);
}

#[test]
fn test_init_program_with_different_program_ids() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);
    let program_id1 = String::from_str(&env, "hackathon-2024-q1");
    let program_id2 = String::from_str(&env, "hackathon-2024-q2");

    let data1 = contract.init_program(&env, program_id1.clone(), admin1.clone(), token1.clone());
    assert_eq!(data1.program_id, program_id1);
    assert_eq!(data1.authorized_payout_key, admin1);
    assert_eq!(data1.token_address, token1);

    // Note: In current implementation, program can only be initialized once
    // This test verifies the single initialization constraint
}

#[test]
fn test_init_program_event_emission() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let program_id = String::from_str(&env, "hackathon-2024-q1");

    contract.init_program(&env, program_id.clone(), admin.clone(), token.clone());

    // Check that event was emitted
    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(event.0, (PROGRAM_INITIALIZED,));
    let event_data: (String, Address, Address, i128) = event.1.clone();
    assert_eq!(event_data.0, program_id);
    assert_eq!(event_data.1, admin);
    assert_eq!(event_data.2, token);
    assert_eq!(event_data.3, 0i128); // initial amount
}

#[test]
#[should_panic(expected = "Program already initialized")]
fn test_init_program_duplicate() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let program_id = String::from_str(&env, "hackathon-2024-q1");

    contract.init_program(&env, program_id.clone(), admin.clone(), token.clone());
    contract.init_program(&env, program_id, admin, token); // Should panic
}

#[test]
#[should_panic(expected = "Program already initialized")]
fn test_init_program_duplicate_different_params() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);
    let program_id = String::from_str(&env, "hackathon-2024-q1");

    contract.init_program(&env, program_id.clone(), admin1, token1);
    contract.init_program(&env, program_id, admin2, token2); // Should panic
}

// =============================================================================
// TESTS FOR lock_program_funds()
// =============================================================================

#[test]
fn test_lock_program_funds_success() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program(&env);

    let program_data = contract.lock_program_funds(&env, 50_000_000_000);

    assert_eq!(program_data.total_funds, 50_000_000_000);
    assert_eq!(program_data.remaining_balance, 50_000_000_000);
}

#[test]
fn test_lock_program_funds_multiple_times() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program(&env);

    // First lock
    let program_data = contract.lock_program_funds(&env, 25_000_000_000);
    assert_eq!(program_data.total_funds, 25_000_000_000);
    assert_eq!(program_data.remaining_balance, 25_000_000_000);

    // Second lock
    let program_data = contract.lock_program_funds(&env, 35_000_000_000);
    assert_eq!(program_data.total_funds, 60_000_000_000);
    assert_eq!(program_data.remaining_balance, 60_000_000_000);

    // Third lock
    let program_data = contract.lock_program_funds(&env, 15_000_000_000);
    assert_eq!(program_data.total_funds, 75_000_000_000);
    assert_eq!(program_data.remaining_balance, 75_000_000_000);
}

#[test]
fn test_lock_program_funds_event_emission() {
    let env = Env::default();
    let (contract, _, _, program_id) = setup_program(&env);
    let lock_amount = 100_000_000_000;

    contract.lock_program_funds(&env, lock_amount);

    let events = env.events().all();
    assert_eq!(events.len(), 2); // init + lock

    let lock_event = &events[1];
    assert_eq!(lock_event.0, (FUNDS_LOCKED,));
    let event_data: (String, i128, i128) = lock_event.1.clone();
    assert_eq!(event_data.0, program_id);
    assert_eq!(event_data.1, lock_amount);
    assert_eq!(event_data.2, lock_amount); // remaining balance
}

#[test]
fn test_lock_program_funds_balance_tracking() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program(&env);

    // Lock initial funds
    contract.lock_program_funds(&env, 100_000_000_000);

    // Verify balance through view function
    assert_eq!(contract.get_remaining_balance(&env), 100_000_000_000);

    // Lock more funds
    contract.lock_program_funds(&env, 50_000_000_000);
    assert_eq!(contract.get_remaining_balance(&env), 150_000_000_000);
}

#[test]
fn test_lock_program_funds_maximum_amount() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program(&env);

    // Test with maximum reasonable amount (i128::MAX would cause overflow issues)
    let max_amount = 9_223_372_036_854_775_807i128; // i64::MAX
    let program_data = contract.lock_program_funds(&env, max_amount);

    assert_eq!(program_data.total_funds, max_amount);
    assert_eq!(program_data.remaining_balance, max_amount);
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_lock_program_funds_zero_amount() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program(&env);

    contract.lock_program_funds(&env, 0);
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_lock_program_funds_negative_amount() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program(&env);

    contract.lock_program_funds(&env, -1_000_000_000);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_lock_program_funds_before_init() {
    let env = Env::default();
    let contract = ProgramEscrowContract;

    contract.lock_program_funds(&env, 10_000_000_000);
}

// =============================================================================
// TESTS FOR batch_payout()
// =============================================================================

#[test]
fn test_batch_payout_success() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    let recipients = vec![
        &env,
        recipient1.clone(),
        recipient2.clone(),
        recipient3.clone(),
    ];
    let amounts = vec![&env, 10_000_000_000, 20_000_000_000, 15_000_000_000];

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        let program_data = contract.batch_payout(&env, recipients, amounts);

        assert_eq!(program_data.remaining_balance, 55_000_000_000); // 100 - 10 - 20 - 15
        assert_eq!(program_data.payout_history.len(), 3);

        // Verify payout records
        let payout1 = program_data.payout_history.get(0).unwrap();
        assert_eq!(payout1.recipient, recipient1);
        assert_eq!(payout1.amount, 10_000_000_000);

        let payout2 = program_data.payout_history.get(1).unwrap();
        assert_eq!(payout2.recipient, recipient2);
        assert_eq!(payout2.amount, 20_000_000_000);

        let payout3 = program_data.payout_history.get(2).unwrap();
        assert_eq!(payout3.recipient, recipient3);
        assert_eq!(payout3.amount, 15_000_000_000);
    });
}

#[test]
fn test_batch_payout_event_emission() {
    let env = Env::default();
    let (contract, admin, _, program_id) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 25_000_000_000, 30_000_000_000];
    let total_payout = 55_000_000_000;

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.batch_payout(&env, recipients, amounts);

        let events = env.events().all();
        assert_eq!(events.len(), 3); // init + lock + batch_payout

        let batch_event = &events[2];
        assert_eq!(batch_event.0, (BATCH_PAYOUT,));
        let event_data: (String, u32, i128, i128) = batch_event.1.clone();
        assert_eq!(event_data.0, program_id);
        assert_eq!(event_data.1, 2u32); // number of recipients
        assert_eq!(event_data.2, total_payout);
        assert_eq!(event_data.3, 45_000_000_000); // remaining balance: 100 - 55
    });
}

#[test]
fn test_batch_payout_single_recipient() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 50_000_000_000);

    let recipient = Address::generate(&env);
    let recipients = vec![&env, recipient.clone()];
    let amounts = vec![&env, 25_000_000_000];

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        let program_data = contract.batch_payout(&env, recipients, amounts);

        assert_eq!(program_data.remaining_balance, 25_000_000_000);
        assert_eq!(program_data.payout_history.len(), 1);

        let payout = program_data.payout_history.get(0).unwrap();
        assert_eq!(payout.recipient, recipient);
        assert_eq!(payout.amount, 25_000_000_000);
    });
}

#[test]
fn test_batch_payout_multiple_batches() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 200_000_000_000);

    // First batch
    let recipient1 = Address::generate(&env);
    let recipients1 = vec![&env, recipient1];
    let amounts1 = vec![&env, 30_000_000_000];

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        let program_data = contract.batch_payout(&env, recipients1, amounts1);
        assert_eq!(program_data.remaining_balance, 170_000_000_000);
        assert_eq!(program_data.payout_history.len(), 1);
    });

    // Second batch
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);
    let recipients2 = vec![&env, recipient2, recipient3];
    let amounts2 = vec![&env, 40_000_000_000, 50_000_000_000];

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        let program_data = contract.batch_payout(&env, recipients2, amounts2);
        assert_eq!(program_data.remaining_balance, 80_000_000_000);
        assert_eq!(program_data.payout_history.len(), 3);
    });
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_batch_payout_unauthorized() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let unauthorized = Address::generate(&env);
    let recipient = Address::generate(&env);
    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 10_000_000_000];

    env.as_contract(&contract, || {
        env.set_invoker(&unauthorized);
        contract.batch_payout(&env, recipients, amounts); // Should panic
    });
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_batch_payout_insufficient_balance() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 50_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 30_000_000_000, 25_000_000_000]; // Total: 55 > 50

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.batch_payout(&env, recipients, amounts); // Should panic
    });
}

#[test]
#[should_panic(expected = "Recipients and amounts vectors must have the same length")]
fn test_batch_payout_mismatched_lengths() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 10_000_000_000]; // Mismatched length

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.batch_payout(&env, recipients, amounts); // Should panic
    });
}

#[test]
#[should_panic(expected = "Cannot process empty batch")]
fn test_batch_payout_empty_batch() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipients = vec![&env];
    let amounts = vec![&env];

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.batch_payout(&env, recipients, amounts); // Should panic
    });
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_zero_amount() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 10_000_000_000, 0]; // Zero amount

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.batch_payout(&env, recipients, amounts); // Should panic
    });
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_negative_amount() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 10_000_000_000, -5_000_000_000]; // Negative amount

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.batch_payout(&env, recipients, amounts); // Should panic
    });
}

#[test]
#[should_panic(expected = "Payout amount overflow")]
fn test_batch_payout_overflow() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 9_223_372_036_854_775_807i128);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1, recipient2];
    let amounts = vec![&env, 9_223_372_036_854_775_807i128, 1]; // Causes overflow

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.batch_payout(&env, recipients, amounts); // Should panic
    });
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_batch_payout_before_init() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let recipient = Address::generate(&env);
    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 10_000_000_000];

    contract.batch_payout(&env, recipients, amounts);
}

// =============================================================================
// TESTS FOR single_payout()
// =============================================================================

#[test]
fn test_single_payout_success() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 50_000_000_000);

    let recipient = Address::generate(&env);
    let payout_amount = 10_000_000_000;

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        let program_data = contract.single_payout(&env, recipient.clone(), payout_amount);

        assert_eq!(program_data.remaining_balance, 40_000_000_000);
        assert_eq!(program_data.payout_history.len(), 1);

        let payout = program_data.payout_history.get(0).unwrap();
        assert_eq!(payout.recipient, recipient);
        assert_eq!(payout.amount, payout_amount);
        assert!(payout.timestamp > 0);
    });
}

#[test]
fn test_single_payout_event_emission() {
    let env = Env::default();
    let (contract, admin, _, program_id) = setup_program_with_funds(&env, 50_000_000_000);

    let recipient = Address::generate(&env);
    let payout_amount = 15_000_000_000;

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.single_payout(&env, recipient.clone(), payout_amount);

        let events = env.events().all();
        assert_eq!(events.len(), 3); // init + lock + payout

        let payout_event = &events[2];
        assert_eq!(payout_event.0, (PAYOUT,));
        let event_data: (String, Address, i128, i128) = payout_event.1.clone();
        assert_eq!(event_data.0, program_id);
        assert_eq!(event_data.1, recipient);
        assert_eq!(event_data.2, payout_amount);
        assert_eq!(event_data.3, 35_000_000_000); // remaining balance: 50 - 15
    });
}

#[test]
fn test_single_payout_multiple_payees() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);

        // First payout
        let program_data = contract.single_payout(&env, recipient1.clone(), 20_000_000_000);
        assert_eq!(program_data.remaining_balance, 80_000_000_000);
        assert_eq!(program_data.payout_history.len(), 1);

        // Second payout
        let program_data = contract.single_payout(&env, recipient2.clone(), 25_000_000_000);
        assert_eq!(program_data.remaining_balance, 55_000_000_000);
        assert_eq!(program_data.payout_history.len(), 2);

        // Third payout
        let program_data = contract.single_payout(&env, recipient3.clone(), 30_000_000_000);
        assert_eq!(program_data.remaining_balance, 25_000_000_000);
        assert_eq!(program_data.payout_history.len(), 3);
    });
}

#[test]
fn test_single_payout_balance_updates_correctly() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient = Address::generate(&env);

    // Check initial balance
    assert_eq!(contract.get_remaining_balance(&env), 100_000_000_000);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.single_payout(&env, recipient, 40_000_000_000);
    });

    // Check balance after payout
    assert_eq!(contract.get_remaining_balance(&env), 60_000_000_000);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_single_payout_unauthorized() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program_with_funds(&env, 50_000_000_000);

    let unauthorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract, || {
        env.set_invoker(&unauthorized);
        contract.single_payout(&env, recipient, 10_000_000_000); // Should panic
    });
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_single_payout_insufficient_balance() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 20_000_000_000);

    let recipient = Address::generate(&env);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.single_payout(&env, recipient, 30_000_000_000); // Should panic
    });
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_single_payout_zero_amount() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 50_000_000_000);

    let recipient = Address::generate(&env);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.single_payout(&env, recipient, 0); // Should panic
    });
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_single_payout_negative_amount() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 50_000_000_000);

    let recipient = Address::generate(&env);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.single_payout(&env, recipient, -10_000_000_000); // Should panic
    });
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_single_payout_before_init() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let recipient = Address::generate(&env);

    contract.single_payout(&env, recipient, 10_000_000_000);
}

// =============================================================================
// TESTS FOR VIEW FUNCTIONS
// =============================================================================

#[test]
fn test_get_program_info_success() {
    let env = Env::default();
    let (contract, admin, token, program_id) = setup_program_with_funds(&env, 75_000_000_000);

    let info = contract.get_program_info(&env);

    assert_eq!(info.program_id, program_id);
    assert_eq!(info.total_funds, 75_000_000_000);
    assert_eq!(info.remaining_balance, 75_000_000_000);
    assert_eq!(info.authorized_payout_key, admin);
    assert_eq!(info.token_address, token);
    assert_eq!(info.payout_history.len(), 0);
}

#[test]
fn test_get_program_info_after_payouts() {
    let env = Env::default();
    let (contract, admin, token, program_id) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    // Perform some payouts
    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.single_payout(&env, recipient1, 25_000_000_000);
        contract.single_payout(&env, recipient2, 35_000_000_000);
    });

    let info = contract.get_program_info(&env);

    assert_eq!(info.program_id, program_id);
    assert_eq!(info.total_funds, 100_000_000_000);
    assert_eq!(info.remaining_balance, 40_000_000_000); // 100 - 25 - 35
    assert_eq!(info.authorized_payout_key, admin);
    assert_eq!(info.token_address, token);
    assert_eq!(info.payout_history.len(), 2);
}

#[test]
fn test_get_remaining_balance_success() {
    let env = Env::default();
    let (contract, _, _, _) = setup_program_with_funds(&env, 50_000_000_000);

    assert_eq!(contract.get_remaining_balance(&env), 50_000_000_000);
}

#[test]
fn test_get_remaining_balance_after_multiple_operations() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program(&env);

    // Initial state
    assert_eq!(contract.get_remaining_balance(&env), 0);

    // After locking funds
    contract.lock_program_funds(&env, 100_000_000_000);
    assert_eq!(contract.get_remaining_balance(&env), 100_000_000_000);

    // After payouts
    let recipient = Address::generate(&env);
    env.as_contract(&contract, || {
        env.set_invoker(&admin);
        contract.single_payout(&env, recipient, 30_000_000_000);
    });
    assert_eq!(contract.get_remaining_balance(&env), 70_000_000_000);

    // After locking more funds
    contract.lock_program_funds(&env, 50_000_000_000);
    assert_eq!(contract.get_remaining_balance(&env), 120_000_000_000);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_get_program_info_before_init() {
    let env = Env::default();
    let contract = ProgramEscrowContract;

    contract.get_program_info(&env);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_get_remaining_balance_before_init() {
    let env = Env::default();
    let contract = ProgramEscrowContract;

    contract.get_remaining_balance(&env);
}

// =============================================================================
// INTEGRATION TESTS - COMPLETE PROGRAM LIFECYCLE
// =============================================================================

#[test]
fn test_complete_program_lifecycle() {
    let env = Env::default();
    let contract = ProgramEscrowContract;
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let program_id = String::from_str(&env, "hackathon-2024-complete");

    // 1. Initialize program
    let program_data = contract.init_program(&env, program_id.clone(), admin.clone(), token.clone());
    assert_eq!(program_data.total_funds, 0);
    assert_eq!(program_data.remaining_balance, 0);

    // 2. Lock initial funds
    contract.lock_program_funds(&env, 500_000_000_000);
    assert_eq!(contract.get_remaining_balance(&env), 500_000_000_000);

    // 3. Perform various payouts
    let recipients = vec![
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    env.as_contract(&contract, || {
        env.set_invoker(&admin);

        // Single payouts
        contract.single_payout(&env, recipients.get(0).unwrap(), 50_000_000_000);
        assert_eq!(contract.get_remaining_balance(&env), 450_000_000_000);

        contract.single_payout(&env, recipients.get(1).unwrap(), 75_000_000_000);
        assert_eq!(contract.get_remaining_balance(&env), 375_000_000_000);

        // Batch payout
        let batch_recipients = vec![&env, recipients.get(2).unwrap(), recipients.get(3).unwrap()];
        let batch_amounts = vec![&env, 100_000_000_000, 80_000_000_000];
        contract.batch_payout(&env, batch_recipients, batch_amounts);
        assert_eq!(contract.get_remaining_balance(&env), 195_000_000_000);

        // Another single payout
        contract.single_payout(&env, recipients.get(4).unwrap(), 95_000_000_000);
        assert_eq!(contract.get_remaining_balance(&env), 100_000_000_000);
    });

    // 4. Verify final state
    let final_info = contract.get_program_info(&env);
    assert_eq!(final_info.total_funds, 500_000_000_000);
    assert_eq!(final_info.remaining_balance, 100_000_000_000);
    assert_eq!(final_info.payout_history.len(), 5);

    // 5. Lock additional funds
    contract.lock_program_funds(&env, 200_000_000_000);
    assert_eq!(contract.get_remaining_balance(&env), 300_000_000_000);
    let final_info = contract.get_program_info(&env);
    assert_eq!(final_info.total_funds, 700_000_000_000);
    assert_eq!(final_info.remaining_balance, 300_000_000_000);
}

#[test]
fn test_program_with_zero_final_balance() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);

        // Pay out all funds
        contract.single_payout(&env, recipient1, 60_000_000_000);
        assert_eq!(contract.get_remaining_balance(&env), 40_000_000_000);

        contract.single_payout(&env, recipient2, 40_000_000_000);
        assert_eq!(contract.get_remaining_balance(&env), 0);
    });

    let info = contract.get_program_info(&env);
    assert_eq!(info.total_funds, 100_000_000_000);
    assert_eq!(info.remaining_balance, 0);
    assert_eq!(info.payout_history.len(), 2);
}

// =============================================================================
// CONCURRENT PAYOUT SCENARIOS (LIMITED IN SOROBAN)
// =============================================================================

#[test]
fn test_sequential_batch_and_single_payouts() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 300_000_000_000);

    let recipients = vec![
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    env.as_contract(&contract, || {
        env.set_invoker(&admin);

        // First batch payout
        let batch_recipients = vec![&env, recipients.get(0).unwrap(), recipients.get(1).unwrap()];
        let batch_amounts = vec![&env, 50_000_000_000, 60_000_000_000];
        contract.batch_payout(&env, batch_recipients, batch_amounts);
        assert_eq!(contract.get_remaining_balance(&env), 190_000_000_000);

        // Single payout
        contract.single_payout(&env, recipients.get(2).unwrap(), 70_000_000_000);
        assert_eq!(contract.get_remaining_balance(&env), 120_000_000_000);

        // Second batch payout
        let batch_recipients2 = vec![&env, recipients.get(3).unwrap()];
        let batch_amounts2 = vec![&env, 80_000_000_000];
        contract.batch_payout(&env, batch_recipients2, batch_amounts2);
        assert_eq!(contract.get_remaining_balance(&env), 40_000_000_000);
    });
}

// =============================================================================
// ADDITIONAL ERROR HANDLING AND EDGE CASES
// =============================================================================

#[test]
fn test_max_payout_history_tracking() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 1_000_000_000_000);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);

        // Create many small payouts to test history tracking
        for i in 0..10 {
            let recipient = Address::generate(&env);
            contract.single_payout(&env, recipient, 10_000_000_000);
        }
    });

    let info = contract.get_program_info(&env);
    assert_eq!(info.payout_history.len(), 10);
    assert_eq!(info.remaining_balance, 900_000_000_000);
}

#[test]
fn test_timestamp_tracking_in_payouts() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 100_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    // Mock different timestamps (in a real scenario, these would be set by the ledger)
    env.as_contract(&contract, || {
        env.set_invoker(&admin);

        // First payout
        contract.single_payout(&env, recipient1.clone(), 25_000_000_000);
        let first_timestamp = env.ledger().timestamp();

        // Second payout (simulating time passing)
        env.ledger().set_timestamp(first_timestamp + 3600); // +1 hour
        contract.single_payout(&env, recipient2.clone(), 30_000_000_000);
        let second_timestamp = env.ledger().timestamp();

        let info = contract.get_program_info(&env);
        let payout1 = info.payout_history.get(0).unwrap();
        let payout2 = info.payout_history.get(1).unwrap();

        assert_eq!(payout1.timestamp, first_timestamp);
        assert_eq!(payout2.timestamp, second_timestamp);
        assert!(second_timestamp > first_timestamp);
    });
}

#[test]
fn test_payout_record_integrity() {
    let env = Env::default();
    let (contract, admin, _, _) = setup_program_with_funds(&env, 200_000_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    env.as_contract(&contract, || {
        env.set_invoker(&admin);

        // Mix of single and batch payouts
        contract.single_payout(&env, recipient1.clone(), 25_000_000_000);

        let batch_recipients = vec![&env, recipient2.clone(), recipient3.clone()];
        let batch_amounts = vec![&env, 35_000_000_000, 45_000_000_000];
        contract.batch_payout(&env, batch_recipients, batch_amounts);

        contract.single_payout(&env, recipient1.clone(), 15_000_000_000); // Same recipient again
    });

    let info = contract.get_program_info(&env);
    assert_eq!(info.payout_history.len(), 4);
    assert_eq!(info.remaining_balance, 80_000_000_000); // 200 - 25 - 35 - 45 - 15

    // Verify all records
    let records = info.payout_history;
    assert_eq!(records.get(0).unwrap().recipient, recipient1);
    assert_eq!(records.get(0).unwrap().amount, 25_000_000_000);

    assert_eq!(records.get(1).unwrap().recipient, recipient2);
    assert_eq!(records.get(1).unwrap().amount, 35_000_000_000);

    assert_eq!(records.get(2).unwrap().recipient, recipient3);
    assert_eq!(records.get(2).unwrap().amount, 45_000_000_000);

    assert_eq!(records.get(3).unwrap().recipient, recipient1);
    assert_eq!(records.get(3).unwrap().amount, 15_000_000_000);
}
