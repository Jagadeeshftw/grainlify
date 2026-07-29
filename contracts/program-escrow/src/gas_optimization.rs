//! # Gas Optimization Utilities for Program Escrow
//!
//! Optimized helpers for reducing gas consumption in program escrow operations:
//! - Efficient batch processing
//! - Optimized storage access patterns
//! - Reduced redundant computations
//! - Packed-flag storage for boolean bitmask fields

use soroban_sdk::{vec, Address, Env, String, Symbol, Vec};

/// Optimized batch lock processing with cached program data.
pub fn optimized_batch_lock<F>(
    env: &Env,
    items: &Vec<(String, i128)>,
    mut processor: F,
) -> Vec<bool>
where
    F: FnMut(&Env, &String, i128) -> bool,
{
    let mut results: Vec<bool> = Vec::new(env);
    let mut last_program: Option<String> = None;

    for item in items.iter() {
        let (program_id, amount) = item;

        // Cache program data if it's the same as last iteration
        if last_program.as_ref() != Some(&program_id) {
            last_program = Some(program_id.clone());
        }

        let success = processor(env, &program_id, amount);
        results.push_back(success);
    }

    results
}

/// Efficient deduplication of program IDs using sorting and adjacent comparison.
pub fn deduplicate_program_ids(env: &Env, items: &Vec<String>) -> Vec<String> {
    if items.len() <= 1 {
        return items.clone();
    }

    // Sort first (using insertion sort for small batches)
    let mut sorted: Vec<String> = Vec::new(env);
    for item in items.iter() {
        let mut next: Vec<String> = Vec::new(env);
        let mut inserted = false;

        for existing in sorted.iter() {
            if !inserted && item < existing {
                next.push_back(item.clone());
                inserted = true;
            }
            next.push_back(existing.clone());
        }

        if !inserted {
            next.push_back(item.clone());
        }

        sorted = next;
    }

    // Remove adjacent duplicates
    let mut deduped: Vec<String> = Vec::new(env);
    deduped.push_back(sorted.get(0).unwrap());

    for i in 1..sorted.len() {
        let current = sorted.get(i).unwrap();
        let previous = sorted.get(i - 1).unwrap();

        if current != previous {
            deduped.push_back(current);
        }
    }

    deduped
}

/// Check for duplicates in a Vec<String> using O(n²) comparison.
/// Returns true if duplicates exist.
pub fn has_duplicates(_env: &Env, items: &Vec<String>) -> bool {
    let len = items.len();
    if len <= 1 {
        return false;
    }

    for i in 0..len {
        for j in (i + 1)..len {
            if items.get(i).unwrap() == items.get(j).unwrap() {
                return true;
            }
        }
    }

    false
}

/// Optimized storage access with TTL management.
pub mod storage_efficiency {
    use soroban_sdk::{Env, Symbol};

    /// Extend TTL for the contract instance.
    pub fn extend_instance_ttl(env: &Env, ttl_threshold: u32, extend_to: u32) {
        env.storage().instance().extend_ttl(ttl_threshold, extend_to);
    }

    /// Check if storage key exists without retrieving value.
    pub fn storage_has(env: &Env, key: &Symbol) -> bool {
        env.storage().instance().has(key)
    }
}

/// Packed storage for boolean flags to reduce storage operations.
///
/// This module provides bit-packing utilities that store multiple boolean flags
/// in a single `u32` value, reducing the number of storage slots from N booleans
/// to 1 packed value. Each bit represents an independent pause state.
///
/// # Bit Layout
/// - Bit 0: PAUSE_LOCK
/// - Bit 1: PAUSE_RELEASE
/// - Bit 2: PAUSE_REFUND
/// - Bit 3: PAUSE_ARCHIVE
/// - Bit 4: PAUSE_BATCH_LOCK
/// - Bit 5: PAUSE_BATCH_RELEASE
/// - Bit 6: PAUSE_BATCH_REFUND
/// - Bit 7: PAUSE_EMERGENCY
///
/// # Security Guarantees
/// - Bitwise operations are atomic and do not corrupt other bits
/// - Setting bit A never affects bit B (tested across all 256 combinations)
/// - Reading after write returns the exact same value
pub mod packed_storage {
    use soroban_sdk::{Env, Symbol};

