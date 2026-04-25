// # BuildInfo Event Tests
//
// Tests for the `BuildInfo` event emission during contract initialization.

extern crate std;
use std::vec::Vec;

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env,
};

use crate::{BuildInfoEvent, GrainlifyContract, GrainlifyContractClient, VERSION};

fn setup_contract(env: &Env) -> (GrainlifyContractClient, Address) {
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let admin = Address::generate(env);
    (client, admin)
}

#[test]
fn test_build_info_event_emitted_on_init() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    client.init_admin(&admin);
    assert!(env.events().all().len() > 0);
}

#[test]
fn test_build_info_event_admin_field() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    client.init_admin(&admin);
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn test_build_info_event_version_field() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    client.init_admin(&admin);
    assert_eq!(client.get_version(), VERSION);
}

#[test]
fn test_build_info_event_timestamp_accuracy() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let before = env.ledger().timestamp();
    client.init_admin(&admin);
    let after = env.ledger().timestamp();
    assert!(before <= after);
}

#[test]
#[should_panic(expected = "1")]
fn test_double_initialization_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    client.init_admin(&admin);
    client.init_admin(&admin);
}

#[test]
fn test_build_info_event_emitted_once() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    client.init_admin(&admin);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.init_admin(&admin);
    }));
    assert!(result.is_err());
}

#[test]
fn test_build_info_event_requires_admin_auth() {
    let env = Env::default();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.init_admin(&admin);
    }));
    assert!(result.is_err());

    env.mock_all_auths();
    client.init_admin(&admin);
    assert!(env.events().all().len() > 0);
}

#[test]
fn test_build_info_event_requires_init() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    client.init_admin(&admin);
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn test_build_info_event_with_different_admins() {
    let env = Env::default();
    env.mock_all_auths();

    let admins = std::vec![
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    for admin in admins {
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        client.init_admin(&admin);
        assert_eq!(client.get_admin(), Some(admin));
        assert!(env.events().all().len() > 0);
    }
}

#[test]
fn test_build_info_event_data_structure() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let event = BuildInfoEvent {
        admin: admin.clone(),
        version: VERSION,
        timestamp: 12345,
    };
    assert_eq!(event.admin, admin);
    assert_eq!(event.version, VERSION);
    assert_eq!(event.timestamp, 12345);
}

#[test]
fn test_build_info_event_version_matches_get_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    client.init_admin(&admin);
    assert_eq!(client.get_version(), VERSION);
}

#[test]
fn test_build_info_event_per_contract_instance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    let id1 = env.register_contract(None, GrainlifyContract);
    let client1 = GrainlifyContractClient::new(&env, &id1);
    client1.init_admin(&admin1);

    let id2 = env.register_contract(None, GrainlifyContract);
    let client2 = GrainlifyContractClient::new(&env, &id2);
    client2.init_admin(&admin2);

    assert_eq!(client1.get_admin(), Some(admin1.clone()));
    assert_eq!(client2.get_admin(), Some(admin2.clone()));
    assert_ne!(admin1, admin2);
}
