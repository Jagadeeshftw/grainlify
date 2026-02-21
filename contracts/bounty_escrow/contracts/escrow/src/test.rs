#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
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

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
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
        token_admin.mint(&depositor, &1_000_000);

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
}

// =============================================================================
// Existing core tests
// =============================================================================

#[test]
fn test_lock_funds_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Verify stored escrow data
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.depositor, setup.depositor);
    assert_eq!(stored_escrow.amount, amount);
    // remaining_amount must equal amount immediately after lock
    assert_eq!(stored_escrow.remaining_amount, amount);
    assert_eq!(stored_escrow.status, EscrowStatus::Locked);
    assert_eq!(stored_escrow.deadline, deadline);

    // Verify contract balance
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BountyExists
fn test_lock_funds_duplicate() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Try to lock again with same bounty_id
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
#[should_panic] // Token transfer fail
fn test_lock_funds_negative_amount() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = -100;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
fn test_get_escrow_info() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.remaining_amount, amount);
    assert_eq!(escrow.deadline, deadline);
    assert_eq!(escrow.depositor, setup.depositor);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_release_funds_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Verify initial balances
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);

    // Release funds
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Verify updated state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Released);

    // Verify balances after release
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_release_funds_already_released() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Try to release again
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_release_funds_not_found() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

#[test]
fn test_refund_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Advance time past deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    // Initial value
    let initial_depositor_balance = setup.token.balance(&setup.depositor);

    // Refund
    setup.escrow.refund(&bounty_id);

    // Verify state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Refunded);

    // Verify balances
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_depositor_balance + amount
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed
fn test_refund_too_early() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Attempt refund before deadline
    setup.escrow.refund(&bounty_id);
}

#[test]
fn test_get_balance() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 500;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Initial balance should be 0
    assert_eq!(setup.escrow.get_balance(), 0);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Balance should be updated
    assert_eq!(setup.escrow.get_balance(), amount);
}

// =============================================================================
// Partial Payout Rounding and Small Amount Tests (Issue #354)
// =============================================================================

/// Releasing the smallest possible unit (1) should succeed.
/// remaining_amount must decrease by exactly 1 with no rounding loss.
#[test]
fn test_partial_release_single_minimum_unit() {
    let setup = TestSetup::new();
    let bounty_id = 42;
    let amount = 1000_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let payout = 1_i128;
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &payout);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, amount - payout); // 999
    assert_eq!(escrow.status, EscrowStatus::Locked); // still locked, funds remain
    assert_eq!(setup.token.balance(&setup.contributor), payout);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount - payout);
}

/// Releasing all but 1 unit must leave a remainder of exactly 1 — not 0 or negative.
/// Verifies no dust is silently consumed by an off-by-one in the subtraction.
#[test]
fn test_partial_release_leaves_tiny_remainder() {
    let setup = TestSetup::new();
    let bounty_id = 43;
    let amount = 1000_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let payout = amount - 1; // leave exactly 1 unit behind
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &payout);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 1);
    assert!(escrow.remaining_amount >= 0); // non-negative invariant
    assert_eq!(escrow.status, EscrowStatus::Locked); // not yet fully released
    assert_eq!(setup.token.balance(&setup.contributor), payout);
}

/// Multiple sequential partial payouts must track remaining_amount correctly at every step.
/// 10 payouts of 10 from a 100-unit escrow: remaining decrements by 10 each time,
/// reaches 0 on the last call, and status flips to Released.
#[test]
fn test_partial_release_multiple_sequential_small_amounts() {
    let setup = TestSetup::new();
    let bounty_id = 44;
    let amount = 100_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let payout_per_step = 10_i128;
    let steps = 10_i128;

    for i in 1..=steps {
        setup
            .escrow
            .partial_release(&bounty_id, &setup.contributor, &payout_per_step);

        let escrow = setup.escrow.get_escrow_info(&bounty_id);
        let expected_remaining = amount - (payout_per_step * i);

        assert_eq!(escrow.remaining_amount, expected_remaining);
        // Non-negative invariant must hold at every intermediate step
        assert!(
            escrow.remaining_amount >= 0,
            "remaining_amount went negative at step {}: {}",
            i,
            escrow.remaining_amount
        );
    }

    // After all steps: fully paid out
    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.remaining_amount, 0);
    assert_eq!(final_escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

/// Releasing 100% of the locked amount in a single partial_release call
/// must behave identically to release_funds: remaining = 0, status = Released.
#[test]
fn test_partial_release_full_amount_in_one_shot_marks_released() {
    let setup = TestSetup::new();
    let bounty_id = 45;
    let amount = 500_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &amount);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 0);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

