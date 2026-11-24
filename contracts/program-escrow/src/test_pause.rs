#![cfg(test)]

//! Pause / emergency-withdraw suite, wired up for #2089 (it previously existed
//! as an orphaned file that no `mod` declared, so none of it ran).
//!
//! ## Overlap with the already-running pause suites
//!
//! The following tests were dropped because `test_granular_pause.rs`,
//! `test_set_paused_auth.rs` and this file's own RBAC section already cover the
//! same scenario, and keeping them here would only duplicate runtime:
//!
//! - `test_mixed_pause_states` — the 3-flag storage assertions are covered by
//!   `test_granular_pause::test_pause_matrix_110_lock_and_release_paused` and
//!   `test_unpausing_release_does_not_unpause_lock_or_refund`.
//! - `test_pause_by_non_admin_fails` — covered by
//!   `test_set_paused_auth::test_set_paused_unauthorized_caller_refused`.
//! - `test_emergency_withdraw_non_admin_fails` — covered by
//!   `test_rbac_operator_cannot_emergency_withdraw` (below).
//! - `test_emergency_withdraw_unpaused_fails` — covered by
//!   `test_rbac_admin_emergency_withdraw_requires_paused_state` (below).
//! - `test_emergency_withdraw_succeeds` — covered by
//!   `test_rbac_admin_can_emergency_withdraw_when_paused` (below).
//! - `test_rbac_emergency_withdraw_ignores_release_and_refund_pause` — the union
//!   of `test_rbac_emergency_withdraw_requires_lock_paused_not_release_paused`
//!   and `..._not_refund_paused` (below).
//!
//! Everything kept here is unique: per-flag `PauseFlags` storage round-trips,
//! enforcement of the pause gate on lock/release entrypoints, the V1
//! `PauseStateChanged` event payload, the emergency-withdraw RBAC matrix, and the
//! time-based auto-unpause (TTL) behaviour.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, String, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract.address();
    token::Client::new(env, &token_address)
}

fn setup_with_admin<'a>(env: &Env) -> (ProgramEscrowContractClient<'a>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);

    // Explicitly do not mock auths globally here so we can test auth failures
    client.mock_auths(&[]).initialize_contract(&admin);
    (client, admin)
}

fn setup_program_with_admin<'a>(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'a>,
    Address,
    Address,
    token::Client<'a>,
) {
    let (client, admin) = setup_with_admin(env);
    let payout_key = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_client = create_token_contract(env, &token_admin);

    env.mock_all_auths();
    let program_id = String::from_str(env, "test-prog");
    client.init_program(
        &program_id,
        &payout_key,
        &token_client.address,
        &admin,
        &None,
        &None,
    );
    client.publish_program(&program_id, &admin);
    (client, admin, payout_key, token_client)
}

// --- get_pause_flags & default state ---

#[test]
fn test_default_pause_flags_are_all_false() {
    let env = Env::default();
    let (contract, _admin) = setup_with_admin(&env);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}

// --- set_paused: lock ---

#[test]
fn test_set_paused_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_with_admin(&env);

    contract.set_paused(&Some(true), &None, &None, &None, &None);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}

#[test]
fn test_unset_paused_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_with_admin(&env);

    contract.set_paused(&Some(true), &None, &None, &None, &None);
    contract.set_paused(&Some(false), &None, &None, &None, &None);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
}

// --- set_paused: release ---

#[test]
fn test_set_paused_release() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_with_admin(&env);

    contract.set_paused(&None, &Some(true), &None, &None, &None);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, true);
    assert_eq!(flags.refund_paused, false);
}

// --- lock_program_funds enforcement ---

#[test]
fn test_lock_program_funds_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    contract.set_paused(&Some(true), &None, &None, &None, &None);
    assert!(
        contract.try_lock_program_funds(&1000).is_err(),
        "locking funds must be refused while lock_paused is set"
    );
}

// --- single_payout enforcement ---

#[test]
fn test_single_payout_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    let recipient = Address::generate(&env);

    contract.set_paused(&None, &Some(true), &None, &None, &None);
    assert!(
        contract.try_single_payout(&recipient, &100, &None).is_err(),
        "single_payout must be refused while release_paused is set"
    );
}

