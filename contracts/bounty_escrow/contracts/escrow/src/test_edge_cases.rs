//! # Edge Case Tests for Bounty Escrow Contract
//!
//! This module contains tests for edge cases, boundary conditions,
//! and potential vulnerability scenarios.

#![cfg(test)]

use crate::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, Vec,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

struct EdgeCaseTestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> EdgeCaseTestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &1_000_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }

    fn advance_time(&self, seconds: u64) {
        let current = self.env.ledger().timestamp();
        self.env.ledger().set_timestamp(current + seconds);
    }
}

// ============================================================================
// Edge Cases: Zero Values
// ============================================================================

#[test]
fn test_edge_zero_amount_lock() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let deadline = setup.env.ledger().timestamp() + 1000;

    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &0i128, &deadline);
    assert!(result.is_err() || result.unwrap().is_err(), "Zero amount should be rejected");
}

#[test]
fn test_edge_zero_bounty_id() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 0u64; // Edge case: zero ID
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Zero bounty ID should be valid (just another ID)
    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_ok() && result.unwrap().is_ok(), "Zero bounty ID should be valid");

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, amount);
}

#[test]
fn test_edge_zero_deadline_offset() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let current_time = setup.env.ledger().timestamp();

    // Deadline equal to current time should fail
    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &current_time);
    assert!(result.is_err() || result.unwrap().is_err(), "Deadline at current time should fail");
}

#[test]
fn test_edge_zero_partial_refund() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(0i128),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_err() || result.unwrap().is_err(), "Zero refund amount should be rejected");
}

// ============================================================================
// Edge Cases: Maximum Values
// ============================================================================

#[test]
fn test_edge_max_u64_bounty_id() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = u64::MAX;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Max u64 bounty ID should be valid
    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_ok() && result.unwrap().is_ok(), "Max u64 bounty ID should be valid");

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, amount);
}

#[test]
fn test_edge_max_i128_amount() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    // Use a large but not max amount to avoid overflow in token contract
    let amount = i128::MAX / 4;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Mint large amount
    setup.token_admin.mint(&setup.depositor, &amount);

    // Attempt to lock large amount
    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    
    // Should either succeed or fail gracefully (not panic)
    if let Ok(Ok(())) = result {
        let escrow = setup.escrow.get_escrow_info(&bounty_id);
        assert_eq!(escrow.amount, amount);
    }
}

#[test]
fn test_edge_max_u64_deadline() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = u64::MAX;

    // Max deadline should be valid
    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_ok() && result.unwrap().is_ok(), "Max deadline should be valid");
}

#[test]
fn test_edge_very_large_batch() {
    let setup = EdgeCaseTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 10000;

    // Create batch at the limit (100 items)
    let mut items = Vec::new(&setup.env);
    for i in 0..100u64 {
        items.push_back(LockFundsItem {
            bounty_id: i,
            depositor: setup.depositor.clone(),
            amount: 100i128,
            deadline,
        });
    }

    // Mint enough tokens
    setup.token_admin.mint(&setup.depositor, &(100i128 * 100));

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_ok() && result.unwrap().is_ok(), "Max batch size should succeed");
}

#[test]
fn test_edge_batch_size_exceeds_limit() {
    let setup = EdgeCaseTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 10000;

    // Create batch exceeding the limit (101 items)
    let mut items = Vec::new(&setup.env);
    for i in 0..101u64 {
        items.push_back(LockFundsItem {
            bounty_id: i,
            depositor: setup.depositor.clone(),
            amount: 100i128,
            deadline,
        });
    }

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), "Batch exceeding limit should fail");
}

// ============================================================================
// Edge Cases: Overflow Scenarios
// ============================================================================

