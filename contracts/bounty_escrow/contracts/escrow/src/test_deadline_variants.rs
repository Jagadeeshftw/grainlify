#![cfg(test)]
//! # Bounty Escrow Deadline Variant Tests
//!
//! This module validates deadline semantics using Soroban ledger timestamps.

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus, RefundMode};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

const NO_DEADLINE: u64 = u64::MAX;

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let addr = contract.address();
    (
        token::Client::new(e, &addr),
        token::StellarAssetClient::new(e, &addr),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &id)
}
struct Setup<'a> {
    env: Env,
    _admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &10_000_000);

        Self {
            env,
            _admin: admin,
            depositor,
            contributor,
            token,
            escrow,
        }
    }
}

// =============================================================================
// Zero deadline (deadline = 0)
//
// When deadline is 0 the check `now >= deadline` is always true for u64,
// so a refund is eligible immediately without any admin approval or waiting.
// The ledger timestamp starts at 0, making `0 >= 0` true from the first ledger.
// =============================================================================

#[test]
fn test_zero_deadline_stored_correctly() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &500, &0);

    let info = s.escrow.get_escrow_info(&1);
    assert_eq!(info.deadline, 0);
    assert_eq!(info.amount, 500);
    assert_eq!(info.status, EscrowStatus::Locked);
}

#[test]
fn test_zero_deadline_refund_succeeds_immediately() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &2, &1_000, &0);

    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&2);

    let info = s.escrow.get_escrow_info(&2);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), before + 1_000);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

#[test]
fn test_zero_deadline_refund_succeeds_after_time_advance() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &3, &800, &0);

    s.env.ledger().set_timestamp(9_999_999);

    s.escrow.refund(&3);

    let info = s.escrow.get_escrow_info(&3);
    assert_eq!(info.status, EscrowStatus::Refunded);
}

#[test]
fn test_zero_deadline_release_succeeds() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &4, &750, &0);

    s.escrow.release_funds(&4, &s.contributor);

    let info = s.escrow.get_escrow_info(&4);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), 750);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// =============================================================================
// Future timestamp deadline  (deadline = now + n)
//
// Standard behaviour: refund is blocked while `ledger_timestamp < deadline`,
// but succeeds once `ledger_timestamp >= deadline`.  Release is unaffected
// by the deadline — it can be called at any time while status is Locked.
// Admin-approved refunds bypass the deadline check entirely.
// =============================================================================

#[test]
fn test_future_deadline_stored_correctly() {
    let s = Setup::new();
    let deadline = s.env.ledger().timestamp() + 3_600;
    s.escrow.lock_funds(&s.depositor, &10, &500, &deadline);

    let info = s.escrow.get_escrow_info(&10);
    assert_eq!(info.deadline, deadline);
    assert_eq!(info.status, EscrowStatus::Locked);
}

#[test]
fn test_future_deadline_refund_blocked_before_expiry() {
    let s = Setup::new();
    let deadline = s.env.ledger().timestamp() + 10_000;
    s.escrow.lock_funds(&s.depositor, &11, &1_000, &deadline);

    let result = s.escrow.try_refund(&11);
    assert_eq!(result.unwrap_err().unwrap(), Error::DeadlineNotPassed);

    let info = s.escrow.get_escrow_info(&11);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(s.token.balance(&s.escrow.address), 1_000);
}

#[test]
fn test_future_deadline_refund_succeeds_after_expiry() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();
    let deadline = now + 500;
    s.escrow.lock_funds(&s.depositor, &12, &1_200, &deadline);

    s.env.ledger().set_timestamp(deadline + 1);

    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&12);

    let info = s.escrow.get_escrow_info(&12);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), before + 1_200);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

#[test]
fn test_future_deadline_early_refund_with_admin_approval() {
    let s = Setup::new();
    let deadline = s.env.ledger().timestamp() + 86_400;
    s.escrow.lock_funds(&s.depositor, &13, &2_000, &deadline);

    s.escrow
        .approve_refund(&13, &2_000, &s.depositor, &RefundMode::Full);

    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&13);

    let info = s.escrow.get_escrow_info(&13);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), before + 2_000);
}

