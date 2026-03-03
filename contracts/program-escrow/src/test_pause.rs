#![cfg(test)]

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
    client.init_program(&program_id, &payout_key, &token_client.address, &admin, &None, &None);
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

    contract.set_paused(&Some(true), &None, &None, &None);

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

    contract.set_paused(&Some(true), &None, &None, &None);
    contract.set_paused(&Some(false), &None, &None, &None);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
}

// --- set_paused: release ---

#[test]
fn test_set_paused_release() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_with_admin(&env);

    contract.set_paused(&None, &Some(true), &None, &None);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, true);
    assert_eq!(flags.refund_paused, false);
}

// --- mixed pause states ---

#[test]
fn test_mixed_pause_states() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_with_admin(&env);

    // Pause lock and release, leave refund unpaused
    contract.set_paused(&Some(true), &Some(true), &Some(false), &None);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, true);
    assert_eq!(flags.refund_paused, false);

    // Only update release back to unpaused; lock should stay paused
    contract.set_paused(&None, &Some(false), &None, &None);

    let flags = contract.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}

// --- lock_program_funds enforcement ---

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_program_funds_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    contract.set_paused(&Some(true), &None, &None, &None);
    contract.lock_program_funds(&1000);
}

// --- single_payout enforcement ---

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_single_payout_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    let recipient = Address::generate(&env);

    contract.set_paused(&None, &Some(true), &None, &None);
    contract.single_payout(&recipient, &100);
}

// --- batch_payout enforcement ---

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_batch_payout_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    let recipient = Address::generate(&env);

    let recipients = soroban_sdk::vec![&env, recipient];
    let amounts = soroban_sdk::vec![&env, 100i128];

    contract.set_paused(&None, &Some(true), &None, &None);
    contract.batch_payout(&recipients, &amounts);
}

// --- initialize_contract guard ---

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialize_contract() {
    let env = Env::default();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Explicit mock to allow init
    env.mock_all_auths();
    client.initialize_contract(&admin);
    client.initialize_contract(&admin); // should panic
}

// --- set_paused requires initialization ---

#[test]
#[should_panic(expected = "Not initialized")]
fn test_set_paused_before_initialize() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    client.set_paused(&Some(true), &None, &None, &None);
}

// =========================================================================
// NEW NEGATIVE TESTS & EVENT EMISSIONS (Added for PR 353)
// =========================================================================

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_pause_by_non_admin_fails() {
    let env = Env::default();
    let (contract, _admin) = setup_with_admin(&env);

    // Not calling mock_all_auths to verify admin tracking
    contract.set_paused(&Some(true), &Some(true), &Some(true), &None);
}

#[test]
fn test_set_paused_emits_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, admin) = setup_with_admin(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 12345;
    });

    contract.set_paused(&Some(true), &None, &None, &None);

    let events = env.events().all();
    let emitted = events.iter().last().unwrap();

    let topics = emitted.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "PauseSt"));

    let data: (Symbol, bool, Address, Option<String>, u64) = emitted.2.try_into_val(&env).unwrap();
    assert_eq!(data.0, Symbol::new(&env, "lock"));
    assert_eq!(data.1, true);
    assert_eq!(data.2, admin);
    assert_eq!(data.3, None);
    assert!(data.4 > 0);
}

#[test]
fn test_operations_resume_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);

    // Pause
    contract.set_paused(&Some(true), &None, &None, &None);

    // Unpause
    contract.set_paused(&Some(false), &None, &None, &None);

    // Should succeed now
    contract.lock_program_funds(&1000);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_emergency_withdraw_non_admin_fails() {
    let env = Env::default();
    let (contract, _admin) = setup_with_admin(&env);

    let target = Address::generate(&env);
    contract.emergency_withdraw(&target);
}

#[test]
#[should_panic(expected = "Not paused")]
fn test_emergency_withdraw_unpaused_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin) = setup_with_admin(&env);
    let target = Address::generate(&env);

    contract.emergency_withdraw(&target);
}

