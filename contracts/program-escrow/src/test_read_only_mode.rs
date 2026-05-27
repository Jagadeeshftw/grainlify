#![cfg(test)]

use crate::{ProgramEscrowContract, ProgramEscrowContractClient, ReadOnlyModeChanged};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, IntoVal, String, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract.address();
    token::Client::new(env, &token_address)
}

fn setup_program_with_admin<'a>(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'a>,
    Address,
    Address,
    token::Client<'a>,
) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_auths(&[]).initialize_contract(&admin);
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
    client.publish_program(&program_id);
    (client, admin, payout_key, token_client)
}

// ---------------------------------------------------------------------------
// Default state
// ---------------------------------------------------------------------------

/// Read-only mode must be off by default — operators should not need to
/// explicitly disable it on a freshly initialised contract.
#[test]
fn test_read_only_default_is_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    assert_eq!(contract.is_read_only(), false);
}

/// Read-only and maintenance mode are stored under different DataKeys and
/// must never alias each other.
#[test]
fn test_read_only_distinct_from_maintenance_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    contract.set_read_only_mode(&true, &None);
    assert_eq!(contract.is_read_only(), true);
    assert_eq!(contract.is_maintenance_mode(), false);
}

// ---------------------------------------------------------------------------
// Toggle and event verification
// ---------------------------------------------------------------------------

/// Enabling read-only emits ROModeChg with correct payload fields.
#[test]
fn test_set_read_only_mode_on_and_off() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, admin, _payout_key, _token) = setup_program_with_admin(&env);
    env.ledger().with_mut(|li| { li.timestamp = 420; });
    let reason = Some(String::from_str(&env, "Testing"));
    contract.set_read_only_mode(&true, &reason);
    assert_eq!(contract.is_read_only(), true);
    let events = env.events().all();
    let emitted = events.iter().last().unwrap();
    let topics = emitted.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "ROModeChg"));
    let data: ReadOnlyModeChanged = emitted.2.try_into_val(&env).unwrap();
    assert_eq!(data.enabled, true);
    assert_eq!(data.admin, admin);
    assert_eq!(data.timestamp, 420);
    assert_eq!(data.reason, reason);
    contract.set_read_only_mode(&false, &None);
    assert_eq!(contract.is_read_only(), false);
}

/// Disabling read-only also emits a ROModeChg event with enabled=false.
#[test]
fn test_read_only_disable_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    contract.set_read_only_mode(&true, &None);
    contract.set_read_only_mode(&false, &None);
    let events = env.events().all();
    let emitted = events.iter().last().unwrap();
    let data: ReadOnlyModeChanged = emitted.2.try_into_val(&env).unwrap();
    assert_eq!(data.enabled, false);
}

// ---------------------------------------------------------------------------
// View calls always succeed in read-only mode
// ---------------------------------------------------------------------------

/// All pure query methods must remain accessible when read-only is active.
/// Blocking reads would make the mode unusable for forensic audits.
#[test]
fn test_view_calls_succeed_in_read_only_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    contract.set_read_only_mode(&true, &None);
    let _flag  = contract.is_read_only();
    let _maint = contract.is_maintenance_mode();
    let _pause = contract.get_pause_flags();
    let _stats = contract.get_program_analytics();
}

// ---------------------------------------------------------------------------
// Blocked write operations
// ---------------------------------------------------------------------------

/// lock_program_funds transfers tokens into escrow — a state mutation.
/// Must be rejected under read-only mode.
#[test]
#[should_panic(expected = "Read-only mode")]
fn test_read_only_blocks_lock_program_funds() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, token) = setup_program_with_admin(&env);
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &1000);
    token.transfer(&depositor, &contract.address, &1000);
    contract.set_read_only_mode(&true, &None);
    contract.lock_program_funds(&1000i128);
}

/// set_paused mutates pause flags — even admin config writes are blocked
/// in read-only mode because the mode is designed for zero state changes.
#[test]
#[should_panic(expected = "Read-only mode")]
fn test_read_only_blocks_set_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    contract.set_read_only_mode(&true, &None);
    contract.set_paused(&Some(true), &None, &None, &None);
}

/// single_payout transfers tokens out of escrow — blocked in read-only mode.
#[test]
#[should_panic(expected = "Read-only mode")]
fn test_read_only_blocks_single_payout() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, token) = setup_program_with_admin(&env);
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &5000);
    token.transfer(&depositor, &contract.address, &5000);
    contract.lock_program_funds(&5000i128);
    contract.set_read_only_mode(&true, &None);
    let recipient = Address::generate(&env);
    contract.single_payout(&recipient, &1000);
}

/// batch_payout transfers tokens to multiple recipients — blocked in read-only mode.
#[test]
#[should_panic(expected = "Read-only mode")]
fn test_read_only_blocks_batch_payout() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, token) = setup_program_with_admin(&env);
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &5000);
    token.transfer(&depositor, &contract.address, &5000);
    contract.lock_program_funds(&5000i128);
    contract.set_read_only_mode(&true, &None);
    let recipient = Address::generate(&env);
    contract.batch_payout(&vec![&env, recipient], &vec![&env, 1000]);
}

// ---------------------------------------------------------------------------
// Operations resume after read-only is lifted
// ---------------------------------------------------------------------------

/// After disabling read-only, the contract must accept deposits again.
/// This verifies the mode flag is correctly cleared from storage.
#[test]
fn test_lock_succeeds_after_read_only_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, token) = setup_program_with_admin(&env);
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &1000);
    token.transfer(&depositor, &contract.address, &1000);
    contract.set_read_only_mode(&true, &None);
    contract.set_read_only_mode(&false, &None);
    contract.lock_program_funds(&1000i128);
}

// ---------------------------------------------------------------------------
// Interaction: read-only + maintenance together
// ---------------------------------------------------------------------------

/// Both modes can be active simultaneously and are stored independently.
/// This confirms no shared flag or aliasing bug.
#[test]
fn test_both_modes_active_simultaneously() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    contract.set_maintenance_mode(&true);
    contract.set_read_only_mode(&true, &None);
    assert_eq!(contract.is_maintenance_mode(), true);
    assert_eq!(contract.is_read_only(), true);
}

/// Disabling read-only must not clear the maintenance mode flag.
/// The two modes are independent; operators manage them separately.
#[test]
fn test_disabling_read_only_does_not_disable_maintenance() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    contract.set_maintenance_mode(&true);
    contract.set_read_only_mode(&true, &None);
    contract.set_read_only_mode(&false, &None);
    assert_eq!(contract.is_maintenance_mode(), true);
    assert_eq!(contract.is_read_only(), false);
}

// ---------------------------------------------------------------------------
// Interaction: read-only + pause flags
// ---------------------------------------------------------------------------

/// Pause flags set before read-only is enabled must survive the mode toggle.
/// Neither enabling nor disabling read-only should clear pause flags.
#[test]
fn test_read_only_does_not_clear_pause_flags() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, _token) = setup_program_with_admin(&env);
    contract.set_paused(&Some(true), &None, &None, &None);
    let flags_before = contract.get_pause_flags();
    contract.set_read_only_mode(&true, &None);
    contract.set_read_only_mode(&false, &None);
    let flags_after = contract.get_pause_flags();
    assert_eq!(flags_before.lock_paused, flags_after.lock_paused);
}
