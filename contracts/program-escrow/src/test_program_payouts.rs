#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
fn test_analytics_metrics_match_operation_counts() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.single_payout(&r1, &10_000_0000000,
    &None
);
    client.single_payout(&r2, &20_000_0000000,
    &None
);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, 5_000_0000000i128];
    client.batch_payout(&recipients, &amounts,
    &None
);

    let stats = client.get_program_aggregate_stats();
    assert_eq!(stats.payout_count, 3);
    assert_eq!(stats.total_paid_out, 35_000_0000000i128);
    assert_eq!(stats.remaining_balance, 65_000_0000000i128);
    assert_eq!(stats.total_funds, 100_000_0000000i128);
}

// =============================================================================
// BATCH PROGRAM REGISTRATION TESTS
// =============================================================================
// These tests validate batch payout functionality including:
// - Happy path with multiple distinct recipients
// - Batches containing duplicate recipient addresses
// - Edge case at maximum allowed batch size
// - Error handling strategy (all-or-nothing atomicity)

#[test]
fn test_batch_payout_happy_path_multiple_recipients() {
    // Test the happy path: valid batch with multiple distinct recipients
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 6_000_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone(), r3.clone()];
    let amounts = vec![&env, 1_000_000, 2_000_000, 3_000_000];

    let data = client.batch_payout(&recipients, &amounts,
    &None
);

    // Verify balance updated correctly (all-or-nothing)
    assert_eq!(data.remaining_balance, 0);

    // Verify payout history has all three records
    assert_eq!(data.payout_history.len(), 3);

    // Verify each payout record
    let payout1 = data.payout_history.get(0).unwrap();
    assert_eq!(payout1.recipient, r1);
    assert_eq!(payout1.amount, 1_000_000);

    let payout2 = data.payout_history.get(1).unwrap();
    assert_eq!(payout2.recipient, r2);
    assert_eq!(payout2.amount, 2_000_000);

    let payout3 = data.payout_history.get(2).unwrap();
    assert_eq!(payout3.recipient, r3);
    assert_eq!(payout3.amount, 3_000_000);

    // Verify token transfers
    assert_eq!(token_client.balance(&r1), 1_000_000);
    assert_eq!(token_client.balance(&r2), 2_000_000);
    assert_eq!(token_client.balance(&r3), 3_000_000);
}

#[test]
fn test_batch_payout_with_duplicate_recipient_addresses() {
    // Test batch containing duplicate recipient addresses
    // This validates that the contract handles repeated recipients correctly
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 4_500_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Create batch with duplicate recipient
    let recipients = vec![&env, r1.clone(), r2.clone(), r1.clone()];
    let amounts = vec![&env, 1_000_000, 2_000_000, 1_500_000];

    let data = client.batch_payout(&recipients, &amounts,
    &None
);

    // Balance should be fully consumed
    assert_eq!(data.remaining_balance, 0);

    // Payout history should have all three records (duplicates are allowed)
    assert_eq!(data.payout_history.len(), 3);

    // Count occurrences of r1 in history
    let mut r1_count = 0;
    let mut r1_total = 0i128;
    for i in 0..data.payout_history.len() {
        let record = data.payout_history.get(i).unwrap();
        if record.recipient == r1 {
            r1_count += 1;
            r1_total += record.amount;
        }
    }

    // r1 should appear twice with correct total
    assert_eq!(r1_count, 2);
    assert_eq!(r1_total, 1_000_000 + 1_500_000);

    // Verify token balances
    assert_eq!(token_client.balance(&r1), 2_500_000);
    assert_eq!(token_client.balance(&r2), 2_000_000);
}

#[test]
fn test_batch_payout_maximum_batch_size() {
    // Test batch at maximum allowed size
    // This validates edge case behavior with large batches
    let env = Env::default();
    let batch_size = 50usize;
    let amount_per_recipient = 100_000i128;
    let total_amount = (batch_size as i128) * amount_per_recipient;

    let (client, _admin, _token_client, _token_admin) = setup_program(&env, total_amount);

    let mut recipients = vec![&env];
    let mut amounts = vec![&env];

    for _ in 0..batch_size {
        recipients.push_back(Address::generate(&env));
        amounts.push_back(amount_per_recipient);
    }

    // Execute large batch payout
    let data = client.batch_payout(&recipients, &amounts,
    &None
);

    // Balance should be fully consumed
    assert_eq!(data.remaining_balance, 0);

    // Payout history should have all records
    assert_eq!(data.payout_history.len(), batch_size as u32);

    // Verify total payout amount
    let mut total_paid = 0i128;
    for i in 0..data.payout_history.len() {
        let record = data.payout_history.get(i).unwrap();
        total_paid += record.amount;
    }
    assert_eq!(total_paid, total_amount);
}

#[test]
#[should_panic(expected = "Cannot process empty batch")]
fn test_batch_payout_empty_batch_panic() {
    // Test that empty batch is rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 1_000_000);

    let recipients = vec![&env];
    let amounts = vec![&env];

    // Should panic
    client.batch_payout(&recipients, &amounts,
    &None
);
}