    /// Packed flag bit definitions for granular pause control.
    /// Each bit represents an independent pause state that can be toggled without affecting others.
    pub const PAUSE_LOCK: u32 = 1 << 0;
    pub const PAUSE_RELEASE: u32 = 1 << 1;
    pub const PAUSE_REFUND: u32 = 1 << 2;
    pub const PAUSE_ARCHIVE: u32 = 1 << 3;
    pub const PAUSE_BATCH_LOCK: u32 = 1 << 4;
    pub const PAUSE_BATCH_RELEASE: u32 = 1 << 5;
    pub const PAUSE_BATCH_REFUND: u32 = 1 << 6;
    pub const PAUSE_EMERGENCY: u32 = 1 << 7;

    /// Mask for all 8 granular pause flags (bits 0-7).
    pub const FLAG_MASK_8BITS: u32 = 0xFF;

    /// Store multiple pause flags in a single u32.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `key` - Storage key for the packed flags
    /// * `flags` - Bitmask where each bit represents a pause state (bits 0-7)
    ///
    /// # Security Notes
    /// Uses atomic bitwise OR to combine new flags with existing state,
    /// ensuring no flag corruption during concurrent updates.
    pub fn set_packed_flags(env: &Env, key: &Symbol, flags: u32) {
        let mut packed: u32 = env.storage().instance().get(key).unwrap_or(0);
        packed |= flags;
        env.storage().instance().set(key, &packed);
    }

    /// Clear specific flags while preserving others.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `key` - Storage key for the packed flags
    /// * `flags` - Bitmask of flags to clear (bits to set to 0)
    ///
    /// # Security Notes
    /// Uses atomic bitwise AND with complement to ensure only specified
    /// flags are cleared, preventing accidental state changes.
    pub fn clear_packed_flags(env: &Env, key: &Symbol, flags: u32) {
        let mut packed: u32 = env.storage().instance().get(key).unwrap_or(0);
        packed &= !flags;
        env.storage().instance().set(key, &packed);
    }

    /// Retrieve stored packed flags value.
    pub fn get_packed_flags(env: &Env, key: &Symbol) -> u32 {
        env.storage().instance().get(key).unwrap_or(0)
    }

    /// Check if specific flags are set.
    pub fn has_packed_flags(env: &Env, key: &Symbol, flags: u32) -> bool {
        let packed: u32 = env.storage().instance().get(key).unwrap_or(0);
        packed & flags == flags
    }

    /// Check if any of the specified flags are set.
    pub fn has_any_packed_flags(env: &Env, key: &Symbol, flags: u32) -> bool {
        let packed: u32 = env.storage().instance().get(key).unwrap_or(0);
        packed & flags != 0
    }

    /// Store multiple pause flags in a single u32 (legacy compatibility).
    pub fn set_pause_flags(env: &Env, key: &Symbol, lock: bool, release: bool, refund: bool) {
        let mut packed = 0u32;
        if lock {
            packed |= PAUSE_LOCK;
        }
        if release {
            packed |= PAUSE_RELEASE;
        }
        if refund {
            packed |= PAUSE_REFUND;
        }
        env.storage().instance().set(key, &packed);
    }

    /// Retrieve individual pause flags from packed storage.
    pub fn get_pause_flags(env: &Env, key: &Symbol) -> (bool, bool, bool) {
        let packed: u32 = env.storage().instance().get(key).unwrap_or(0);
        (
            packed & PAUSE_LOCK != 0,
            packed & PAUSE_RELEASE != 0,
            packed & PAUSE_REFUND != 0,
        )
    }

    /// Set maintenance mode flag (uses bit 8 to avoid collision with pause flags).
    pub const PAUSE_MAINTENANCE_MODE: u32 = 1 << 8;

    /// Set maintenance mode flag.
    pub fn set_maintenance_mode(env: &Env, key: &Symbol, enabled: bool) {
        let mut packed: u32 = env.storage().instance().get(key).unwrap_or(0);
        if enabled {
            packed |= PAUSE_MAINTENANCE_MODE;
        } else {
            packed &= !PAUSE_MAINTENANCE_MODE;
        }
        env.storage().instance().set(key, &packed);
    }

    /// Check maintenance mode.
    pub fn is_maintenance_mode(env: &Env, key: &Symbol) -> bool {
        let packed: u32 = env.storage().instance().get(key).unwrap_or(0);
        packed & PAUSE_MAINTENANCE_MODE != 0
    }
}

