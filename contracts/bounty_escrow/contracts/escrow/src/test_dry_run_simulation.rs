//! Tests for dry-run simulation entrypoints (Issue #567).
//!
//! Each test verifies that the simulation function returns the same
//! verdict (success / error code) as the corresponding real operation,
//! and that no state is mutated by the simulation.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_token<'a>(
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

fn create_escrow<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &id)
}

struct SimSetup<'a> {
    env: Env,
    _admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> SimSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token(&env, &admin);
        let escrow = create_escrow(&env);
        escrow.init(&admin, &token.address);

        token_admin.mint(&depositor, &10_000_000);

        Self {
            env,
            _admin: admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

// ===========================================================================
// simulate_lock
// ===========================================================================

#[test]
fn test_simulate_lock_success() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    let result = s
        .escrow
        .simulate_lock(&s.depositor, &1_u64, &1_000, &deadline);

    assert!(result.success);
    assert_eq!(result.error_code, 0);
    assert_eq!(result.amount, 1_000);
    assert_eq!(result.resulting_status, EscrowStatus::Locked);
    assert_eq!(result.remaining_amount, 1_000);
}

#[test]
fn test_simulate_lock_does_not_mutate_state() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;
    let balance_before = s.token.balance(&s.depositor);

    let result = s
        .escrow
        .simulate_lock(&s.depositor, &1_u64, &1_000, &deadline);
    assert!(result.success);

    // No funds moved, no escrow created
    assert_eq!(s.token.balance(&s.depositor), balance_before);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
    // Real lock still works afterward
    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    assert_eq!(s.token.balance(&s.escrow.address), 1_000);
}

#[test]
fn test_simulate_lock_matches_real_execution_success() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    let sim = s
        .escrow
        .simulate_lock(&s.depositor, &1_u64, &1_000, &deadline);
    assert!(sim.success);

    // Real execution should also succeed
    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    let info = s.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, sim.resulting_status);
    assert_eq!(info.remaining_amount, sim.remaining_amount);
    assert_eq!(info.amount, sim.amount);
}

#[test]
fn test_simulate_lock_duplicate_bounty() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    let result = s
        .escrow
        .simulate_lock(&s.depositor, &1_u64, &500, &deadline);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::BountyExists as u32);
}

#[test]
fn test_simulate_lock_insufficient_balance() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;
    let huge_amount = 999_999_999_i128;

    let result = s
        .escrow
        .simulate_lock(&s.depositor, &1_u64, &huge_amount, &deadline);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::InsufficientFunds as u32);
}

#[test]
fn test_simulate_lock_invalid_amount() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    let result = s.escrow.simulate_lock(&s.depositor, &1_u64, &0, &deadline);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::InvalidAmount as u32);
}

#[test]
fn test_simulate_lock_paused() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.set_paused(
        &Some(true),
        &None::<bool>,
        &None::<bool>,
        &Some(soroban_sdk::String::from_str(&s.env, "test")),
    );

    let result = s
        .escrow
        .simulate_lock(&s.depositor, &1_u64, &1_000, &deadline);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::FundsPaused as u32);
}

// ===========================================================================
// simulate_release
// ===========================================================================

#[test]
fn test_simulate_release_success() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    let result = s.escrow.simulate_release(&1_u64, &s.contributor);

    assert!(result.success);
    assert_eq!(result.error_code, 0);
    assert_eq!(result.amount, 1_000);
    assert_eq!(result.resulting_status, EscrowStatus::Released);
    assert_eq!(result.remaining_amount, 0);
}

#[test]
fn test_simulate_release_does_not_mutate_state() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    let result = s.escrow.simulate_release(&1_u64, &s.contributor);
    assert!(result.success);

    // Escrow should still be Locked
    let info = s.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(info.remaining_amount, 1_000);
    // No tokens moved
    assert_eq!(s.token.balance(&s.contributor), 0);
}

#[test]
fn test_simulate_release_matches_real_execution() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    let sim = s.escrow.simulate_release(&1_u64, &s.contributor);
    assert!(sim.success);

    s.escrow.release_funds(&1_u64, &s.contributor);
    let info = s.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, sim.resulting_status);
    assert_eq!(info.remaining_amount, sim.remaining_amount);
}

#[test]
fn test_simulate_release_not_found() {
    let s = SimSetup::new();

    let result = s.escrow.simulate_release(&999_u64, &s.contributor);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::BountyNotFound as u32);
}

#[test]
fn test_simulate_release_already_released() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.release_funds(&1_u64, &s.contributor);

    let result = s.escrow.simulate_release(&1_u64, &s.contributor);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::FundsNotLocked as u32);
}

// ===========================================================================
// simulate_refund
// ===========================================================================

#[test]
fn test_simulate_refund_success_after_deadline() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    s.env.ledger().set_timestamp(deadline + 1);

    let result = s.escrow.simulate_refund(&1_u64);

    assert!(result.success);
    assert_eq!(result.error_code, 0);
    assert_eq!(result.amount, 1_000);
    assert_eq!(result.resulting_status, EscrowStatus::Refunded);
    assert_eq!(result.remaining_amount, 0);
}

#[test]
fn test_simulate_refund_does_not_mutate_state() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.env.ledger().set_timestamp(deadline + 1);

    let depositor_before = s.token.balance(&s.depositor);

    let result = s.escrow.simulate_refund(&1_u64);
    assert!(result.success);

    // State unchanged
    let info = s.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(s.token.balance(&s.depositor), depositor_before);
}

#[test]
fn test_simulate_refund_matches_real_execution() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.env.ledger().set_timestamp(deadline + 1);

    let sim = s.escrow.simulate_refund(&1_u64);
    assert!(sim.success);

    s.escrow.refund(&1_u64);
    let info = s.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, sim.resulting_status);
    assert_eq!(info.remaining_amount, sim.remaining_amount);
}

#[test]
fn test_simulate_refund_before_deadline_fails() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 10_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    let result = s.escrow.simulate_refund(&1_u64);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::DeadlineNotPassed as u32);
}

#[test]
fn test_simulate_refund_not_found() {
    let s = SimSetup::new();

    let result = s.escrow.simulate_refund(&999_u64);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::BountyNotFound as u32);
}

#[test]
fn test_simulate_refund_already_released() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.release_funds(&1_u64, &s.contributor);

    let result = s.escrow.simulate_refund(&1_u64);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::FundsNotLocked as u32);
}

#[test]
fn test_simulate_refund_with_pending_claim_fails() {
    let s = SimSetup::new();
    let deadline = s.env.ledger().timestamp() + 10_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.set_claim_window(&500_u64);
    s.escrow
        .authorize_claim(&1_u64, &s.contributor, &DisputeReason::Other);

    s.env.ledger().set_timestamp(deadline + 1);

    let result = s.escrow.simulate_refund(&1_u64);

    assert!(!result.success);
    assert_eq!(result.error_code, Error::ClaimPending as u32);
}
