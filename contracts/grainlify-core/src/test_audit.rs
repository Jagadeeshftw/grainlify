#![cfg(test)]

use crate::{GrainlifyContract, GrainlifyContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_audit_admin_integrity() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    // Initial state: No check (init_admin not called yet - wait, init is required for auth)
    // But audit_state is public.
    
    // Scenario 1: Uninitialized (Admin not set)
    // Actually, we can't easily test "admin not set" after init because init sets it.
    // But we can test that a fresh contract has issues if we expose audit before init, 
    // OR just test happy path after init.
    
    // Let's initialize properly
    let admin = Address::generate(&env);
    client.init_admin(&admin);

    let report = client.audit_state();
    
    assert_eq!(report.contract_id, String::from_str(&env, "Grainlify Core"));
    
    // Admin integrity should pass
    let mut admin_passed = false;
    for check in report.checks_passed.iter() {
        if check == String::from_str(&env, "Admin Integrity") {
            admin_passed = true;
        }
    }
    assert!(admin_passed, "Admin Integrity check should pass");
    
    // Version consistency should pass (set to 1 on init)
    let mut version_passed = false;
    for check in report.checks_passed.iter() {
        if check == String::from_str(&env, "Version Consistency") {
            version_passed = true;
        }
    }
    assert!(version_passed, "Version Consistency check should pass");
}

#[test]
fn test_audit_version_consistency() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init_admin(&admin);

    client.set_version(&2);
    
    let report = client.audit_state();
     // Version consistency should still pass
    let mut version_passed = false;
    for check in report.checks_passed.iter() {
        if check == String::from_str(&env, "Version Consistency") {
            version_passed = true;
        }
    }
    assert!(version_passed, "Version Consistency check should pass after update");
}
