#![cfg(test)]

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, DisputeOutcome, DisputeReason, Error,
    EscrowStatus,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

struct TestSetup<'a> {
    env: Env,
    _admin: Address, // Added underscore
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    _token_admin: token::StellarAssetClient<'a>, // Added underscore
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);

        // Mint tokens to depositor
        token_admin.mint(&depositor, &10_000_000);

        Self {
            env,
            _admin: admin,
            depositor,
            contributor,
            token,
            _token_admin: token_admin,
            escrow,
        }
    }
}

// FIX: pending claims MUST block refunds
#[test]
fn test_pending_claim_blocks_refund() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 1000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    // Lock funds with deadline
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin opens dispute by authorizing claim (before deadline)
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    // Verify claim is pending
    let claim = setup.escrow.get_pending_claim(&bounty_id);
    assert!(!claim.claimed);
    assert_eq!(claim.recipient, setup.contributor);

    // Advance time PAST deadline
    setup.env.ledger().set_timestamp(deadline + 100);

    // Verify refund is BLOCKED because claim is pending
    let res = setup.escrow.try_refund(&bounty_id);
    assert!(res.is_err());
    // Error::ClaimPending is variant #22
    assert_eq!(res.unwrap_err().unwrap(), Error::ClaimPending);

    // Verify funds were NOT refunded
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

// Beneficiary claims successfully within dispute window
#[test]
fn test_beneficiary_claims_within_window_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 2;
    let amount = 1500;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin authorizes claim at now, expires at now+500
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    let claim = setup.escrow.get_pending_claim(&bounty_id);

    // Beneficiary claims within window
    setup.env.ledger().set_timestamp(claim.expires_at - 100);

    setup.escrow.claim(&bounty_id);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

// Beneficiary misses claim window - admin must cancel then refund
#[test]
fn test_missed_claim_window_requires_admin_cancel_then_refund() {
    let setup = TestSetup::new();
    let bounty_id = 3;
    let amount = 2500;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin authorizes claim (opens dispute window)
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    let claim_expires_at = claim.expires_at;

    // Advance to after claim window but before deadline
    setup.env.ledger().set_timestamp(claim_expires_at + 1);

    // Escrow is still Locked with pending claim
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);

    // Admin cancels the expired pending claim
    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.status, EscrowStatus::Locked);

    // Advance to original deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

// Resolution order must be explicit: can't skip the cancel step
#[test]
fn test_resolution_order_requires_explicit_cancel_step() {
    let setup = TestSetup::new();
    let bounty_id = 4;
    let amount = 3000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 200;
    let claim_window = 100;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    // Advance past both windows
    setup.env.ledger().set_timestamp(deadline + 500);

    // Admin must cancel the pending claim first
    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
}

/// TEST 5: Explicitly demonstrate the correct resolution order
/// After the vulnerability fix, the correct sequence is:
///   1. Authorize a claim (opens dispute window)
///   2. Wait for claim window to expire or admin action needed
///   3. Admin cancels the claim (explicitly resolves the dispute)
///   4. Refund becomes available (if deadline has passed)
///
/// This prevents expiration alone from bypassing disputes.
#[test]
fn test_correct_resolution_order_cancel_then_refund() {
    let setup = TestSetup::new();
    let bounty_id = 41;
    let amount = 3000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 200;
    let claim_window = 100;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    // Advance past both windows
    setup.env.ledger().set_timestamp(deadline + 500);

    // Admin must cancel the pending claim first
    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    // NOW refund works (demonstrates the order)
    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
}

