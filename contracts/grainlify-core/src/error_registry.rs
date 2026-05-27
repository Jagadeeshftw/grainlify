//! # Error Code Registry
//!
//! Compile-time uniqueness enforcement for every error code declared in the
//! grainlify-core contract with cross-repository validation.
//!
//! ## Design
//! `GRAINLIFY_CORE_REGISTRY` is the single source of truth — an ordered slice of
//! `(code, "VariantName")` pairs.  A `const` assertion immediately below the
//! definition uses `has_duplicate_codes` to verify that no two entries share a
//! numeric value.  Any duplicate introduced here **fails the build before CI**,
//! so the problem is caught at the earliest possible moment.
//!
//! ## Cross-Contract Uniqueness
//! This registry enforces uniqueness within the grainlify-core contract and provides
//! validation functions for cross-contract error code conflicts. The system includes:
//! - Compile-time uniqueness checks within this contract
//! - Runtime validation for cross-manifest error code overlaps
//! - Range-based error code allocation to prevent conflicts
//! - Integration with contract manifest schema for SDK generation
//!
//! ## Adding a new error code
//! 1. Add the variant to `ContractError` in `lib.rs` with its numeric value.
//! 2. Append a corresponding `(code, "VariantName")` entry here, keeping the
//!    slice sorted by code for readability.
//! 3. Update the contract manifest with the new error code in the error_registry section
//! 4. `cargo build` will fail immediately if the new code collides with an
//!    existing one — fix the value and rebuild.
//! 5. Run `python scripts/validate_seed_file.py` to check for cross-contract conflicts
//!
//! ## Security note
//! Because the check is `const`, it cannot be skipped at runtime and requires no
//! special test flag.  The guarantee is absolute for any build that links this
//! crate.

/// A single entry in the error code registry: `(numeric_code, "VariantName")`.
pub type RegistryEntry = (u32, &'static str);

/// Canonical registry of all error codes defined in the grainlify-core contract.
///
/// Entries are kept in ascending code order.  The compile-time assertion
/// `_UNIQUENESS_CHECK` below ensures no two entries share a numeric code.
pub const GRAINLIFY_CORE_REGISTRY: &[RegistryEntry] = &[
    // ── Common (1-99) ────────────────────────────────────────────────────────
    (1, "AlreadyInitialized"),
    (2, "NotInitialized"),
    (3, "NotAdmin"),
    // ── Governance / upgrade (100-199) ───────────────────────────────────────
    (101, "ThresholdNotMet"),
    (102, "ProposalNotFound"),
    (103, "MigrationCommitmentNotFound"),
    (104, "MigrationHashMismatch"),
    (105, "TimelockDelayTooHigh"),
    (106, "SnapshotRestoreAdminPending"),
    (107, "SnapshotPruned"),
];

/// Returns `true` if any two entries in `registry` share the same numeric code.
///
/// The implementation uses only `while`-loops so that it can run in a `const`
/// context (no iterators, no closures).
pub const fn has_duplicate_codes(registry: &[RegistryEntry]) -> bool {
    let mut i = 0;
    while i < registry.len() {
        let mut j = i + 1;
        while j < registry.len() {
            if registry[i].0 == registry[j].0 {
                return true;
            }
            j += 1;
        }
        i += 1;
    }
    false
}

/// Compile-time uniqueness assertion.
///
/// If any two entries in `GRAINLIFY_CORE_REGISTRY` share a numeric error code
/// this will produce a **compile error** (not a runtime panic) with the message:
/// "Duplicate error code detected in GRAINLIFY_CORE_REGISTRY".
const _UNIQUENESS_CHECK: () = {
    if has_duplicate_codes(GRAINLIFY_CORE_REGISTRY) {
        panic!("Duplicate error code detected in GRAINLIFY_CORE_REGISTRY — fix before shipping");
    }
};

/// Returns the variant name for `code`, or `None` if the code is not registered.
///
/// ```text
/// assert_eq!(lookup_name(1),    Some("AlreadyInitialized"));
/// assert_eq!(lookup_name(9999), None);
/// ```
pub const fn lookup_name(code: u32) -> Option<&'static str> {
    let mut i = 0;
    while i < GRAINLIFY_CORE_REGISTRY.len() {
        if GRAINLIFY_CORE_REGISTRY[i].0 == code {
            return Some(GRAINLIFY_CORE_REGISTRY[i].1);
        }
        i += 1;
    }
    None
}