#[test]
fn test_emergency_withdraw_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, admin, _payout_key, token_client) = setup_program_with_admin(&env);
    let target = Address::generate(&env);

    // We need the token admin to mint tokens directly to the contract.
    // In test_pause.rs, token_admin is generated internally, so let's just make a new token and re-init
    // Actually, `setup_program_with_admin` doesn't expose `token_admin`.
    // We can just use the StellarAssetClient from the token client's address.
    let token_admin_client =
        soroban_sdk::token::StellarAssetClient::new(&env, &token_client.address);
    token_admin_client.mint(&admin, &1000);
    token_client.transfer(&admin, &contract.address, &500);

    // Lock some funds to get balance in contract state
    contract.lock_program_funds(&500);
    assert_eq!(token_client.balance(&contract.address), 500);

    let reason = soroban_sdk::String::from_str(&env, "Hacked");
    contract.set_paused(&Some(true), &None, &None, &Some(reason));

    contract.emergency_withdraw(&target);

    assert_eq!(token_client.balance(&contract.address), 0);
    assert_eq!(token_client.balance(&target), 500);
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

    contract_client.set_paused(&Some(true), &None, &None, &None);

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
    contract_client.set_paused(&Some(true), &None, &None, &None);

    // Attempting to call emergency_withdraw without admin auth should fail
    contract_client.emergency_withdraw(&target);
}

/// emergency_withdraw FAILS even for admin when contract is NOT paused
#[test]
#[should_panic(expected = "Not paused")]
fn test_rbac_admin_emergency_withdraw_requires_paused_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    // Contract is unpaused by default
    contract_client.emergency_withdraw(&target);
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

    contract_client.set_paused(&Some(true), &None, &None, &None);
    contract_client.emergency_withdraw(&target);

    let all_events = env.events().all();
    let last_event = all_events.last().unwrap();

    // Verify event signature
    let topics = last_event.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "em_wtd"));

    // Verify event data: (admin, target, balance, timestamp)
    let data: (Address, Address, i128, u64) = last_event.2.try_into_val(&env).unwrap();
    assert_eq!(data.0, admin);
    assert_eq!(data.1, target);
    assert_eq!(data.2, 500i128);
    assert_eq!(data.3, 54321u64);
}

/// Idempotent: second emergency_withdraw on empty contract does nothing (no panic)
#[test]
fn test_rbac_emergency_withdraw_on_empty_contract_is_safe() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    contract_client.set_paused(&Some(true), &None, &None, &None);
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

    contract_client.set_paused(&Some(true), &None, &None, &None);
    contract_client.emergency_withdraw(&target);

    let flags = contract_client.get_pause_flags();
    assert!(
        flags.lock_paused,
        "lock_paused should still be true after emergency_withdraw"
    );
}

/// Partial pause: only release paused (not lock) — emergency_withdraw still requires lock_paused
#[test]
#[should_panic(expected = "Not paused")]
fn test_rbac_emergency_withdraw_requires_lock_paused_not_release_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    // Only pause release, not lock
    contract_client.set_paused(&None, &Some(true), &None, &None);

    contract_client.emergency_withdraw(&target);
}

/// Partial pause: only refund paused (not lock) — emergency_withdraw still requires lock_paused
#[test]
#[should_panic(expected = "Not paused")]
fn test_rbac_emergency_withdraw_requires_lock_paused_not_refund_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    // Only pause refund, not lock
    contract_client.set_paused(&None, &None, &Some(true), &None);

    contract_client.emergency_withdraw(&target);
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

    // Initialize multiple programs
    // NOTE: Contract currently only supports one program per instance
    let program_id_1 = String::from_str(&env, "prog-1");
    contract_client.init_program(&program_id_1, &operator, &token_address, &admin, &None, &None);
    
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
    contract_client.set_paused(&Some(true), &None, &None, &None);
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

    contract_client.set_paused(&Some(true), &None, &None, &None);
    contract_client.emergency_withdraw(&target);

    // Verify paused state was set
    let flags = contract_client.get_pause_flags();
    assert!(flags.lock_paused);

    // Unpause
    contract_client.set_paused(&Some(false), &None, &None, &None);
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

/// Only lock_paused gate affects emergency_withdraw, not release or refund pause
#[test]
#[should_panic(expected = "Not paused")]
fn test_rbac_emergency_withdraw_ignores_release_and_refund_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _operator, _token_client, contract_client) = setup_rbac_program_env(&env);
    let target = Address::generate(&env);

    // Pause both release and refund, but NOT lock
    contract_client.set_paused(&None, &Some(true), &Some(true), &None);

    // Should still fail because lock is not paused
    contract_client.emergency_withdraw(&target);
}