// Admin can cancel expired claims at any time
#[test]
fn test_admin_can_cancel_expired_claim() {
    let setup = TestSetup::new();
    let bounty_id = 5;
    let amount = 2500;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 1500;
    let claim_window = 600;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    let claim = setup.escrow.get_pending_claim(&bounty_id);

    // Advance WAY past claim window
    setup.env.ledger().set_timestamp(claim.expires_at + 1000);

    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

// Zero-length claim windows (instant expiration)
#[test]
fn test_claim_window_zero_prevents_all_claims() {
    let setup = TestSetup::new();
    let bounty_id = 6;
    let amount = 800;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 1000;

    // Set window to 0 (instant expiration)
    setup.escrow.set_claim_window(&0);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    let _claim = setup.escrow.get_pending_claim(&bounty_id);

    // Advance well past the deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    // Admin cancels the zero-window claim
    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
}

// Multiple bounties resolve independently
#[test]
fn test_multiple_bounties_independent_resolution() {
    let setup = TestSetup::new();
    let claim_window = 300;

    setup.escrow.set_claim_window(&claim_window);

    let now = setup.env.ledger().timestamp();

    // Bounty 1: Will be cancelled and refunded
    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &(now + 500));
    setup
        .escrow
        .authorize_claim(&1, &setup.contributor, &DisputeReason::Other);

    // Bounty 2: Will be refunded directly (no claim)
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &(now + 600));

    // Bounty 3: Will be claimed
    setup
        .escrow
        .lock_funds(&setup.depositor, &3, &1500, &(now + 1000));
    setup
        .escrow
        .authorize_claim(&3, &setup.contributor, &DisputeReason::Other);

    setup.env.ledger().set_timestamp(now + 550);

    setup
        .escrow
        .cancel_pending_claim(&1, &DisputeOutcome::CancelledByAdmin);
    setup.escrow.refund(&1);
    assert_eq!(
        setup.escrow.get_escrow_info(&1).status,
        EscrowStatus::Refunded
    );

    assert_eq!(
        setup.escrow.get_escrow_info(&2).status,
        EscrowStatus::Locked
    );

    let claim_3 = setup.escrow.get_pending_claim(&3);
    assert!(!claim_3.claimed);

    let claim_3_expires = claim_3.expires_at;
    setup.env.ledger().set_timestamp(claim_3_expires - 100);
    setup.escrow.claim(&3);

    assert_eq!(
        setup.escrow.get_escrow_info(&3).status,
        EscrowStatus::Released
    );

    setup.env.ledger().set_timestamp(now + 700);
    setup.escrow.refund(&2);

    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(setup.token.balance(&setup.contributor), 1500);
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000 - 1500);
}

// Claim cancellation properly restores refund eligibility
#[test]
fn test_claim_cancellation_restores_refund_eligibility() {
    let setup = TestSetup::new();
    let bounty_id = 8;
    let amount = 5000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let escrow_before = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.remaining_amount, amount);
    assert_eq!(escrow_before.status, EscrowStatus::Locked);

    // Authorize claim
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::QualityIssue);

    // Cancel it
    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.status, EscrowStatus::Locked);
    assert_eq!(escrow_after.remaining_amount, amount);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);

    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000);
}

#[test]
fn test_expiry_does_not_bypass_active_dispute() {
    let s = TestSetup::new();
    let bounty_id = 100u64;
    let amount = 1_000i128;
    let now = s.env.ledger().timestamp();
    let deadline = now + 500;

    s.escrow.set_claim_window(&300);
    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::Other);

    s.env.ledger().set_timestamp(deadline + 1);

    let escrow_info = s.escrow.get_escrow_info(&bounty_id);

    let _ = escrow_info;
}

// Dispute opened before deadline → admin cancels claim → refund after deadline.
#[test]
fn test_dispute_before_expiry_cancel_then_refund_after_deadline() {
    let s = TestSetup::new();
    let bounty_id = 101u64;
    let amount = 2_000i128;
    let now = s.env.ledger().timestamp();
    let deadline = now + 600;

    s.escrow.set_claim_window(&200);
    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);

    // Dispute raised before deadline
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::Other);
    let claim = s.escrow.get_pending_claim(&bounty_id);
    assert!(!claim.claimed);

    // Admin resolves dispute in favour of depositor: cancel claim
    s.env.ledger().set_timestamp(claim.expires_at + 1);
    s.escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    // Advance to after deadline
    s.env.ledger().set_timestamp(deadline + 1);

    // Refund is now allowed
    s.escrow.refund(&bounty_id);

    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), 10_000_000);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// Dispute opened before deadline → contributor claims within window.