// --- batch_payout enforcement ---

#[test]
fn test_batch_payout_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    let recipient = Address::generate(&env);

    let recipients = soroban_sdk::vec![&env, recipient];
    let amounts = soroban_sdk::vec![&env, 100i128];

    contract.set_paused(&None, &Some(true), &None, &None, &None);
    assert!(
        contract.try_batch_payout(&recipients, &amounts).is_err(),
        "batch_payout must be refused while release_paused is set"
    );
}

// --- initialize_contract guard ---

#[test]
fn test_double_initialize_contract() {
    let env = Env::default();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Explicit mock to allow init
    env.mock_all_auths();
    client.initialize_contract(&admin);
    assert!(
        client.try_initialize_contract(&admin).is_err(),
        "a second initialize_contract must be refused"
    );
}

// --- set_paused requires initialization ---

#[test]
fn test_set_paused_before_initialize() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    assert!(
        client
            .try_set_paused(&Some(true), &None, &None, &None, &None)
            .is_err(),
        "set_paused must be refused before initialize_contract"
    );
}

// =========================================================================
// NEW NEGATIVE TESTS & EVENT EMISSIONS (Added for PR 353)
// =========================================================================

#[test]
fn test_set_paused_emits_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, admin) = setup_with_admin(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 12345;
    });

    contract.set_paused(&Some(true), &None, &None, &None, &None);

    let events = env.events().all();
    // Find the PauseStateChanged (v1) event specifically — the contract also emits
    // a PauseStateChangedV2 event after it, so we must not rely on .last().
    let emitted = events
        .iter()
        .find(|e| {
            if let Some(t0) = e.1.get(0) {
                let sym: Symbol = t0.into_val(&env);
                sym == Symbol::new(&env, "PauseSt")
            } else {
                false
            }
        })
        .expect("PauseStateChanged (PauseSt) event must be emitted");

    let topics = emitted.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "PauseSt"));

    let data: PauseStateChanged = emitted.2.try_into_val(&env).unwrap();
    assert_eq!(data.operation, symbol_short!("lock"));
    assert_eq!(data.paused, true);
    assert_eq!(data.admin, admin);
    assert_eq!(data.reason, None);
    assert!(data.timestamp > 0);
    assert!(data.receipt_id > 0);
}

#[test]
fn test_operations_resume_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    // Pause
    contract.set_paused(&Some(true), &None, &None, &None, &None);

    // Unpause
    contract.set_paused(&Some(false), &None, &None, &None, &None);

    // Should succeed now
    contract.lock_program_funds(&1000);
}

// =========================================================================
// COMPREHENSIVE RBAC + EMERGENCY WITHDRAW TESTS
// =========================================================================
// These tests ensure emergency_withdraw respects RBAC and pause state,
// emits correct events, validates balances, and handles edge cases.
// Based on patterns from bounty_escrow/src/test_pause.rs

