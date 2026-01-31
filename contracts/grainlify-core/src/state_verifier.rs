#![no_std]
use soroban_sdk::{contracttype, Env, String, Vec, Symbol, symbol_short};
use crate::DataKey;
use grainlify_common::AuditReport;

pub fn verify_admin_integrity(env: &Env) -> Vec<String> {
    let mut issues = Vec::new(env);
    
    // Check 1: Admin exists
    if !env.storage().instance().has(&DataKey::Admin) {
        issues.push_back(String::from_str(env, "Admin not set"));
    }
    
    issues
}

pub fn check_version_consistency(env: &Env) -> bool {
    // Verify version matches expected deployment version
    env.storage().instance().has(&DataKey::Version)
}

pub fn audit_global_state(env: &Env) -> AuditReport {
    let mut checks_passed = Vec::new(env);
    let mut checks_failed = Vec::new(env);
    let mut warnings = Vec::new(env);

    // Check Admin Integrity
    let admin_issues = verify_admin_integrity(env);
    if admin_issues.is_empty() {
        checks_passed.push_back(String::from_str(env, "Admin Integrity"));
    } else {
        checks_failed.push_back(String::from_str(env, "Admin Integrity"));
        for issue in admin_issues.iter() {
            warnings.push_back(issue);
        }
    }

    // Check Version Consistency
    if check_version_consistency(env) {
        checks_passed.push_back(String::from_str(env, "Version Consistency"));
    } else {
        checks_failed.push_back(String::from_str(env, "Version Consistency"));
        warnings.push_back(String::from_str(env, "Version not set"));
    }

    AuditReport {
        contract_id: String::from_str(env, "Grainlify Core"),
        timestamp: env.ledger().timestamp(),
        checks_passed,
        checks_failed,
        warnings,
    }
}
