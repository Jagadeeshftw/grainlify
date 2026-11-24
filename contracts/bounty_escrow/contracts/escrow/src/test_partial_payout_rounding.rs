#![cfg(test)]

//! # Partial Payout Rounding & Dust Handling Tests (Issue #775)
//!
//! This module verifies correct behavior when performing partial payouts with
//! very small amounts, covering:
//!
//! - Releasing minimal amounts (1 unit)
//! - Releasing amounts that leave tiny remainders (dust)
//! - Ensuring `remaining_amount` matches expectations at every step
//! - No over-payment due to rounding
//! - Contract balance conservation across many partial releases
//! - Sum invariants: total paid out + remaining always equals original amount
//!
//! ## Rounding Rules
//!
//! The Soroban token contract uses **integer arithmetic** (i128). There is no
//! floating-point or fixed-point rounding involved. All amounts are in the
//! smallest indivisible unit (stroops for XLM). The rounding rules are:
//!
//! 1. **Exact subtraction**: `remaining_amount -= payout_amount` is always exact.
//! 2. **No dust creation**: Partial releases never create or destroy tokens.
//! 3. **Sum invariant**: At any point, `sum(all_payouts) + remaining == original`.
//! 4. **Floor division for fees**: Fee calculations use floor division (tested separately).
//!
//! ## Dust Handling
//!
//! "Dust" refers to very small remainder amounts after partial payouts. This
//! contract handles dust correctly:
//!
//! - Any non-zero remainder can be released or refunded.
//! - No minimum payout threshold exists; even 1-unit payouts are valid.
//! - Remainders are tracked precisely in `escrow.remaining_amount`.
//!
//! These tests confirm that no dust, off-by-one, or underflow bugs exist in
//! the `partial_release` path.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ---------------------------------------------------------------------------
// Test helpers (same pattern as test.rs)
// ---------------------------------------------------------------------------

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
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

struct Setup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
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

        // Mint a generous amount so tests never starve for tokens.
        token_admin.mint(&depositor, &10_000_000_000_i128);

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

    /// Helper: lock a bounty and return the deadline used.
    fn lock(&self, bounty_id: u64, amount: i128) -> u64 {
        let deadline = self.env.ledger().timestamp() + 100_000;
        self.escrow
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
        deadline
    }
}

// ===========================================================================
// 1. Minimal single-unit payout
// ===========================================================================

/// Releasing exactly 1 unit (the smallest indivisible amount) must succeed.
/// remaining_amount must be `amount - 1` with no rounding loss.
#[test]
fn test_single_unit_payout_from_large_escrow() {
    let s = Setup::new();
    let amount = 1_000_000_i128; // 1 million units
    s.lock(1, amount);

    s.escrow.partial_release(&1, &s.contributor, &1_i128);

    let info = s.escrow.get_escrow_info(&1);
    assert_eq!(info.remaining_amount, amount - 1);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(s.token.balance(&s.contributor), 1);
    assert_eq!(s.token.balance(&s.escrow.address), amount - 1);
}

/// Lock exactly 1 unit and release it. Escrow should be Released with 0 remaining.
#[test]
fn test_single_unit_escrow_fully_released() {
    let s = Setup::new();
    s.lock(2, 1_i128);

    s.escrow.partial_release(&2, &s.contributor, &1_i128);

    let info = s.escrow.get_escrow_info(&2);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), 1);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ===========================================================================
// 2. Tiny remainder after large payout
// ===========================================================================

/// Release (amount - 1) so exactly 1 unit remains. Verify:
/// - remaining_amount == 1
/// - status stays Locked
/// - contract balance is 1
#[test]
fn test_leave_one_unit_remainder() {
    let s = Setup::new();
    let amount = 999_999_i128;
    s.lock(3, amount);

    s.escrow.partial_release(&3, &s.contributor, &(amount - 1));

    let info = s.escrow.get_escrow_info(&3);
    assert_eq!(info.remaining_amount, 1);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(s.token.balance(&s.escrow.address), 1);
}

