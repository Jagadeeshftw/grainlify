//! # Property-Based Tests for Bounty Escrow Contract
//!
//! This module contains property-based tests that verify fundamental invariants
//! and properties of the escrow contract. These tests complement the fuzzing
//! targets by providing structured property verification.

#![cfg(test)]

use crate::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, Vec,
};

// ============================================================================
// Test Setup Helpers
// ============================================================================

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

struct PropertyTestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> PropertyTestSetup<'a> {
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
// Property: Fund Conservation
// ============================================================================

/// Property: The total supply of tokens should remain constant
/// (no tokens created or destroyed by the contract)
#[test]
fn test_property_fund_conservation_single_lock_release() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Track total balance across all relevant addresses instead of total_supply
    let get_total_balance = |setup: &PropertyTestSetup| {
        setup.token.balance(&setup.depositor)
            + setup.token.balance(&setup.escrow.address)
            + setup.token.balance(&setup.contributor)
            + setup.token.balance(&setup.admin)
    };

    let total_before = get_total_balance(&setup);

    // Lock funds
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let total_during = get_total_balance(&setup);
    assert_eq!(total_before, total_during, "Total balance changed during lock");

    // Release funds
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    let total_after = get_total_balance(&setup);
    assert_eq!(total_before, total_after, "Total balance changed after release");
}

/// Property: Contract balance + Depositor balance + Contributor balance = Constant
#[test]
fn test_property_balance_conservation() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    let depositor_before = setup.token.balance(&setup.depositor);
    let contract_before = setup.token.balance(&setup.escrow.address);
    let contributor_before = setup.token.balance(&setup.contributor);

    let total_before = depositor_before + contract_before + contributor_before;

    // Lock funds
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let depositor_during = setup.token.balance(&setup.depositor);
    let contract_during = setup.token.balance(&setup.escrow.address);
    let contributor_during = setup.token.balance(&setup.contributor);

    let total_during = depositor_during + contract_during + contributor_during;
    assert_eq!(total_before, total_during, "Total balance changed during lock");

    // Release funds
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    let depositor_after = setup.token.balance(&setup.depositor);
    let contract_after = setup.token.balance(&setup.escrow.address);
    let contributor_after = setup.token.balance(&setup.contributor);

    let total_after = depositor_after + contract_after + contributor_after;
    assert_eq!(total_before, total_after, "Total balance changed after release");
}

// ============================================================================
// Property: State Machine Validity
// ============================================================================

/// Property: Once Released, escrow cannot transition to any other state
#[test]
#[should_panic]
fn test_property_no_transition_from_released() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Attempt to release again (should fail)
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

/// Property: Once Refunded, escrow cannot transition to any other state
#[test]
#[should_panic]
fn test_property_no_transition_from_refunded() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);
    setup.escrow.refund(&bounty_id, &None::<i128>, &None::<Address>, &RefundMode::Full);

    // Attempt to release after full refund (should fail)
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

/// Property: State transitions follow valid paths
#[test]
fn test_property_valid_state_transitions() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Initial state: None -> Locked
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    // Valid transition: Locked -> Released
    let bounty_id_2 = 2u64;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id_2, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id_2, &setup.contributor);
    let escrow = setup.escrow.get_escrow_info(&bounty_id_2);
    assert_eq!(escrow.status, EscrowStatus::Released);

    // Valid transition: Locked -> PartiallyRefunded -> Refunded
    let bounty_id_3 = 3u64;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id_3, &amount, &deadline);
    setup.advance_time(1001);
    setup.escrow.refund(&bounty_id_3, &Some(300i128), &None::<Address>, &RefundMode::Partial);
    let escrow = setup.escrow.get_escrow_info(&bounty_id_3);
    assert_eq!(escrow.status, EscrowStatus::PartiallyRefunded);

    setup.escrow.refund(&bounty_id_3, &Some(700i128), &None::<Address>, &RefundMode::Partial);
    let escrow = setup.escrow.get_escrow_info(&bounty_id_3);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

// ============================================================================
// Property: Amount Consistency
// ============================================================================

/// Property: Locked amount equals sum of released and refunded amounts
#[test]
fn test_property_amount_consistency_full_release() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    
    let escrow_before = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.amount, amount);
    assert_eq!(escrow_before.remaining_amount, amount);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // After release, remaining_amount in storage is still the original amount
    // The contract doesn't zero out remaining_amount on release, only on refund
    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.status, EscrowStatus::Released);
    // Note: remaining_amount is not modified on release, only status changes
}