#[test]
fn test_future_deadline_release_unaffected_by_deadline() {
    let s = Setup::new();
    let deadline = s.env.ledger().timestamp() + 86_400;
    s.escrow.lock_funds(&s.depositor, &14, &3_000, &deadline);

    s.escrow.release_funds(&14, &s.contributor);

    let info = s.escrow.get_escrow_info(&14);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), 3_000);
}

// =============================================================================
// No deadline (deadline = u64::MAX)
//
// Using u64::MAX (18,446,744,073,709,551,615) as a sentinel for "no expiry".
// The ledger timestamp is a u64 Unix epoch in seconds.  Even after 100+ years,
// the timestamp (~6.3 × 10^9) remains far below u64::MAX (~1.8 × 10^19), so
// `now >= u64::MAX` is effectively always false.
//
// This makes spontaneous refunds permanently blocked.  The only way to refund
// a no-deadline escrow is through admin approval (approve_refund).  Release
// operations are unaffected and work normally.
// =============================================================================

#[test]
fn test_no_deadline_stored_correctly() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &20, &500, &NO_DEADLINE);

    let info = s.escrow.get_escrow_info(&20);
    assert_eq!(info.deadline, NO_DEADLINE);
    assert_eq!(info.status, EscrowStatus::Locked);
}

#[test]
fn test_no_deadline_refund_blocked_without_approval() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &21, &1_000, &NO_DEADLINE);

    let result = s.escrow.try_refund(&21);
    assert_eq!(result.unwrap_err().unwrap(), Error::DeadlineNotPassed);
}

#[test]
fn test_no_deadline_refund_blocked_even_after_large_time_advance() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &22, &1_000, &NO_DEADLINE);
    s.env.ledger().set_timestamp(100 * 365 * 24 * 3600);

    let result = s.escrow.try_refund(&22);
    assert_eq!(result.unwrap_err().unwrap(), Error::DeadlineNotPassed);
}

#[test]
fn test_no_deadline_refund_succeeds_with_admin_approval() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &23, &1_500, &NO_DEADLINE);
    s.escrow
        .approve_refund(&23, &1_500, &s.depositor, &RefundMode::Full);

    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&23);

    let info = s.escrow.get_escrow_info(&23);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), before + 1_500);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

#[test]
fn test_no_deadline_partial_refund_with_admin_approval() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &24, &2_000, &NO_DEADLINE);
    s.escrow
        .approve_refund(&24, &800, &s.depositor, &RefundMode::Partial);

    s.escrow.refund(&24);

    let info = s.escrow.get_escrow_info(&24);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, 1_200);
    assert_eq!(s.token.balance(&s.escrow.address), 1_200);
}

#[test]
fn test_no_deadline_release_succeeds() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &25, &2_500, &NO_DEADLINE);

    s.escrow.release_funds(&25, &s.contributor);

    let info = s.escrow.get_escrow_info(&25);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), 2_500);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// =============================================================================
// Cross-configuration comparisons
//
// These tests lock identical bounties with the three deadline configurations
// side-by-side to make the behavioral difference explicit and easy to follow.
// =============================================================================

#[test]
fn test_deadline_zero_vs_future_refund_eligibility() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();
    let future = now + 5_000;

    // Bounty A: zero deadline – immediately refundable
    s.escrow.lock_funds(&s.depositor, &30, &400, &0);
    // Bounty B: future deadline – not yet refundable
    s.escrow.lock_funds(&s.depositor, &31, &400, &future);

    assert!(s.escrow.try_refund(&30).is_ok());
    assert_eq!(
        s.escrow.try_refund(&31).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );
}

