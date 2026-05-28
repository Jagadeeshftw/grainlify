//! Boundary tests for amount policy, deadlines, fee configuration, batch/query limits, and escrow cardinality.
//!
//! Also covers `EscrowStatus::PartiallyRefunded` balance invariants (Issue #1294):
//! - `total_locked - sum(refunded) == remaining_amount` holds after every partial refund.
//! - Sequential partial refunds each decrement `remaining_amount` exactly.
//! - Refunding exactly the full remaining amount transitions to `Refunded`, not `PartiallyRefunded`.
//! - Zero-amount refund approval is rejected before any state change.
//!
//! # Related contract limits
//! - [`crate::BountyEscrowContract::set_amount_policy`] — inclusive `[min_amount, max_amount]`; invalid
//!   ordering (`min > max`) panics.
//! - [`crate::MAX_FEE_RATE`] — fee basis points cap (5000 = 50%).
//! - [`crate::MAX_BATCH_SIZE`] — batch lock/release size (see `test_batch_failure_modes`).
//! - Deadlines: `u64::MAX` is accepted as a sentinel for "no expiry" style locking; past timestamps are allowed at lock time.

#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus, RefundMode};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

#[test]
fn test_focused_amount_and_deadline_boundaries() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let recipient = Address::generate(&e);

    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_id.address();
    let token_admin_client = token::StellarAssetClient::new(&e, &token);

    e.mock_all_auths();
    client.init(&admin, &token);

    token_admin_client.mint(&depositor, &1_000_000_000i128);

    let min_amount = 100i128;
    let max_amount = 10_000i128;
    client.set_amount_policy(&admin, &min_amount, &max_amount);

    let now = e.ledger().timestamp();
    let future_deadline = now + 1_000;

    client.lock_funds(&depositor, &101u64, &min_amount, &future_deadline);
    let info = client.get_escrow_info(&101u64);
    assert_eq!(
        info.amount, min_amount,
        "stored amount should match minimum"
    );

    client.lock_funds(&depositor, &102u64, &(min_amount + 1), &future_deadline);
    client.lock_funds(&depositor, &103u64, &(max_amount - 1), &future_deadline);
    client.lock_funds(&depositor, &104u64, &max_amount, &future_deadline);
    let info = client.get_escrow_info(&104u64);
    assert_eq!(
        info.amount, max_amount,
        "stored amount should match maximum"
    );

    let past_deadline = now.saturating_sub(1);
    client.lock_funds(&depositor, &200u64, &(min_amount + 10), &past_deadline);
    client.refund(&200u64);

    client.lock_funds(&depositor, &201u64, &(min_amount + 10), &now);

    let far_future = now + 1_000_000;
    client.lock_funds(&depositor, &202u64, &(min_amount + 10), &far_future);
    let info = client.get_escrow_info(&202u64);
    assert_eq!(
        info.deadline, far_future,
        "stored deadline should match far future"
    );

    let no_deadline = u64::MAX;
    client.lock_funds(&depositor, &203u64, &(min_amount + 10), &no_deadline);
    let info = client.get_escrow_info(&203u64);
    assert_eq!(
        info.deadline, no_deadline,
        "stored deadline should be NO_DEADLINE"
    );

    let ok_zero_fee = client.try_update_fee_config(&Some(0), &Some(0), &None, &None, &None, &None);
    assert!(ok_zero_fee.is_ok(), "zero fee rate should be allowed");

    let ok_max_fee =
        client.try_update_fee_config(&Some(5_000), &Some(5_000), &None, &None, &None, &None);
    assert!(ok_max_fee.is_ok(), "MAX_FEE_RATE (5000) should be allowed");

    let err_over_max =
        client.try_update_fee_config(&Some(5_001), &None, &None, &None, &None, &None);
    assert!(
        err_over_max.is_err(),
        "fee rate above maximum should be rejected"
    );

    let err_overflow =
        client.try_update_fee_config(&Some(i128::MAX), &None, &None, &None, &None, &None);
    assert!(
        err_overflow.is_err(),
        "overflow fee rate should be rejected"
    );

    let count = client.get_escrow_count();
    assert!(
        count > 0,
        "escrow count should be greater than zero after creating escrows"
    );

    let _ = recipient;
}

/// One below minimum and one above maximum must fail with explicit contract errors (not panic).
#[test]
fn test_amount_policy_rejects_out_of_range() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token_admin = Address::generate(&e);
    let token = e
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let sac = token::StellarAssetClient::new(&e, &token);
    client.init(&admin, &token);
    sac.mint(&depositor, &1_000_000i128);

    let min_amount = 500i128;
    let max_amount = 600i128;
    client.set_amount_policy(&admin, &min_amount, &max_amount);
    let deadline = e.ledger().timestamp() + 10_000;

    assert_eq!(
        client
            .try_lock_funds(&depositor, &1u64, &(min_amount - 1), &deadline)
            .unwrap_err()
            .unwrap(),
        Error::InvalidAmount
    );
    assert_eq!(
        client
            .try_lock_funds(&depositor, &2u64, &(max_amount + 1), &deadline)
            .unwrap_err()
            .unwrap(),
        Error::InvalidAmount
    );

    assert!(client
        .try_lock_funds(&depositor, &3u64, &min_amount, &deadline)
        .is_ok());
}

