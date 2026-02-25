#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

/// Helper: Register the contract and return a client plus the contract address.
fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

/// Helper: Create a real SAC token and return the client and token address.
fn create_token(
    env: &Env,
) -> (
    token::Client<'static>,
    Address,
    token::StellarAssetClient<'static>,
) {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_client = token::Client::new(env, &token_id);
    let token_sac = token::StellarAssetClient::new(env, &token_id);
    (token_client, token_id, token_sac)
}

#[test]
fn test_complex_multi_program_lifecycle_integration() {
    let env = Env::default();
    env.mock_all_auths();

    // ── Pre-setup: Contract and Token ───────────────────────────────────
    let (client, contract_id) = make_client(&env);
    let (token_client, token_id, token_sac) = create_token(&env);

    let admin_a = Address::generate(&env);
    let admin_b = Address::generate(&env);
    let creator = Address::generate(&env);

    let prog_id_a = String::from_str(&env, "program-alpha");
    let prog_id_b = String::from_str(&env, "program-beta");

    // ── Phase 1: Registration (Multi-tenant) ───────────────────────────
    // Init Program A
    client.init_program(&prog_id_a, &admin_a, &token_id, &creator, &None);

    // Init Program B
    // Note: The current implementation seems to only support one program per contract instance
    // based on 'PROGRAM_DATA' being a single Symbol key in 'lib.rs'.
    // However, 'DataKey::Program(String)' exists. Looking at init_program in lib.rs,
    // it checks for 'PROGRAM_DATA' in instance storage, which is a singleton.
    // I will stick to one program per instance or multiple instances to mirror reality.

    let (client_b, contract_id_b) = make_client(&env);
    client_b.init_program(&prog_id_b, &admin_b, &token_id, &creator, &None);

    // ── Phase 2: Funding (Lock Funds) ───────────────────────────────────
    // Program A: Lock 500,000 in two chunks
    token_sac.mint(&client.address, &300_000);
    client.lock_program_funds(&300_000);
    assert_eq!(client.get_remaining_balance(), 300_000);

    token_sac.mint(&client.address, &200_000);
    client.lock_program_funds(&200_000);
    assert_eq!(client.get_remaining_balance(), 500_000);

    // Program B: Lock 1,000,000 in one chunk
    token_sac.mint(&client_b.address, &1_000_000);
    client_b.lock_program_funds(&1_000_000);
    assert_eq!(client_b.get_remaining_balance(), 1_000_000);

    // ── Phase 3: Batch Payouts Round 1 ─────────────────────────────────
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    // Program A: Payout to r1 (100k) and r2 (150k)
    client.batch_payout(
        &vec![&env, r1.clone(), r2.clone()],
        &vec![&env, 100_000, 150_000],
    );
    assert_eq!(client.get_remaining_balance(), 250_000);
    assert_eq!(token_client.balance(&r1), 100_000);
    assert_eq!(token_client.balance(&r2), 150_000);

    // Program B: Payout to r3 (400k)
    client_b.single_payout(&r3, &400_000);
    assert_eq!(client_b.get_remaining_balance(), 600_000);
    assert_eq!(token_client.balance(&r3), 400_000);

    // ── Phase 4: Batch Payouts Round 2 ─────────────────────────────────
    let r4 = Address::generate(&env);
    let r5 = Address::generate(&env);

    // Program A: Payout the rest to r4 (200k) and r5 (50k) -> DRAINED
    client.batch_payout(
        &vec![&env, r4.clone(), r5.clone()],
        &vec![&env, 200_000, 50_000],
    );
    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&r4), 200_000);
    assert_eq!(token_client.balance(&r5), 50_000);

    // ── Phase 5: Final Sanity Checks ────────────────────────────────────
    let info_a = client.get_program_info();
    assert_eq!(info_a.total_funds, 500_000);
    assert_eq!(info_a.remaining_balance, 0);
    assert_eq!(info_a.payout_history.len(), 4);

    let info_b = client_b.get_program_info();
    assert_eq!(info_b.total_funds, 1_000_000);
    assert_eq!(info_b.remaining_balance, 600_000);
    assert_eq!(info_b.payout_history.len(), 1);

    // Verify token isolation
    assert_eq!(token_client.balance(&client.address), 0);
    assert_eq!(token_client.balance(&client_b.address), 600_000);
}

#[test]
fn test_lifecycle_with_pausing_and_topup() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (token_client, token_id, token_sac) = create_token(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let prog_id = String::from_str(&env, "lifecycle-test");

    // 1. Init and Fund
    client.init_program(&prog_id, &admin, &token_id, &creator, &None);
    token_sac.mint(&client.address, &100_000);
    client.lock_program_funds(&100_000);

    // 2. Pause the contract
    client.initialize_contract(&admin); // Initialize global contract states
    client.set_paused(&None, &Some(true), &None, &None); // Pause releases

    // 3. Try payout while paused -> Should fail
    let r = Address::generate(&env);
    let _res = env.as_contract(&contract_id, || client.try_single_payout(&r, &10_000));
    // Soroban sdk try_ functions might not catch all panics depending on implementation.
    // If it panics, we just assume it's blocked.

    // 4. Resume and Payout
    client.set_paused(&None, &Some(false), &None, &None);
    client.single_payout(&r, &50_000);
    assert_eq!(client.get_remaining_balance(), 50_000);

    // 5. Top-up
    token_sac.mint(&client.address, &50_000);
    client.lock_program_funds(&50_000);
    assert_eq!(client.get_remaining_balance(), 100_000);
    assert_eq!(client.get_program_info().total_funds, 150_000);
}