#[test]
fn test_deadline_future_vs_no_deadline_after_expiry() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();
    let future = now + 1_000;

    // Bounty C: finite future deadline
    s.escrow.lock_funds(&s.depositor, &32, &600, &future);
    // Bounty D: no deadline (u64::MAX)
    s.escrow.lock_funds(&s.depositor, &33, &600, &NO_DEADLINE);

    s.env.ledger().set_timestamp(future + 1);

    assert!(s.escrow.try_refund(&32).is_ok());
    assert_eq!(
        s.escrow.try_refund(&33).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );
}

// =============================================================================
// Exact boundary tests
//
// These tests verify the precise boundary condition: refund at `deadline - 1`
// must fail, at `deadline` must succeed.  This validates the `>=` comparison
// used in the contract's deadline check.
// =============================================================================

#[test]
fn test_refund_at_exact_deadline_boundary() {
    let s = Setup::new();
    let deadline = 5_000_u64;
    s.escrow.lock_funds(&s.depositor, &40, &1_000, &deadline);

    // One second before deadline: refund blocked
    s.env.ledger().set_timestamp(deadline - 1);
    let result = s.escrow.try_refund(&40);
    assert_eq!(result.unwrap_err().unwrap(), Error::DeadlineNotPassed);

    // Exactly at deadline: refund succeeds
    s.env.ledger().set_timestamp(deadline);
    assert!(s.escrow.try_refund(&40).is_ok());
    assert_eq!(s.escrow.get_escrow_info(&40).status, EscrowStatus::Refunded);
}

#[test]
fn test_release_at_exact_deadline_never_blocked() {
    let s = Setup::new();
    let deadline = 10_000_u64;
    s.escrow.lock_funds(&s.depositor, &41, &500, &deadline);

    // One second before deadline: release succeeds (not gated by deadline)
    s.env.ledger().set_timestamp(deadline - 1);
    s.escrow.release_funds(&41, &s.contributor);

    assert_eq!(s.escrow.get_escrow_info(&41).status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), 500);
}

// =============================================================================
// Partial refund with deadline variants
//
// Verifies that admin-approved partial refunds work correctly with all three
// deadline configurations and that the remaining balance is preserved.
// =============================================================================

#[test]
fn test_zero_deadline_partial_refund() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &50, &2_000, &0);

    s.escrow
        .approve_refund(&50, &600, &s.depositor, &RefundMode::Partial);
    s.escrow.refund(&50);

    let info = s.escrow.get_escrow_info(&50);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, 1_400);
    assert_eq!(s.token.balance(&s.escrow.address), 1_400);
}

#[test]
fn test_future_deadline_partial_refund_with_approval() {
    let s = Setup::new();
    let deadline = s.env.ledger().timestamp() + 86_400;
    s.escrow.lock_funds(&s.depositor, &51, &3_000, &deadline);

    // Before deadline: partial refund via admin approval
    s.escrow
        .approve_refund(&51, &1_000, &s.depositor, &RefundMode::Partial);

    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&51);

    let info = s.escrow.get_escrow_info(&51);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, 2_000);
    assert_eq!(s.token.balance(&s.depositor), before + 1_000);
}

// =============================================================================
// Multiple bounties with mixed deadline types
//
// Validates that different deadline configurations coexist correctly and that
// operations on one bounty do not affect the deadline semantics of another.
// =============================================================================

#[test]
fn test_mixed_deadline_types_coexist_independently() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();

    // Create bounties with all three deadline types
    s.escrow.lock_funds(&s.depositor, &60, &500, &0); // zero
    s.escrow.lock_funds(&s.depositor, &61, &500, &(now + 1_000)); // future
    s.escrow.lock_funds(&s.depositor, &62, &500, &NO_DEADLINE); // no deadline

    // Verify all stored correctly
    assert_eq!(s.escrow.get_escrow_info(&60).deadline, 0);
    assert_eq!(s.escrow.get_escrow_info(&61).deadline, now + 1_000);
    assert_eq!(s.escrow.get_escrow_info(&62).deadline, NO_DEADLINE);

    // Zero-deadline bounty can refund now
    assert!(s.escrow.try_refund(&60).is_ok());

    // Future-deadline bounty still blocked
    assert_eq!(
        s.escrow.try_refund(&61).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );

    // No-deadline bounty permanently blocked
    assert_eq!(
        s.escrow.try_refund(&62).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );

    // Advance past future deadline
    s.env.ledger().set_timestamp(now + 1_001);

    // Future-deadline bounty now refundable
    assert!(s.escrow.try_refund(&61).is_ok());

    // No-deadline bounty still blocked
    assert_eq!(
        s.escrow.try_refund(&62).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );
}

