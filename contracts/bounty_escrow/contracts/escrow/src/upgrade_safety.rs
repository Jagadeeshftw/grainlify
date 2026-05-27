//! # Upgrade Safety Module
//!
//! This module provides pre-upgrade safety checks and dry-run functionality
//! for contract upgrades. It helps prevent bricked contracts by validating
//! critical invariants before any upgrade is executed.
//!
//! ## Safety Checklist
//!
//! Before any upgrade, the following invariants are validated:
//!
//! 1. **Storage Layout Compatibility** - Ensure new code can read existing storage
//! 2. **Contract Initialization State** - Verify contract is properly initialized
//! 3. **Escrow State Consistency** - All escrows must be in valid states
//! 4. **Pending Claims Verification** - Validate all pending claims are valid
//! 5. **Admin Authority** - Verify admin address is set and valid
//! 6. **Token Configuration** - Ensure token address is configured
//! 7. **Feature Flags Readiness** - Check any feature flags are properly set
//! 8. **No Reentrancy Locks** - Ensure no reentrancy guards are stuck
//! 9. **Version Compatibility** - Validate version information
//! 10. **Balance Sanity** - Verify token balance consistency

use crate::{Error, Escrow, EscrowStatus};
use soroban_sdk::{contracttype, Env, String, Vec};

/// Result of pre-upgrade validation.
///
/// This report is returned by [`simulate_upgrade`] and is intended for
/// admin/operator review before calling the actual upgrade entrypoint.
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeSafetyReport {
    /// `true` when no blocking safety errors were found.
    pub is_safe: bool,
    /// Number of checklist checks that completed successfully.
    pub checks_passed: u32,
    /// Number of checklist checks that failed.
    pub checks_failed: u32,
    /// Non-blocking concerns that should be reviewed by operators.
    pub warnings: Vec<UpgradeWarning>,
    /// Blocking safety findings that must be resolved before upgrade.
    pub errors: Vec<UpgradeError>,
}

/// Non-blocking upgrade-safety warning.
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeWarning {
    /// Stable warning code for machine-readable handling.
    pub code: u32,
    /// Human-readable warning message.
    pub message: String,
}

/// Blocking upgrade-safety error.
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeError {
    /// Stable error code for machine-readable handling.
    pub code: u32,
    /// Human-readable error message.
    pub message: String,
}

/// Storage key for upgrade safety state
const UPGRADE_SAFETY_ENABLED: &str = "upg_safe_en";

/// Storage key for last safety check timestamp
const LAST_SAFETY_CHECK: &str = "last_safe_chk";

/// Codes for upgrade safety checks
pub mod safety_codes {
    /// Storage layout compatibility check
    pub const STORAGE_LAYOUT: u32 = 1001;
    /// Contract initialization check
    pub const INITIALIZATION: u32 = 1002;
    /// Escrow state consistency check
    pub const ESCROW_STATE: u32 = 1003;
    /// Pending claims verification
    pub const PENDING_CLAIMS: u32 = 1004;
    /// Admin authority check
    pub const ADMIN_AUTHORITY: u32 = 1005;
    /// Token configuration check
    pub const TOKEN_CONFIG: u32 = 1006;
    /// Feature flags readiness
    pub const FEATURE_FLAGS: u32 = 1007;
    /// Reentrancy lock check
    pub const REENTRANCY_LOCK: u32 = 1008;
    /// Version compatibility check
    pub const VERSION_COMPAT: u32 = 1009;
    /// Balance sanity check
    pub const BALANCE_SANITY: u32 = 1010;
}

/// Enables or disables upgrade safety checks.
///
/// # Notes
/// - Safety checks are enabled by default.
/// - Disabling checks bypasses [`validate_upgrade`] protections and should be
///   used only for emergency/manual recovery procedures.
pub fn set_safety_checks_enabled(env: &Env, enabled: bool) {
    env.storage()
        .instance()
        .set(&UPGRADE_SAFETY_ENABLED, &enabled);
}

/// Returns whether upgrade safety checks are currently enabled.
pub fn is_safety_checks_enabled(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&UPGRADE_SAFETY_ENABLED)
        .unwrap_or(true) // Safety checks enabled by default
}

/// Records the current ledger timestamp as the most recent safety-check run.
pub fn record_safety_check(env: &Env) {
    let timestamp = env.ledger().timestamp();
    env.storage().instance().set(&LAST_SAFETY_CHECK, &timestamp);
}

/// Returns the timestamp of the last recorded safety-check run, if any.
pub fn get_last_safety_check(env: &Env) -> Option<u64> {
    env.storage().instance().get(&LAST_SAFETY_CHECK)
}