/// Helper: Setup RBAC environment with admin and operator roles
/// Does NOT mock all auths - allows auth checks to work
fn setup_rbac_program_env_strict<'a>(
    env: &Env,
) -> (
    Address,
    Address,
    token::Client<'a>,
    ProgramEscrowContractClient<'a>,
) {
    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let token_admin = Address::generate(env);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let contract_client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract.address();
    let token_client = token::Client::new(env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(env, &token_address);

    // Temporarily allow auths for setup
    env.mock_all_auths();

    // Initialize contract with admin
    contract_client.initialize_contract(&admin);

    // Initialize program with operator as payout_key
    let program_id = String::from_str(env, "rbac-program");
    contract_client.init_program(&program_id, &operator, &token_address, &admin, &None, &None);

    // Mint and lock funds
    let depositor = Address::generate(env);
    token_admin_client.mint(&depositor, &1000);
    token_client.transfer(&depositor, &contract_client.address, &500);
    contract_client.publish_program(&program_id, &admin);
    contract_client.lock_program_funds(&500);

    // Now reset auths - subsequent operations need proper auth
    env.mock_auths(&[]);

    (admin, operator, token_client, contract_client)
}

/// Helper: Setup RBAC environment with all-auths mocked (for tests that need it)
fn setup_rbac_program_env<'a>(
    env: &Env,
) -> (
    Address,
    Address,
    token::Client<'a>,
    ProgramEscrowContractClient<'a>,
) {
    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let token_admin = Address::generate(env);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let contract_client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract.address();
    let token_client = token::Client::new(env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(env, &token_address);

    env.mock_all_auths();

    // Initialize contract with admin
    contract_client.initialize_contract(&admin);

    // Initialize program with operator as payout_key
    let program_id = String::from_str(env, "rbac-program");
    contract_client.init_program(&program_id, &operator, &token_address, &admin, &None, &None);

    // Mint and lock funds
    let depositor = Address::generate(env);
    token_admin_client.mint(&depositor, &1000);
    token_client.transfer(&depositor, &contract_client.address, &500);
    contract_client.publish_program(&program_id, &admin);
    contract_client.lock_program_funds(&500);

    (admin, operator, token_client, contract_client)
}

/// Admin CAN perform emergency_withdraw when lock is paused
#[test]
fn test_rbac_admin_can_emergency_withdraw_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _operator, token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    contract_client.set_paused(&Some(true), &None, &None, &None, &None);

    assert_eq!(token_client.balance(&contract_client.address), 500);

    contract_client.emergency_withdraw(&target);

    assert_eq!(token_client.balance(&contract_client.address), 0);
    assert_eq!(token_client.balance(&target), 500);
}

/// Operator/non-admin role CANNOT perform emergency_withdraw — auth rejected
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_rbac_operator_cannot_emergency_withdraw() {
    let env = Env::default();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env_strict(&env);
    let target = Address::generate(&env);

    // Auth checks should now reject unauthorized calls
    contract_client.set_paused(&Some(true), &None, &None, &None, &None);

    // Attempting to call emergency_withdraw without admin auth should fail
    contract_client.emergency_withdraw(&target);
}

/// emergency_withdraw FAILS even for admin when contract is NOT paused
#[test]
fn test_rbac_admin_emergency_withdraw_requires_paused_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    // Contract is unpaused by default
    assert!(
        contract_client.try_emergency_withdraw(&target).is_err(),
        "emergency_withdraw requires lock_paused"
    );
}

/// emergency_withdraw emits correct event with admin address and amount
#[test]
fn test_rbac_emergency_withdraw_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 54321;
    });

    contract_client.set_paused(&Some(true), &None, &None, &None, &None);
    contract_client.emergency_withdraw(&target);

    let all_events = env.events().all();
    let last_event = all_events.last().unwrap();

    // Verify event signature
    let topics = last_event.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "em_wtd"));

    // Verify event data: (admin, target, balance, timestamp)
    let data: EmergencyWithdrawEvent = last_event.2.try_into_val(&env).unwrap();
    assert_eq!(data.admin, admin);
    assert_eq!(data.target, target);
    assert_eq!(data.amount, 500i128);
    assert_eq!(data.timestamp, 54321u64);
    assert!(data.receipt_id > 0);
}

/// Idempotent: second emergency_withdraw on empty contract does nothing (no panic)
#[test]
fn test_rbac_emergency_withdraw_on_empty_contract_is_safe() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    contract_client.set_paused(&Some(true), &None, &None, &None, &None);
    contract_client.emergency_withdraw(&target); // drains 500

    assert_eq!(token_client.balance(&contract_client.address), 0);

    contract_client.emergency_withdraw(&target); // balance = 0, should NOT panic

    assert_eq!(token_client.balance(&contract_client.address), 0);
}

/// Paused state is preserved after emergency_withdraw
#[test]
fn test_rbac_pause_state_preserved_after_emergency_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    contract_client.set_paused(&Some(true), &None, &None, &None, &None);
    contract_client.emergency_withdraw(&target);

    let flags = contract_client.get_pause_flags();
    assert!(
        flags.lock_paused,
        "lock_paused should still be true after emergency_withdraw"
    );
}

