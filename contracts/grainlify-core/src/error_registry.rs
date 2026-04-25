//! # Error Code Registry
//!
//! Compile-time uniqueness enforcement for every error code declared in the
//! grainlify-core contract.
//!
//! ## Design
//! `GRAINLIFY_CORE_REGISTRY` is the single source of truth — an ordered slice of
//! `(code, "VariantName")` pairs.  A `const` assertion immediately below the
//! definition uses `has_duplicate_codes` to verify that no two entries share a
//! numeric value.  Any duplicate introduced here **fails the build before CI**,
//! so the problem is caught at the earliest possible moment.
//!
//! ## Adding a new error code
//! 1. Add the variant to `ContractError` in `lib.rs` with its numeric value.
//! 2. Append a corresponding `(code, "VariantName")` entry here, keeping the
//!    slice sorted by code for readability.
//! 3. `cargo build` will fail immediately if the new code collides with an
//!    existing one — fix the value and rebuild.
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