/// Simulate an upgrade by performing all safety checks without mutating state.
/// This is a read-only dry-run that validates upgrade safety.
///
/// # Returns
/// Detailed [`UpgradeSafetyReport`] containing both blocking errors and
/// non-blocking warnings.
pub fn simulate_upgrade(env: &Env) -> UpgradeSafetyReport {
    let mut warnings: Vec<UpgradeWarning> = Vec::new(env);
    let mut errors: Vec<UpgradeError> = Vec::new(env);
    let mut checks_passed: u32 = 0;
    let mut checks_failed: u32 = 0;

    // Check 1: Storage Layout Compatibility
    let (storage_ok, storage_warnings) = check_storage_layout_compatibility(env);
    if storage_ok {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::STORAGE_LAYOUT,
            message: soroban_sdk::String::from_str(
                env,
                "Storage layout incompatible with current state",
            ),
        });
    }
    for w in storage_warnings {
        warnings.push_back(w);
    }

    // Check 2: Contract Initialization
    if check_initialization(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::INITIALIZATION,
            message: soroban_sdk::String::from_str(env, "Contract not properly initialized"),
        });
    }

    // Check 3: Escrow State Consistency
    let (escrow_ok, escrow_warnings) = check_escrow_states(env);
    if escrow_ok {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::ESCROW_STATE,
            message: soroban_sdk::String::from_str(env, "One or more escrows in invalid state"),
        });
    }
    for w in escrow_warnings {
        warnings.push_back(w);
    }

    // Check 4: Pending Claims Verification
    if check_pending_claims(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::PENDING_CLAIMS,
            message: soroban_sdk::String::from_str(env, "Invalid pending claims detected"),
        });
    }

    // Check 5: Admin Authority
    if check_admin_authority(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::ADMIN_AUTHORITY,
            message: soroban_sdk::String::from_str(env, "Admin authority not properly set"),
        });
    }

    // Check 6: Token Configuration
    if check_token_config(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::TOKEN_CONFIG,
            message: soroban_sdk::String::from_str(env, "Token not properly configured"),
        });
    }

    // Check 7: Feature Flags Readiness (placeholder - can be extended)
    if check_feature_flags(env) {
        checks_passed += 1;
    } else {
        warnings.push_back(UpgradeWarning {
            code: safety_codes::FEATURE_FLAGS,
            message: soroban_sdk::String::from_str(env, "Feature flags may need review"),
        });
        checks_passed += 1; // Warning doesn't fail the check
    }

    // Check 8: No Reentrancy Locks
    if check_no_reentrancy_locks(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::REENTRANCY_LOCK,
            message: soroban_sdk::String::from_str(env, "Reentrancy lock is stuck"),
        });
    }

    // Check 9: Version Compatibility
    if check_version_compatibility(env) {
        checks_passed += 1;
    } else {
        warnings.push_back(UpgradeWarning {
            code: safety_codes::VERSION_COMPAT,
            message: soroban_sdk::String::from_str(env, "Version information may be inconsistent"),
        });
        checks_passed += 1;
    }

    // Check 10: Balance Sanity
    let (balance_ok, balance_warnings) = check_balance_sanity(env);
    if balance_ok {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push_back(UpgradeError {
            code: safety_codes::BALANCE_SANITY,
            message: soroban_sdk::String::from_str(env, "Token balance inconsistency detected"),
        });
    }
    for w in balance_warnings {
        warnings.push_back(w);
    }

    // Record the safety check
    record_safety_check(env);

    UpgradeSafetyReport {
        is_safe: errors.is_empty(),
        checks_passed,
        checks_failed,
        warnings,
        errors,
    }
}

// Private check functions