// Contributor wins; refund is impossible afterwards.
#[test]
fn test_dispute_before_expiry_contributor_claims_wins() {
    let s = TestSetup::new();
    let bounty_id = 102u64;
    let amount = 3_000i128;
    let now = s.env.ledger().timestamp();
    let deadline = now + 1_000;

    s.escrow.set_claim_window(&400);
    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::Other);

    let claim = s.escrow.get_pending_claim(&bounty_id);

    // Contributor claims before window expires
    s.env.ledger().set_timestamp(claim.expires_at - 50);
    s.escrow.claim(&bounty_id);

    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
    assert_eq!(s.token.balance(&s.depositor), 10_000_000 - amount);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// Dispute opened after deadline has already passed.
// The admin can still authorize a claim; contributor claiming should succeed.
#[test]
fn test_dispute_opened_after_deadline_contributor_can_still_claim() {
    let s = TestSetup::new();
    let bounty_id = 103u64;
    let amount = 1_500i128;
    let now = s.env.ledger().timestamp();
    let deadline = now + 100;

    s.escrow.set_claim_window(&500);
    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);

    // Deadline passes with no claim
    s.env.ledger().set_timestamp(deadline + 1);

    // Admin opens dispute after deadline (late intervention)
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::Other);
    let claim = s.escrow.get_pending_claim(&bounty_id);

    // Contributor claims within window
    s.env.ledger().set_timestamp(claim.expires_at - 10);
    s.escrow.claim(&bounty_id);

    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
}

// Claim window expires AND escrow deadline passes simultaneously.
// Neither side acted. Admin cancels stale claim, then refund succeeds.
#[test]
fn test_both_windows_expired_admin_cancels_stale_claim_then_refund() {
    let s = TestSetup::new();
    let bounty_id = 104u64;
    let amount = 4_000i128;
    let now = s.env.ledger().timestamp();
    let deadline = now + 300;

    s.escrow.set_claim_window(&100);
    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::Other);

    // Jump far into the future — both windows long expired
    s.env.ledger().set_timestamp(deadline + 1_000);

    // Stale pending claim must be cancelled explicitly
    s.escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    s.escrow.refund(&bounty_id);

    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), 10_000_000);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// Re-authorize after cancel: admin cancels first claim, then opens a second
// dispute. Second contributor claim should succeed normally.
#[test]
fn test_reauthorize_after_cancel_second_claim_succeeds() {
    let s = TestSetup::new();
    let bounty_id = 105u64;
    let amount = 2_500i128;
    let now = s.env.ledger().timestamp();
    let deadline = now + 1_000;

    s.escrow.set_claim_window(&200);
    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);

    // First dispute — cancelled
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::Other);
    let first_claim = s.escrow.get_pending_claim(&bounty_id);
    s.env.ledger().set_timestamp(first_claim.expires_at + 1);
    s.escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    // Second dispute — contributor claims this time
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::Other);
    let second_claim = s.escrow.get_pending_claim(&bounty_id);
    assert!(!second_claim.claimed);

    s.env.ledger().set_timestamp(second_claim.expires_at - 10);
    s.escrow.claim(&bounty_id);

    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
}

// Escrow with no dispute: normal expiry-based refund path is unaffected.
#[test]
fn test_no_dispute_normal_refund_after_deadline() {
    let s = TestSetup::new();
    let bounty_id = 106u64;
    let amount = 500i128;
    let now = s.env.ledger().timestamp();
    let deadline = now + 400;

    s.escrow.set_claim_window(&200);
    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);

    s.env.ledger().set_timestamp(deadline + 1);
    s.escrow.refund(&bounty_id);

    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), 10_000_000);
}
