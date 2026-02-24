#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, String, Symbol, TryIntoVal, IntoVal
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

fn setup_program_with_admin<'a>(env: &Env) -> (ProgramEscrowContractClient<'a>, Address, Address, token::Client<'a>) {
    let (client, admin) = setup_with_admin(env);
    let payout_key = Address::generate(env);
    
    let token_admin = Address::generate(env);
    let token_client = create_token_contract(env, &token_admin);
    
    env.mock_all_auths();
    let program_id = String::from_str(env, "test-prog");
    client.init_program(&program_id, &payout_key, &token_client.address);
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
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_client.address);
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
