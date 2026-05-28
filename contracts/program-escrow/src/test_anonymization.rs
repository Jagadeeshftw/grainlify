//! Tests for the anonymous resolver feature (Issue #1291).
//!
//! The anonymous resolver allows an admin to designate an intermediary address
//! that receives payouts on behalf of real recipients, keeping recipient
//! identities off-chain while preserving on-chain auditability.
#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (ProgramEscrowContractClient<'static>, Address, String) {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    client.initialize_contract(&admin);

    let token_admin = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let program_id = String::from_str(env, "anon-prog");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    (client, admin, program_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// get_anonymous_resolver — initial state
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_get_anonymous_resolver_returns_none_when_not_set() {
    let env = Env::default();
    let (client, _admin, program_id) = setup(&env);

    let result = client.get_anonymous_resolver(&program_id);
    assert!(result.is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// set_anonymous_resolver — happy path
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_set_anonymous_resolver_stores_resolver() {
    let env = Env::default();
    let (client, _admin, program_id) = setup(&env);

    let resolver = Address::generate(&env);
    client.set_anonymous_resolver(&program_id, &resolver);

    let stored = client.get_anonymous_resolver(&program_id).unwrap();
    assert_eq!(stored.resolver, resolver);
}

#[test]
fn test_set_anonymous_resolver_records_set_by_admin() {
    let env = Env::default();
    let (client, admin, program_id) = setup(&env);

    let resolver = Address::generate(&env);
    client.set_anonymous_resolver(&program_id, &resolver);

    let stored = client.get_anonymous_resolver(&program_id).unwrap();
    assert_eq!(stored.set_by, admin);
}

#[test]
fn test_set_anonymous_resolver_records_timestamp() {
    let env = Env::default();
    let (client, _admin, program_id) = setup(&env);

    let resolver = Address::generate(&env);
    client.set_anonymous_resolver(&program_id, &resolver);

    let stored = client.get_anonymous_resolver(&program_id).unwrap();
    // Timestamp must be a non-zero ledger timestamp
    assert_eq!(stored.updated_at, env.ledger().timestamp());
}

// ─────────────────────────────────────────────────────────────────────────────
// set_anonymous_resolver — update / overwrite
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_set_anonymous_resolver_can_be_updated() {
    let env = Env::default();
    let (client, _admin, program_id) = setup(&env);

    let resolver1 = Address::generate(&env);
    let resolver2 = Address::generate(&env);

    client.set_anonymous_resolver(&program_id, &resolver1);
    client.set_anonymous_resolver(&program_id, &resolver2);

    let stored = client.get_anonymous_resolver(&program_id).unwrap();
    assert_eq!(stored.resolver, resolver2);
}

// ─────────────────────────────────────────────────────────────────────────────
// remove_anonymous_resolver — happy path
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_remove_anonymous_resolver_clears_storage() {
    let env = Env::default();
    let (client, _admin, program_id) = setup(&env);

    let resolver = Address::generate(&env);
    client.set_anonymous_resolver(&program_id, &resolver);
    client.remove_anonymous_resolver(&program_id);

    let result = client.get_anonymous_resolver(&program_id);
    assert!(result.is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// remove_anonymous_resolver — error: not set
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "No anonymous resolver set for this program")]
fn test_remove_anonymous_resolver_panics_when_not_set() {
    let env = Env::default();
    let (client, _admin, program_id) = setup(&env);

    client.remove_anonymous_resolver(&program_id);
}

// ─────────────────────────────────────────────────────────────────────────────
// set_anonymous_resolver — error: program not found
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Program not found")]
fn test_set_anonymous_resolver_panics_for_unknown_program() {
    let env = Env::default();
    let (client, _admin, _program_id) = setup(&env);

    let resolver = Address::generate(&env);
    let bad_id = String::from_str(&env, "does-not-exist");
    client.set_anonymous_resolver(&bad_id, &resolver);
}

// ─────────────────────────────────────────────────────────────────────────────
// set_anonymous_resolver — error: not admin (auth check)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_set_anonymous_resolver_requires_admin_auth() {
    let env = Env::default();
    // Do NOT mock all auths — only mock the non-admin caller
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize_contract(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let program_id = String::from_str(&env, "anon-prog");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    // Clear mocked auths so the next call must provide real auth
    env.set_auths(&[]);

    let resolver = Address::generate(&env);
    // This should panic because no auth is provided for the admin
    client.set_anonymous_resolver(&program_id, &resolver);
}

// ─────────────────────────────────────────────────────────────────────────────
// Isolation: different programs have independent resolvers
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_anonymous_resolver_is_per_program() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let prog_a = String::from_str(&env, "prog-a");
    let prog_b = String::from_str(&env, "prog-b");
    client.init_program(&prog_a, &admin, &token_id, &admin, &None, &None);
    client.init_program(&prog_b, &admin, &token_id, &admin, &None, &None);

    let resolver_a = Address::generate(&env);
    let resolver_b = Address::generate(&env);

    client.set_anonymous_resolver(&prog_a, &resolver_a);
    client.set_anonymous_resolver(&prog_b, &resolver_b);

    assert_eq!(
        client.get_anonymous_resolver(&prog_a).unwrap().resolver,
        resolver_a
    );
    assert_eq!(
        client.get_anonymous_resolver(&prog_b).unwrap().resolver,
        resolver_b
    );
}

#[test]
fn test_remove_resolver_on_one_program_does_not_affect_another() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let prog_a = String::from_str(&env, "prog-a");
    let prog_b = String::from_str(&env, "prog-b");
    client.init_program(&prog_a, &admin, &token_id, &admin, &None, &None);
    client.init_program(&prog_b, &admin, &token_id, &admin, &None, &None);

    let resolver = Address::generate(&env);
    client.set_anonymous_resolver(&prog_a, &resolver);
    client.set_anonymous_resolver(&prog_b, &resolver);

    client.remove_anonymous_resolver(&prog_a);

    assert!(client.get_anonymous_resolver(&prog_a).is_none());
    assert!(client.get_anonymous_resolver(&prog_b).is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// set → remove → set cycle
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_set_after_remove_works() {
    let env = Env::default();
    let (client, _admin, program_id) = setup(&env);

    let resolver1 = Address::generate(&env);
    let resolver2 = Address::generate(&env);

    client.set_anonymous_resolver(&program_id, &resolver1);
    client.remove_anonymous_resolver(&program_id);
    client.set_anonymous_resolver(&program_id, &resolver2);

    let stored = client.get_anonymous_resolver(&program_id).unwrap();
    assert_eq!(stored.resolver, resolver2);
}