/// `min_amount > max_amount` is a programmer error; the contract panics with a clear message.
#[test]
#[should_panic(expected = "invalid policy: min_amount cannot exceed max_amount")]
fn test_set_amount_policy_rejects_inverted_range() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    client.init(&admin, &token);
    client.set_amount_policy(&admin, &100i128, &50i128);
}

/// Query APIs with `limit == 0` yield no rows (pagination edge).
#[test]
fn test_escrow_status_query_limit_zero_returns_empty() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token_admin = Address::generate(&e);
    let token = e
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let sac = token::StellarAssetClient::new(&e, &token);
    client.init(&admin, &token);
    sac.mint(&depositor, &10_000i128);
    let deadline = e.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &50u64, &1_000i128, &deadline);

    let empty = client.get_escrow_ids_by_status(&EscrowStatus::Locked, &0u32, &0u32);
    assert_eq!(empty.len(), 0);
}

// ============================================================================
// Issue #1294 — PartiallyRefunded balance invariant tests
// ============================================================================
//
// Invariant under test:
//   escrow.amount - sum(refund_history[*].amount) == escrow.remaining_amount
//
// Equivalently, after each partial refund:
//   remaining_amount_before - refund_amount == remaining_amount_after
//
// All tests below use `approve_refund` + `refund` to drive the contract into
// the PartiallyRefunded state and then assert the accounting invariant.

/// Assert the core balance invariant for a single escrow:
/// `total_locked - sum(refund_history amounts) == remaining_amount`.
fn assert_balance_invariant(
    client: &BountyEscrowContractClient,
    bounty_id: u64,
    total_locked: i128,
) {
    let info = client.get_escrow_info(&bounty_id);
    let refunded_sum: i128 = info.refund_history.iter().map(|r| r.amount).sum();
    assert_eq!(
        total_locked - refunded_sum,
        info.remaining_amount,
        "balance invariant violated: total_locked({}) - refunded_sum({}) != remaining_amount({})",
        total_locked,
        refunded_sum,
        info.remaining_amount,
    );
}

/// Sequential partial refunds each decrement `remaining_amount` exactly and
/// the balance invariant holds after every step.
///
/// Scenario: lock 1000, refund 300 × 3 times (leaves 100 remaining).
#[test]
fn test_sequential_partial_refunds_balance_invariant() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 1001u64;
    let total = 1_000i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    // Three sequential partial refunds: 300, 300, 300 (leaves 100 remaining)
    for refund_amount in [300i128, 300i128, 300i128] {
        let before = client.get_escrow_info(&bounty_id).remaining_amount;
        client.approve_refund(&bounty_id, &refund_amount, &depositor, &RefundMode::Partial);
        client.refund(&bounty_id);
        let after = client.get_escrow_info(&bounty_id).remaining_amount;

        assert_eq!(
            before - refund_amount,
            after,
            "remaining_amount must decrease by exactly the refund amount"
        );
        assert_eq!(
            client.get_escrow_info(&bounty_id).status,
            EscrowStatus::PartiallyRefunded,
            "status must be PartiallyRefunded while funds remain"
        );
        assert_balance_invariant(&client, bounty_id, total);
    }

    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 100);
}

/// Refunding exactly the full remaining amount via `RefundMode::Partial` must
/// transition to `Refunded`, not `PartiallyRefunded`.
///
/// The contract rule: `if amount >= remaining_amount → Refunded`.
#[test]
fn test_partial_refund_of_full_amount_transitions_to_refunded() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 1002u64;
    let total = 500i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    // Approve and execute a refund for the entire locked amount using Partial mode.
    client.approve_refund(&bounty_id, &total, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);

    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(
        info.status,
        EscrowStatus::Refunded,
        "refunding the full amount must yield Refunded, not PartiallyRefunded"
    );
    assert_eq!(info.remaining_amount, 0);
    assert_balance_invariant(&client, bounty_id, total);
}

/// `approve_refund` with amount == 0 must be rejected before any state change.
#[test]
fn test_zero_amount_partial_refund_is_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 1003u64;
    let total = 500i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    let result =
        client.try_approve_refund(&bounty_id, &0i128, &depositor, &RefundMode::Partial);
    assert!(result.is_err(), "zero-amount refund approval must be rejected");

    // State must be completely unchanged
    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(info.remaining_amount, total);
    assert_eq!(info.refund_history.len(), 0);
}

