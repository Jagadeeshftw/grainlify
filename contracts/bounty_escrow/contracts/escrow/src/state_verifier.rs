#![no_std]
use soroban_sdk::{contracttype, Env, String, Vec, Symbol, symbol_short, Address, token};
use crate::{DataKey, Escrow, EscrowStatus};
use grainlify_common::AuditReport;

pub fn verify_bounty_escrow(env: &Env, bounty_id: u64) -> Vec<String> {
    let mut issues = Vec::new(env);
    
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        issues.push_back(String::from_str(env, "Bounty not found"));
        return issues;
    }
    
    let escrow: Escrow = env.storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();
    
    // Check 1: Amount is positive
    if escrow.amount <= 0 {
        issues.push_back(String::from_str(env, "Invalid amount"));
    }
    
    // Check 2: Deadline is in the future (for Locked escrows)
    if escrow.status == EscrowStatus::Locked {
        if escrow.deadline < env.ledger().timestamp() {
            issues.push_back(String::from_str(env, "Deadline passed for locked funds"));
        }
    }
    
    issues
}

pub fn check_escrow_integrity(env: &Env, bounty_id: u64) -> bool {
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(env, &token_addr);
    
    let escrow: Escrow = env.storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();
    
    let contract_balance = client.balance(&env.current_contract_address());
    
    // For Locked escrows, verify funds are actually in the contract
    // Note: This is a loose check as contract balance holds ALL locked funds
    if escrow.status == EscrowStatus::Locked {
        contract_balance >= escrow.amount
    } else {
        true
    }
}

pub fn audit_bounty(env: &Env, bounty_id: u64) -> AuditReport {
    let mut checks_passed = Vec::new(env);
    let mut checks_failed = Vec::new(env);
    let mut warnings = Vec::new(env);

    let bounty_issues = verify_bounty_escrow(env, bounty_id);
    if bounty_issues.is_empty() {
        checks_passed.push_back(String::from_str(env, "Bounty State Valid"));
    } else {
        checks_failed.push_back(String::from_str(env, "Bounty State Invalid"));
        for issue in bounty_issues.iter() {
            warnings.push_back(issue);
        }
    }

    if check_escrow_integrity(env, bounty_id) {
         checks_passed.push_back(String::from_str(env, "Funds Integrity"));
    } else {
         checks_failed.push_back(String::from_str(env, "Funds Integrity"));
         warnings.push_back(String::from_str(env, "Contract balance insufficient"));
    }

    AuditReport {
        contract_id: String::from_str(env, "Bounty Escrow"),
        timestamp: env.ledger().timestamp(),
        checks_passed,
        checks_failed,
        warnings,
    }
}
