#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

#[test]
fn test_registry_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GrainlifyRegistry);
    let client = GrainlifyRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let contract_a = Address::generate(&env);
    let name = Symbol::new(&env, "vault");
    client.init(&admin);
    client.set_addr(&name, &contract_a);
    let retrieved_addr = client.get_addr(&name);
    assert_eq!(retrieved_addr, contract_a);
}

#[test]
#[should_panic(expected = "Init")]
fn test_cannot_init_twice() {
    let env = Env::default();
    let contract_id = env.register_contract(None, GrainlifyRegistry);
    let client = GrainlifyRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);
    client.init(&admin);
}

#[test]
#[should_panic(expected = "NF")]
fn test_get_nonexistent_address() {
    let env = Env::default();
    let contract_id = env.register_contract(None, GrainlifyRegistry);
    let client = GrainlifyRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);
    client.get_addr(&Symbol::new(&env, "nothing"));
}