/// Attempting to release more than remaining_amount must be rejected.
/// After a partial release leaves 10 units, trying to release 11 must panic
/// with InsufficientFunds — ensuring no overpayment or rounding exploit is possible.
#[test]
#[should_panic(expected = "Error(Contract, #16)")] // InsufficientFunds
fn test_partial_release_overpayment_panics() {
    let setup = TestSetup::new();
    let bounty_id = 46;
    let amount = 100_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // First partial release leaves 10 remaining
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &90_i128);

    // 11 > 10: must be rejected
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &11_i128);
}

/// Releasing exactly the remaining amount (no more, no less) after a prior
/// partial release must succeed and leave remaining_amount at 0.
#[test]
fn test_partial_release_exact_remaining_after_prior_release() {
    let setup = TestSetup::new();
    let bounty_id = 49;
    let amount = 100_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Release 60, then release the exact remaining 40
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &60_i128);

    let mid_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(mid_escrow.remaining_amount, 40);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &40_i128);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.remaining_amount, 0);
    assert_eq!(final_escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

/// Passing zero as payout_amount must be rejected as InvalidAmount.
/// Zero-value transfers would waste gas and corrupt event logs.
#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_partial_release_zero_amount_rejected() {
    let setup = TestSetup::new();
    let bounty_id = 47;
    let amount = 1000_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &0_i128);
}

/// Using an odd total (7 units) split into uneven steps (3 + 3 + 1) must never
/// let remaining_amount go negative and must reach exactly 0 at the end.
/// This catches any integer underflow or off-by-one in the subtraction path.
#[test]
fn test_partial_release_remaining_amount_never_goes_negative() {
    let setup = TestSetup::new();
    let bounty_id = 48;
    let amount = 7_i128; // odd number to stress uneven splits
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Uneven splits that sum to exactly 7
    for payout in [3_i128, 3_i128, 1_i128] {
        setup
            .escrow
            .partial_release(&bounty_id, &setup.contributor, &payout);

        let escrow = setup.escrow.get_escrow_info(&bounty_id);
        assert!(
            escrow.remaining_amount >= 0,
            "remaining_amount went negative: {}",
            escrow.remaining_amount
        );
    }

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.remaining_amount, 0);
    assert_eq!(final_escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

/// Partial release on a bounty that does not exist must return BountyNotFound.
#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_partial_release_bounty_not_found() {
    let setup = TestSetup::new();
    setup
        .escrow
        .partial_release(&999_u64, &setup.contributor, &100_i128);
}

/// Partial release on an already-released bounty must return FundsNotLocked.
/// Once status is Released no further releases are permitted.
#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_partial_release_on_already_released_bounty_panics() {
    let setup = TestSetup::new();
    let bounty_id = 50;
    let amount = 200_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Full release via the standard path
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Any further partial_release must be rejected
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &1_i128);
}

/// Refund after a partial release must only return what is still remaining,
/// not the original full amount. Verifies remaining_amount drives refund size.
#[test]
fn test_refund_after_partial_release_returns_only_remainder() {
    let setup = TestSetup::new();
    let bounty_id = 51;
    let amount = 1000_i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Partially release 300 to contributor
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &300_i128);

    // Advance time past deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    let depositor_balance_before = setup.token.balance(&setup.depositor);

    // Refund should only return the remaining 700
    setup.escrow.refund(&bounty_id);

    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Refunded);

    // Contributor keeps their 300; depositor gets back only 700
    assert_eq!(setup.token.balance(&setup.contributor), 300);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        depositor_balance_before + 700
    );
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