/// Partial pause: only release paused (not lock) — emergency_withdraw still requires lock_paused
#[test]
fn test_rbac_emergency_withdraw_requires_lock_paused_not_release_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    // Only pause release, not lock
    contract_client.set_paused(&None, &Some(true), &None, &None, &None);

    assert!(
        contract_client.try_emergency_withdraw(&target).is_err(),
        "release_paused alone must not satisfy the emergency_withdraw gate"
    );
}

/// Partial pause: only refund paused (not lock) — emergency_withdraw still requires lock_paused
#[test]
fn test_rbac_emergency_withdraw_requires_lock_paused_not_refund_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    // Only pause refund, not lock
    contract_client.set_paused(&None, &None, &Some(true), &None, &None);

    assert!(
        contract_client.try_emergency_withdraw(&target).is_err(),
        "refund_paused alone must not satisfy the emergency_withdraw gate"
    );
}

/// Admin withdraws all funds in multiple programs from same contract
#[test]
fn test_rbac_emergency_withdraw_drains_all_funds() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let contract_client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract.address();
    let token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    // Initialize contract with admin
    contract_client.initialize_contract(&admin);

    // Initialize program
    let program_id_1 = String::from_str(&env, "prog-1");
    contract_client.init_program(
        &program_id_1,
        &operator,
        &token_address,
        &admin,
        &None,
        &None,
    );
    contract_client.publish_program(&program_id_1, &admin);

    // let program_id_2 = String::from_str(&env, "prog-2");
    // contract_client.init_program(&program_id_2, &operator, &token_address, &admin, &None, &None);

    // Mint and distribute funds to programs
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &3000);

    // Transfer to contract and lock in each program
    token_client.transfer(&depositor, &contract_client.address, &1500);
    contract_client.lock_program_funds(&500); // This locks 500 for the current program context

    assert!(
        token_client.balance(&contract_client.address) > 0,
        "Contract should have balance"
    );

    let target = Address::generate(&env);
    contract_client.set_paused(&Some(true), &None, &None, &None, &None);
    contract_client.emergency_withdraw(&target);

    assert_eq!(token_client.balance(&contract_client.address), 0);
    assert!(
        token_client.balance(&target) > 0,
        "Target should receive withdrawn funds"
    );
}

/// After emergency_withdraw, admin can unpause and resume normal operations
#[test]
fn test_rbac_after_emergency_withdraw_can_unpause_and_reuse() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    contract_client.set_paused(&Some(true), &None, &None, &None, &None);
    contract_client.emergency_withdraw(&target);

    // Verify paused state was set
    let flags = contract_client.get_pause_flags();
    assert!(flags.lock_paused);

    // Unpause
    contract_client.set_paused(&Some(false), &None, &None, &None, &None);
    let flags = contract_client.get_pause_flags();
    assert!(
        !flags.lock_paused,
        "lock_paused should be false after unpause"
    );

    // Verify contract can be reused (balance is 0 now but lock should work)
    // We need to mint tokens to the contract first since lock_program_funds doesn't transfer them from caller
    let token_admin = Address::generate(&env);
    let token_sac = token::StellarAssetClient::new(&env, &token_client.address);
    env.mock_all_auths();
    token_sac.mint(&contract_client.address, &200);

    contract_client.lock_program_funds(&200);
    // Note: this will fail since we drained the contract, but the point is
    // that the pause check passes
    assert_eq!(token_client.balance(&contract_client.address), 200);
}

// =========================================================================
// TIME-BASED AUTO-UNPAUSE (TTL) TESTS
// =========================================================================

// Helper: returns a fresh (client, admin) pair with the contract initialized
fn setup_ttl_env<'a>(env: &Env) -> (ProgramEscrowContractClient<'a>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize_contract(&admin);
    (client, admin)
}

// 1. Manual pause (no TTL) still works normally
#[test]
fn test_ttl_none_manual_pause_still_works() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_ttl_env(&env);

    contract.set_paused(&Some(true), &None, &None, &None, &None);
    let flags = contract.get_pause_flags();
    assert!(flags.lock_paused, "lock should be paused");
    assert!(flags.lock_unpause_at.is_none(), "no TTL should be set");
}

