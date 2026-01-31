#![no_std]
use soroban_sdk::{contracttype, String, Vec};

/// Standardized audit report structure for all Grainlify contracts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditReport {
    /// The human-readable ID/Name of the contract being audited.
    pub contract_id: String,
    /// Timestamp when the audit was performed.
    pub timestamp: u64,
    /// List of invariant checks that passed.
    pub checks_passed: Vec<String>,
    /// List of invariant checks that failed.
    pub checks_failed: Vec<String>,
    /// List of non-critical warnings or observations.
    pub warnings: Vec<String>,
}