// Claim Period Expiry and Cancellation Tests

/// Set a claim window, authorize a claim, and have the beneficiary claim
/// within the window. Funds must transfer and escrow status must be Released.
#[test]
fn test_claim_within_window_transfers_funds() {
    let setup = TestSetup::new();
    let bounty_id = 100_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&500_u64);

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.recipient, setup.contributor);
    assert_eq!(pending.amount, amount);
    assert!(!pending.claimed);
    assert!(pending.expires_at > setup.env.ledger().timestamp());

    let before = setup.token.balance(&setup.contributor);
    setup.escrow.claim(&bounty_id);
    assert_eq!(setup.token.balance(&setup.contributor), before + amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Released);
    let claim_after = setup.escrow.get_pending_claim(&bounty_id);
    assert!(claim_after.claimed);
}

/// Authorize a claim then advance time past the window. Calling claim() must
/// fail — funds must NOT leave the contract and escrow must stay Locked.
#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed reused for ClaimExpired
fn test_claim_after_window_expires_panics() {
    let setup = TestSetup::new();
    let bounty_id = 101_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin sets a 200-second window
    setup.escrow.set_claim_window(&200_u64);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Advance ledger time past the claim window
    let now = setup.env.ledger().timestamp();
    setup.env.ledger().set_timestamp(now + 201);

    // This must panic — window has expired
    setup.escrow.claim(&bounty_id);
}

/// Admin cancels a pending claim. The PendingClaim record must be removed and
/// the escrow must remain Locked so it can be re-authorized or refunded later.
#[test]
fn test_cancel_pending_claim_restores_escrow() {
    let setup = TestSetup::new();
    let bounty_id = 102_u64;
    let amount = 2_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&300_u64);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.amount, amount);
    setup.escrow.cancel_pending_claim(&bounty_id);
    let result = setup.escrow.try_get_pending_claim(&bounty_id);
    assert!(
        result.is_err(),
        "PendingClaim should be removed after cancel"
    );
    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);
}

/// Cancelling a claim that does not exist must return BountyNotFound.
#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_cancel_pending_claim_not_found() {
    let setup = TestSetup::new();
    setup.escrow.cancel_pending_claim(&999_u64);
}

/// After cancelling an expired claim the admin can authorize a new one for a
/// different (or same) recipient, and that new claim can be claimed normally.
#[test]
fn test_cancel_expired_claim_then_authorize_new_one() {
    let setup = TestSetup::new();
    let bounty_id = 103_u64;
    let amount = 1_500_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;
    let new_contributor = Address::generate(&setup.env);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&100_u64);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
    let now = setup.env.ledger().timestamp();
    setup.env.ledger().set_timestamp(now + 101);
    setup.escrow.cancel_pending_claim(&bounty_id);
    setup.escrow.set_claim_window(&1_000_u64);
    setup.escrow.authorize_claim(&bounty_id, &new_contributor);

    let new_pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(new_pending.recipient, new_contributor);
    assert!(!new_pending.claimed);

    setup.escrow.claim(&bounty_id);

    assert_eq!(setup.token.balance(&new_contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);

    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Released);
}

/// After cancelling a pending claim, calling release_funds normally must still
/// work — the escrow is Locked and available for the standard release path.
#[test]
fn test_cancel_claim_then_use_release_funds_normally() {
    let setup = TestSetup::new();
    let bounty_id = 104_u64;
    let amount = 800_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&300_u64);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Admin cancels the claim
    setup.escrow.cancel_pending_claim(&bounty_id);

    // Standard release still works
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    assert_eq!(setup.token.balance(&setup.contributor), amount);
    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Released);
}

/// Attempting to claim a bounty twice must fail with FundsNotLocked on the
/// second attempt (funds already claimed / escrow already Released).
#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_claim_twice_panics() {
    let setup = TestSetup::new();
    let bounty_id = 105_u64;
    let amount = 500_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&500_u64);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // First claim succeeds
    setup.escrow.claim(&bounty_id);

    // Second claim must panic
    setup.escrow.claim(&bounty_id);
}

