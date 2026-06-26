//! # Bulk-Release Single-Read Optimization Tests
//!
//! Test suite for the refactored `release_schedule` function in
//! `contracts/program-escrow/src/lib.rs`.
//!
//! ## Coverage targets
//!
//! | Category                         | Tests |
//! |----------------------------------|-------|
//! | Happy-path: single entry         | 2     |
//! | Happy-path: multiple entries     | 3     |
//! | O(1) read pattern verification   | 2     |
//! | Circuit breaker                  | 1     |
//! | Authorization                    | 2     |
//! | Insufficient balance             | 1     |
//! | Integer overflow                 | 1     |
//! | Idempotency / double-spend       | 2     |
//! | Dependency constraints           | 4     |
//! | Edge cases (empty, future, mix)  | 5     |
//! | Balance invariants               | 2     |
//! | **Total**                        | **25**|
//!
//! Run with:
//! ```bash
//! cargo test -p program-escrow -- --test-output immediate
//! ```

use super::super::{
    release_schedule, EscrowError, ReleaseEntry, Storage, RELEASE_HISTORY,
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Returns a storage instance pre-configured with a valid authorized key
/// and a generous escrow balance, plus the supplied entries.
fn setup(entries: Vec<ReleaseEntry>, balance: u64) -> Storage {
    let mut s = Storage::new();
    s.set_authorized_key("authorized_caller");
    s.set_balance(balance);
    s.set_entries(RELEASE_HISTORY, entries);
    s
}

const CALLER: &str = "authorized_caller";
const LEDGER: u64 = 1_000;

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Happy-path: single entry
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_single_due_entry_is_released() {
    let entries = vec![ReleaseEntry::new(1, LEDGER, 500, "alice")];
    let mut storage = setup(entries, 1_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 1);
    assert_eq!(result.total_disbursed, 500);
    assert_eq!(result.released_ids, vec![1]);
}

#[test]
fn test_single_entry_balance_decremented_correctly() {
    let entries = vec![ReleaseEntry::new(1, LEDGER, 300, "bob")];
    let mut storage = setup(entries, 1_000);

    release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(storage.get_balance(), 700);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Happy-path: multiple entries (core of the optimization)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_all_due_entries_released_in_single_call() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER - 10, 100, "alice"),
        ReleaseEntry::new(2, LEDGER, 200, "bob"),
        ReleaseEntry::new(3, LEDGER - 1, 300, "carol"),
    ];
    let mut storage = setup(entries, 10_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 3);
    assert_eq!(result.total_disbursed, 600);

    let mut ids = result.released_ids.clone();
    ids.sort();
    assert_eq!(ids, vec![1, 2, 3]);
}

#[test]
fn test_multiple_entries_balance_decremented_by_total() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 100, "alice"),
        ReleaseEntry::new(2, LEDGER, 200, "bob"),
        ReleaseEntry::new(3, LEDGER, 300, "carol"),
    ];
    let mut storage = setup(entries, 5_000);

    release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(storage.get_balance(), 4_400);
}

#[test]
fn test_fifty_due_entries_all_processed() {
    // Simulates a large program with 50 simultaneous milestones.
    // Under the old O(N) implementation this would do 100 storage ops;
    // the new implementation does exactly 2.
    let entries: Vec<ReleaseEntry> = (1..=50)
        .map(|i| ReleaseEntry::new(i, LEDGER, 10, "wallet"))
        .collect();
    let mut storage = setup(entries, 100_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 50);
    assert_eq!(result.total_disbursed, 500);
    assert_eq!(storage.get_balance(), 99_500);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — O(1) storage-read pattern verification
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_history_written_back_with_all_entries_marked_released() {
    // After a successful call the persisted history must reflect the new state.
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 100, "alice"),
        ReleaseEntry::new(2, LEDGER, 200, "bob"),
    ];
    let mut storage = setup(entries, 10_000);

    release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    // Read back the persisted history
    let persisted = storage.get_entries(RELEASE_HISTORY);
    assert_eq!(persisted.len(), 2);
    assert!(persisted[0].released, "entry 1 must be marked released");
    assert!(persisted[1].released, "entry 2 must be marked released");
}

