#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String, token};
use grainlify_common::AuditReport;

#[test]
fn test_audit_single_bounty() {
    let env = Env::default();
    env.mock_all_auths();
    
    // Setup
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    // Deploy Token
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::Client::new(&env, &token_contract.address());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract.address());
    
    // Deploy Escrow
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    client.init(&admin, &token_contract.address());
    
    let depositor = Address::generate(&env);
    let amount = 1000_0000;
    
    // Mint tokens to depositor
    token_admin_client.mint(&depositor, &amount);
    
    // Lock funds
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &1, &amount, &deadline);
    
    // Audit bounty 1
    let report = client.audit_state(&Some(1));
    
    assert_eq!(report.contract_id, String::from_str(&env, "Bounty Escrow"));
    
    let mut valid_state = false;
    for check in report.checks_passed.iter() {
        if check == String::from_str(&env, "Bounty State Valid") {
            valid_state = true;
        }
    }
    assert!(valid_state, "Bounty should be valid");
}

#[test]
fn test_audit_global_integrity() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_client = token::Client::new(&env, &token_contract.address());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract.address());
    
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    
    client.init(&admin, &token_contract.address());
    
    let depositor = Address::generate(&env);
    let amount = 1000_0000;
    token_admin_client.mint(&depositor, &amount);
    
    client.lock_funds(&depositor, &1, &amount, &(env.ledger().timestamp() + 1000));
    
    // Audit Global
    let report = client.audit_state(&None);
    
    assert_eq!(report.contract_id, String::from_str(&env, "Bounty Escrow Global"));
    
    let mut all_valid = false;
    for check in report.checks_passed.iter() {
        if check == String::from_str(&env, "All Checked Bounties Valid") {
            all_valid = true;
        }
    }
    assert!(all_valid, "Global audit should pass");
}