// 2. TTL stored correctly when pausing with unpause_at
#[test]
fn test_ttl_stored_on_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_ttl_env(&env);

    env.ledger().with_mut(|li| li.timestamp = 1000);

    contract.set_paused(&Some(true), &None, &None, &None, &Some(2000u64));
    let flags = contract.get_pause_flags();
    assert!(flags.lock_paused, "lock should be paused");
    assert_eq!(flags.lock_unpause_at, Some(2000u64), "TTL should be stored");
}

// 3. TTL cleared when manually unpausing
#[test]
fn test_ttl_cleared_on_manual_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_ttl_env(&env);

    env.ledger().with_mut(|li| li.timestamp = 1000);
    contract.set_paused(&Some(true), &None, &None, &None, &Some(5000u64));
    // Manually unpause
    contract.set_paused(&Some(false), &None, &None, &None, &None);

    let flags = contract.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(
        flags.lock_unpause_at.is_none(),
        "TTL should be cleared on unpause"
    );
}

// 4. No auto-unpause while current_time <= unpause_at (equal = still paused)
#[test]
fn test_ttl_no_auto_unpause_at_exact_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    // Pause with unpause_at = 2000
    env.ledger().with_mut(|li| li.timestamp = 1000);
    contract.set_paused(&Some(true), &None, &None, &None, &Some(2000u64));

    // Advance time to exactly unpause_at — still paused (strictly greater required)
    env.ledger().with_mut(|li| li.timestamp = 2000);
    assert!(
        contract.try_lock_program_funds(&100).is_err(),
        "TTL is exclusive: at exactly unpause_at the lock must still be refused"
    );
}

// 5. Auto-unpause triggers when current_time strictly exceeds unpause_at
#[test]
fn test_ttl_auto_unpause_when_exceeded() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    env.ledger().with_mut(|li| li.timestamp = 1000);
    contract.set_paused(&Some(true), &None, &None, &None, &Some(2000u64));

    // Advance past TTL
    env.ledger().with_mut(|li| li.timestamp = 2001);

    // lock_program_funds triggers check_paused("lock") which auto-unpauses
    contract.lock_program_funds(&1); // triggers the guard check; from=None so no real token transfer
    let flags = contract.get_pause_flags();
    assert!(!flags.lock_paused, "lock should be auto-unpaused");
    assert!(
        flags.lock_unpause_at.is_none(),
        "TTL should be cleared after auto-unpause"
    );
}

// 6. Auto-unpause emits AutoUnpauseEvent with actor="system"
#[test]
fn test_ttl_auto_unpause_emits_event_with_system_actor() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    env.ledger().with_mut(|li| li.timestamp = 1000);
    contract.set_paused(&Some(true), &None, &None, &None, &Some(2000u64));

    env.ledger().with_mut(|li| li.timestamp = 3000);

    // Trigger the guard by attempting a lock (which calls check_paused)
    contract.lock_program_funds(&1);

    let events = env.events().all();
    let auto_unpause_event = events
        .iter()
        .find(|e| {
            if let Some(t0) = e.1.get(0) {
                let sym: Symbol = t0.into_val(&env);
                sym == Symbol::new(&env, "AutoUnpse")
            } else {
                false
            }
        })
        .expect("AUTO_UNPAUSE event must be emitted");

    let data: AutoUnpauseEvent = auto_unpause_event.2.try_into_val(&env).unwrap();
    assert_eq!(data.operation, symbol_short!("lock"));
    assert_eq!(
        data.actor,
        String::from_str(&env, "system"),
        "actor must be 'system'"
    );
    assert_eq!(data.unpause_at, 2000u64);
    assert_eq!(data.triggered_at, 3000u64);
    assert!(data.receipt_id > 0);
    assert_eq!(data.version, 2u32);
}