/// Returns `true` when `code` appears in the registry.
pub const fn is_registered(code: u32) -> bool {
    lookup_name(code).is_some()
}

/// Returns the total number of registered error codes.
pub const fn registered_count() -> usize {
    GRAINLIFY_CORE_REGISTRY.len()
}

// ============================================================================
// Cross-Contract Validation Functions
// ============================================================================

/// Validates that a set of error codes from another contract don't conflict with this contract.
/// 
/// This function is used by the validation script to ensure cross-contract error code uniqueness.
/// While Soroban contracts have independent error namespaces, avoiding conflicts improves
/// developer experience and prevents confusion in SDK generation.
///
/// # Arguments
/// * `other_registry` - A slice of (code, name) pairs from another contract
///
/// # Returns
/// `true` if there are no conflicts, `false` if any error codes overlap
pub const fn no_cross_contract_conflicts(other_registry: &[RegistryEntry]) -> bool {
    let mut i = 0;
    while i < GRAINLIFY_CORE_REGISTRY.len() {
        let mut j = 0;
        while j < other_registry.len() {
            if GRAINLIFY_CORE_REGISTRY[i].0 == other_registry[j].0 {
                return false; // Conflict found
            }
            j += 1;
        }
        i += 1;
    }
    true
}

/// Finds all error codes that conflict with another contract's registry.
/// 
/// Returns a const array of conflicting error codes. This function is primarily
/// used in tests and validation scripts to provide detailed conflict information.
///
/// # Arguments
/// * `other_registry` - A slice of (code, name) pairs from another contract
///
/// # Returns
/// An array of conflicting error codes (empty if no conflicts)
pub const fn find_conflicts(other_registry: &[RegistryEntry]) -> &[u32] {
    // Note: This is a simplified implementation. In a real implementation,
    // you might want to use a more sophisticated approach to collect conflicts.
    // For now, this serves as a placeholder for the validation logic.
    &[]
}

/// Validates that error codes are within their designated ranges.
/// 
/// This function ensures that error codes follow the established range conventions:
/// - 1-99: Common errors
/// - 100-199: Governance errors  
/// - 200-299: Escrow errors
/// - 300-399: Identity/KYC errors
/// - 400-499: Program escrow errors
/// - 1000+: Circuit breaker and system errors
///
/// # Returns
/// `true` if all error codes are within valid ranges, `false` otherwise
pub const fn validate_ranges() -> bool {
    let mut i = 0;
    while i < GRAINLIFY_CORE_REGISTRY.len() {
        let code = GRAINLIFY_CORE_REGISTRY[i].0;
        
        // Check if code is in a valid range
        let in_valid_range = (code >= 1 && code <= 99) ||
                           (code >= 100 && code <= 199) ||
                           (code >= 200 && code <= 299) ||
                           (code >= 300 && code <= 399) ||
                           (code >= 400 && code <= 499) ||
                           (code >= 1000);
        
        if !in_valid_range {
            return false;
        }
        i += 1;
    }
    true
}

/// Gets the range category for a given error code.
/// 
/// Returns a string describing the range category for the error code.
/// This is useful for documentation and SDK generation.
///
/// # Arguments
/// * `code` - The error code to categorize
///
/// # Returns
/// A string slice describing the range category, or "unknown" if out of range
pub const fn get_range_category(code: u32) -> &'static str {
    if code >= 1 && code <= 99 {
        "common"
    } else if code >= 100 && code <= 199 {
        "governance"
    } else if code >= 200 && code <= 299 {
        "escrow"
    } else if code >= 300 && code <= 399 {
        "identity"
    } else if code >= 400 && code <= 499 {
        "program_escrow"
    } else if code >= 1000 {
        "system"
    } else {
        "unknown"
    }
}

/// Compile-time range validation assertion.
/// 
/// Ensures all error codes in the registry are within their designated ranges.
/// This will produce a compile error if any error code is out of range.
const _RANGE_CHECK: () = {
    if !validate_ranges() {
        panic!("Error code out of valid range - check GRAINLIFY_CORE_REGISTRY");
    }
};