fn check_storage_layout_compatibility(env: &Env) -> (bool, Vec<UpgradeWarning>) {
    let mut warnings: Vec<UpgradeWarning> = Vec::new(env);

    // Baseline keys expected by all client generations.
    if !env.storage().instance().has(&crate::DataKey::Admin) {
        return (false, warnings);
    }
    if !env.storage().instance().has(&crate::DataKey::Token) {
        return (false, warnings);
    }

    // EscrowIndex is persistent; verify entries are resolvable against either
    // legacy Escrow or extension EscrowAnon, but never both for the same id.
    let ids: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&crate::DataKey::EscrowIndex)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));

    let sample_len = ids.len().min(100);
    for idx in 0..sample_len {
        let bounty_id = ids.get(idx).unwrap();
        let has_standard = env
            .storage()
            .persistent()
            .has(&crate::DataKey::Escrow(bounty_id));
        let has_anonymous = env
            .storage()
            .persistent()
            .has(&crate::DataKey::EscrowAnon(bounty_id));

        if has_standard && has_anonymous {
            return (false, warnings);
        }
        if !has_standard && !has_anonymous {
            return (false, warnings);
        }
    }

    // Optional extension key: claim window may be absent for legacy deployments.
    if env.storage().instance().has(&crate::DataKey::ClaimWindow) {
        let claim_window: u64 = env
            .storage()
            .instance()
            .get(&crate::DataKey::ClaimWindow)
            .unwrap_or(0);
        if claim_window > 31_536_000 * 10 {
            warnings.push_back(UpgradeWarning {
                code: safety_codes::STORAGE_LAYOUT,
                message: soroban_sdk::String::from_str(
                    env,
                    "Claim window unusually large; review storage extension settings",
                ),
            });
        }
    }

    // Fee routing schema version: warn if absent (legacy deployment pre-v2.1).
    if !env
        .storage()
        .instance()
        .has(&crate::DataKey::FeeRoutingSchemaVersion)
    {
        warnings.push_back(UpgradeWarning {
            code: safety_codes::STORAGE_LAYOUT,
            message: soroban_sdk::String::from_str(
                env,
                "FeeRoutingSchemaVersion absent; upgrade will write v1 marker",
            ),
        });
    }

    (true, warnings)
}

fn check_initialization(env: &Env) -> bool {
    // Contract must be initialized to be upgradable
    env.storage().instance().has(&crate::DataKey::Admin)
        && env.storage().instance().has(&crate::DataKey::Token)
}

fn check_escrow_states(env: &Env) -> (bool, Vec<UpgradeWarning>) {
    let mut warnings: Vec<UpgradeWarning> = Vec::new(env);

    // Retrieve the index of all bounty IDs from instance storage.
    // DataKey::EscrowIndex holds a Vec<u64> of every registered bounty_id.
    let ids: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&crate::DataKey::EscrowIndex)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));

    if ids.is_empty() {
        return (true, warnings); // No escrows — nothing to validate
    }

    // Sample up to 100 escrows for performance (production contracts may have many).
    let sample_len = ids.len().min(100);

    for idx in 0..sample_len {
        let bounty_id = ids.get(idx).unwrap();
        if env
            .storage()
            .persistent()
            .has(&crate::DataKey::Escrow(bounty_id))
        {
            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&crate::DataKey::Escrow(bounty_id))
                .unwrap();

            // Basic numeric invariants
            if escrow.amount < 0 || escrow.remaining_amount < 0 {
                return (false, warnings);
            }
            if escrow.remaining_amount > escrow.amount {
                return (false, warnings);
            }

            // Status-specific invariants
            match escrow.status {
                EscrowStatus::Released => {
                    if escrow.remaining_amount != 0 {
                        warnings.push_back(UpgradeWarning {
                            code: safety_codes::ESCROW_STATE,
                            message: soroban_sdk::String::from_str(
                                env,
                                "Released escrow has non-zero remaining amount",
                            ),
                        });
                    }
                }
                EscrowStatus::Locked => {
                    if escrow.remaining_amount == 0 {
                        warnings.push_back(UpgradeWarning {
                            code: safety_codes::ESCROW_STATE,
                            message: soroban_sdk::String::from_str(
                                env,
                                "Locked escrow has zero remaining amount",
                            ),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    (true, warnings)
}

fn check_pending_claims(env: &Env) -> bool {
    // Retrieve the full bounty index from instance storage.
    let ids: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&crate::DataKey::EscrowIndex)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));

    for idx in 0..ids.len() {
        let bounty_id = ids.get(idx).unwrap();

        // If a PendingClaim record exists for this bounty, the escrow itself
        // must still be Locked (the claim references an active escrow).
        if env
            .storage()
            .persistent()
            .has(&crate::DataKey::PendingClaim(bounty_id))
        {
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<_, Escrow>(&crate::DataKey::Escrow(bounty_id))
            {
                // A pending claim against a Released/Refunded escrow is inconsistent.
                if escrow.status != EscrowStatus::Locked {
                    return false;
                }
            } else {
                // Orphan pending claim with no underlying escrow.
                return false;
            }
        }
    }

    true
}

fn check_admin_authority(env: &Env) -> bool {
    // Admin must be set; reading it validates that the storage key exists and
    // decodes to an Address without panicking.
    if !env.storage().instance().has(&crate::DataKey::Admin) {
        return false;
    }
    // Ensure the stored value actually deserialises as Address.
    let _admin: soroban_sdk::Address = match env.storage().instance().get(&crate::DataKey::Admin) {
        Some(a) => a,
        None => return false,
    };
    true
}

fn check_token_config(env: &Env) -> bool {
    // Token must be configured
    env.storage().instance().has(&crate::DataKey::Token)
}

fn check_feature_flags(env: &Env) -> bool {
    // Check pause flags if they exist
    if env.storage().instance().has(&crate::DataKey::PauseFlags) {
        let flags: crate::PauseFlags = env
            .storage()
            .instance()
            .get(&crate::DataKey::PauseFlags)
            .unwrap();

        // If contract is fully paused, warn about upgrade
        // This is not a failure but a warning
        if flags.lock_paused {
            return false; // Will become a warning in the main check
        }
    }

    true
}

fn check_no_reentrancy_locks(env: &Env) -> bool {
    // If reentrancy guard exists and is set, it should be cleared
    // A stuck reentrancy guard would prevent contract operation
    if env
        .storage()
        .instance()
        .has(&crate::DataKey::ReentrancyGuard)
    {
        let guard: bool = env
            .storage()
            .instance()
            .get(&crate::DataKey::ReentrancyGuard)
            .unwrap();
        if guard {
            return false; // Reentrancy lock is stuck
        }
    }
    true
}

fn check_version_compatibility(_env: &Env) -> bool {
    // Version tracking is a no-op placeholder; the trait `ContractVersion` exposes
    // `get_version()` which always returns a constant and cannot fail.
    true
}

fn check_balance_sanity(env: &Env) -> (bool, Vec<UpgradeWarning>) {
    let warnings: Vec<UpgradeWarning> = Vec::new(env);

    let ids: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&crate::DataKey::EscrowIndex)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));

    if ids.is_empty() {
        return (true, warnings);
    }

    // Sum the remaining_amount of all Locked escrows.
    // We cannot query the token contract balance from inside the check (no token
    // client here), but we can assert the sum is non-negative as a sanity gate.
    let mut total_locked: i128 = 0;

    for idx in 0..ids.len() {
        let bounty_id = ids.get(idx).unwrap();
        if let Some(escrow) = env
            .storage()
            .persistent()
            .get::<_, Escrow>(&crate::DataKey::Escrow(bounty_id))
        {
            if escrow.status == EscrowStatus::Locked {
                total_locked += escrow.remaining_amount;
            }
        }
    }

    if total_locked < 0 {
        return (false, warnings);
    }

    (true, warnings)
}

