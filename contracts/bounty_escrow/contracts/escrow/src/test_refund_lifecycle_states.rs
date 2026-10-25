//! Lifecycle-state coverage for `refund` (src/lib.rs `refund` / src/refund.rs).
//!
//! Drives `refund` from every `EscrowStatus` variant:
//! - `Draft`             -> refused with `Error::FundsNotLocked`
//! - `Locked`            -> succeeds (deadline passed) / `Error::DeadlineNotPassed` (before)
//! - `PartiallyRefunded` -> succeeds (deadline passed) / `Error::DeadlineNotPassed` (before)
//! - `Released`          (terminal) -> refused with `Error::FundsNotLocked`, no mutation
//! - `Refunded`          (terminal) -> refused with `Error::FundsNotLocked`, no mutation
//!
//! Plus the absent-record case (`Error::BountyNotFound`).
//! Every outcome is asserted via `try_refund` as a value or typed error.

#![cfg(test)]
extern crate std;

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, DataKey, Error, Escrow, EscrowStatus,
    RefundMode,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

const BOUNTY_ID: u64 = 1;
const AMOUNT: i128 = 1_000;
const START: u64 = 1_000;
const DEADLINE: u64 = START + 500;

struct Setup<'a> {
    env: Env,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(START);

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = token::Client::new(&env, &sac.address());
        token::StellarAssetClient::new(&env, &sac.address()).mint(&depositor, &1_000_000);

        let escrow = BountyEscrowContractClient::new(
            &env,
            &env.register_contract(None, BountyEscrowContract),
        );
        escrow.init(&admin, &token.address);

        Self {
            env,
            depositor,
            contributor,
            token,
            escrow,
        }
    }

    fn lock(&self) {
        self.escrow
            .lock_funds(&self.depositor, &BOUNTY_ID, &AMOUNT, &DEADLINE);
        assert_eq!(self.status(), EscrowStatus::Locked);
    }

    fn status(&self) -> EscrowStatus {
        self.escrow.get_escrow_info(&BOUNTY_ID).status
    }

    fn pass_deadline(&self) {
        self.env.ledger().set_timestamp(DEADLINE + 1);
    }

    /// Forces the stored record into `Draft`. No public entry point lets a
    /// plain `lock_funds` record return to `Draft`, so it is set directly.
    fn force_draft(&self) {
        self.env.as_contract(&self.escrow.address, || {
            let key = DataKey::Escrow(BOUNTY_ID);
            let mut e: Escrow = self.env.storage().persistent().get(&key).unwrap();
            e.status = EscrowStatus::Draft;
            self.env.storage().persistent().set(&key, &e);
        });
        assert_eq!(self.status(), EscrowStatus::Draft);
    }

    fn to_partially_refunded(&self) {
        self.lock();
        self.escrow
            .approve_refund(&BOUNTY_ID, &300, &self.depositor, &RefundMode::Partial);
        assert_eq!(self.escrow.try_refund(&BOUNTY_ID), Ok(Ok(())));
        assert_eq!(self.status(), EscrowStatus::PartiallyRefunded);
        assert_eq!(
            self.escrow.get_escrow_info(&BOUNTY_ID).remaining_amount,
            700
        );
    }

    /// Asserts a refused call returns `expected` and leaves the escrow record
    /// and both token balances untouched.
    fn assert_refused_without_mutation(&self, expected: Error) {
        let before = self.escrow.get_escrow_info(&BOUNTY_ID);
        let contract_bal = self.token.balance(&self.escrow.address);
        let depositor_bal = self.token.balance(&self.depositor);

        assert_eq!(self.escrow.try_refund(&BOUNTY_ID), Err(Ok(expected)));

        assert_eq!(self.escrow.get_escrow_info(&BOUNTY_ID), before);
        assert_eq!(self.token.balance(&self.escrow.address), contract_bal);
        assert_eq!(self.token.balance(&self.depositor), depositor_bal);
    }
}

// ---------------------------------------------------------------------------
// No record
// ---------------------------------------------------------------------------

#[test]
fn refund_from_no_record_returns_bounty_not_found() {
    let s = Setup::new();
    assert_eq!(
        s.escrow.try_refund(&BOUNTY_ID),
        Err(Ok(Error::BountyNotFound))
    );
}

// ---------------------------------------------------------------------------
// Draft
// ---------------------------------------------------------------------------

#[test]
fn refund_from_draft_is_refused_with_funds_not_locked() {
    let s = Setup::new();
    s.lock();
    s.force_draft();
    s.pass_deadline();

    s.assert_refused_without_mutation(Error::FundsNotLocked);
    assert_eq!(s.status(), EscrowStatus::Draft);
}

// ---------------------------------------------------------------------------
// Locked
// ---------------------------------------------------------------------------

#[test]
fn refund_from_locked_before_deadline_returns_deadline_not_passed() {
    let s = Setup::new();
    s.lock();

    s.assert_refused_without_mutation(Error::DeadlineNotPassed);
    assert_eq!(s.status(), EscrowStatus::Locked);
}

#[test]
fn refund_from_locked_after_deadline_transitions_to_refunded() {
    let s = Setup::new();
    s.lock();
    s.pass_deadline();
    let depositor_before = s.token.balance(&s.depositor);

    assert_eq!(s.escrow.try_refund(&BOUNTY_ID), Ok(Ok(())));

    let info = s.escrow.get_escrow_info(&BOUNTY_ID);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.refund_history.len(), 1);
    assert_eq!(s.token.balance(&s.depositor), depositor_before + AMOUNT);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ---------------------------------------------------------------------------
// PartiallyRefunded
// ---------------------------------------------------------------------------

#[test]
fn refund_from_partially_refunded_before_deadline_returns_deadline_not_passed() {
    let s = Setup::new();
    s.to_partially_refunded();

    s.assert_refused_without_mutation(Error::DeadlineNotPassed);
    assert_eq!(s.status(), EscrowStatus::PartiallyRefunded);
}

#[test]
fn refund_from_partially_refunded_after_deadline_transitions_to_refunded() {
    let s = Setup::new();
    s.to_partially_refunded();
    s.pass_deadline();
    let depositor_before = s.token.balance(&s.depositor);

    assert_eq!(s.escrow.try_refund(&BOUNTY_ID), Ok(Ok(())));

    let info = s.escrow.get_escrow_info(&BOUNTY_ID);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.refund_history.len(), 2);
    assert_eq!(s.token.balance(&s.depositor), depositor_before + 700);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
}

// ---------------------------------------------------------------------------
// Released (terminal)
// ---------------------------------------------------------------------------

#[test]
fn refund_from_released_terminal_state_is_refused_without_mutation() {
    let s = Setup::new();
    s.lock();
    s.escrow.release_funds(&BOUNTY_ID, &s.contributor);
    assert_eq!(s.status(), EscrowStatus::Released);
    s.pass_deadline();

    s.assert_refused_without_mutation(Error::FundsNotLocked);
    assert_eq!(s.status(), EscrowStatus::Released);
}

// ---------------------------------------------------------------------------
// Refunded (terminal)
// ---------------------------------------------------------------------------

#[test]
fn refund_from_refunded_terminal_state_is_refused_without_mutation() {
    let s = Setup::new();
    s.lock();
    s.pass_deadline();
    assert_eq!(s.escrow.try_refund(&BOUNTY_ID), Ok(Ok(())));
    assert_eq!(s.status(), EscrowStatus::Refunded);

    s.assert_refused_without_mutation(Error::FundsNotLocked);
    assert_eq!(s.status(), EscrowStatus::Refunded);
}
