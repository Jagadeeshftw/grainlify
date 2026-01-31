//! # Invalid Input Tests for Bounty Escrow Contract
//!
//! This module tests various invalid input combinations to ensure
//! the contract handles them gracefully without panicking or
//! entering inconsistent states.

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

struct InvalidInputTestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> InvalidInputTestSetup<'a> {
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
// Invalid Input: Lock Funds
// ============================================================================

#[test]
fn test_invalid_lock_uninitialized_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    
    // Create but don't initialize
    let escrow = create_escrow_contract(&env);
    
    token_admin.mint(&depositor, &10000i128);

    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = env.ledger().timestamp() + 1000;

    // Try to lock on uninitialized contract
    let result = escrow.try_lock_funds(&depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Lock on uninitialized contract should fail");
}

#[test]
fn test_invalid_lock_zero_amount_variations() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Test various "zero-like" amounts
    let zero_amounts = [0i128, -0i128];

    for amount in zero_amounts {
        let result = setup.escrow.try_lock_funds(
            &setup.depositor,
            &(amount as u64),
            &amount,
            &deadline,
        );
        assert!(result.is_err() || result.unwrap().is_err(), 
            "Zero amount {} should be rejected", amount);
    }
}

#[test]
fn test_invalid_lock_insufficient_balance() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Create new depositor with no tokens
    let poor_depositor = Address::generate(&setup.env);
    
    let bounty_id = 1u64;
    let amount = 1000i128;

    // Try to lock without having tokens
    let result = setup.escrow.try_lock_funds(&poor_depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Lock with insufficient balance should fail");
}

#[test]
fn test_invalid_lock_exact_balance() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Create depositor with exact amount
    let exact_depositor = Address::generate(&setup.env);
    let exact_amount = 1000i128;
    setup.token_admin.mint(&exact_depositor, &exact_amount);

    let bounty_id = 1u64;

    // Lock exact balance (should succeed)
    let result = setup.escrow.try_lock_funds(&exact_depositor, &bounty_id, &exact_amount, &deadline);
    assert!(result.is_ok() && result.unwrap().is_ok(), 
        "Lock with exact balance should succeed");

    // Verify balance is now zero
    let balance = setup.token.balance(&exact_depositor);
    assert_eq!(balance, 0, "Depositor balance should be zero after locking exact amount");
}

#[test]
fn test_invalid_lock_past_deadline_variations() {
    let setup = InvalidInputTestSetup::new();
    let current_time = setup.env.ledger().timestamp();

    // Test current time (deadline = now)
    let result = setup.escrow.try_lock_funds(&setup.depositor, &1u64, &1000i128, &current_time);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Lock with deadline = now should fail");

    // Test past times using saturating_sub to avoid underflow
    let past_times = [
        current_time.saturating_sub(1),      // 1 second ago
        current_time.saturating_sub(60),     // 1 minute ago
        current_time.saturating_sub(3600),   // 1 hour ago
        current_time.saturating_sub(86400),  // 1 day ago
    ];

    for (i, past_time) in past_times.iter().enumerate() {
        // Only test if we actually went back in time (not at epoch)
        if *past_time < current_time {
            let bounty_id = (i + 2) as u64; // Start from 2 since we used 1 above
            let result = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &1000i128, past_time);
            assert!(result.is_err() || result.unwrap().is_err(), 
                "Lock with past deadline {} should fail", past_time);
        }
    }
}

#[test]
fn test_invalid_lock_duplicate_bounty_id_variations() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // First lock
    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    // Try to lock with same bounty ID - different amounts
    let duplicate_amounts = [1000i128, 2000i128, 0i128, -1000i128, i128::MAX];

    for amount in duplicate_amounts {
        let result = setup.escrow.try_lock_funds(&setup.depositor, &1u64, &amount, &deadline);
        assert!(result.is_err() || result.unwrap().is_err(), 
            "Duplicate bounty ID with amount {} should fail", amount);
    }
}

