#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_registry_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let contract_a = Address::generate(&env);
    let name = String::from_str(&env, "vault");
    client.init_admin(&admin);
    client.register_deployed_contract(&contract_a, &name, &ContractKind::Other, &1u32);
    let retrieved = client.get_deployed_contract(&contract_a).unwrap();
    assert_eq!(retrieved.address, contract_a);
}

#[test]
fn test_cannot_init_twice() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init_admin(&admin);
    let result = client.try_init_admin(&admin);
    assert!(result.is_err());
}

#[test]
fn test_get_nonexistent_address() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init_admin(&admin);
    assert!(client.get_deployed_contract(&Address::generate(&env)).is_none());
}