/// Claiming one bounty must not affect the balance or status of another
/// unrelated bounty sitting in the same contract.
#[test]
fn test_claim_does_not_affect_other_bounties() {
    let setup = TestSetup::new();
    let bounty_a = 106_u64;
    let bounty_b = 107_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_a, &amount, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_b, &amount, &deadline);

    setup.escrow.set_claim_window(&500_u64);
    setup.escrow.authorize_claim(&bounty_a, &setup.contributor);

    // Claim only bounty_a
    setup.escrow.claim(&bounty_a);

    // bounty_b must remain Locked and untouched
    let escrow_b = setup.escrow.get_escrow_info(&bounty_b);
    assert_eq!(escrow_b.status, EscrowStatus::Locked);
    assert_eq!(escrow_b.remaining_amount, amount);

    // Total contract balance = only bounty_b funds remain
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

/// When no claim window is explicitly set (default 0) authorize_claim creates a
/// claim that expires immediately (expires_at == now). Any claim() call must fail.
#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed / ClaimExpired
fn test_authorize_claim_zero_window_expires_immediately() {
    let setup = TestSetup::new();
    let bounty_id = 108_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Do NOT set a claim window — default is 0
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Even without time advancing, expires_at == now so claim must fail
    // Advance by 1 second to make now > expires_at
    let now = setup.env.ledger().timestamp();
    setup.env.ledger().set_timestamp(now + 1);

    setup.escrow.claim(&bounty_id);
}

/// Claim at the exact boundary (now == expires_at) must succeed — the window
/// is inclusive of the final second.
#[test]
fn test_claim_at_exact_window_boundary_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 109_u64;
    let amount = 1_000_i128;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;
    let window = 300_u64;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&window);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    setup.env.ledger().set_timestamp(pending.expires_at);
    setup.escrow.claim(&bounty_id);

    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

/// authorize_claim on a bounty_id that does not exist must return BountyNotFound.
#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_authorize_claim_on_nonexistent_bounty() {
    let setup = TestSetup::new();
    setup.escrow.authorize_claim(&999_u64, &setup.contributor);
}

/// authorize_claim on a bounty that is already Released must return FundsNotLocked.
#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_authorize_claim_on_released_bounty() {
    let setup = TestSetup::new();
    let bounty_id = 110_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
}

/// authorize_claim on a Refunded bounty must return FundsNotLocked.
#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_authorize_claim_on_refunded_bounty() {
    let setup = TestSetup::new();
    let bounty_id = 111_u64;
    let amount = 1_000_i128;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 500;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
}

/// When set_claim_window has never been called the default window (0) is used.
/// The pending claim's expires_at must equal the ledger timestamp at auth time.
#[test]
fn test_authorize_claim_default_window_used_when_not_set() {
    let setup = TestSetup::new();
    let bounty_id = 112_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let auth_time = setup.env.ledger().timestamp();
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.expires_at, auth_time);
}

/// Verifies set_claim_window stores the value and authorize_claim uses it.
#[test]
fn test_set_claim_window_success() {
    let setup = TestSetup::new();
    let bounty_id = 113_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;
    let window = 600_u64;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&window);

    let auth_time = setup.env.ledger().timestamp();
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.expires_at, auth_time + window);
}

/// get_pending_claim on a bounty with no pending claim must return BountyNotFound.
#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_get_pending_claim_not_found() {
    let setup = TestSetup::new();
    setup.escrow.get_pending_claim(&999_u64);
}