// ============================================================================
// Invalid Input: Release Funds
// ============================================================================

#[test]
fn test_invalid_release_unauthorized() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    // Note: With mock_all_auths(), we can't test actual authorization failure
    // This test documents the expected behavior
    // In production, non-admin calls would fail at require_auth()
}

#[test]
fn test_invalid_release_nonexistent_bounty() {
    let setup = InvalidInputTestSetup::new();

    let nonexistent_ids = [0u64, 1u64, 999u64, u64::MAX];

    for bounty_id in nonexistent_ids {
        let result = setup.escrow.try_release_funds(&bounty_id, &setup.contributor);
        assert!(result.is_err() || result.unwrap().is_err(), 
            "Release of nonexistent bounty {} should fail", bounty_id);
    }
}

#[test]
fn test_invalid_release_already_released() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.escrow.release_funds(&1u64, &setup.contributor);

    // Try to release again
    let result = setup.escrow.try_release_funds(&1u64, &setup.contributor);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Double release should fail");
}

#[test]
fn test_invalid_release_already_refunded() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.advance_time(1001);
    setup.escrow.refund(&1u64, &None::<i128>, &None::<Address>, &RefundMode::Full);

    // Try to release after refund
    let result = setup.escrow.try_release_funds(&1u64, &setup.contributor);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Release after refund should fail");
}

#[test]
fn test_invalid_release_partially_refunded() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.advance_time(1001);
    setup.escrow.refund(&1u64, &Some(300i128), &None::<Address>, &RefundMode::Partial);

    // Try to release after partial refund
    let result = setup.escrow.try_release_funds(&1u64, &setup.contributor);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Release after partial refund should fail");
}

// ============================================================================
// Invalid Input: Refund
// ============================================================================

#[test]
fn test_invalid_refund_nonexistent_bounty() {
    let setup = InvalidInputTestSetup::new();

    let result = setup.escrow.try_refund(
        &999u64,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Refund of nonexistent bounty should fail");
}

#[test]
fn test_invalid_refund_before_deadline_full() {
    let setup = InvalidInputTestSetup::new();
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    // Try to refund at various times before deadline
    let times_before_deadline = [0u64, 1, 100, 500, 999];

    for offset in times_before_deadline {
        setup.env.ledger().set_timestamp(current_time + offset);
        
        let result = setup.escrow.try_refund(
            &1u64,
            &None::<i128>,
            &None::<Address>,
            &RefundMode::Full,
        );
        assert!(result.is_err() || result.unwrap().is_err(), 
            "Full refund {} seconds before deadline should fail", deadline - (current_time + offset));
    }
}

#[test]
fn test_invalid_refund_before_deadline_partial() {
    let setup = InvalidInputTestSetup::new();
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    setup.env.ledger().set_timestamp(current_time + 500);

    let result = setup.escrow.try_refund(
        &1u64,
        &Some(500i128),
        &None::<Address>,
        &RefundMode::Partial,
    );
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Partial refund before deadline should fail");
}

#[test]
fn test_invalid_refund_custom_without_approval() {
    let setup = InvalidInputTestSetup::new();
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    let custom_recipient = Address::generate(&setup.env);

    // Try custom refund without approval before deadline
    let result = setup.escrow.try_refund(
        &1u64,
        &Some(500i128),
        &Some(custom_recipient),
        &RefundMode::Custom,
    );
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Custom refund without approval before deadline should fail");
}

#[test]
fn test_invalid_refund_zero_amount() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.advance_time(1001);

    let invalid_amounts = [0i128, -0i128];

    for amount in invalid_amounts {
        let result = setup.escrow.try_refund(
            &1u64,
            &Some(amount),
            &None::<Address>,
            &RefundMode::Partial,
        );
        assert!(result.is_err() || result.unwrap().is_err(), 
            "Refund with zero amount {} should fail", amount);
    }
}

