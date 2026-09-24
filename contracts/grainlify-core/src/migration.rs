//! Version migration hooks kept outside the contract facade.
//!
//! These hooks intentionally remain no-ops until a versioned storage migration
//! is required. Keeping the dispatch targets here makes the migration boundary
//! explicit without changing storage keys or entrypoint behavior.

use soroban_sdk::Env;

pub(crate) fn migrate_v1_to_v2(_env: &Env) {
    #[cfg(test)]
    crate::migration_failure_injection::maybe_trap(crate::MigrationTrapPoint::DuringV1ToV2);
}

pub(crate) fn migrate_v2_to_v3(_env: &Env) {
    #[cfg(test)]
    crate::migration_failure_injection::maybe_trap(crate::MigrationTrapPoint::DuringV2ToV3);
}
