#![cfg(test)]
//! # `refund` boundary tests (issue #2000)
//!
//! Pins the extremes of every numeric or time-valued input `refund` reads:
//! the bounty id (`0`, `1`, `u64::MAX`), the ledger time against the deadline
//! (one before, equal, one after, `u64::MAX`), and the refund amount
//! (`0`, `1`, the remaining balance, and one past it).
//!
//! Every refusal is asserted through the `try_*` client so a panic would fail
//! the test: a refusal must surface as a typed `Error`.

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, DataKey, Error, EscrowStatus, RefundApproval,
    RefundMode,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

struct Setup<'a> {
    env: Env,
    depositor: Address,
    token: token::Client<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = token::Client::new(&env, &sac.address());
        let token_admin = token::StellarAssetClient::new(&env, &sac.address());
        let escrow = BountyEscrowContractClient::new(
            &env,
            &env.register_contract(None, BountyEscrowContract),
        );

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            depositor,
            token,
            escrow,
        }
    }

    fn set_time(&self, ts: u64) {
        self.env.ledger().set_timestamp(ts);
    }

    /// Store an approval directly, bypassing `approve_refund`'s own validation,
    /// so `refund`'s independent amount guard can be exercised.
    fn plant_approval(&self, bounty_id: u64, amount: i128) {
        let approval = RefundApproval {
            bounty_id,
            amount,
            recipient: self.depositor.clone(),
            mode: RefundMode::Partial,
            approved_by: self.depositor.clone(),
            approved_at: 0,
        };
        self.env.as_contract(&self.escrow.address, || {
            self.env
                .storage()
                .persistent()
                .set(&DataKey::RefundApproval(bounty_id), &approval);
        });
    }
}

// ── bounty id: 0, 1, u64::MAX ───────────────────────────────────────────────

#[test]
fn refund_of_unknown_id_is_bounty_not_found_at_extremes() {
    let s = Setup::new();
    for id in [0u64, 1, u64::MAX] {
        assert_eq!(
            s.escrow.try_refund(&id),
            Err(Ok(Error::BountyNotFound)),
            "id {id}"
        );
    }
}

#[test]
fn refund_succeeds_for_ids_zero_one_and_max() {
    let s = Setup::new();
    for id in [0u64, 1, u64::MAX] {
        s.escrow.lock_funds(&s.depositor, &id, &100, &10);
    }
    s.set_time(10);
    let before = s.token.balance(&s.depositor);
    for id in [0u64, 1, u64::MAX] {
        s.escrow.refund(&id);
        assert_eq!(s.escrow.get_escrow_info(&id).status, EscrowStatus::Refunded);
    }
    assert_eq!(s.token.balance(&s.depositor), before + 300);
}

// ── time: deadline - 1, deadline, deadline + 1, u64::MAX ────────────────────

#[test]
fn refund_one_before_deadline_is_refused() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &100, &1_000);
    s.set_time(999);
    assert_eq!(s.escrow.try_refund(&1), Err(Ok(Error::DeadlineNotPassed)));
    assert_eq!(s.escrow.get_escrow_info(&1).status, EscrowStatus::Locked);
}

#[test]
fn refund_exactly_at_deadline_succeeds() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &100, &1_000);
    s.set_time(1_000);
    s.escrow.refund(&1);
    assert_eq!(s.escrow.get_escrow_info(&1).status, EscrowStatus::Refunded);
}

#[test]
fn refund_one_after_deadline_succeeds() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &100, &1_000);
    s.set_time(1_001);
    s.escrow.refund(&1);
    assert_eq!(s.escrow.get_escrow_info(&1).status, EscrowStatus::Refunded);
}

#[test]
fn refund_with_zero_deadline_succeeds_at_time_zero() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &100, &0);
    s.escrow.refund(&1);
    assert_eq!(s.escrow.get_escrow_info(&1).status, EscrowStatus::Refunded);
}

#[test]
fn refund_with_max_deadline_is_refused_until_the_last_second() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &100, &u64::MAX);

    s.set_time(u64::MAX - 1);
    assert_eq!(s.escrow.try_refund(&1), Err(Ok(Error::DeadlineNotPassed)));

    s.set_time(u64::MAX);
    s.escrow.refund(&1);
    assert_eq!(s.escrow.get_escrow_info(&1).status, EscrowStatus::Refunded);
}

// ── amount: 0, 1, remaining, remaining + 1 ──────────────────────────────────

#[test]
fn refund_of_smallest_locked_amount_returns_one_unit() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &1, &10);
    s.set_time(10);
    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&1);
    assert_eq!(s.token.balance(&s.depositor), before + 1);
    assert_eq!(s.escrow.get_escrow_info(&1).remaining_amount, 0);
}

#[test]
fn approved_partial_refund_of_one_unit_leaves_the_rest() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &1_000, &u64::MAX);
    s.escrow
        .approve_refund(&1, &1, &s.depositor, &RefundMode::Partial);

    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&1);

    let info = s.escrow.get_escrow_info(&1);
    assert_eq!(s.token.balance(&s.depositor), before + 1);
    assert_eq!(info.remaining_amount, 999);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
}

#[test]
fn approved_refund_of_the_full_remaining_amount_succeeds() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &1_000, &u64::MAX);
    s.escrow
        .approve_refund(&1, &1_000, &s.depositor, &RefundMode::Partial);

    let before = s.token.balance(&s.depositor);
    s.escrow.refund(&1);

    let info = s.escrow.get_escrow_info(&1);
    assert_eq!(s.token.balance(&s.depositor), before + 1_000);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Refunded);
}

#[test]
fn approval_of_zero_or_one_past_remaining_is_refused_at_approval() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &1_000, &u64::MAX);
    for amount in [0i128, 1_001, i128::MAX] {
        assert_eq!(
            s.escrow
                .try_approve_refund(&1, &amount, &s.depositor, &RefundMode::Partial),
            Err(Ok(Error::InvalidAmount)),
            "amount {amount}"
        );
    }
}

#[test]
fn refund_refuses_a_stored_approval_of_zero_or_one_past_remaining() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &1_000, &u64::MAX);
    let before = s.token.balance(&s.depositor);

    for amount in [0i128, -1, 1_001, i128::MAX] {
        s.plant_approval(1, amount);
        assert_eq!(
            s.escrow.try_refund(&1),
            Err(Ok(Error::InvalidAmount)),
            "amount {amount}"
        );
    }

    // Nothing moved and the escrow is untouched.
    let info = s.escrow.get_escrow_info(&1);
    assert_eq!(s.token.balance(&s.depositor), before);
    assert_eq!(info.remaining_amount, 1_000);
    assert_eq!(info.status, EscrowStatus::Locked);
}

#[test]
fn refund_after_everything_is_refunded_is_funds_not_locked() {
    let s = Setup::new();
    s.escrow.lock_funds(&s.depositor, &1, &1, &0);
    s.escrow.refund(&1);
    assert_eq!(s.escrow.try_refund(&1), Err(Ok(Error::FundsNotLocked)));
}