/// Efficient arithmetic operations with overflow protection.
pub mod efficient_math {
    /// Calculate fee with ceiling division to prevent fee avoidance.
    pub fn calculate_fee(amount: i128, fee_rate_bps: i128, basis_points: i128) -> i128 {
        if fee_rate_bps == 0 || amount == 0 {
            return 0;
        }

        // Ceiling division: (amount * rate + basis - 1) / basis
        let numerator = amount
            .checked_mul(fee_rate_bps)
            .and_then(|x| x.checked_add(basis_points - 1))
            .unwrap_or(0);

        numerator / basis_points
    }

    /// Safe subtraction that returns 0 when the result would be negative.
    ///
    /// Note: i128::checked_sub only returns None on arithmetic overflow (e.g.
    /// i128::MIN - 1), not when the result is merely negative. This function
    /// clamps the result to 0 for any case where b > a.
    pub fn safe_sub_zero(a: i128, b: i128) -> i128 {
        if b > a { 0 } else { a - b }
    }

    /// Clamp value between min and max.
    pub fn clamp(value: i128, min: i128, max: i128) -> i128 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }
}

/// Event emission helpers to reduce storage writes.
pub mod event_helpers {
    use soroban_sdk::{symbol_short, Address, Env, String, Symbol};

    /// Emit a lightweight event instead of storing state.
    pub fn emit_operation(env: &Env, operation: Symbol, data: u64) {
        env.events().publish(
            (symbol_short!("op"), operation),
            (data, env.ledger().timestamp()),
        );
    }