/// authorize_claim creates a ClaimRecord with the correct bounty_id and amount.
#[test]
fn test_authorize_claim_creates_pending_claim() {
    let setup = TestSetup::new();
    let bounty_id = 114_u64;
    let amount = 3_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&400_u64);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.bounty_id, bounty_id);
    assert_eq!(pending.amount, amount);
    assert_eq!(pending.recipient, setup.contributor);
    assert!(!pending.claimed);
    // ============================================================================
    // BATCH LOCK AND RELEASE FAILURE MODE TESTS
    // Tests for invalid batch sizes, duplicate IDs, mixed valid/invalid entries,
    // and partial failure scenarios (Issue #358)
    // ============================================================================

    // --- BATCH SIZE BOUNDARY TESTS ---

    #[test]
    fn test_batch_lock_funds_success() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Create batch items
        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: 1000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 2,
                depositor: setup.depositor.clone(),
                amount: 2000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 3,
                depositor: setup.depositor.clone(),
                amount: 3000,
                deadline,
            },
        ];

        // Mint enough tokens
        setup.token_admin.mint(&setup.depositor, &10_000);

        // Batch lock funds
        let count = setup.escrow.batch_lock_funds(&items);
        assert_eq!(count, 3);

        // Verify all bounties are locked
        for i in 1..=3 {
            let escrow = setup.escrow.get_escrow_info(&i);
            assert_eq!(escrow.status, EscrowStatus::Locked);
        }

        // Verify contract balance
        assert_eq!(setup.escrow.get_balance(), 6000);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // InvalidBatchSize
    fn test_batch_lock_funds_empty() {
        let setup = TestSetup::new();
        let items: Vec<LockFundsItem> = vec![&setup.env];
        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    fn test_batch_lock_funds_single_item() {
        // Edge case: minimum valid batch size (1 item)
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: 1000,
                deadline,
            },
        ];

        setup.token_admin.mint(&setup.depositor, &1000);
        let count = setup.escrow.batch_lock_funds(&items);
        assert_eq!(count, 1);

        let escrow = setup.escrow.get_escrow_info(&1);
        assert_eq!(escrow.status, EscrowStatus::Locked);
        assert_eq!(escrow.amount, 1000);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // InvalidBatchSize
    fn test_batch_lock_funds_exceeds_max_batch_size() {
        // Test batch size exceeding MAX_BATCH_SIZE (20)
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Create batch with 21 items (exceeds MAX_BATCH_SIZE of 20)
        let mut items = Vec::new(&setup.env);
        for i in 1..=21 {
            items.push_back(LockFundsItem {
                bounty_id: i,
                depositor: setup.depositor.clone(),
                amount: 100,
                deadline,
            });
        }

        setup.token_admin.mint(&setup.depositor, &10_000);
        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    fn test_batch_lock_funds_at_max_batch_size() {
        // Test exactly at MAX_BATCH_SIZE boundary (20 items)
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let mut items = Vec::new(&setup.env);
        for i in 1..=20 {
            items.push_back(LockFundsItem {
                bounty_id: i,
                depositor: setup.depositor.clone(),
                amount: 100,
                deadline,
            });
        }

        setup.token_admin.mint(&setup.depositor, &10_000);
        let count = setup.escrow.batch_lock_funds(&items);
        assert_eq!(count, 20);
    }

    // --- DUPLICATE BOUNTY ID TESTS ---

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")] // BountyExists
    fn test_batch_lock_funds_duplicate_bounty_id() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock a bounty first
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);

        // Try to batch lock with duplicate bounty_id
        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1, // Already exists
                depositor: setup.depositor.clone(),
                amount: 2000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 2,
                depositor: setup.depositor.clone(),
                amount: 3000,
                deadline,
            },
        ];

        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #12)")] // DuplicateBountyId
    fn test_batch_lock_funds_duplicate_in_batch() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: 1000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 1, // Duplicate in same batch
                depositor: setup.depositor.clone(),
                amount: 2000,
                deadline,
            },
        ];

        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #12)")] // DuplicateBountyId
    fn test_batch_lock_funds_triple_duplicate_in_batch() {
        // Three items with same bounty_id in batch
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: 1000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 1, // Duplicate
                depositor: setup.depositor.clone(),
                amount: 2000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 1, // Triple duplicate
                depositor: setup.depositor.clone(),
                amount: 3000,
                deadline,
            },
        ];

        setup.token_admin.mint(&setup.depositor, &10000);
        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #12)")] // DuplicateBountyId
    fn test_batch_lock_funds_non_adjacent_duplicates() {
        // Duplicates that are not adjacent in the batch
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1, // First occurrence
                depositor: setup.depositor.clone(),
                amount: 1000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 2, // Different
                depositor: setup.depositor.clone(),
                amount: 2000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 1, // Duplicate of first (non-adjacent)
                depositor: setup.depositor.clone(),
                amount: 3000,
                deadline,
            },
        ];

        setup.token_admin.mint(&setup.depositor, &10000);
        setup.escrow.batch_lock_funds(&items);
    }

    // --- INVALID AMOUNT TESTS ---

    #[test]
    #[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
    fn test_batch_lock_funds_zero_amount() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: 0, // Invalid: zero amount
                deadline,
            },
        ];

        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
    fn test_batch_lock_funds_negative_amount() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: -100, // Invalid: negative amount
                deadline,
            },
        ];

        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
    fn test_batch_lock_funds_mixed_valid_invalid_amounts() {
        // First item valid, second item has zero amount
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: 1000, // Valid
                deadline,
            },
            LockFundsItem {
                bounty_id: 2,
                depositor: setup.depositor.clone(),
                amount: 0, // Invalid
                deadline,
            },
        ];

        setup.token_admin.mint(&setup.depositor, &2000);
        setup.escrow.batch_lock_funds(&items);
    }

    // --- MIXED VALIDITY TESTS ---

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")] // BountyExists
    fn test_batch_lock_funds_first_valid_second_exists() {
        // First item is valid, second already exists - entire batch should fail
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock bounty 2 first
        setup
            .escrow
            .lock_funds(&setup.depositor, &2, &1000, &deadline);

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1, // Valid - doesn't exist yet
                depositor: setup.depositor.clone(),
                amount: 1000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 2, // Invalid - already exists
                depositor: setup.depositor.clone(),
                amount: 2000,
                deadline,
            },
        ];

        setup.token_admin.mint(&setup.depositor, &5000);
        setup.escrow.batch_lock_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")] // BountyExists
    fn test_batch_operations_atomicity() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock one bounty successfully
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);

        // Try to batch lock with one valid and one that would fail (duplicate)
        // This should fail entirely due to atomicity
        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 2, // Valid
                depositor: setup.depositor.clone(),
                amount: 2000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 1, // Already exists - should cause entire batch to fail
                depositor: setup.depositor.clone(),
                amount: 3000,
                deadline,
            },
        ];

        // This should panic and no bounties should be locked
        setup.escrow.batch_lock_funds(&items);
    }

    // --- BATCH RELEASE TESTS ---

    #[test]
    fn test_batch_release_funds_success() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock multiple bounties
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);
        setup
            .escrow
            .lock_funds(&setup.depositor, &2, &2000, &deadline);
        setup
            .escrow
            .lock_funds(&setup.depositor, &3, &3000, &deadline);

        // Create contributors
        let contributor1 = Address::generate(&setup.env);
        let contributor2 = Address::generate(&setup.env);
        let contributor3 = Address::generate(&setup.env);

        // Create batch release items
        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1,
                contributor: contributor1.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 2,
                contributor: contributor2.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 3,
                contributor: contributor3.clone(),
            },
        ];

        // Batch release funds
        let count = setup.escrow.batch_release_funds(&items);
        assert_eq!(count, 3);

        // Verify all bounties are released
        for i in 1..=3 {
            let escrow = setup.escrow.get_escrow_info(&i);
            assert_eq!(escrow.status, EscrowStatus::Released);
        }

        // Verify balances
        assert_eq!(setup.token.balance(&contributor1), 1000);
        assert_eq!(setup.token.balance(&contributor2), 2000);
        assert_eq!(setup.token.balance(&contributor3), 3000);
        assert_eq!(setup.escrow.get_balance(), 0);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // InvalidBatchSize
    fn test_batch_release_funds_empty() {
        let setup = TestSetup::new();
        let items: Vec<ReleaseFundsItem> = vec![&setup.env];
        setup.escrow.batch_release_funds(&items);
    }

    #[test]
    fn test_batch_release_funds_single_item() {
        // Edge case: minimum valid batch size (1 item) for release
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);

        let contributor = Address::generate(&setup.env);
        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1,
                contributor: contributor.clone(),
            },
        ];

        let count = setup.escrow.batch_release_funds(&items);
        assert_eq!(count, 1);

        let escrow = setup.escrow.get_escrow_info(&1);
        assert_eq!(escrow.status, EscrowStatus::Released);
        assert_eq!(setup.token.balance(&contributor), 1000);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // InvalidBatchSize
    fn test_batch_release_funds_exceeds_max_batch_size() {
        // Test batch size exceeding MAX_BATCH_SIZE (20) for release
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock 20 bounties first (at max batch size)
        let mut lock_items = Vec::new(&setup.env);
        for i in 1..=20 {
            lock_items.push_back(LockFundsItem {
                bounty_id: i,
                depositor: setup.depositor.clone(),
                amount: 100,
                deadline,
            });
        }
        setup.token_admin.mint(&setup.depositor, &10_000);
        setup.escrow.batch_lock_funds(&lock_items);

        // Try to release 21 items (including one that doesn't exist)
        let mut release_items = Vec::new(&setup.env);
        for i in 1..=21 {
            release_items.push_back(ReleaseFundsItem {
                bounty_id: i,
                contributor: Address::generate(&setup.env),
            });
        }

        setup.escrow.batch_release_funds(&release_items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
    fn test_batch_release_funds_not_found() {
        let setup = TestSetup::new();
        let contributor = Address::generate(&setup.env);

        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 999, // Doesn't exist
                contributor: contributor.clone(),
            },
        ];

        setup.escrow.batch_release_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
    fn test_batch_release_funds_already_released() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock and release one bounty
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);
        setup.escrow.release_funds(&1, &setup.contributor);

        // Lock another bounty
        setup
            .escrow
            .lock_funds(&setup.depositor, &2, &2000, &deadline);

        let contributor2 = Address::generate(&setup.env);

        // Try to batch release including already released bounty
        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1, // Already released
                contributor: setup.contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 2,
                contributor: contributor2.clone(),
            },
        ];

        setup.escrow.batch_release_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #12)")] // DuplicateBountyId
    fn test_batch_release_funds_duplicate_in_batch() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);

        let contributor = Address::generate(&setup.env);

        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1,
                contributor: contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 1, // Duplicate in same batch
                contributor: contributor.clone(),
            },
        ];

        setup.escrow.batch_release_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
    fn test_batch_release_funds_first_valid_second_not_found() {
        // First item valid, second doesn't exist - entire batch should fail
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock only bounty 1
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);

        let contributor = Address::generate(&setup.env);
        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1, // Valid - exists and locked
                contributor: contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 999, // Invalid - doesn't exist
                contributor: contributor.clone(),
            },
        ];

        setup.escrow.batch_release_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
    fn test_batch_release_funds_mixed_locked_and_refunded() {
        // First bounty locked, second refunded - should fail
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 100; // Short deadline

        // Lock two bounties
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);
        setup
            .escrow
            .lock_funds(&setup.depositor, &2, &2000, &deadline);

        // Advance time past deadline and refund bounty 2
        setup.env.ledger().set_timestamp(deadline + 1);
        setup.escrow.refund(&2);

        let contributor = Address::generate(&setup.env);
        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1, // Valid - still locked
                contributor: contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 2, // Invalid - already refunded (not locked)
                contributor: contributor.clone(),
            },
        ];

        setup.escrow.batch_release_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #12)")] // DuplicateBountyId
    fn test_batch_release_funds_non_adjacent_duplicates() {
        // Duplicates that are not adjacent in release batch
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock three bounties
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);
        setup
            .escrow
            .lock_funds(&setup.depositor, &2, &2000, &deadline);

        let contributor = Address::generate(&setup.env);
        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1, // First occurrence
                contributor: contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 2, // Different
                contributor: contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 1, // Duplicate of first (non-adjacent)
                contributor: contributor.clone(),
            },
        ];

        setup.escrow.batch_release_funds(&items);
    }

    // --- LARGE BATCH AND MULTIPLE DEPOSITOR/CONTRIBUTOR TESTS ---

    #[test]
    fn test_batch_operations_large_batch() {
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Create a batch of 10 bounties
        let mut items = Vec::new(&setup.env);
        for i in 1..=10 {
            items.push_back(LockFundsItem {
                bounty_id: i,
                depositor: setup.depositor.clone(),
                amount: (i * 100) as i128,
                deadline,
            });
        }

        // Mint enough tokens
        setup.token_admin.mint(&setup.depositor, &10_000);

        // Batch lock
        let count = setup.escrow.batch_lock_funds(&items);
        assert_eq!(count, 10);

        // Verify all are locked
        for i in 1..=10 {
            let escrow = setup.escrow.get_escrow_info(&i);
            assert_eq!(escrow.status, EscrowStatus::Locked);
        }

        // Create batch release items
        let mut release_items = Vec::new(&setup.env);
        for i in 1..=10 {
            release_items.push_back(ReleaseFundsItem {
                bounty_id: i,
                contributor: Address::generate(&setup.env),
            });
        }

        // Batch release
        let release_count = setup.escrow.batch_release_funds(&release_items);
        assert_eq!(release_count, 10);
    }

    #[test]
    fn test_batch_operations_multiple_depositors() {
        // Test batch lock with multiple different depositors
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        let depositor2 = Address::generate(&setup.env);

        // Get initial balance of setup.depositor (already has 1,000,000 from TestSetup)
        let initial_depositor_balance = setup.token.balance(&setup.depositor);

        // Mint tokens for depositor2 only (setup.depositor already has enough)
        setup.token_admin.mint(&depositor2, &5000);

        let items = vec![
            &setup.env,
            LockFundsItem {
                bounty_id: 1,
                depositor: setup.depositor.clone(),
                amount: 1000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 2,
                depositor: depositor2.clone(),
                amount: 2000,
                deadline,
            },
            LockFundsItem {
                bounty_id: 3,
                depositor: setup.depositor.clone(),
                amount: 3000,
                deadline,
            },
        ];

        let count = setup.escrow.batch_lock_funds(&items);
        assert_eq!(count, 3);

        // Verify each bounty has correct depositor
        let escrow1 = setup.escrow.get_escrow_info(&1);
        let escrow2 = setup.escrow.get_escrow_info(&2);
        let escrow3 = setup.escrow.get_escrow_info(&3);

        assert_eq!(escrow1.depositor, setup.depositor);
        assert_eq!(escrow2.depositor, depositor2);
        assert_eq!(escrow3.depositor, setup.depositor);

        // Verify balances
        // setup.depositor: initial - 1000 - 3000 = initial - 4000
        assert_eq!(
            setup.token.balance(&setup.depositor),
            initial_depositor_balance - 4000
        );
        // depositor2: 5000 - 2000 = 3000
        assert_eq!(setup.token.balance(&depositor2), 3000);
        assert_eq!(setup.escrow.get_balance(), 6000);
    }

    #[test]
    fn test_batch_release_funds_to_multiple_contributors() {
        // Test batch release to different contributors
        let setup = TestSetup::new();
        let deadline = setup.env.ledger().timestamp() + 1000;

        // Lock bounties
        setup
            .escrow
            .lock_funds(&setup.depositor, &1, &1000, &deadline);
        setup
            .escrow
            .lock_funds(&setup.depositor, &2, &2000, &deadline);
        setup
            .escrow
            .lock_funds(&setup.depositor, &3, &3000, &deadline);

        let contributor1 = Address::generate(&setup.env);
        let contributor2 = Address::generate(&setup.env);
        let contributor3 = Address::generate(&setup.env);

        let items = vec![
            &setup.env,
            ReleaseFundsItem {
                bounty_id: 1,
                contributor: contributor1.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 2,
                contributor: contributor2.clone(),
            },
            ReleaseFundsItem {
                bounty_id: 3,
                contributor: contributor3.clone(),
            },
        ];

        let count = setup.escrow.batch_release_funds(&items);
        assert_eq!(count, 3);

        // Verify each contributor received correct amount
        assert_eq!(setup.token.balance(&contributor1), 1000);
        assert_eq!(setup.token.balance(&contributor2), 2000);
        assert_eq!(setup.token.balance(&contributor3), 3000);
        assert_eq!(setup.escrow.get_balance(), 0);
    }
}
