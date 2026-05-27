//! Tests for deterministic error ordering and prioritization.
//!
//! ## Validation Precedence (enforced order)
//!
//! 1. Reentrancy guard
//! 2. Contract initialized (panic "Program not initialized")
//! 3. Paused / deprecated (operational state)
//! 4. Authorization
//! 5. Input validation (amounts, batch size)
//! 6. Business logic (insufficient balance, etc.)
//!
//! When multiple error conditions apply simultaneously, the contract must always
//! return the highest-priority error, making behavior predictable across versions.

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token, vec, Address, Env, String,
};

fn setup_env() -> (Env, ProgramEscrowContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();

    (env, client, admin, token_id)
}

fn setup_initialized(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    token::StellarAssetClient<'static>,
) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    let program_id = String::from_str(env, "test-program");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id);
    client.initialize_contract(&admin);

    (client, admin, token_admin_client)
}

// ── lock_program_funds ────────────────────────────────────────────────────────

/// Priority 2 beats priority 3: not-initialized is returned even when amount is also invalid.
#[test]
#[should_panic(expected = "Program not initialized")]
fn test_lock_not_initialized_beats_invalid_amount() {
    let (_env, client, _admin, _token_id) = setup_env();
    // Contract not initialized AND amount is invalid (0) — must get NotInitialized first
    client.lock_program_funds(&0);
}

/// Priority 2 beats priority 3: not-initialized is returned even when paused would also apply.
#[test]
#[should_panic(expected = "Program not initialized")]
fn test_lock_not_initialized_beats_paused() {
    let (_env, client, _admin, _token_id) = setup_env();
    // No program initialized — must get NotInitialized, not FundsPaused
    client.lock_program_funds(&1000);
}

/// Priority 3 beats priority 5: paused is returned even when amount is also invalid.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_paused_beats_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _token_admin) = setup_initialized(&env);

    // Pause lock operations
    client.set_paused(&Some(true), &None, &None, &None);

    // Both paused AND amount=0 are invalid — must get FundsPaused first
    client.lock_program_funds(&0);
}

/// Priority 5 (amount validation) fires after all higher-priority checks pass.
#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_lock_invalid_amount_after_all_higher_checks_pass() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);

    token_admin.mint(&client.address, &10_000);

    // Contract initialized, not paused — only amount is invalid
    client.lock_program_funds(&0);
}

// ── batch_payout ──────────────────────────────────────────────────────────────

/// Priority 2 beats priority 5: not-initialized is returned even when batch is also empty.
#[test]
#[should_panic(expected = "Program not initialized")]
fn test_batch_payout_not_initialized_beats_empty_batch() {
    let (env, client, _admin, _token_id) = setup_env();
    let recipient = Address::generate(&env);
    client.batch_payout(&vec![&env, recipient], &vec![&env, 0i128]);
}

/// Priority 3 beats priority 5: paused is returned even when batch inputs are also invalid.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_batch_payout_paused_beats_invalid_input() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);
    token_admin.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    // Pause release operations
    client.set_paused(&None, &Some(true), &None, &None);

    let recipient = Address::generate(&env);
    // Both paused AND amount=0 — must get FundsPaused first
    client.batch_payout(&vec![&env, recipient], &vec![&env, 0i128]);
}

/// Priority 5 (empty batch) fires after all higher-priority checks pass.
#[test]
#[should_panic(expected = "Cannot process empty batch")]
fn test_batch_payout_empty_batch_after_higher_checks_pass() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);
    token_admin.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    client.batch_payout(&vec![&env], &vec![&env]);
}

/// Priority 5 (length mismatch) fires after all higher-priority checks pass.
#[test]
#[should_panic(expected = "Recipients and amounts vectors must have the same length")]
fn test_batch_payout_length_mismatch_after_higher_checks_pass() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);
    token_admin.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    // 2 recipients, 1 amount — length mismatch
    client.batch_payout(&vec![&env, r1, r2], &vec![&env, 500i128]);
}

/// Priority 6 (insufficient balance) fires after all higher-priority checks pass.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_batch_payout_insufficient_balance_after_higher_checks_pass() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);
    token_admin.mint(&client.address, &1_000);
    client.lock_program_funds(&1_000);

    let recipient = Address::generate(&env);
    // Amount exceeds balance — insufficient balance
    client.batch_payout(&vec![&env, recipient], &vec![&env, 999_999i128]);
}

// ── single_payout ─────────────────────────────────────────────────────────────

/// Priority 2 beats priority 5: not-initialized is returned even when amount is also invalid.
#[test]
#[should_panic(expected = "Program not initialized")]
fn test_single_payout_not_initialized_beats_invalid_amount() {
    let (env, client, _admin, _token_id) = setup_env();
    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &0);
}

/// Priority 3 beats priority 5: paused is returned even when amount is also invalid.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_single_payout_paused_beats_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);
    token_admin.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    client.set_paused(&None, &Some(true), &None, &None);

    let recipient = Address::generate(&env);
    // Both paused AND amount=0 — must get FundsPaused first
    client.single_payout(&recipient, &0);
}

/// Priority 5 (invalid amount) fires after all higher-priority checks pass.
#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_single_payout_invalid_amount_after_higher_checks_pass() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);
    token_admin.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &0);
}

/// Priority 6 (insufficient balance) fires after all higher-priority checks pass.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_single_payout_insufficient_balance_after_higher_checks_pass() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_admin) = setup_initialized(&env);
    token_admin.mint(&client.address, &500);
    client.lock_program_funds(&500);

    let recipient = Address::generate(&env);
    // Amount exceeds balance
    client.single_payout(&recipient, &999_999);
}