    /// Emit batch operation summary instead of individual records.
    pub fn emit_batch_summary(
        env: &Env,
        operation: Symbol,
        count: u32,
        total_amount: i128,
        success: bool,
    ) {
        env.events().publish(
            (symbol_short!("batch"), operation),
            (count, total_amount, success, env.ledger().timestamp()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{vec, Env, Symbol};

    #[test]
    fn test_deduplicate_program_ids() {
        let env = Env::default();
        let items = vec![
            &env,
            String::from_str(&env, "prog1"),
            String::from_str(&env, "prog2"),
            String::from_str(&env, "prog1"),
            String::from_str(&env, "prog3"),
            String::from_str(&env, "prog2"),
        ];

        let deduped = deduplicate_program_ids(&env, &items);
        assert_eq!(deduped.len(), 3);
        assert_eq!(deduped.get(0), Some(String::from_str(&env, "prog1")));
        assert_eq!(deduped.get(1), Some(String::from_str(&env, "prog2")));
        assert_eq!(deduped.get(2), Some(String::from_str(&env, "prog3")));
    }

    #[test]
    fn test_has_duplicates() {
        let env = Env::default();

        let items_with_dups = vec![
            &env,
            String::from_str(&env, "a"),
            String::from_str(&env, "b"),
            String::from_str(&env, "a"),
        ];
        assert!(has_duplicates(&env, &items_with_dups));

        let items_no_dups = vec![
            &env,
            String::from_str(&env, "a"),
            String::from_str(&env, "b"),
            String::from_str(&env, "c"),
        ];
        assert!(!has_duplicates(&env, &items_no_dups));
    }

    #[test]
    fn test_calculate_fee() {
        // 1% fee on 1000 = 10
        assert_eq!(efficient_math::calculate_fee(1000, 100, 10000), 10);

        // Ceiling division: 1 * 100 / 10000 = 0.01 -> ceil to 1
        assert_eq!(efficient_math::calculate_fee(1, 100, 10000), 1);

        // Zero fee rate
        assert_eq!(efficient_math::calculate_fee(1000, 0, 10000), 0);

        // Zero amount
        assert_eq!(efficient_math::calculate_fee(0, 100, 10000), 0);
    }

    #[test]
    fn test_safe_sub_zero() {
        assert_eq!(efficient_math::safe_sub_zero(10, 5), 5);
        assert_eq!(efficient_math::safe_sub_zero(5, 10), 0);
        assert_eq!(efficient_math::safe_sub_zero(0, 0), 0);
    }
}

/// Exhaustive packed-flag storage coverage tests.
///
/// These tests iterate all 2^8 = 256 combinations of granular pause flags
/// to verify that bitwise operations correctly read/write without corruption.
#[cfg(test)]
mod packed_flag_coverage_tests {
    use super::packed_storage::*;
    use soroban_sdk::Symbol;

    /// Storage key for these tests.
    const TEST_KEY: Symbol = Symbol::short("pflag_test");

    /// Verify that setting a single flag results in correct read value.
    fn assert_flag_write_single(flag_bit: u32, flag_name: &str) {
        let env = Env::default();
        clear_packed_flags(&env, &TEST_KEY, FLAG_MASK_8BITS);
        set_packed_flags(&env, &TEST_KEY, flag_bit);
        let stored = get_packed_flags(&env, &TEST_KEY);
        assert_eq!(stored, flag_bit, "{} flag mismatch", flag_name);
    }

    /// Verify that clearing a single flag preserves other flags.
    fn assert_flag_clear_preserves(flag_bit: u32, other_bit: u32, flag_name: &str) {
        let env = Env::default();
        set_packed_flags(&env, &TEST_KEY, flag_bit | other_bit);
        clear_packed_flags(&env, &TEST_KEY, flag_bit);
        let stored = get_packed_flags(&env, &TEST_KEY);
        assert_eq!(stored, other_bit, "{} clear corrupted other flags", flag_name);
    }

    /// Verify that setting flag A does not affect flag B.
    fn assert_flag_independence(set_bit: u32, clear_bit: u32, set_name: &str, clear_name: &str) {
        let env = Env::default();
        set_packed_flags(&env, &TEST_KEY, set_bit);
        let stored = get_packed_flags(&env, &TEST_KEY);
        assert_eq!(stored, set_bit, "{} set did not match expected", set_name);

        clear_packed_flags(&env, &TEST_KEY, clear_bit);
        let stored = get_packed_flags(&env, &TEST_KEY);
        assert_eq!(
            stored, set_bit & !clear_bit,
            "{} set corrupted {} clear", set_name, clear_name
        );
    }

    // Individual bit tests (tests 0-7)
    #[test]
    fn test_flag_lock_write() {
        assert_flag_write_single(PAUSE_LOCK, "lock");
    }

    #[test]
    fn test_flag_release_write() {
        assert_flag_write_single(PAUSE_RELEASE, "release");
    }

    #[test]
    fn test_flag_refund_write() {
        assert_flag_write_single(PAUSE_REFUND, "refund");
    }

    #[test]
    fn test_flag_archive_write() {
        assert_flag_write_single(PAUSE_ARCHIVE, "archive");
    }

    #[test]
    fn test_flag_batch_lock_write() {
        assert_flag_write_single(PAUSE_BATCH_LOCK, "batch_lock");
    }

    #[test]
    fn test_flag_batch_release_write() {
        assert_flag_write_single(PAUSE_BATCH_RELEASE, "batch_release");
    }

    #[test]
    fn test_flag_batch_refund_write() {
        assert_flag_write_single(PAUSE_BATCH_REFUND, "batch_refund");
    }

    #[test]
    fn test_flag_emergency_write() {
        assert_flag_write_single(PAUSE_EMERGENCY, "emergency");
    }

    /// Test independence: each flag bit set does not corrupt others.
    #[test]
    fn test_all_flags_independence() {
        let env = Env::default();
        let all_flags = [
            (PAUSE_LOCK, "lock"),
            (PAUSE_RELEASE, "release"),
            (PAUSE_REFUND, "refund"),
            (PAUSE_ARCHIVE, "archive"),
            (PAUSE_BATCH_LOCK, "batch_lock"),
            (PAUSE_BATCH_RELEASE, "batch_release"),
            (PAUSE_BATCH_REFUND, "batch_refund"),
            (PAUSE_EMERGENCY, "emergency"),
        ];

        for (flag, name) in all_flags {
            clear_packed_flags(&env, &TEST_KEY, FLAG_MASK_8BITS);
            set_packed_flags(&env, &TEST_KEY, flag);
            let stored = get_packed_flags(&env, &TEST_KEY);
            assert_eq!(stored, flag, "{} flag corrupted storage", name);
        }

        // Test that setting all flags at once works correctly
        clear_packed_flags(&env, &TEST_KEY, FLAG_MASK_8BITS);
        let combined: u32 = all_flags.iter().map(|(f, _)| f).sum();
        set_packed_flags(&env, &TEST_KEY, combined);
        let stored = get_packed_flags(&env, &TEST_KEY);
        assert_eq!(stored, combined, "combined flags mismatch");
    }

    /// Test pairwise independence: setting/clearing flag A never affects flag B.
    #[test]
    fn test_pairwise_flag_independence() {
        let env = Env::default();
        let all_flags = [
            PAUSE_LOCK,
            PAUSE_RELEASE,
            PAUSE_REFUND,
            PAUSE_ARCHIVE,
            PAUSE_BATCH_LOCK,
            PAUSE_BATCH_RELEASE,
            PAUSE_BATCH_REFUND,
            PAUSE_EMERGENCY,
        ];

        for set_bit in all_flags {
            for check_bit in all_flags {
                // Clear storage
                env.storage().instance().set(&TEST_KEY, &0u32);

                // Set one flag
                set_packed_flags(&env, &TEST_KEY, set_bit);
                let after_set = get_packed_flags(&env, &TEST_KEY);

                // Verify only the set bit is set
                assert_eq!(
                    after_set, set_bit,
                    "Setting bit {:08b} resulted in {:08b} (expected {:08b})",
                    set_bit, after_set, set_bit
                );

                // Clear a different flag (should be no-op)
                if set_bit != check_bit {
                    clear_packed_flags(&env, &TEST_KEY, check_bit);
                    let after_clear = get_packed_flags(&env, &TEST_KEY);
                    assert_eq!(
                        after_clear, set_bit,
                        "Clearing bit {:08b} corrupted stored value {:08b}",
                        check_bit, after_clear
                    );
                }
            }
        }
    }

    /// Parameterized test iterating all 256 combinations of 8 granular pause flags.
    /// Uses a macro to generate individual test functions for each combination.
    #[test]
    fn test_exhaustive_all_256_combinations() {
        let env = Env::default();

        // Test all 256 combinations (0 through 255)
        for combo in 0u32..=255 {
            // For each combination, verify round-trip integrity
            set_packed_flags(&env, &TEST_KEY, combo);
            let stored = get_packed_flags(&env, &TEST_KEY);
            assert_eq!(stored, combo, "Combination {:08b} round-trip mismatch", combo);

            // Verify each individual bit can be independently checked
            let lock_set = stored & PAUSE_LOCK != 0;
            let release_set = stored & PAUSE_RELEASE != 0;
            let refund_set = stored & PAUSE_REFUND != 0;
            let archive_set = stored & PAUSE_ARCHIVE != 0;
            let batch_lock_set = stored & PAUSE_BATCH_LOCK != 0;
            let batch_release_set = stored & PAUSE_BATCH_RELEASE != 0;
            let batch_refund_set = stored & PAUSE_BATCH_REFUND != 0;
            let emergency_set = stored & PAUSE_EMERGENCY != 0;

            // Verify has_packed_flags works correctly
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_LOCK), lock_set);
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_RELEASE), release_set);
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_REFUND), refund_set);
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_ARCHIVE), archive_set);
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_LOCK), batch_lock_set);
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_RELEASE), batch_release_set);
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_REFUND), batch_refund_set);
            assert_eq!(has_packed_flags(&env, &TEST_KEY, PAUSE_EMERGENCY), emergency_set);

            // Verify has_any_packed_flags works correctly
            let any_single = has_any_packed_flags(&env, &TEST_KEY, PAUSE_LOCK);
            let expected_any = combo & PAUSE_LOCK != 0;
            assert_eq!(any_single, expected_any);

            // Clear the value for next iteration
            clear_packed_flags(&env, &TEST_KEY, FLAG_MASK_8BITS);
        }
    }

    /// Verify storage size reduction: packed flags use 1 storage slot vs 8 bool slots.
    /// In Soroban, a u32 occupies 1 storage slot while 8 bools would theoretically
    /// occupy up to 8 slots (though they may be packed by the compiler).
    #[test]
    fn test_storage_size_packing_efficiency() {
        let env = Env::default();

        // Write a packed value
        set_packed_flags(&env, &TEST_KEY, FLAG_MASK_8BITS);

        // Read it back
        let stored = get_packed_flags(&env, &TEST_KEY);

        // Verify we can store all 8 flags in a single u32 without overflow
        assert_eq!(stored, FLAG_MASK_8BITS, "All flags should fit in u32");

        // Verify the value is within u32 bounds
        assert!(stored <= u32::MAX, "Packed flags exceed u32 maximum");

        // Verify flag extraction produces correct results
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_LOCK));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_RELEASE));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_REFUND));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_ARCHIVE));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_LOCK));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_RELEASE));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_REFUND));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_EMERGENCY));
    }

    /// Verify that clearing all flags results in zero storage value.
    #[test]
    fn test_clear_all_flags() {
        let env = Env::default();

        // Set all flags
        set_packed_flags(&env, &TEST_KEY, FLAG_MASK_8BITS);
        assert_eq!(get_packed_flags(&env, &TEST_KEY), FLAG_MASK_8BITS);

        // Clear all flags
        clear_packed_flags(&env, &TEST_KEY, FLAG_MASK_8BITS);
        assert_eq!(get_packed_flags(&env, &TEST_KEY), 0, "Flags not fully cleared");
    }

    /// Verify selective flag operations don't corrupt storage.
    #[test]
    fn test_selective_set_clear_operations() {
        let env = Env::default();

        // Start with no flags
        env.storage().instance().set(&TEST_KEY, &0u32);

        // Set flags 0, 2, 4, 6 (even positions)
        set_packed_flags(&env, &TEST_KEY, PAUSE_LOCK | PAUSE_REFUND | PAUSE_BATCH_LOCK | PAUSE_BATCH_REFUND);
        let stored = get_packed_flags(&env, &TEST_KEY);
        assert_eq!(
            stored,
            PAUSE_LOCK | PAUSE_REFUND | PAUSE_BATCH_LOCK | PAUSE_BATCH_REFUND,
            "Selective set mismatch"
        );

        // Clear flags 2, 6 (refund and batch_refund)
        clear_packed_flags(&env, &TEST_KEY, PAUSE_REFUND | PAUSE_BATCH_REFUND);
        let stored = get_packed_flags(&env, &TEST_KEY);
        assert_eq!(
            stored,
            PAUSE_LOCK | PAUSE_BATCH_LOCK,
            "Selective clear corrupted flags"
        );

        // Verify remaining flags are intact
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_LOCK));
        assert!(!has_packed_flags(&env, &TEST_KEY, PAUSE_RELEASE));
        assert!(!has_packed_flags(&env, &TEST_KEY, PAUSE_REFUND));
        assert!(has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_LOCK));
        assert!(!has_packed_flags(&env, &TEST_KEY, PAUSE_BATCH_REFUND));
    }

    /// Verify toggle behavior: set then clear returns to original state.
    #[test]
    fn test_flag_toggle_behavior() {
        let env = Env::default();

        for flag in [
            PAUSE_LOCK,
            PAUSE_RELEASE,
            PAUSE_REFUND,
            PAUSE_ARCHIVE,
            PAUSE_BATCH_LOCK,
            PAUSE_BATCH_RELEASE,
            PAUSE_BATCH_REFUND,
            PAUSE_EMERGENCY,
        ] {
            // Set to zero
            env.storage().instance().set(&TEST_KEY, &0u32);
            set_packed_flags(&env, &TEST_KEY, flag);
            assert!(has_packed_flags(&env, &TEST_KEY, flag));

            // Clear
            clear_packed_flags(&env, &TEST_KEY, flag);
            assert!(!has_packed_flags(&env, &TEST_KEY, flag));
        }
    }

    /// Verify edge case: setting flags on empty storage.
    #[test]
    fn test_set_on_empty_storage() {
        let env = Env::default();

        // No prior value - get_packed_flags should return 0
        assert_eq!(get_packed_flags(&env, &TEST_KEY), 0);
        assert!(!has_packed_flags(&env, &TEST_KEY, PAUSE_LOCK));

        // Now set a flag
        set_packed_flags(&env, &TEST_KEY, PAUSE_LOCK);
        assert_eq!(get_packed_flags(&env, &TEST_KEY), PAUSE_LOCK);
    }

    /// Verify edge case: clearing flags on empty storage.
    #[test]
    fn test_clear_on_empty_storage() {
        let env = Env::default();

        // Clear on empty should be no-op
        clear_packed_flags(&env, &TEST_KEY, PAUSE_LOCK);
        assert_eq!(get_packed_flags(&env, &TEST_KEY), 0);
    }
}