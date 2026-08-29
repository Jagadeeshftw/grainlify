#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
fn test_token_allowlist_enforcement_default_allows_all() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);

    assert!(client.is_token_allowed(&token1));
    assert!(client.is_token_allowed(&token2));

    let program_id = String::from_str(&env, "prog-1");
    // Should succeed because allowlist is empty
    env.mock_all_auths();
    client.init_program(&program_id, &admin, &token1, &admin, &None, &None);
}

#[test]
fn test_token_allowlist_enforcement_blocks_unlisted() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let allowed_token = Address::generate(&env);
    let unlisted_token = Address::generate(&env);

    // Add token to allowlist - this enables enforcement
    env.mock_all_auths();
    client.add_allowed_token(&allowed_token);

    assert!(client.is_token_allowed(&allowed_token));
    assert!(!client.is_token_allowed(&unlisted_token));

    // Using allowed token succeeds
    let program1 = String::from_str(&env, "prog-1");
    client.init_program(&program1, &admin, &allowed_token, &admin, &None, &None);
}

#[test]
#[should_panic(expected = "Token not on allowlist")]
fn test_token_allowlist_enforcement_panic_unlisted() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let allowed_token = Address::generate(&env);
    let unlisted_token = Address::generate(&env);

    env.mock_all_auths();
    client.add_allowed_token(&allowed_token);

    let program2 = String::from_str(&env, "prog-2");
    // This should panic
    client.init_program(&program2, &admin, &unlisted_token, &admin, &None, &None);
}

#[test]
fn test_token_allowlist_batch_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let allowed_token = Address::generate(&env);

    env.mock_all_auths();
    client.add_allowed_token(&allowed_token);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-batch-1"),
        authorized_payout_key: admin.clone(),
        token_address: allowed_token.clone(),
        reference_hash: None,
    });

    let count = client.try_batch_initialize_programs(&items).unwrap().unwrap();
    assert_eq!(count, 1);
}

#[test]
#[should_panic(expected = "Token not on allowlist")]
fn test_token_allowlist_batch_initialization_unlisted() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let allowed_token = Address::generate(&env);
    let unlisted_token = Address::generate(&env);

    env.mock_all_auths();
    client.add_allowed_token(&allowed_token);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-batch-1"),
        authorized_payout_key: admin.clone(),
        token_address: allowed_token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-batch-2"),
        authorized_payout_key: admin.clone(),
        token_address: unlisted_token.clone(),
        reference_hash: None,
    });

    let _ = client.batch_initialize_programs(&items);
}

#[test]
fn test_token_allowlist_remove_token() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);

    env.mock_all_auths();
    client.add_allowed_token(&token1);
    client.add_allowed_token(&token2);

    assert!(client.is_token_allowed(&token1));
    assert!(client.is_token_allowed(&token2));

    client.remove_allowed_token(&token1);

    assert!(!client.is_token_allowed(&token1));
    assert!(client.is_token_allowed(&token2));

    let tokens = client.get_allowed_tokens();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens.get(0).unwrap(), token2);

    // Removing last token disables enforcement
    client.remove_allowed_token(&token2);
    assert!(client.is_token_allowed(&token1)); // Enforcement disabled
}

// =============================================================================
// TESTS FOR MAXIMUM PROGRAM COUNT (#501)
// =============================================================================

/// Stress test: create many programs via sequential batches and verify counts
/// and sampling queries remain accurate (bounded for CI).
#[test]
fn test_max_program_count_sequential_batches_queries_accurate() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    const BATCH_SIZE: u32 = 10;
    const NUM_BATCHES: u32 = 3;
    let total_programs = BATCH_SIZE * NUM_BATCHES;

    for batch in 0..NUM_BATCHES {
        let mut items = Vec::new(&env);
        for i in 0..BATCH_SIZE {
            let idx = batch * BATCH_SIZE + i;
            items.push_back(ProgramInitItem {
                program_id: make_program_id(&env, idx),
                authorized_payout_key: admin.clone(),
                token_address: token.clone(),
                reference_hash: None,
            });
        }
        let count = client
            .try_batch_initialize_programs(&items)
            .unwrap()
            .unwrap();
        assert_eq!(count, BATCH_SIZE);
    }

    for i in 0..total_programs {
        assert!(
            client.program_exists_by_id(&make_program_id(&env, i)),
            "program {} should exist",
            i
        );
    }
    assert!(client.program_exists());
}

// =============================================================================
// TESTS FOR MULTI-TENANT ISOLATION (#473)
// =============================================================================

/// Verify funds, schedules, and analytics for one program cannot affect or
/// be read as another program's data (tenant isolation).
#[test]
fn test_multi_tenant_no_cross_program_balance_or_analytics() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_a = env.register_contract(None, ProgramEscrowContract);
    let client_a = ProgramEscrowContractClient::new(&env, &contract_a);
    let contract_b = env.register_contract(None, ProgramEscrowContract);
    let client_b = ProgramEscrowContractClient::new(&env, &contract_b);

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let _token_client = token::Client::new(&env, &token_id);
    let token_sac = token::StellarAssetClient::new(&env, &token_id);

    let admin_a = Address::generate(&env);
    let admin_b = Address::generate(&env);
    let creator = Address::generate(&env);

    let program_id_a = String::from_str(&env, "prog-isolation-a");
    client_a.init_program(&program_id_a, &admin_a, &token_id, &creator, &None, &None);
    client_a.publish_program(program_id_a.clone(), admin_a.clone());

    let program_id_b = String::from_str(&env, "prog-isolation-b");
    client_b.init_program(&program_id_b, &admin_b, &token_id, &creator, &None, &None);
    client_b.publish_program(program_id_b.clone(), admin_b.clone());

    token_sac.mint(&client_a.address, &500_000);
    token_sac.mint(&client_b.address, &300_000);
    client_a.lock_program_funds(&500_000);
    client_b.lock_program_funds(&300_000);

    let stats_a = client_a.get_program_aggregate_stats();
    let stats_b = client_b.get_program_aggregate_stats();
    assert_eq!(stats_a.total_funds, 500_000);
    assert_eq!(stats_a.remaining_balance, 500_000);
    assert_eq!(stats_b.total_funds, 300_000);
    assert_eq!(stats_b.remaining_balance, 300_000);

    let r = Address::generate(&env);
    client_a.single_payout(&r, &100_000,
    &None
);

    assert_eq!(client_a.get_remaining_balance(), 400_000);
    assert_eq!(client_b.get_remaining_balance(), 300_000);
    let info_a = client_a.get_program_info();
    let info_b = client_b.get_program_info();
    assert_eq!(info_a.payout_history.len(), 1);
    assert_eq!(info_b.payout_history.len(), 0);
    assert_eq!(client_a.get_program_aggregate_stats().payout_count, 1);
    assert_eq!(client_b.get_program_aggregate_stats().payout_count, 0);
}

// Note: Additional multi-tenant isolation tests exist above (test_batch_payout_no_cross_program_interference, etc.)

// =============================================================================
// TESTS FOR PROGRAM ANALYTICS AND MONITORING VIEWS
// =============================================================================

// Test: get_program_aggregate_stats returns correct initial values