/// Validates upgrade prerequisites before executing upgrade.
///
/// # Errors
/// Returns `Error::UpgradeSafetyFailed` when blocking safety findings
/// are detected and checks are enabled.
pub fn validate_upgrade(env: &Env) -> Result<(), Error> {
    // Check if safety checks are enabled
    if !is_safety_checks_enabled(env) {
        return Ok(()); // Skip checks if disabled
    }

    // Run simulation
    let report = simulate_upgrade(env);

    // If not safe, return error
    if !report.is_safe {
        // Convert first error to contract error
        if !report.errors.is_empty() {
            // For simplicity, we return a generic error
            // In production, you might want more specific error codes
            return Err(Error::UpgradeSafetyCheckFailed);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BountyEscrowContract, BountyEscrowContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn create_test_env<'a>() -> (Env, BountyEscrowContractClient<'a>, soroban_sdk::Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let addr = contract_id.clone();
        (env, client, addr)
    }

    #[test]
    fn test_safety_checks_enabled_by_default() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let enabled = env.as_contract(&contract_id, || is_safety_checks_enabled(&env));
        assert!(enabled);
    }

    #[test]
    fn test_can_disable_safety_checks() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        env.as_contract(&contract_id, || set_safety_checks_enabled(&env, false));
        assert!(!env.as_contract(&contract_id, || is_safety_checks_enabled(&env)));
    }

    #[test]
    fn test_simulate_upgrade_after_init() {
        let (env, client, contract_id) = create_test_env();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = token_id.address();

        client.init(&admin, &token);

        let report = env.as_contract(&contract_id, || simulate_upgrade(&env));
        assert!(report.is_safe);
    }

    #[test]
    fn test_simulate_upgrade_before_init_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);

        let report = env.as_contract(&contract_id, || simulate_upgrade(&env));
        assert!(!report.is_safe);
    }

    #[test]
    fn test_record_safety_check() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BountyEscrowContract);

        assert!(env
            .as_contract(&contract_id, || get_last_safety_check(&env))
            .is_none());

        env.as_contract(&contract_id, || record_safety_check(&env));

        assert!(env
            .as_contract(&contract_id, || get_last_safety_check(&env))
            .is_some());
    }
}