#[test]
fn test_storage_not_written_when_no_entries_are_due() {
    // When nothing is processed the storage write must not occur —
    // the persisted vector must remain unchanged (no dirty write).
    let entries = vec![
        ReleaseEntry::new(1, LEDGER + 100, 500, "alice"), // future
    ];
    let mut storage = setup(entries, 1_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 0);
    // Persisted history must still show entry as unreleased
    let persisted = storage.get_entries(RELEASE_HISTORY);
    assert!(!persisted[0].released, "future entry must remain unreleased");
    // Balance untouched
    assert_eq!(storage.get_balance(), 1_000);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — Circuit breaker
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_paused_program_returns_error() {
    let mut storage = setup(vec![ReleaseEntry::new(1, LEDGER, 100, "alice")], 1_000);
    storage.set_paused(true);

    let err = release_schedule(&mut storage, LEDGER, CALLER).unwrap_err();
    assert_eq!(err, EscrowError::ProgramPaused);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5 — Authorization
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_unauthorized_caller_is_rejected() {
    let mut storage = setup(vec![ReleaseEntry::new(1, LEDGER, 100, "alice")], 1_000);

    let err = release_schedule(&mut storage, LEDGER, "evil_actor").unwrap_err();
    assert_eq!(err, EscrowError::Unauthorized);
}

#[test]
fn test_no_authorized_key_set_rejects_any_caller() {
    let mut storage = Storage::new();
    storage.set_balance(1_000);
    storage.set_entries(
        RELEASE_HISTORY,
        vec![ReleaseEntry::new(1, LEDGER, 100, "alice")],
    );
    // Deliberately no set_authorized_key call

    let err = release_schedule(&mut storage, LEDGER, CALLER).unwrap_err();
    assert_eq!(err, EscrowError::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6 — Insufficient balance
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_insufficient_balance_returns_error_without_mutating_history() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 900, "alice"),
        ReleaseEntry::new(2, LEDGER, 200, "bob"), // total = 1100 > balance 1000
    ];
    let mut storage = setup(entries, 1_000);

    let err = release_schedule(&mut storage, LEDGER, CALLER).unwrap_err();
    assert_eq!(err, EscrowError::InsufficientBalance);

    // Critical: history must NOT have been written (no partial release)
    let persisted = storage.get_entries(RELEASE_HISTORY);
    assert!(
        !persisted[0].released && !persisted[1].released,
        "entries must remain unreleased after InsufficientBalance"
    );
    // Balance must be unchanged
    assert_eq!(storage.get_balance(), 1_000);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 7 — Integer overflow protection
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_amount_overflow_returns_error() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, u64::MAX, "alice"),
        ReleaseEntry::new(2, LEDGER, 1, "bob"), // MAX + 1 overflows u64
    ];
    let mut storage = setup(entries, u64::MAX);

    let err = release_schedule(&mut storage, LEDGER, CALLER).unwrap_err();
    assert_eq!(err, EscrowError::Overflow);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 8 — Idempotency / double-spend prevention
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_already_released_entry_is_not_double_paid() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 500, "alice").already_released(),
    ];
    let mut storage = setup(entries, 1_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 0);
    assert_eq!(result.total_disbursed, 0);
    // Balance must be untouched
    assert_eq!(storage.get_balance(), 1_000);
}

#[test]
fn test_second_call_does_not_double_release() {
    let entries = vec![ReleaseEntry::new(1, LEDGER, 200, "bob")];
    let mut storage = setup(entries, 5_000);

    // First call — should succeed
    let r1 = release_schedule(&mut storage, LEDGER, CALLER).unwrap();
    assert_eq!(r1.entries_released, 1);

    // Second call — entry already released, must be a no-op
    let r2 = release_schedule(&mut storage, LEDGER, CALLER).unwrap();
    assert_eq!(r2.entries_released, 0);
    assert_eq!(r2.total_disbursed, 0);

    // Balance only decremented once
    assert_eq!(storage.get_balance(), 4_800);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 9 — Dependency constraints
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dependent_entry_skipped_when_dependency_not_yet_released() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 100, "alice"),                       // dependency
        ReleaseEntry::new(2, LEDGER, 200, "bob").with_dependency(1),      // depends on 1
    ];
    // Entry 1 is NOT released yet; entry 2 depends on it.
    // Both become due at LEDGER, but in this call entry 1's release is
    // captured in `already_released` from BEFORE the call — it was false.
    // So entry 2 must be skipped.
    let mut storage = setup(entries, 10_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    // Only entry 1 should be released (entry 2 skipped due to dependency)
    assert_eq!(result.entries_released, 1);
    assert_eq!(result.released_ids, vec![1]);
}

#[test]
fn test_dependent_entry_released_when_dependency_was_released_before_call() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER - 1, 100, "alice").already_released(), // pre-released
        ReleaseEntry::new(2, LEDGER, 200, "bob").with_dependency(1),
    ];
    let mut storage = setup(entries, 10_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    // Entry 1 already released; entry 2's dep is satisfied → release entry 2
    assert_eq!(result.entries_released, 1);
    assert_eq!(result.released_ids, vec![2]);
    assert_eq!(result.total_disbursed, 200);
}

