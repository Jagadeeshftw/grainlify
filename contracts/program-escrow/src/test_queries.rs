#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String, vec};

#[test]
fn test_query_programs_filtering() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin1.clone()).address();
    
    // Setup programs
    let pid1 = String::from_str(&env, "p1");
    let pid2 = String::from_str(&env, "p2");
    let pid3 = String::from_str(&env, "p3");

    client.initialize_program(&pid1, &admin1, &token);
    client.initialize_program(&pid2, &admin2, &token);
    client.initialize_program(&pid3, &admin1, &token);

    // Lock funds for p1
    // Mint tokens to admin so they can lock funds
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&admin1, &10000); 
    
    // Transfer funds to contract (required before locking)
    let token_auth_client = soroban_sdk::token::Client::new(&env, &token);
    token_auth_client.transfer(&admin1, &contract_id, &1000);

    client.lock_program_funds(&pid1, &1000); 
    
    // Test 1: Filter by Authorized Key (admin1)
    let filter_admin = ProgramFilter {
        authorized_payout_key: Some(admin1.clone()),
        min_total_funds: None,
        min_remaining_balance: None,
        token_address: None,
    };
    let pagination = Pagination { limit: 10, offset: 0 };
    
    let results_admin = client.query_programs(&filter_admin, &pagination);
    assert_eq!(results_admin.len(), 2);
    // Order might need check
}

#[test]
fn test_get_payouts_filtering() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    
    let pid = String::from_str(&env, "p1");
    client.initialize_program(&pid, &admin, &token);
    
    // Mint tokens to admin so they can lock funds
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&admin, &100000);

    // Transfer funds to contract (required before locking)
    let token_client = soroban_sdk::token::Client::new(&env, &token);
    token_client.transfer(&admin, &contract_id, &10000);

    client.lock_program_funds(&pid, &10000);
    
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    
    client.single_payout(&pid, &recipient1, &1000);
    
    env.ledger().set_timestamp(10000);
    client.single_payout(&pid, &recipient2, &2000);
    
    let filter_recipient = PayoutFilter {
        recipient: Some(recipient1.clone()),
        min_amount: None,
        max_amount: None,
        min_timestamp: None,
        max_timestamp: None,
    };
    let pagination = Pagination { limit: 10, offset: 0 };
    
    let results = client.get_payouts(&pid, &filter_recipient, &pagination);
    assert_eq!(results.len(), 1);
    assert_eq!(results.get(0).unwrap().recipient, recipient1);
}

#[test]
fn test_get_global_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();

    // Init programs
    let pid1 = String::from_str(&env, "p1");
    let pid2 = String::from_str(&env, "p2");
    client.initialize_program(&pid1, &admin, &token);
    client.initialize_program(&pid2, &admin, &token);

    // Lock funds
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&admin, &10000); 
    
    let token_auth_client = soroban_sdk::token::Client::new(&env, &token);
    token_auth_client.transfer(&admin, &contract_id, &5000);

    client.lock_program_funds(&pid1, &1000); 
    client.lock_program_funds(&pid2, &2000); 

    let stats = client.get_global_stats();
    assert_eq!(stats.total_programs, 2);
    assert_eq!(stats.total_value_locked, 3000);
}
