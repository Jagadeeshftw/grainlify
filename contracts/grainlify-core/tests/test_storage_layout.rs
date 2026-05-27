use grainlify_core::{GrainlifyContract, STORAGE_SCHEMA_VERSION};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup_test(env: &Env) -> Address {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = grainlify_core::GrainlifyContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    contract_id
}

#[test]
fn test_storage_schema_version_constant() {
    assert_eq!(STORAGE_SCHEMA_VERSION, 1);
}

#[test]
fn test_verify_storage_layout_after_init() {
    let env = Env::default();
    let contract_id = setup_test(&env);
    let client = grainlify_core::GrainlifyContractClient::new(&env, &contract_id);
    assert!(client.verify_storage_layout());
}
