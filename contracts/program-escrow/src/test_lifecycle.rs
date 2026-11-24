#![cfg(test)]

use crate::test_support::setup_program;
use crate::{ProgramEscrowContract, ProgramEscrowContractClient, ProgramStatus};
use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, String};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

fn make_token(
    env: &Env,
    admin: &Address,
) -> (
    token::Client<'static>,
    Address,
    token::StellarAssetClient<'static>,
) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = sac.address();
    let token_client = token::Client::new(env, &token_id);
    let token_sac = token::StellarAssetClient::new(env, &token_id);
    (token_client, token_id, token_sac)
}

fn pid(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

fn assert_conservation(
    client: &ProgramEscrowContractClient<'_>,
    token_client: &token::Client<'_>,
    sum_locked: i128,
    sum_paid: i128,
) {
    let remaining = client.get_remaining_balance();
    let token_balance = token_client.balance(&client.address);
    assert_eq!(
        sum_locked,
        sum_paid + remaining,
        "conservation violated: sum(lock) != sum(payout) + remaining"
    );
    assert_eq!(
        token_balance, remaining,
        "token balance must match contract remaining_balance"
    );
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_contract_sets_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn test_double_initialize_contract_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);
    let result = client.try_initialize_contract(&admin);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// init_program & Draft status
// ---------------------------------------------------------------------------

#[test]
fn test_init_program_starts_as_draft() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    let (_token_client, token_id, _token_sac) = make_token(&env, &admin);

    let program_id = pid(&env, "p-001");
    let info = client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    assert_eq!(info.status, ProgramStatus::Draft);
    assert_eq!(info.remaining_balance, 0);
    assert_eq!(info.total_funds, 0);
}

#[test]
fn test_init_duplicate_program_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    let (_token_client, token_id, _token_sac) = make_token(&env, &admin);

    let program_id = pid(&env, "p-001");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    let result = client.try_init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// publish_program
// ---------------------------------------------------------------------------

#[test]
fn test_publish_transitions_draft_to_active() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    let (_token_client, token_id, _token_sac) = make_token(&env, &admin);

    let program_id = pid(&env, "p-001");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    let published = client.publish_program(&program_id, &admin);
    assert_eq!(published.status, ProgramStatus::Active);
}

#[test]
fn test_publish_active_program_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    let (_token_client, token_id, _token_sac) = make_token(&env, &admin);

    let program_id = pid(&env, "p-001");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);
    let result = client.try_publish_program(&program_id, &admin);
    assert!(result.is_err());
}

#[test]
fn test_publish_nonexistent_program_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    let result = client.try_publish_program(&pid(&env, "nonexistent"), &admin);
    assert!(result.is_err());
}

#[test]
fn test_lifecycle_timeline_records_publish() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = make_client(&env);
    let admin = Address::generate(&env);
    let (_token_client, token_id, _token_sac) = make_token(&env, &admin);

    let program_id = pid(&env, "p-001");
    let draft = client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    assert_eq!(draft.status, ProgramStatus::Draft);
    assert_eq!(client.get_program_lifecycle_timeline(&program_id).len(), 1);

    let active = client.publish_program(&program_id, &admin);
    assert_eq!(active.status, ProgramStatus::Active);
    assert_eq!(client.get_program_lifecycle_timeline(&program_id).len(), 2);
}

// ---------------------------------------------------------------------------
// Lock funds validation
// ---------------------------------------------------------------------------

#[test]
fn test_lock_zero_funds_fails() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_sac) = setup_program(&env, 0);
    let result = client.try_lock_program_funds(&0);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// End-to-end lifecycle & funds conservation
// ---------------------------------------------------------------------------

#[test]
fn test_full_lifecycle_funds_conservation() {
    let env = Env::default();
    let (client, _admin, token_client, token_sac) = setup_program(&env, 0);

    let mut sum_locked: i128 = 0;
    let mut sum_paid: i128 = 0;
    assert_conservation(&client, &token_client, sum_locked, sum_paid);

    // Step 1: Lock initial funds
    token_sac.mint(&client.address, &100_000);
    client.lock_program_funds(&100_000);
    sum_locked += 100_000;
    assert_conservation(&client, &token_client, sum_locked, sum_paid);

    // Step 2: Top up funds
    token_sac.mint(&client.address, &50_000);
    client.lock_program_funds(&50_000);
    sum_locked += 50_000;
    assert_conservation(&client, &token_client, sum_locked, sum_paid);

    // Step 3: Single payout
    let w1 = Address::generate(&env);
    client.single_payout(&w1, &40_000, &None);
    sum_paid += 40_000;
    assert_eq!(token_client.balance(&w1), 40_000);
    assert_conservation(&client, &token_client, sum_locked, sum_paid);

    // Step 4: Batch payout
    let w2 = Address::generate(&env);
    let w3 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, w2.clone(), w3.clone()],
        &vec![&env, 60_000, 50_000],
    );
    sum_paid += 110_000;
    assert_eq!(token_client.balance(&w2), 60_000);
    assert_eq!(token_client.balance(&w3), 50_000);
    assert_conservation(&client, &token_client, sum_locked, sum_paid);
    assert_eq!(client.get_remaining_balance(), 0);
}

// ---------------------------------------------------------------------------
// STATE: Draft - Refund Operations Blocked
// Refund operations should fail when program is in Draft status.
// ---------------------------------------------------------------------------

/// cancel_claim should fail when program is in Draft status.
#[test]
fn test_cancel_claim_fails_on_draft_program() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    let (_token_client, token_id, token_sac) = make_token(&env, &admin);
    token_sac.mint(&contract_id, &50_000);
    let program_id = pid(&env, "draft-prog");

    // Initialize program but do NOT publish - stays in Draft status
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    // Attempt to cancel a claim on Draft program should fail
    let result = client.try_cancel_claim(&program_id, &1, &admin);
    assert!(result.is_err());
}

/// emergency_withdraw should fail when program is in Draft status.
#[test]
fn test_emergency_withdraw_fails_on_draft_program() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);
    let (_token_client, token_id, token_sac) = make_token(&env, &admin);
    token_sac.mint(&contract_id, &50_000);
    let program_id = pid(&env, "draft-prog");

    // Initialize program but do NOT publish - stays in Draft status
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    // Set lock_paused to true (required for emergency_withdraw)
    client.set_paused(&Some(true), &Some(false), &Some(false), &None, &None);

    // Attempt emergency withdraw on Draft program should fail
    let target = Address::generate(&env);
    let result = client.try_emergency_withdraw(&target);
    assert!(result.is_err());
}

/// emergency_withdraw should succeed when program is in Active status.
#[test]
fn test_emergency_withdraw_succeeds_on_active_program() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);
    let (token_client, token_id, token_sac) = make_token(&env, &admin);
    let program_id = pid(&env, "active-prog");

    // Initialize and publish program - transitions to Active status
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);
    token_sac.mint(&client.address, &50_000);
    client.lock_program_funds(&50_000);
    assert_conservation(&client, &token_client, 50_000, 0);

    // Set lock_paused to true (required for emergency_withdraw)
    client.set_paused(&Some(true), &Some(false), &Some(false), &None, &None);

    // Emergency withdraw should succeed on Active program
    let target = Address::generate(&env);
    let initial_balance = token_client.balance(&target);
    client.emergency_withdraw(&target);
    let final_balance = token_client.balance(&target);

    // Target should have received the funds and contract balance is drained
    assert_eq!(final_balance - initial_balance, 50_000);
    assert_eq!(token_client.balance(&client.address), 0);
}