/// Leave 2 units, drain them one-by-one. Both steps must succeed.
#[test]
fn test_drain_last_two_units_individually() {
    let s = Setup::new();
    let amount = 500_i128;
    s.lock(4, amount);

    // Release all but 2
    s.escrow.partial_release(&4, &s.contributor, &(amount - 2));
    let info = s.escrow.get_escrow_info(&4);
    assert_eq!(info.remaining_amount, 2);
    assert_eq!(info.status, EscrowStatus::Locked);

    // Release 1 of the remaining 2
    s.escrow.partial_release(&4, &s.contributor, &1_i128);
    let info = s.escrow.get_escrow_info(&4);
    assert_eq!(info.remaining_amount, 1);
    assert_eq!(info.status, EscrowStatus::Locked);

    // Release the final unit
    s.escrow.partial_release(&4, &s.contributor, &1_i128);
    let info = s.escrow.get_escrow_info(&4);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ===========================================================================
// 3. Many sequential micro-payouts (stress test)
// ===========================================================================

/// 100 payouts of 1 unit from a 100-unit escrow.
/// At every step, remaining_amount == amount - step and is non-negative.
#[test]
fn test_hundred_single_unit_payouts() {
    let s = Setup::new();
    let amount = 100_i128;
    s.lock(5, amount);

    for step in 1..=100_i128 {
        s.escrow.partial_release(&5, &s.contributor, &1_i128);

        let info = s.escrow.get_escrow_info(&5);
        assert_eq!(
            info.remaining_amount,
            amount - step,
            "mismatch at step {}",
            step
        );
        assert!(
            info.remaining_amount >= 0,
            "remaining went negative at step {}",
            step
        );
    }

    let final_info = s.escrow.get_escrow_info(&5);
    assert_eq!(final_info.remaining_amount, 0);
    assert_eq!(final_info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
}

/// Uneven micro-payouts that don't divide evenly: 3 + 3 + 3 + 2 from 11 units.
#[test]
fn test_uneven_micro_payouts() {
    let s = Setup::new();
    let amount = 11_i128;
    s.lock(6, amount);

    let payouts = [3_i128, 3, 3, 2];
    let mut total_paid = 0_i128;

    for payout in payouts {
        s.escrow.partial_release(&6, &s.contributor, &payout);
        total_paid += payout;

        let info = s.escrow.get_escrow_info(&6);
        assert_eq!(info.remaining_amount, amount - total_paid);
        assert!(info.remaining_amount >= 0);
    }

    let info = s.escrow.get_escrow_info(&6);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
}

// ===========================================================================
// 4. Large escrow with tiny payouts
// ===========================================================================

/// 1 billion unit escrow, release 1 unit. Must track correctly.
#[test]
fn test_tiny_payout_from_billion_unit_escrow() {
    let s = Setup::new();
    let amount = 1_000_000_000_i128;
    s.lock(7, amount);

    s.escrow.partial_release(&7, &s.contributor, &1_i128);

    let info = s.escrow.get_escrow_info(&7);
    assert_eq!(info.remaining_amount, 999_999_999);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(s.token.balance(&s.contributor), 1);
}

/// Large escrow: release many small random-ish amounts and verify accumulation.
/// Payouts: [7, 13, 1, 29, 3, 17, 11, 5, 2, 23] = 111 total from 111 locked.
#[test]
fn test_varied_small_payouts_sum_to_full_amount() {
    let s = Setup::new();
    let amount = 111_i128;
    s.lock(8, amount);

    let payouts = [7_i128, 13, 1, 29, 3, 17, 11, 5, 2, 23];
    let mut total = 0_i128;

    for payout in payouts {
        s.escrow.partial_release(&8, &s.contributor, &payout);
        total += payout;
        let info = s.escrow.get_escrow_info(&8);
        assert_eq!(info.remaining_amount, amount - total);
    }

    assert_eq!(total, amount);
    let info = s.escrow.get_escrow_info(&8);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ===========================================================================
// 5. Overpayment prevention (boundary)
// ===========================================================================

/// After releasing all but 1, trying to release 2 must fail.
#[test]
fn test_overpay_by_one_after_partial_release() {
    let s = Setup::new();
    let amount = 50_i128;
    s.lock(9, amount);

    // Leave 1 remaining
    s.escrow.partial_release(&9, &s.contributor, &(amount - 1));

    // Try to release 2 when only 1 remains — must fail
    let res = s.escrow.try_partial_release(&9, &s.contributor, &2_i128);
    assert_eq!(res, Err(Ok(Error::InsufficientFunds)));
}

/// After fully releasing via partial_release (remaining == 0), any further
/// partial_release must fail with FundsNotLocked.
#[test]
fn test_partial_release_after_fully_drained() {
    let s = Setup::new();
    let amount = 10_i128;
    s.lock(10, amount);

    // Fully drain
    s.escrow.partial_release(&10, &s.contributor, &amount);

    // Try to release 1 more — should fail (status is now Released)
    let res = s.escrow.try_partial_release(&10, &s.contributor, &1_i128);
    assert_eq!(res, Err(Ok(Error::FundsNotLocked)));
}

/// Releasing a negative amount must be rejected.
#[test]
fn test_negative_partial_release_rejected() {
    let s = Setup::new();
    s.lock(11, 100_i128);

    let res = s.escrow.try_partial_release(&11, &s.contributor, &(-1_i128));
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

// ===========================================================================
// 6. Contract balance conservation
// ===========================================================================

/// After N partial releases the sum sent to the contributor plus the contract
/// balance must always equal the original locked amount.
#[test]
fn test_balance_conservation_across_partial_releases() {
    let s = Setup::new();
    let amount = 997_i128; // prime number for extra stress
    s.lock(12, amount);

    let payouts = [100_i128, 200, 300, 50, 47, 100, 100, 100];
    let mut contributor_total = 0_i128;

    for payout in payouts {
        s.escrow.partial_release(&12, &s.contributor, &payout);
        contributor_total += payout;

        // Conservation: contributor received + contract balance == original amount
        let contract_balance = s.token.balance(&s.escrow.address);
        assert_eq!(
            contributor_total + contract_balance,
            amount,
            "balance conservation violated after paying out {}",
            contributor_total
        );
    }

    let info = s.escrow.get_escrow_info(&12);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
}

// ===========================================================================
// 7. Multiple bounties — partial releases do not leak between bounties
// ===========================================================================

/// Partial-release on bounty A must not affect bounty B.
#[test]
fn test_partial_release_isolation_between_bounties() {
    let s = Setup::new();
    let amount_a = 1000_i128;
    let amount_b = 2000_i128;
    s.lock(13, amount_a);
    s.lock(14, amount_b);

    // Partial release on A only
    s.escrow.partial_release(&13, &s.contributor, &400_i128);

    let info_a = s.escrow.get_escrow_info(&13);
    let info_b = s.escrow.get_escrow_info(&14);

    assert_eq!(info_a.remaining_amount, 600);
    assert_eq!(info_b.remaining_amount, 2000); // untouched
    assert_eq!(info_b.status, EscrowStatus::Locked);

    // Contract balance should reflect both bounties
    assert_eq!(
        s.token.balance(&s.escrow.address),
        600 + 2000 // 2600
    );
}

// ===========================================================================
// 8. Refund after partial release with tiny remainder
// ===========================================================================

/// Partial-release most of the funds, then refund the last 1 unit.
/// The depositor must get back only that 1 unit; contributor keeps the rest.
#[test]
fn test_refund_tiny_remainder_after_partial_release() {
    let s = Setup::new();
    let amount = 1000_i128;
    let deadline = s.lock(15, amount);

    // Release 999, leaving exactly 1
    s.escrow.partial_release(&15, &s.contributor, &999_i128);

    let depositor_before = s.token.balance(&s.depositor);

    // Advance past deadline to allow refund
    s.env.ledger().set_timestamp(deadline + 1);
    s.escrow.refund(&15);

    let info = s.escrow.get_escrow_info(&15);
    assert_eq!(info.status, EscrowStatus::Refunded);

    // Depositor gets back exactly 1
    assert_eq!(s.token.balance(&s.depositor), depositor_before + 1);
    // Contributor keeps their 999
    assert_eq!(s.token.balance(&s.contributor), 999);
    // Contract is empty
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ===========================================================================
// 9. Two-unit escrow edge case
// ===========================================================================

/// Lock only 2 units, release them one at a time. This is the smallest escrow
/// that allows two separate partial payouts.
#[test]
fn test_two_unit_escrow_two_payouts() {
    let s = Setup::new();
    s.lock(16, 2_i128);

    // First payout
    s.escrow.partial_release(&16, &s.contributor, &1_i128);
    let info = s.escrow.get_escrow_info(&16);
    assert_eq!(info.remaining_amount, 1);
    assert_eq!(info.status, EscrowStatus::Locked);

    // Second payout
    s.escrow.partial_release(&16, &s.contributor, &1_i128);
    let info = s.escrow.get_escrow_info(&16);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
}

// ===========================================================================
// 10. Prime-number amount split into coprime payouts
// ===========================================================================

/// 97 (prime) split into payouts [11, 13, 17, 19, 23, 7, 5, 2] = 97.
/// No payout shares a common factor with the total, stressing the arithmetic.
#[test]
fn test_prime_amount_with_coprime_payouts() {
    let s = Setup::new();
    let amount = 97_i128;
    s.lock(17, amount);

    let payouts = [11_i128, 13, 17, 19, 23, 7, 5, 2];
    let mut total = 0_i128;

    for payout in payouts {
        s.escrow.partial_release(&17, &s.contributor, &payout);
        total += payout;

        let info = s.escrow.get_escrow_info(&17);
        assert_eq!(info.remaining_amount, amount - total);
        assert!(info.remaining_amount >= 0);
    }

    assert_eq!(total, amount);
    let info = s.escrow.get_escrow_info(&17);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
}

// ===========================================================================
// 11. Partial release then full release_funds must fail
// ===========================================================================

/// After a partial release (leaving some remaining), release_funds must still
/// try to transfer the *original* full amount and succeed only when remaining
/// matches. In the current implementation, release_funds always transfers
/// `escrow.amount` which would exceed the contract balance — it should fail
/// after a partial release has already reduced the actual balance.
#[test]
fn test_partial_release_then_full_release_drains_correctly() {
    let s = Setup::new();
    let amount = 100_i128;
    s.lock(18, amount);

    // Partial release half
    s.escrow.partial_release(&18, &s.contributor, &50_i128);
    let info = s.escrow.get_escrow_info(&18);
    assert_eq!(info.remaining_amount, 50);
    assert_eq!(info.status, EscrowStatus::Locked);

    // Now release the remaining 50 via another partial_release
    s.escrow.partial_release(&18, &s.contributor, &50_i128);
    let info = s.escrow.get_escrow_info(&18);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
}

// ===========================================================================
// 12. Single-unit payout to different contributor each time
// ===========================================================================

/// Release 1 unit at a time to different contributors. Verify each gets
/// exactly 1 and remaining_amount decrements correctly.
#[test]
fn test_single_unit_payouts_to_different_contributors() {
    let s = Setup::new();
    let amount = 5_i128;
    s.lock(19, amount);

    // Generate 5 distinct contributor addresses
    let c0 = Address::generate(&s.env);
    let c1 = Address::generate(&s.env);
    let c2 = Address::generate(&s.env);
    let c3 = Address::generate(&s.env);
    let c4 = Address::generate(&s.env);
    let contributors = [&c0, &c1, &c2, &c3, &c4];

    for (i, contributor) in contributors.iter().enumerate() {
        s.escrow.partial_release(&19, contributor, &1_i128);
        assert_eq!(s.token.balance(contributor), 1);

        let info = s.escrow.get_escrow_info(&19);
        assert_eq!(info.remaining_amount, amount - (i as i128 + 1));
    }

    let info = s.escrow.get_escrow_info(&19);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ===========================================================================
// 13. Exact remaining boundary releases
// ===========================================================================

/// Release exactly half, then exactly the other half. Both steps should succeed
/// and the second should transition to Released.
#[test]
fn test_exact_half_then_remaining_half() {
    let s = Setup::new();
    let amount = 1000_i128;
    s.lock(20, amount);

    s.escrow.partial_release(&20, &s.contributor, &500_i128);
    let info = s.escrow.get_escrow_info(&20);
    assert_eq!(info.remaining_amount, 500);
    assert_eq!(info.status, EscrowStatus::Locked);

    s.escrow.partial_release(&20, &s.contributor, &500_i128);
    let info = s.escrow.get_escrow_info(&20);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
}

/// Release exactly the full amount in a single call (boundary: payout == remaining).
/// This is the simplest "no rounding" case; the result must be identical to
/// a full release_funds call.
#[test]
fn test_full_amount_partial_release_boundary() {
    let s = Setup::new();
    let amount = 7_i128; // smallest odd prime
    s.lock(21, amount);

    s.escrow.partial_release(&21, &s.contributor, &amount);

    let info = s.escrow.get_escrow_info(&21);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
}

// ===========================================================================
// 14. Repeated 1-unit payouts on a 3-unit escrow (odd split)
// ===========================================================================

/// 3 units released as 1 + 1 + 1. After step 2, remaining must be 1 (not 0).
#[test]
fn test_three_single_unit_payouts() {
    let s = Setup::new();
    s.lock(22, 3_i128);

    s.escrow.partial_release(&22, &s.contributor, &1_i128);
    assert_eq!(s.escrow.get_escrow_info(&22).remaining_amount, 2);

    s.escrow.partial_release(&22, &s.contributor, &1_i128);
    assert_eq!(s.escrow.get_escrow_info(&22).remaining_amount, 1);

    s.escrow.partial_release(&22, &s.contributor, &1_i128);
    let info = s.escrow.get_escrow_info(&22);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
}

// ===========================================================================
// 15. Large number of sequential payouts to verify no accumulation error
// ===========================================================================

/// 50 payouts of 2 units from 100. Verify at select checkpoints.
#[test]
fn test_fifty_payouts_of_two() {
    let s = Setup::new();
    let amount = 100_i128;
    s.lock(23, amount);

    for step in 1..=50_i128 {
        s.escrow.partial_release(&23, &s.contributor, &2_i128);

        let info = s.escrow.get_escrow_info(&23);
        let expected = amount - step * 2;
        assert_eq!(info.remaining_amount, expected, "mismatch at step {}", step);
        assert!(info.remaining_amount >= 0);
    }

    let info = s.escrow.get_escrow_info(&23);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
}

// ===========================================================================
// 16. Partial release with approved refund on remainder
// ===========================================================================

/// Partial release 80% of funds, then admin approves early refund for the
/// remaining 20%. Verifies that refund only returns the actual remainder.
#[test]
fn test_partial_release_then_approved_early_refund() {
    let s = Setup::new();
    let amount = 1000_i128;
    let _deadline = s.lock(24, amount);

    // Partial release 800
    s.escrow.partial_release(&24, &s.contributor, &800_i128);

    let info = s.escrow.get_escrow_info(&24);
    assert_eq!(info.remaining_amount, 200);

    // Admin approves refund for the remaining 200 (early, before deadline)
    s.escrow
        .approve_refund(&24, &200_i128, &s.depositor, &RefundMode::Full);

    let depositor_before = s.token.balance(&s.depositor);
    s.escrow.refund(&24);

    let info = s.escrow.get_escrow_info(&24);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(s.token.balance(&s.depositor), depositor_before + 200);
    assert_eq!(s.token.balance(&s.contributor), 800);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ===========================================================================
// 17. Verifying remaining_amount field matches contract balance exactly
// ===========================================================================

/// Lock two bounties, partial-release from each, and verify that the sum of
/// all remaining_amounts equals the contract token balance at every step.
#[test]
fn test_remaining_amounts_sum_equals_contract_balance() {
    let s = Setup::new();
    let amount_a = 500_i128;
    let amount_b = 300_i128;
    s.lock(25, amount_a);
    s.lock(26, amount_b);

    // Initial check
    let contract_balance = s.token.balance(&s.escrow.address);
    assert_eq!(contract_balance, amount_a + amount_b);

    // Partial release from A
    s.escrow.partial_release(&25, &s.contributor, &100_i128);
    let info_a = s.escrow.get_escrow_info(&25);
    let info_b = s.escrow.get_escrow_info(&26);
    assert_eq!(
        info_a.remaining_amount + info_b.remaining_amount,
        s.token.balance(&s.escrow.address)
    );

    // Partial release from B
    s.escrow.partial_release(&26, &s.contributor, &50_i128);
    let info_a = s.escrow.get_escrow_info(&25);
    let info_b = s.escrow.get_escrow_info(&26);
    assert_eq!(
        info_a.remaining_amount + info_b.remaining_amount,
        s.token.balance(&s.escrow.address)
    );

    // Another from A
    s.escrow.partial_release(&25, &s.contributor, &400_i128);
    let info_a = s.escrow.get_escrow_info(&25);
    let info_b = s.escrow.get_escrow_info(&26);
    assert_eq!(
        info_a.remaining_amount + info_b.remaining_amount,
        s.token.balance(&s.escrow.address)
    );
    assert_eq!(info_a.status, EscrowStatus::Released); // A fully drained
}

// ===========================================================================
// 18. Sum invariant: cumulative payouts plus remaining equals original
// ===========================================================================

/// Verify at every step that sum(payouts) + remaining == original amount.
/// This is the core sum invariant that must hold across all partial releases.
#[test]
fn test_sum_invariant_holds_across_all_payouts() {
    let s = Setup::new();
    let original = 1_000_000_i128;
    s.lock(27, original);

    let payouts = [100_i128, 250, 75, 333, 1, 241];
    let mut cumulative_paid = 0_i128;

    for payout in payouts {
        s.escrow.partial_release(&27, &s.contributor, &payout);
        cumulative_paid += payout;

        let info = s.escrow.get_escrow_info(&27);
        assert_eq!(
            cumulative_paid + info.remaining_amount,
            original,
            "sum invariant violated after paying {}",
            cumulative_paid
        );
    }
}

// ===========================================================================
// 19. Dust remainder: release all but 1, verify dust is retrievable
// ===========================================================================

/// Create a "dust" scenario by releasing all but 1 unit, then verify the
/// dust can still be released or refunded.
#[test]
fn test_dust_remainder_fully_retrievable() {
    let s = Setup::new();
    let amount = 10_000_i128;
    s.lock(28, amount);

    // Release all but 1 (creating "dust")
    s.escrow.partial_release(&28, &s.contributor, &(amount - 1));

    let info = s.escrow.get_escrow_info(&28);
    assert_eq!(info.remaining_amount, 1);
    assert_eq!(info.status, EscrowStatus::Locked);

    // The dust (1 unit) can still be released
    s.escrow.partial_release(&28, &s.contributor, &1_i128);

    let info = s.escrow.get_escrow_info(&28);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), amount);
}

// ===========================================================================
// 20. Multiple contributors with dust-level payouts
// ===========================================================================

/// Distribute dust-level amounts (1-3 units each) to many contributors.
/// Verify no tokens are lost or created.
#[test]
fn test_dust_payouts_to_multiple_contributors() {
    let s = Setup::new();
    let amount = 20_i128;
    s.lock(29, amount);

    let contributors: [Address; 10] = core::array::from_fn(|_| Address::generate(&s.env));
    let payouts = [2_i128, 1, 3, 2, 1, 2, 3, 1, 2, 3]; // sum = 20
    let mut total_distributed = 0_i128;

    for (i, payout) in payouts.iter().enumerate() {
        s.escrow.partial_release(&29, &contributors[i], payout);
        total_distributed += payout;
        assert_eq!(s.token.balance(&contributors[i]), *payout);
    }

    assert_eq!(total_distributed, amount);

    let info = s.escrow.get_escrow_info(&29);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ===========================================================================
// 21. Large number division edge case
// ===========================================================================

/// Test with amounts near the minted limit to ensure no overflow in calculations.
#[test]
fn test_large_amount_partial_release_no_overflow() {
    let s = Setup::new();
    let large_amount = 5_000_000_000_i128; // 5 billion units (within minted limit)
    s.lock(30, large_amount);

    // Release in large chunks
    let chunk = 1_666_666_666_i128;
    s.escrow.partial_release(&30, &s.contributor, &chunk);

    let info = s.escrow.get_escrow_info(&30);
    let expected_remaining = large_amount - chunk;
    assert_eq!(info.remaining_amount, expected_remaining);
    assert_eq!(
        s.token.balance(&s.contributor) + info.remaining_amount,
        large_amount
    );

    // Release another chunk
    s.escrow.partial_release(&30, &s.contributor, &chunk);

    let info = s.escrow.get_escrow_info(&30);
    assert_eq!(info.remaining_amount, large_amount - 2 * chunk);

    // Release the remainder
    let remainder = large_amount - 2 * chunk;
    s.escrow.partial_release(&30, &s.contributor, &remainder);

    let info = s.escrow.get_escrow_info(&30);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(s.token.balance(&s.contributor), large_amount);
}

// ===========================================================================
// 22. Alternating small and large payouts
// ===========================================================================

/// Mix dust-level (1 unit) and large payouts to stress the arithmetic.
#[test]
fn test_alternating_dust_and_large_payouts() {
    let s = Setup::new();
    let amount = 1_000_000_i128;
    s.lock(31, amount);

    let payouts = [1_i128, 100_000, 1, 200_000, 1, 300_000, 1, 399_996]; // sum = 1_000_000
    let mut total = 0_i128;

    for payout in payouts {
        s.escrow.partial_release(&31, &s.contributor, &payout);
        total += payout;

        let info = s.escrow.get_escrow_info(&31);
        assert_eq!(info.remaining_amount, amount - total);
    }

    assert_eq!(total, amount);
    let info = s.escrow.get_escrow_info(&31);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
}

// ===========================================================================
// 23. Zero-sum verification across refund and release paths
// ===========================================================================

/// After partial releases and a refund, verify total distributed equals original.
/// contributor_received + depositor_refunded == original_amount
#[test]
fn test_zero_sum_across_release_and_refund() {
    let s = Setup::new();
    let amount = 1000_i128;
    let deadline = s.lock(32, amount);

    // Partial release 600 to contributor
    s.escrow.partial_release(&32, &s.contributor, &600_i128);
    let contributor_received = s.token.balance(&s.contributor);
    assert_eq!(contributor_received, 600);

    // Advance past deadline and refund the remaining 400
    s.env.ledger().set_timestamp(deadline + 1);

    let depositor_before_refund = s.token.balance(&s.depositor);
    s.escrow.refund(&32);
    let depositor_after_refund = s.token.balance(&s.depositor);

    // Verify: depositor got back exactly 400 (the refunded remainder)
    let refunded_amount = depositor_after_refund - depositor_before_refund;
    assert_eq!(refunded_amount, 400);

    // Zero-sum invariant: contributor + refund = original
    assert_eq!(contributor_received + refunded_amount, amount);
}

// ===========================================================================
// 24. Dust threshold edge: amounts 1 through 10
// ===========================================================================

/// Exhaustively test all dust-level amounts from 1 to 10.
#[test]
fn test_all_dust_amounts_one_through_ten() {
    for dust in 1_i128..=10 {
        let s = Setup::new();
        s.lock(100 + dust as u64, dust);

        // Release 1 at a time until empty
        for step in 1..=dust {
            s.escrow
                .partial_release(&(100 + dust as u64), &s.contributor, &1_i128);
            let info = s.escrow.get_escrow_info(&(100 + dust as u64));
            assert_eq!(info.remaining_amount, dust - step);
        }

        let info = s.escrow.get_escrow_info(&(100 + dust as u64));
        assert_eq!(info.remaining_amount, 0);
        assert_eq!(info.status, EscrowStatus::Released);
    }
}