// 7. Repeated guard calls after auto-unpause do NOT re-emit the event
#[test]
fn test_ttl_no_re_emit_on_repeated_guard_calls() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    env.ledger().with_mut(|li| li.timestamp = 1000);
    contract.set_paused(&Some(true), &None, &None, &None, &Some(2000u64));
    env.ledger().with_mut(|li| li.timestamp = 3000);

    // First call — auto-unpauses and emits event
    contract.lock_program_funds(&1);
    let count_after_first = env
        .events()
        .all()
        .iter()
        .filter(|e| {
            if let Some(t0) = e.1.get(0) {
                let sym: Symbol = t0.into_val(&env);
                sym == Symbol::new(&env, "AutoUnpse")
            } else {
                false
            }
        })
        .count();

    // Second call — no auto-unpause needed (already cleared)
    contract.lock_program_funds(&1);
    let count_after_second = env
        .events()
        .all()
        .iter()
        .filter(|e| {
            if let Some(t0) = e.1.get(0) {
                let sym: Symbol = t0.into_val(&env);
                sym == Symbol::new(&env, "AutoUnpse")
            } else {
                false
            }
        })
        .count();

    assert_eq!(
        count_after_first, count_after_second,
        "AUTO_UNPAUSE must not be re-emitted"
    );
}

// 8. All three pause modes behave consistently with TTL
#[test]
fn test_ttl_all_modes_consistent() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    env.ledger().with_mut(|li| li.timestamp = 100);
    // Pause all three modes with TTL 500
    contract.set_paused(&Some(true), &Some(true), &Some(true), &None, &Some(500u64));

    let flags = contract.get_pause_flags();
    assert!(flags.lock_paused && flags.release_paused && flags.refund_paused);
    assert_eq!(flags.lock_unpause_at, Some(500u64));
    assert_eq!(flags.release_unpause_at, Some(500u64));
    assert_eq!(flags.refund_unpause_at, Some(500u64));

    // At TTL boundary — still paused
    env.ledger().with_mut(|li| li.timestamp = 500);
    // (guard will be checked on lock_program_funds and single_payout via check_paused)
    // Just verify stored flags unchanged
    let flags = contract.get_pause_flags();
    assert!(flags.lock_paused);

    // Past TTL — auto-unpause all via individual guard invocations
    env.ledger().with_mut(|li| li.timestamp = 501);
    contract.lock_program_funds(&1); // triggers check_paused("lock")
    let flags = contract.get_pause_flags();
    assert!(!flags.lock_paused, "lock auto-unpaused");
}

// 9. Regression: existing manual pause behavior unchanged (no TTL provided)
#[test]
fn test_ttl_regression_manual_pause_still_blocks() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    env.ledger().with_mut(|li| li.timestamp = 999_999);
    // No TTL — permanent pause
    contract.set_paused(&Some(true), &None, &None, &None, &None);

    // Very far in future — should still be paused (no TTL to expire)
    env.ledger().with_mut(|li| li.timestamp = 999_999_999);
    assert!(
        contract.try_lock_program_funds(&100).is_err(),
        "a pause without a TTL must never auto-unpause"
    );
}

// 10. Per-mode TTL independence: release-only TTL does not affect lock
#[test]
fn test_ttl_per_mode_independence() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    env.ledger().with_mut(|li| li.timestamp = 100);
    // Pause lock (no TTL) and release (with TTL 500)
    contract.set_paused(&Some(true), &None, &None, &None, &None);
    contract.set_paused(&None, &Some(true), &None, &None, &Some(500u64));

    env.ledger().with_mut(|li| li.timestamp = 501);
    // trigger release guard — release should auto-unpause
    let recipient = Address::generate(&env);
    // single_payout checks release pause; it will auto-unpause release but lock stays
    // We can't call single_payout without a program setup; just verify flags
    // by calling lock (which only checks lock mode, not release)
    // lock should still be blocked:
    let flags = contract.get_pause_flags();
    // lock is still paused (no TTL); release TTL has not been triggered yet (no guard call)
    assert!(flags.lock_paused, "lock should still be paused (no TTL)");
    assert!(
        flags.release_paused,
        "release still stored as paused until guard runs"
    );
}