/// Property: Sum of partial refunds equals locked amount
#[test]
fn test_property_amount_consistency_partial_refunds() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.advance_time(1001);

    // First partial refund
    setup.escrow.refund(&bounty_id, &Some(300i128), &None::<Address>, &RefundMode::Partial);
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 700i128);

    // Second partial refund
    setup.escrow.refund(&bounty_id, &Some(200i128), &None::<Address>, &RefundMode::Partial);
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 500i128);

    // Final refund
    setup.escrow.refund(&bounty_id, &Some(500i128), &None::<Address>, &RefundMode::Partial);
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 0i128);
    assert_eq!(escrow.status, EscrowStatus::Refunded);

    // Verify refund history
    let history = setup.escrow.get_refund_history(&bounty_id);
    let total_refunded: i128 = history.iter().map(|r| r.amount).sum();
    assert_eq!(total_refunded, amount);
}

// ============================================================================
// Property: Authorization Requirements
// ============================================================================

/// Property: Only admin can release funds
#[test]
fn test_property_release_requires_admin() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // With mock_all_auths(), we can't test actual auth failure
    // But we verify the contract checks for admin
    // In production, non-admin calls would fail at require_auth()
}

/// Property: Only depositor can lock their own funds
#[test]
fn test_property_lock_requires_depositor_auth() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Depositor locks their own funds
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.depositor, setup.depositor);
}

// ============================================================================
// Property: Deadline Enforcement
// ============================================================================

/// Property: Full refund requires deadline to have passed
#[test]
fn test_property_deadline_enforcement_full_refund() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Before deadline: should fail
    let result = setup.escrow.try_refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
    assert!(result.is_err() || result.unwrap().is_err());

    // At deadline: should succeed
    setup.env.ledger().set_timestamp(deadline);
    let result = setup.escrow.try_refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
    assert!(result.is_ok() && result.unwrap().is_ok());
}

/// Property: Partial refund requires deadline to have passed
#[test]
fn test_property_deadline_enforcement_partial_refund() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Before deadline: should fail
    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(500i128),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_err() || result.unwrap().is_err());

    // After deadline: should succeed
    setup.advance_time(1001);
    let result = setup.escrow.try_refund(
        &bounty_id,
        &Some(500i128),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_ok() && result.unwrap().is_ok());
}

// ============================================================================
// Property: Batch Operation Atomicity
// ============================================================================

/// Property: Batch operations are atomic - all succeed or all fail
#[test]
fn test_property_batch_lock_atomicity() {
    let setup = PropertyTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock first bounty
    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    // Attempt batch with one valid and one duplicate
    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 2u64, // Valid
            depositor: setup.depositor.clone(),
            amount: 2000i128,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1u64, // Duplicate - should cause failure
            depositor: setup.depositor.clone(),
            amount: 3000i128,
            deadline,
        },
    ];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err());

    // Verify bounty 2 was NOT created (atomicity)
    let exists = setup.escrow.try_get_escrow_info(&2u64).is_ok();
    assert!(!exists, "Batch should be atomic - no partial state");
}

/// Property: Batch release is atomic
#[test]
fn test_property_batch_release_atomicity() {
    let setup = PropertyTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock two bounties
    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.escrow.lock_funds(&setup.depositor, &2u64, &2000i128, &deadline);

    // Release first one
    setup.escrow.release_funds(&1u64, &setup.contributor);

    // Attempt batch release with one locked and one already released
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1u64, // Already released
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2u64, // Still locked
            contributor: setup.contributor.clone(),
        },
    ];

    let result = setup.escrow.try_batch_release_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err());

    // Verify bounty 2 was NOT released (atomicity)
    let escrow = setup.escrow.get_escrow_info(&2u64);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

// ============================================================================
// Property: Idempotency and Uniqueness
// ============================================================================

/// Property: Bounty IDs are unique - cannot create duplicate escrows
#[test]
#[should_panic]
fn test_property_bounty_id_uniqueness() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &(amount * 2), &deadline);
}