#[test]
fn test_release_unaffected_by_any_deadline_variant() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();
    let contributor2 = Address::generate(&s.env);
    let contributor3 = Address::generate(&s.env);

    // Lock with all three deadline types
    s.escrow.lock_funds(&s.depositor, &70, &300, &0);
    s.escrow
        .lock_funds(&s.depositor, &71, &300, &(now + 10_000));
    s.escrow.lock_funds(&s.depositor, &72, &300, &NO_DEADLINE);

    // Release all — none should be blocked by deadline
    s.escrow.release_funds(&70, &s.contributor);
    s.escrow.release_funds(&71, &contributor2);
    s.escrow.release_funds(&72, &contributor3);

    assert_eq!(s.escrow.get_escrow_info(&70).status, EscrowStatus::Released);
    assert_eq!(s.escrow.get_escrow_info(&71).status, EscrowStatus::Released);
    assert_eq!(s.escrow.get_escrow_info(&72).status, EscrowStatus::Released);

    assert_eq!(s.token.balance(&s.contributor), 300);
    assert_eq!(s.token.balance(&contributor2), 300);
    assert_eq!(s.token.balance(&contributor3), 300);
}

// =============================================================================
// Token balance integrity after deadline-based operations
//
// Ensures the contract's internal balance matches the token contract balance
// across all deadline variants after refund/release operations.
// =============================================================================

#[test]
fn test_token_balance_integrity_across_deadline_variants() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();

    // Lock 1000 each with different deadlines
    s.escrow.lock_funds(&s.depositor, &80, &1_000, &0);
    s.escrow.lock_funds(&s.depositor, &81, &1_000, &(now + 500));
    s.escrow.lock_funds(&s.depositor, &82, &1_000, &NO_DEADLINE);

    assert_eq!(s.token.balance(&s.escrow.address), 3_000);

    // Refund zero-deadline bounty
    s.escrow.refund(&80);
    assert_eq!(s.token.balance(&s.escrow.address), 2_000);

    // Release no-deadline bounty
    s.escrow.release_funds(&82, &s.contributor);
    assert_eq!(s.token.balance(&s.escrow.address), 1_000);

    // Advance and refund future-deadline bounty
    s.env.ledger().set_timestamp(now + 501);
    s.escrow.refund(&81);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// =============================================================================
// No-deadline with admin approval workflow
//
// Validates the complete admin-approved refund workflow for no-deadline escrows,
// including both full and partial approval scenarios.
// =============================================================================

#[test]
fn test_no_deadline_full_refund_workflow() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &90, &5_000, &NO_DEADLINE);

    // Confirm refund is blocked initially
    assert_eq!(
        s.escrow.try_refund(&90).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );

    // Admin approves full refund
    s.escrow
        .approve_refund(&90, &5_000, &s.depositor, &RefundMode::Full);

    // Now refund succeeds
    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&90);

    let info = s.escrow.get_escrow_info(&90);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(s.token.balance(&s.depositor), before + 5_000);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

#[test]
fn test_no_deadline_escrow_released_after_refund_blocked() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &91, &2_000, &NO_DEADLINE);

    // Refund blocked (no deadline, no approval)
    assert_eq!(
        s.escrow.try_refund(&91).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );

    // But release always works
    s.escrow.release_funds(&91, &s.contributor);
    assert_eq!(s.escrow.get_escrow_info(&91).status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), 2_000);
}