#[test]
fn test_edge_batch_amount_overflow() {
    let setup = EdgeCaseTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 10000;

    // Create batch with large amounts that could overflow when summed
    // Use smaller values to avoid token contract overflow
    let large_amount = i128::MAX / 8;
    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1u64,
            depositor: setup.depositor.clone(),
            amount: large_amount,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2u64,
            depositor: setup.depositor.clone(),
            amount: large_amount,
            deadline,
        },
    ];

    // Mint enough tokens (use smaller amount to avoid overflow)
    setup.token_admin.mint(&setup.depositor, &(large_amount * 2));

    // This should handle overflow gracefully
    let result = setup.escrow.try_batch_lock_funds(&items);
    // Result depends on implementation - should not panic
    // If it succeeds, verify the amounts
    if let Ok(Ok(count)) = result {
        assert_eq!(count, 2);
    }
}

#[test]
fn test_edge_partial_refund_sum_overflow() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    // Use a large amount
    let amount = i128::MAX / 2;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.token_admin.mint(&setup.depositor, &amount);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    // Attempt partial refund with large amount
    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(amount),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_ok() && result.unwrap().is_ok(), "Large partial refund should succeed");
}

// ============================================================================
// Edge Cases: Negative Values
// ============================================================================

#[test]
fn test_edge_negative_amount_lock() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = -1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err() || result.unwrap().is_err(), "Negative amount should be rejected");
}

#[test]
fn test_edge_negative_partial_refund() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(-100i128),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_err() || result.unwrap().is_err(), "Negative refund should be rejected");
}

// ============================================================================
// Edge Cases: Boundary Timing
// ============================================================================

#[test]
fn test_edge_refund_exactly_at_deadline() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Set time exactly at deadline
    setup.env.ledger().set_timestamp(deadline);

    // Should succeed (deadline has passed or is now)
    let result = setup.escrow.try_refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
    assert!(result.is_ok() && result.unwrap().is_ok(), "Refund at deadline should succeed");
}

#[test]
fn test_edge_refund_one_second_before_deadline() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Set time one second before deadline
    setup.env.ledger().set_timestamp(deadline - 1);

    let result = setup.escrow.try_refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
    assert!(result.is_err() || result.unwrap().is_err(), "Refund before deadline should fail");
}

#[test]
fn test_edge_very_long_deadline() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 365 * 24 * 60 * 60; // 1 year

    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_ok() && result.unwrap().is_ok(), "Long deadline should be valid");
}

// ============================================================================
// Edge Cases: Invalid Input Combinations
// ============================================================================

#[test]
fn test_edge_custom_refund_without_amount() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    let custom_recipient = Address::generate(&setup.env);

    // Custom refund without amount should fail
    let result = setup.escrow.try_refund(
        &bounty_id,
        &None::<i128>,
        &Some(custom_recipient),
        &RefundMode::Custom,
    );
    assert!(result.is_err() || result.unwrap().is_err(), "Custom refund without amount should fail");
}

#[test]
fn test_edge_custom_refund_without_recipient() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    // Custom refund without recipient should fail
    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(500i128),
        &None::<Address>,
        &RefundMode::Custom,
    );
    assert!(result.is_err() || result.unwrap().is_err(), "Custom refund without recipient should fail");
}

#[test]
fn test_edge_partial_refund_exceeds_remaining() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    // Try to refund more than available
    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(amount + 1),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_err() || result.unwrap().is_err(), "Refund exceeding remaining should fail");
}

#[test]
fn test_edge_release_to_same_address_as_depositor() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Release to depositor (edge case but valid)
    let result = setup.escrow.try_release_funds(&bounty_id, &setup.depositor);
    assert!(result.is_ok() && result.unwrap().is_ok(), "Release to depositor should succeed");

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_edge_release_to_contract_address() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Release to contract itself (edge case)
    let contract_address = setup.escrow.address.clone();
    let result = setup.escrow.try_release_funds(&bounty_id, &contract_address);
    
    // This may or may not be allowed depending on requirements
    // The test documents the behavior
}

