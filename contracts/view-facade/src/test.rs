#![cfg(test)]

use crate::{ViewFacade, ViewFacadeClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_bounty_batch_query_correctness() {
    let env = Env::default();
    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);
    
    let bounty_contract = Address::generate(&env);
    let mut bounty_ids = Vec::new(&env);
    bounty_ids.push_back(1u64);
    bounty_ids.push_back(2u64);
    
    let results = facade.get_bounty_batch(&bounty_contract, &bounty_ids);
    
    assert!(results.len() <= bounty_ids.len());
}

#[test]
fn test_depositor_summary_aggregation() {
    let env = Env::default();
    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);
    
    let bounty_contract = Address::generate(&env);
    let depositor = Address::generate(&env);
    
    let summary = facade.get_depositor_summary(&bounty_contract, &depositor);
    
    assert_eq!(summary.depositor, depositor);
    assert!(summary.total_deposited >= 0);
    assert!(summary.active_bounties >= 0);
    assert!(summary.completed_bounties >= 0);
}

#[test]
fn test_program_batch_query() {
    let env = Env::default();
    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);
    
    let program_contract = Address::generate(&env);
    let mut program_ids = Vec::new(&env);
    program_ids.push_back(String::from_str(&env, "program1"));
    program_ids.push_back(String::from_str(&env, "program2"));
    
    let results = facade.get_program_batch(&program_contract, &program_ids);
    
    assert!(results.len() <= program_ids.len());
}

#[test]
fn test_aggregated_stats_non_negative() {
    let env = Env::default();
    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);
    
    let bounty_contract = Address::generate(&env);
    
    let stats = facade.get_aggregated_bounty_stats(&bounty_contract);
    
    assert!(stats.total_locked >= 0);
    assert!(stats.total_released >= 0);
    assert!(stats.total_refunded >= 0);
    assert!(stats.active_bounties <= stats.total_bounties);
}

#[test]
fn test_empty_batch_returns_empty() {
    let env = Env::default();
    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);
    
    let bounty_contract = Address::generate(&env);
    let empty_ids: Vec<u64> = Vec::new(&env);
    
    let results = facade.get_bounty_batch(&bounty_contract, &empty_ids);
    
    assert_eq!(results.len(), 0);
}