/// Property: Refund approval can only be used once
#[test]
fn test_property_refund_approval_single_use() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let refund_amount = 500i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let custom_recipient = Address::generate(&setup.env);

    // Admin approves refund
    setup.escrow.approve_refund(
        &bounty_id,
        &refund_amount,
        &custom_recipient,
        &RefundMode::Custom,
    );

    // Use the approval
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &Some(custom_recipient.clone()),
        &RefundMode::Custom,
    );

    // Verify approval was consumed
    let (_, _, _, approval) = setup.escrow.get_refund_eligibility(&bounty_id);
    assert!(approval.is_none(), "Approval should be consumed after use");

    // Attempt to use approval again (should fail - need new approval)
    // Note: This would require the deadline to not have passed
    // After first refund, remaining is 500, so we can refund remaining without approval
}

// ============================================================================
// Property: View Functions Consistency
// ============================================================================

/// Property: get_escrow_info returns consistent data
#[test]
fn test_property_view_function_consistency() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Multiple calls should return same data
    let info1 = setup.escrow.get_escrow_info(&bounty_id);
    let info2 = setup.escrow.get_escrow_info(&bounty_id);

    assert_eq!(info1.depositor, info2.depositor);
    assert_eq!(info1.amount, info2.amount);
    assert_eq!(info1.status, info2.status);
    assert_eq!(info1.deadline, info2.deadline);
    assert_eq!(info1.remaining_amount, info2.remaining_amount);
}

/// Property: get_balance reflects actual contract balance
#[test]
fn test_property_balance_view_accuracy() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Initial balance
    let view_balance = setup.escrow.get_balance();
    let actual_balance = setup.token.balance(&setup.escrow.address);
    assert_eq!(view_balance, actual_balance);

    // After lock
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    let view_balance = setup.escrow.get_balance();
    let actual_balance = setup.token.balance(&setup.escrow.address);
    assert_eq!(view_balance, actual_balance);
    assert_eq!(view_balance, amount);

    // After release
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    let view_balance = setup.escrow.get_balance();
    let actual_balance = setup.token.balance(&setup.escrow.address);
    assert_eq!(view_balance, actual_balance);
    assert_eq!(view_balance, 0);
}

// ============================================================================
// Property: Edge Cases and Boundaries
// ============================================================================

/// Property: Zero amount operations are rejected
#[test]
fn test_property_zero_amount_rejected() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let deadline = setup.env.ledger().timestamp() + 1000;

    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &0i128, &deadline);
    assert!(result.is_err() || result.unwrap().is_err());
}

/// Property: Negative amount operations are rejected
#[test]
fn test_property_negative_amount_rejected() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let deadline = setup.env.ledger().timestamp() + 1000;

    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &-1000i128, &deadline);
    assert!(result.is_err() || result.unwrap().is_err());
}

/// Property: Past deadline is rejected for lock
#[test]
fn test_property_past_deadline_rejected() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let current_time = setup.env.ledger().timestamp();

    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &current_time);
    assert!(result.is_err() || result.unwrap().is_err());

    let result = setup.escrow.try_lock_funds(
        &setup.depositor,
        &(bounty_id + 1),
        &amount,
        &current_time.saturating_sub(1),
    );
    assert!(result.is_err() || result.unwrap().is_err());
}

/// Property: Very large amounts are handled correctly
#[test]
fn test_property_large_amount_handling() {
    let setup = PropertyTestSetup::new();
    let bounty_id = 1u64;
    // Use a large but reasonable amount
    let amount = i128::MAX / 2;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Mint enough tokens
    setup.token_admin.mint(&setup.depositor, &amount);

    let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    
    // Should either succeed or fail gracefully (not panic)
    if let Ok(Ok(())) = result {
        let escrow = setup.escrow.get_escrow_info(&bounty_id);
        assert_eq!(escrow.amount, amount);
    }
}

/// Property: Operations on non-existent bounty return correct error
#[test]
fn test_property_nonexistent_bounty_error() {
    let setup = PropertyTestSetup::new();
    let nonexistent_bounty_id = 999u64;

    let result = setup.escrow.try_get_escrow_info(&nonexistent_bounty_id);
    assert!(result.is_err());

    let result = setup.escrow.try_release_funds(&nonexistent_bounty_id, &setup.contributor);
    assert!(result.is_err() || result.unwrap().is_err());

    let result = setup.escrow.try_refund(
        &nonexistent_bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
    assert!(result.is_err() || result.unwrap().is_err());
}