#[test]
#[should_panic(expected = "Recipients and amounts vectors must have the same length")]
fn test_batch_payout_mismatched_arrays_panic() {
    // Test that mismatched recipient/amount arrays are rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env), Address::generate(&env)];
    let amounts = vec![&env, 1_000_000]; // Only 1 amount for 2 recipients

    // Should panic
    client.batch_payout(&recipients, &amounts,
    &None
);
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_invalid_amount_zero_panic() {
    // Test that zero amounts are rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, 0i128]; // Zero amount - invalid

    // Should panic
    client.batch_payout(&recipients, &amounts,
    &None
);
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_invalid_amount_negative_panic() {
    // Test that negative amounts are rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, -1_000_000]; // Negative amount - invalid

    // Should panic
    client.batch_payout(&recipients, &amounts,
    &None
);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_batch_payout_insufficient_balance_panic() {
    // Test that insufficient balance is rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, 10_000_000]; // More than available

    // Should panic
    client.batch_payout(&recipients, &amounts,
    &None
);
}

#[test]
fn test_batch_payout_partial_spend() {
    // Test batch payout that doesn't spend entire balance
    // This validates that partial payouts work correctly
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1, r2];
    let amounts = vec![&env, 3_000_000, 3_000_000];

    let data = client.batch_payout(&recipients, &amounts,
    &None
);

    // Remaining balance should be correct
    assert_eq!(data.remaining_balance, 4_000_000);

    // Payout history should have both records
    assert_eq!(data.payout_history.len(), 2);
}

#[test]
fn test_batch_payout_atomicity_all_or_nothing() {
    // Test that batch payout maintains atomicity (all-or-nothing semantics)
    // Verify that either all payouts succeed or the entire transaction fails
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 3_000_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Get program state before payout
    let program_data_before = client.get_program_info();
    let history_len_before = program_data_before.payout_history.len();
    let balance_before = program_data_before.remaining_balance;

    // Execute successful batch payout
    let recipients = vec![&env, r1, r2];
    let amounts = vec![&env, 1_000_000, 2_000_000];

    let data = client.batch_payout(&recipients, &amounts,
    &None
);

    // All records must be written
    assert_eq!(data.payout_history.len(), history_len_before + 2);

    // Balance must be fully updated
    assert_eq!(data.remaining_balance, balance_before - 3_000_000);

    // All conditions should be satisfied together (atomicity)
    assert_eq!(data.payout_history.len(), 2);
    assert_eq!(data.remaining_balance, 0);
}

#[test]
fn test_spend_threshold_single_payout_at_boundary_allowed() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 50_000);
    let program_id = String::from_str(&env, "hack-2026");

    client.set_program_spend_threshold(&program_id, &10_000);

    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &10_000,
    &None
);

    assert_eq!(data.remaining_balance, 40_000);
    assert_eq!(token_client.balance(&recipient), 10_000);
}

#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_threshold_single_payout_above_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 50_000);
    let program_id = String::from_str(&env, "hack-2026");

    client.set_program_spend_threshold(&program_id, &10_000);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &10_001,
    &None
);
}

#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_threshold_batch_total_above_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 50_000);
    let program_id = String::from_str(&env, "hack-2026");

    client.set_program_spend_threshold(&program_id, &10_000);

    let recipients = vec![&env, Address::generate(&env), Address::generate(&env)];
    let amounts = vec![&env, 6_000, 5_000];
    client.batch_payout(&recipients, &amounts,
    &None
);
}

#[test]
#[should_panic(expected = "Invalid spend threshold")]
fn test_spend_threshold_must_be_positive() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 1_000);
    let program_id = String::from_str(&env, "hack-2026");
    client.set_program_spend_threshold(&program_id, &0);
}

#[test]
fn test_batch_payout_sequential_batches() {
    // Test multiple sequential batch payouts to same program
    // Validates that history accumulates correctly
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 9_000_000);

    // First batch
    let r1 = Address::generate(&env);
    let recipients1 = vec![&env, r1];
    let amounts1 = vec![&env, 3_000_000];
    let data1 = client.batch_payout(&recipients1, &amounts1,
    &None
);

    // Verify after first batch
    assert_eq!(data1.payout_history.len(), 1);
    assert_eq!(data1.remaining_balance, 6_000_000);

    // Second batch
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let recipients2 = vec![&env, r2, r3];
    let amounts2 = vec![&env, 2_000_000, 4_000_000];
    let data2 = client.batch_payout(&recipients2, &amounts2,
    &None
);

    // Verify after second batch
    assert_eq!(data2.payout_history.len(), 3);
    assert_eq!(data2.remaining_balance, 0);

    // Verify history order
    let record1 = data2.payout_history.get(0).unwrap();
    assert_eq!(record1.amount, 3_000_000);

    let record2 = data2.payout_history.get(1).unwrap();
    assert_eq!(record2.amount, 2_000_000);

    let record3 = data2.payout_history.get(2).unwrap();
    assert_eq!(record3.amount, 4_000_000);
}

// PROGRAM ESCROW HISTORY QUERY FILTER TESTS
// Tests for recipient, amount, timestamp filters + pagination on payout history