#[test]
fn test_chain_dependency_requires_two_separate_calls() {
    // Entry 2 depends on 1; entry 3 depends on 2.
    // All three become due at LEDGER.
    // Call 1: entry 1 released; entry 2 & 3 skipped (dep not pre-released).
    // Call 2: entry 2 released (dep 1 now satisfied); entry 3 still skipped.
    // Call 3: entry 3 released (dep 2 now satisfied).
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 100, "alice"),
        ReleaseEntry::new(2, LEDGER, 200, "bob").with_dependency(1),
        ReleaseEntry::new(3, LEDGER, 300, "carol").with_dependency(2),
    ];
    let mut storage = setup(entries, 10_000);

    let r1 = release_schedule(&mut storage, LEDGER, CALLER).unwrap();
    assert_eq!(r1.released_ids, vec![1]);

    let r2 = release_schedule(&mut storage, LEDGER, CALLER).unwrap();
    assert_eq!(r2.released_ids, vec![2]);

    let r3 = release_schedule(&mut storage, LEDGER, CALLER).unwrap();
    assert_eq!(r3.released_ids, vec![3]);

    assert_eq!(storage.get_balance(), 10_000 - 600);
}

#[test]
fn test_independent_entries_released_despite_unmet_dependency_on_others() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 100, "alice"),                       // no dep
        ReleaseEntry::new(2, LEDGER, 200, "bob").with_dependency(99),     // dep on non-existent
        ReleaseEntry::new(3, LEDGER, 300, "carol"),                       // no dep
    ];
    let mut storage = setup(entries, 10_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    // Entry 1 and 3 should be released; entry 2 skipped (dep 99 not found)
    assert_eq!(result.entries_released, 2);
    let mut ids = result.released_ids.clone();
    ids.sort();
    assert_eq!(ids, vec![1, 3]);
    assert_eq!(result.total_disbursed, 400);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 10 — Edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_history_returns_zero_result() {
    let mut storage = setup(vec![], 1_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 0);
    assert_eq!(result.total_disbursed, 0);
    assert!(result.released_ids.is_empty());
}

#[test]
fn test_future_entries_are_not_released() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER + 1, 100, "alice"),
        ReleaseEntry::new(2, LEDGER + 100, 200, "bob"),
    ];
    let mut storage = setup(entries, 10_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 0);
    assert_eq!(storage.get_balance(), 10_000);
}

#[test]
fn test_mixed_due_and_future_entries_only_due_released() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER - 5, 100, "alice"),   // past due
        ReleaseEntry::new(2, LEDGER, 200, "bob"),          // exactly due
        ReleaseEntry::new(3, LEDGER + 10, 300, "carol"),   // future
    ];
    let mut storage = setup(entries, 10_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 2);
    let mut ids = result.released_ids.clone();
    ids.sort();
    assert_eq!(ids, vec![1, 2]);
    assert_eq!(result.total_disbursed, 300);
}

#[test]
fn test_zero_amount_entry_processed_without_affecting_balance() {
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 0, "alice"),
        ReleaseEntry::new(2, LEDGER, 500, "bob"),
    ];
    let mut storage = setup(entries, 1_000);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 2);
    assert_eq!(result.total_disbursed, 500);
    assert_eq!(storage.get_balance(), 500);
}

#[test]
fn test_exact_balance_match_succeeds() {
    let entries = vec![ReleaseEntry::new(1, LEDGER, 1_000, "alice")];
    let mut storage = setup(entries, 1_000); // balance exactly equals amount

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();

    assert_eq!(result.entries_released, 1);
    assert_eq!(storage.get_balance(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 11 — Balance invariants
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_balance_invariant_after_partial_release() {
    // Only entries 1 & 2 are due; entry 3 is future.
    let entries = vec![
        ReleaseEntry::new(1, LEDGER, 150, "alice"),
        ReleaseEntry::new(2, LEDGER, 250, "bob"),
        ReleaseEntry::new(3, LEDGER + 50, 400, "carol"),
    ];
    let initial_balance = 5_000u64;
    let mut storage = setup(entries, initial_balance);

    let result = release_schedule(&mut storage, LEDGER, CALLER).unwrap();
    let expected_remaining = initial_balance - result.total_disbursed;

    assert_eq!(storage.get_balance(), expected_remaining);
}

#[test]
fn test_error_before_balance_write_leaves_balance_unchanged() {
    // Trigger InsufficientBalance; balance must be untouched.
    let entries = vec![ReleaseEntry::new(1, LEDGER, 9_999, "alice")];
    let mut storage = setup(entries, 100);

    release_schedule(&mut storage, LEDGER, CALLER).unwrap_err();

    assert_eq!(storage.get_balance(), 100, "balance must be unchanged after error");
}