//! Test module for role separation and controller rotation functionality

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_role_management_schema_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    // Initialize contract - should set up role management schema
    client.initialize_contract(&admin);
    
    // Verify schema version is set
    let schema_version = client.get_role_management_schema_version();
    assert_eq!(schema_version, 1);
}

#[test]
fn test_admin_proposal_basic() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    
    // Initialize contract
    client.initialize_contract(&admin);
    
    // Test admin proposal
    client.propose_admin(&new_admin);
    
    // Verify events were emitted
    let events = env.events().all();
    assert!(events.len() >= 2); // init + propose
}

#[test] 
fn test_controller_proposal_basic() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let program_id = String::from_str(&env, "test-program");
    let new_controller = Address::generate(&env);
    
    // Initialize program
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    
    // Test controller proposal
    client.propose_controller(&program_id, &admin, &new_controller);
    
    // Verify events were emitted
    let events = env.events().all();
    assert!(events.len() >= 3); // init + publish + propose
}