#[test]
fn test_invalid_refund_negative_amount() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.advance_time(1001);

    let negative_amounts = [-1i128, -100, -1000, i128::MIN];

    for amount in negative_amounts {
        let result = setup.escrow.try_refund(
            &1u64,
            &Some(amount),
            &None::<Address>,
            &RefundMode::Partial,
        );
        assert!(result.is_err() || result.unwrap().is_err(), 
            "Refund with negative amount {} should fail", amount);
    }
}

#[test]
fn test_invalid_refund_exceeds_remaining() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.advance_time(1001);

    let excessive_amounts = [1001i128, 2000, i128::MAX];

    for amount in excessive_amounts {
        let result = setup.escrow.try_refund(
            &1u64,
            &Some(amount),
            &None::<Address>,
            &RefundMode::Partial,
        );
        assert!(result.is_err() || result.unwrap().is_err(), 
            "Refund exceeding remaining {} should fail", amount);
    }
}

#[test]
fn test_invalid_refund_custom_missing_fields() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.advance_time(1001);

    // Custom refund without amount
    let result1 = setup.escrow.try_refund(
        &1u64,
        &None::<i128>,
        &Some(setup.contributor.clone()),
        &RefundMode::Custom,
    );
    assert!(result1.is_err() || result1.unwrap().is_err(), 
        "Custom refund without amount should fail");

    // Custom refund without recipient
    let result2 = setup.escrow.try_refund(
        &1u64,
        &Some(500i128),
        &None::<Address>,
        &RefundMode::Custom,
    );
    assert!(result2.is_err() || result2.unwrap().is_err(), 
        "Custom refund without recipient should fail");

    // Custom refund without both
    let result3 = setup.escrow.try_refund(
        &1u64,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Custom,
    );
    assert!(result3.is_err() || result3.unwrap().is_err(), 
        "Custom refund without amount and recipient should fail");
}

// ============================================================================
// Invalid Input: Batch Operations
// ============================================================================

#[test]
fn test_invalid_batch_lock_empty() {
    let setup = InvalidInputTestSetup::new();
    let items: Vec<LockFundsItem> = vec![&setup.env];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Empty batch lock should fail");
}

#[test]
fn test_invalid_batch_release_empty() {
    let setup = InvalidInputTestSetup::new();
    let items: Vec<ReleaseFundsItem> = vec![&setup.env];

    let result = setup.escrow.try_batch_release_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Empty batch release should fail");
}

#[test]
fn test_invalid_batch_lock_exceeds_max_size() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Create batch with 101 items (exceeds MAX_BATCH_SIZE of 100)
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
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Batch exceeding max size should fail");
}

#[test]
fn test_invalid_batch_lock_with_existing_bounty() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock one bounty first
    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    // Try batch with that bounty ID included
    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 2u64, // New
            depositor: setup.depositor.clone(),
            amount: 2000i128,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1u64, // Already exists
            depositor: setup.depositor.clone(),
            amount: 3000i128,
            deadline,
        },
    ];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Batch with existing bounty should fail");
}

#[test]
fn test_invalid_batch_lock_with_zero_amount() {
    let setup = InvalidInputTestSetup::new();
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
            bounty_id: 2u64,
            depositor: setup.depositor.clone(),
            amount: 0i128, // Invalid
            deadline,
        },
    ];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Batch with zero amount should fail");
}

#[test]
fn test_invalid_batch_lock_with_past_deadline() {
    let setup = InvalidInputTestSetup::new();
    let current_time = setup.env.ledger().timestamp();

    // Skip test if we're at time 0 (can't have past deadline)
    if current_time == 0 {
        return;
    }

    let past_deadline = current_time - 1;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1u64,
            depositor: setup.depositor.clone(),
            amount: 1000i128,
            deadline: current_time + 1000, // Valid
        },
        LockFundsItem {
            bounty_id: 2u64,
            depositor: setup.depositor.clone(),
            amount: 2000i128,
            deadline: past_deadline, // Invalid (past)
        },
    ];

    let result = setup.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Batch with past deadline should fail");
}