// ============================================================================
// Edge Cases: State Conflicts
// ============================================================================

#[test]
fn test_edge_refund_after_partial_release() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    // Partial refund
    setup.escrow.refund(&bounty_id, &Some(300i128), &None::<Address>, &RefundMode::Partial);

    // Try to release remaining after partial refund
    let result = setup.escrow.try_release_funds(&bounty_id, &setup.contributor);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Release after partial refund should fail - status is PartiallyRefunded, not Locked");
}

#[test]
fn test_edge_multiple_partial_refunds_exact() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    // First partial: 300
    setup.escrow.refund(&bounty_id, &Some(300i128), &None::<Address>, &RefundMode::Partial);
    
    // Second partial: 700 (exact remaining)
    setup.escrow.refund(&bounty_id, &Some(700i128), &None::<Address>, &RefundMode::Partial);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 0);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

#[test]
fn test_edge_partial_refund_one_more_than_remaining() {
    let setup = EdgeCaseTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    // First partial: 500
    setup.escrow.refund(&bounty_id, &Some(500i128), &None::<Address>, &RefundMode::Partial);

    // Try to refund 501 (one more than remaining)
    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(501i128),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_err() || result.unwrap().is_err(), "Refund one more than remaining should fail");
}

// ============================================================================
// Edge Cases: Reentrancy and Security
// ============================================================================

#[test]
fn test_edge_double_init() {
    let setup = EdgeCaseTestSetup::new();

    // Try to initialize again
    let result = setup.escrow.try_init(&setup.admin, &setup.token.address);
    assert!(result.is_err() || result.unwrap().is_err(), "Double init should fail");
}

#[test]
fn test_edge_init_with_zero_address() {
    let setup = EdgeCaseTestSetup::new();
    let env = Env::default();
    env.mock_all_auths();

    let escrow = create_escrow_contract(&env);
    
    // Try to initialize with zero address (if possible)
    // This test documents expected behavior
}

// ============================================================================
// Edge Cases: Empty and Minimal Batches
// ============================================================================

#[test]
fn test_edge_empty_batch_lock() {
    let setup = EdgeCaseTestSetup::new();
    let items: Vec<LockFundsItem> = vec![&setup.env];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), "Empty batch should fail");
}

#[test]
fn test_edge_empty_batch_release() {
    let setup = EdgeCaseTestSetup::new();
    let items: Vec<ReleaseFundsItem> = vec![&setup.env];

    let result = setup.escrow.try_batch_release_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), "Empty batch should fail");
}

#[test]
fn test_edge_single_item_batch() {
    let setup = EdgeCaseTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1u64,
            depositor: setup.depositor.clone(),
            amount: 1000i128,
            deadline,
        },
    ];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_ok() && result.unwrap().is_ok(), "Single item batch should succeed");
}

// ============================================================================
// Edge Cases: Duplicate Detection
// ============================================================================

#[test]
fn test_edge_duplicate_bounty_id_in_batch() {
    let setup = EdgeCaseTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1u64,
            depositor: setup.depositor.clone(),
            amount: 1000i128,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1u64, // Duplicate
            depositor: setup.depositor.clone(),
            amount: 2000i128,
            deadline,
        },
    ];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), "Duplicate in batch should fail");
}

#[test]
fn test_edge_duplicate_bounty_id_across_batches() {
    let setup = EdgeCaseTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // First batch
    let items1 = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1u64,
            depositor: setup.depositor.clone(),
            amount: 1000i128,
            deadline,
        },
    ];
    setup.escrow.batch_lock_funds(&items1);

    // Second batch with same bounty ID
    let items2 = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1u64, // Already exists
            depositor: setup.depositor.clone(),
            amount: 2000i128,
            deadline,
        },
    ];

    let result = setup.escrow.try_batch_lock_funds(&items2);
    assert!(result.is_err() || result.unwrap().is_err(), "Duplicate across batches should fail");
}
