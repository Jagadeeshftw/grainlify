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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

    // Verify stored escrow data
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.depositor, setup.depositor);
    assert_eq!(stored_escrow.amount, amount);
    // remaining_amount must equal amount immediately after lock
    assert_eq!(stored_escrow.remaining_amount, amount);
    assert_eq!(stored_escrow.status, EscrowStatus::Locked);
    assert_eq!(stored_escrow.deadline, Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

    // Try to lock again with same bounty_id
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));
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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));
}

#[test]
fn test_get_escrow_info() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.remaining_amount, amount);
    assert_eq!(escrow.deadline, Some(deadline));
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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));
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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
        .lock_funds(&setup.depositor, &bounty_id, &amount, &Some(deadline));

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