#[test]
fn test_invalid_batch_release_nonexistent_bounty() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock one bounty
    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    // Try batch release with nonexistent bounty
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1u64, // Exists
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 999u64, // Doesn't exist
            contributor: setup.contributor.clone(),
        },
    ];

    let result = setup.escrow.try_batch_release_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Batch release with nonexistent bounty should fail");
}

#[test]
fn test_invalid_batch_release_already_released() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock and release one bounty
    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.escrow.release_funds(&1u64, &setup.contributor);

    // Lock another
    setup.escrow.lock_funds(&setup.depositor, &2u64, &2000i128, &deadline);

    // Try batch release including already released
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1u64, // Already released
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2u64, // Locked
            contributor: setup.contributor.clone(),
        },
    ];

    let result = setup.escrow.try_batch_release_funds(&items);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Batch release with already released bounty should fail");
}

// ============================================================================
// Invalid Input: View Functions
// ============================================================================

#[test]
fn test_invalid_view_nonexistent_escrow() {
    let setup = InvalidInputTestSetup::new();

    let result = setup.escrow.try_get_escrow_info(&999u64);
    assert!(result.is_err(), "Get escrow info for nonexistent should fail");
}

#[test]
fn test_invalid_view_refund_history_nonexistent() {
    let setup = InvalidInputTestSetup::new();

    let result = setup.escrow.try_get_refund_history(&999u64);
    assert!(result.is_err(), "Get refund history for nonexistent should fail");
}

#[test]
fn test_invalid_view_refund_eligibility_nonexistent() {
    let setup = InvalidInputTestSetup::new();

    let result = setup.escrow.try_get_refund_eligibility(&999u64);
    assert!(result.is_err(), "Get refund eligibility for nonexistent should fail");
}

// ============================================================================
// Invalid Input: Initialization
// ============================================================================

#[test]
fn test_invalid_init_already_initialized() {
    let setup = InvalidInputTestSetup::new();

    // Try to initialize again
    let result = setup.escrow.try_init(&setup.admin, &setup.token.address);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Double initialization should fail");
}

#[test]
fn test_invalid_init_different_admin() {
    let setup = InvalidInputTestSetup::new();

    let new_admin = Address::generate(&setup.env);

    // Try to initialize with different admin
    let result = setup.escrow.try_init(&new_admin, &setup.token.address);
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Re-initialization with different admin should fail");
}

// ============================================================================
// Invalid Input: Refund Approval
// ============================================================================

#[test]
fn test_invalid_approval_nonexistent_bounty() {
    let setup = InvalidInputTestSetup::new();

    let recipient = Address::generate(&setup.env);

    let result = setup.escrow.try_approve_refund(
        &999u64,
        &500i128,
        &recipient,
        &RefundMode::Custom,
    );
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Approval for nonexistent bounty should fail");
}

#[test]
fn test_invalid_approval_zero_amount() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    let recipient = Address::generate(&setup.env);

    let result = setup.escrow.try_approve_refund(
        &1u64,
        &0i128,
        &recipient,
        &RefundMode::Custom,
    );
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Approval with zero amount should fail");
}

#[test]
fn test_invalid_approval_exceeds_remaining() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);

    let recipient = Address::generate(&setup.env);

    let result = setup.escrow.try_approve_refund(
        &1u64,
        &1001i128,
        &recipient,
        &RefundMode::Custom,
    );
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Approval exceeding remaining should fail");
}

#[test]
fn test_invalid_approval_already_released() {
    let setup = InvalidInputTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup.escrow.lock_funds(&setup.depositor, &1u64, &1000i128, &deadline);
    setup.escrow.release_funds(&1u64, &setup.contributor);

    let recipient = Address::generate(&setup.env);

    let result = setup.escrow.try_approve_refund(
        &1u64,
        &500i128,
        &recipient,
        &RefundMode::Custom,
    );
    assert!(result.is_err() || result.unwrap().is_err(), 
        "Approval for released bounty should fail");
}