/// Two sequential partial refunds from `PartiallyRefunded` state each decrement
/// `remaining_amount` correctly and the invariant holds at every step.
#[test]
fn test_two_sequential_partial_refunds_invariant() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 1004u64;
    let total = 900i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    // First partial refund: 400 → remaining = 500
    client.approve_refund(&bounty_id, &400i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 500);
    assert_eq!(
        client.get_escrow_info(&bounty_id).status,
        EscrowStatus::PartiallyRefunded
    );
    assert_balance_invariant(&client, bounty_id, total);

    // Second partial refund: 300 → remaining = 200
    client.approve_refund(&bounty_id, &300i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 200);
    assert_eq!(
        client.get_escrow_info(&bounty_id).status,
        EscrowStatus::PartiallyRefunded
    );
    assert_balance_invariant(&client, bounty_id, total);
}

/// After multiple partial refunds, a final refund that drains the remainder
/// transitions to `Refunded` and the invariant still holds.
#[test]
fn test_partial_refunds_then_full_drain_transitions_to_refunded() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 1005u64;
    let total = 600i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    // Two partial refunds of 200 each → remaining = 200
    client.approve_refund(&bounty_id, &200i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);
    client.approve_refund(&bounty_id, &200i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 200);
    assert_eq!(
        client.get_escrow_info(&bounty_id).status,
        EscrowStatus::PartiallyRefunded
    );

    // Final refund drains the rest → Refunded
    client.approve_refund(&bounty_id, &200i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);

    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    assert_balance_invariant(&client, bounty_id, total);
}

/// Partial refunds on independent escrows do not affect each other's balances.
#[test]
fn test_partial_refund_isolation_between_escrows() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let (id_a, id_b) = (2001u64, 2002u64);
    let (total_a, total_b) = (800i128, 600i128);
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &id_a, &total_a, &deadline);
    client.lock_funds(&depositor, &id_b, &total_b, &deadline);

    // Partially refund escrow A only
    client.approve_refund(&id_a, &300i128, &depositor, &RefundMode::Partial);
    client.refund(&id_a);

    // Escrow B must be completely unaffected
    let info_b = client.get_escrow_info(&id_b);
    assert_eq!(info_b.status, EscrowStatus::Locked);
    assert_eq!(info_b.remaining_amount, total_b);
    assert_eq!(info_b.refund_history.len(), 0);

    // Escrow A invariant still holds
    assert_balance_invariant(&client, id_a, total_a);
}

/// Minimum-unit partial refund (1 stroop) is accepted and the invariant holds.
#[test]
fn test_minimum_unit_partial_refund_invariant() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 3001u64;
    let total = 100i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    client.approve_refund(&bounty_id, &1i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);

    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.remaining_amount, 99);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_balance_invariant(&client, bounty_id, total);
}

/// `approve_refund` with amount exceeding `remaining_amount` is rejected;
/// state is unchanged after the failed attempt.
#[test]
fn test_partial_refund_exceeding_remaining_is_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 4001u64;
    let total = 200i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    // First partial refund reduces remaining to 100
    client.approve_refund(&bounty_id, &100i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 100);

    // Attempt to approve a refund larger than remaining — must fail
    let result =
        client.try_approve_refund(&bounty_id, &101i128, &depositor, &RefundMode::Partial);
    assert!(
        result.is_err(),
        "refund exceeding remaining_amount must be rejected"
    );

    // State must be unchanged
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 100);
    assert_balance_invariant(&client, bounty_id, total);
}

/// `RefundMode::Full` always transitions to `Refunded` regardless of the
/// approved amount, because the mode flag overrides the amount comparison.
#[test]
fn test_full_mode_refund_always_transitions_to_refunded() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 5001u64;
    let total = 400i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    // Approve with Full mode and only 200 amount
    client.approve_refund(&bounty_id, &200i128, &depositor, &RefundMode::Full);
    client.refund(&bounty_id);

    assert_eq!(
        client.get_escrow_info(&bounty_id).status,
        EscrowStatus::Refunded,
        "RefundMode::Full must always yield Refunded regardless of amount"
    );
}

/// `refund_history` grows by exactly 1 per partial refund call, and the
/// balance invariant holds after each entry is appended.
#[test]
fn test_refund_history_grows_per_partial_refund() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&e, &contract_id);
    let token = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&e, &token).mint(&depositor, &1_000_000i128);
    client.init(&admin, &token);

    let bounty_id = 6001u64;
    let total = 900i128;
    let deadline = e.ledger().timestamp() + 100_000;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);

    for (i, amount) in [100i128, 100i128, 100i128].iter().enumerate() {
        client.approve_refund(&bounty_id, amount, &depositor, &RefundMode::Partial);
        client.refund(&bounty_id);
        assert_eq!(
            client.get_escrow_info(&bounty_id).refund_history.len(),
            (i + 1) as u32,
            "refund_history must grow by exactly 1 per refund"
        );
        assert_balance_invariant(&client, bounty_id, total);
    }
}
