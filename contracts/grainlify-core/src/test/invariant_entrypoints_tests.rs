#![cfg(test)]

use crate::{DataKey, GrainlifyContract, GrainlifyContractClient};
use soroban_sdk::{testutils::{Address as _, Ledger as _}, Address, Env, String, Symbol};

fn setup_contract(env: &Env) -> (GrainlifyContractClient<'_>, Address) {
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

#[test]
fn test_check_invariants_healthy_after_init() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let report = client.check_invariants();
    assert!(report.healthy);
    assert!(report.config_sane);
    assert!(report.metrics_sane);
    assert!(report.admin_set);
    assert!(report.version_set);
    assert_eq!(report.violation_count, 0);
    assert!(client.verify_invariants());
}

#[test]
fn test_check_invariants_detects_metric_drift() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    env.as_contract(&client.address, || {
        let op_key = Symbol::new(&env, "op_count");
        let err_key = Symbol::new(&env, "err_count");
        env.storage().persistent().set(&op_key, &2_u64);
        env.storage().persistent().set(&err_key, &5_u64);
    });

    let report = client.check_invariants();
    assert!(report.config_sane);
    assert!(!report.metrics_sane);
    assert!(!report.healthy);
    assert!(report.violation_count > 0);
    assert!(!client.verify_invariants());
}

#[test]
fn test_check_invariants_detects_config_drift() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    env.as_contract(&client.address, || {
        env.storage().instance().remove(&DataKey::Version);
    });

    let report = client.check_invariants();
    assert!(!report.config_sane);
    assert!(!report.healthy);
    assert!(report.violation_count > 0);
    assert!(!client.verify_invariants());
}

#[test]
fn test_monitoring_views_are_safe_on_uninitialized_state() {
    let env = Env::default();
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let health = client.health_check();
    let analytics = client.get_analytics();

    assert!(!health.is_healthy);
    assert_eq!(health.last_operation, 0);
    assert_eq!(health.total_operations, 0);
    assert_eq!(health.contract_version, String::from_str(&env, "0.0.0"));

    assert_eq!(analytics.operation_count, 0);
    assert_eq!(analytics.unique_users, 0);
    assert_eq!(analytics.error_count, 0);
    assert_eq!(analytics.error_rate, 0);
}

#[test]
fn test_monitoring_views_report_tracked_activity() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let other_user = Address::generate(&env);

    env.ledger().set_timestamp(100);
    env.as_contract(&client.address, || {
        crate::monitoring::track_operation(&env, Symbol::new(&env, "init"), admin.clone(), true);
    });

    env.ledger().set_timestamp(250);
    env.as_contract(&client.address, || {
        crate::monitoring::track_operation(&env, Symbol::new(&env, "noop"), admin.clone(), true);
    });

    env.ledger().set_timestamp(900);
    env.as_contract(&client.address, || {
        crate::monitoring::track_operation(
            &env,
            Symbol::new(&env, "retry"),
            other_user.clone(),
            false,
        );
    });

    let health = client.health_check();
    let analytics = client.get_analytics();

    assert!(health.is_healthy);
    assert_eq!(health.last_operation, 900);
    assert_eq!(health.total_operations, 4);
    assert_eq!(health.contract_version, String::from_str(&env, "2.0.0"));

    assert_eq!(analytics.operation_count, 4);
    assert_eq!(analytics.unique_users, 2);
    assert_eq!(analytics.error_count, 1);
    assert_eq!(analytics.error_rate, 2500);
}
