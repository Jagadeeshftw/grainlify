//! # Tests for Batch Payout Size Limits
//!
//! Verifies that `MAX_BATCH_SIZE` is correctly calibrated and that
//! `batch_payout` rejects oversized batches with the typed
//! `BatchError::BatchTooLarge` (code 410) error rather than a generic panic.
#![cfg(test)]
extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{
    BatchError, ProgramEscrowContract, ProgramEscrowContractClient, MAX_BATCH_SIZE,
};

// ── constant sanity ──────────────────────────────────────────────────────────

/// MAX_BATCH_SIZE must stay at 100 (the production-safe default derived from
/// Soroban's 100 M instruction budget — see derivation comment in lib.rs).
#[test]
fn test_max_batch_size_constant_is_100() {
    assert_eq!(MAX_BATCH_SIZE, 100);
}

#[test]
fn test_max_batch_size_within_soroban_budget() {
    // Must be positive and within the empirically safe range (≤ 1 400).
    assert!(MAX_BATCH_SIZE > 0);
    assert!(MAX_BATCH_SIZE <= 1_400);
}

// ── contract-level pre-flight rejection ──────────────────────────────────────

fn setup_contract(env: &Env) -> (ProgramEscrowContractClient<'static>, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    client.initialize_contract(&admin);
    (client, admin, token_id)
}

fn init_funded_program(
    env: &Env,
    client: &ProgramEscrowContractClient,
    token_id: &Address,
    admin: &Address,
    amount: i128,
) {
    let creator = Address::generate(env);
    soroban_sdk::token::StellarAssetClient::new(env, token_id).mint(&creator, &amount);
    client.init_program(
        &String::from_str(env, "PROG"),
        admin,
        token_id,
        &creator,
        &Some(amount),
        &None,
    );
    client.publish_program();
}

/// A batch of exactly MAX_BATCH_SIZE recipients must succeed.
#[test]
fn test_batch_payout_at_max_size_succeeds() {
    let env = Env::default();
    let (client, admin, token_id) = setup_contract(&env);
    let per_recipient: i128 = 10;
    let total = per_recipient * MAX_BATCH_SIZE as i128;
    init_funded_program(&env, &client, &token_id, &admin, total);

    let recipients: soroban_sdk::Vec<Address> = (0..MAX_BATCH_SIZE)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(Address::generate(&env));
            v
        });
    let amounts: soroban_sdk::Vec<i128> = (0..MAX_BATCH_SIZE)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(per_recipient);
            v
        });

    let result = client.try_batch_payout(&recipients, &amounts, &None);
    assert!(result.is_ok(), "batch at MAX_BATCH_SIZE should succeed");
}

/// A batch of MAX_BATCH_SIZE + 1 must be rejected with BatchTooLarge (410).
#[test]
fn test_batch_payout_exceeds_max_returns_batch_too_large() {
    let env = Env::default();
    let (client, admin, token_id) = setup_contract(&env);
    let oversized = MAX_BATCH_SIZE + 1;
    init_funded_program(&env, &client, &token_id, &admin, oversized as i128 * 10);

    let recipients: soroban_sdk::Vec<Address> = (0..oversized)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(Address::generate(&env));
            v
        });
    let amounts: soroban_sdk::Vec<i128> = (0..oversized)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(10_i128);
            v
        });

    let result = client.try_batch_payout(&recipients, &amounts, &None);
    assert!(
        matches!(result, Err(Ok(BatchError::BatchTooLarge))),
        "expected BatchError::BatchTooLarge (410), got: {:?}",
        result
    );
}

/// A significantly oversized batch (2× limit) must also return BatchTooLarge.
#[test]
fn test_batch_payout_double_max_returns_batch_too_large() {
    let env = Env::default();
    let (client, admin, token_id) = setup_contract(&env);
    let oversized = MAX_BATCH_SIZE * 2;
    init_funded_program(&env, &client, &token_id, &admin, oversized as i128 * 10);

    let recipients: soroban_sdk::Vec<Address> = (0..oversized)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(Address::generate(&env));
            v
        });
    let amounts: soroban_sdk::Vec<i128> = (0..oversized)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(10_i128);
            v
        });

    let result = client.try_batch_payout(&recipients, &amounts, &None);
    assert!(matches!(result, Err(Ok(BatchError::BatchTooLarge))));
}

/// Pre-flight rejection must fire before any token transfer (no partial state).
#[test]
fn test_batch_too_large_fires_before_any_transfer() {
    let env = Env::default();
    let (client, admin, token_id) = setup_contract(&env);
    let oversized = MAX_BATCH_SIZE + 1;
    let initial_balance: i128 = oversized as i128 * 10;
    init_funded_program(&env, &client, &token_id, &admin, initial_balance);

    let recipients: soroban_sdk::Vec<Address> = (0..oversized)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(Address::generate(&env));
            v
        });
    let amounts: soroban_sdk::Vec<i128> = (0..oversized)
        .fold(soroban_sdk::Vec::new(&env), |mut v, _| {
            v.push_back(10_i128);
            v
        });

    let _ = client.try_batch_payout(&recipients, &amounts, &None);

    // Balance must be unchanged — no transfer occurred.
    let prog = client.get_program_info_v2(&String::from_str(&env, "PROG"));
    assert_eq!(
        prog.remaining_balance, initial_balance,
        "no funds should be transferred when batch is too large"
    );
}
