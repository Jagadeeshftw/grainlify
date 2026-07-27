# Program Escrow — Archived Payout History Storage Tiering

**Issue:** Migrate archived program `payout_history` to the persistent storage tier on `archive_program`
**Contract:** `contracts/program-escrow/src/lib.rs`

---

## Problem

`archive_program` and `get_archived_programs` marked a program as archived but left its full
`payout_history: Vec<PayoutRecord>` in the same **instance-storage** entry as active programs.

Instance storage in Soroban is TTL-extended on every contract invocation and contributes to the
per-invocation rent cost.  Archived programs are read-only and rarely accessed, so keeping their
payout history in the hot instance tier was both wasteful and costly:

- Every contract call extended the TTL on potentially large `Vec<PayoutRecord>` data that would
  never change again.
- As the number of archived programs grew, so did the instance-storage footprint — increasing
  rent for all users of the contract.

---

## Solution

On `archive_program`, migrate `payout_history` out of the `ProgramData` instance-storage entry
into Soroban's **persistent-storage** tier, then clear the inline vector.

### Storage key added

```rust
// DataKey enum — contracts/program-escrow/src/lib.rs
ArchivedPayoutHistory(String),   // program_id → Vec<PayoutRecord>  (persistent)
```

### `archive_program` — before

```rust
program_data.archived = true;
program_data.archived_at = Some(env.ledger().timestamp());
env.storage().instance().set(&program_key, &program_data);
// payout_history remains in instance storage untouched
```

### `archive_program` — after

```rust
// 1. Write history to persistent tier (once; guarded against double-archival)
let history_key = DataKey::ArchivedPayoutHistory(program_id.clone());
if !env.storage().persistent().has(&history_key) {
    env.storage().persistent().set(&history_key, &program_data.payout_history);
}

// 2. Clear inline vector to shrink the instance-storage entry
program_data.payout_history = soroban_sdk::Vec::new(&env);

// 3. Mark archived and persist the now-smaller ProgramData
program_data.archived = true;
program_data.archived_at = Some(env.ledger().timestamp());
env.storage().instance().set(&program_key, &program_data);
```

### New read function

```rust
/// Return the full payout history for an archived program.
pub fn get_archived_program_payout_history(
    env: Env,
    program_id: String,
) -> soroban_sdk::Vec<PayoutRecord>
```

Reads directly from `DataKey::ArchivedPayoutHistory` in persistent storage.
Returns an empty `Vec` if the program has not been archived or had no payouts.

---

## Storage tier comparison

| Tier | When to use | Rent model | TTL behaviour |
|------|------------|-----------|---------------|
| **Instance** | Hot data accessed on every call | Extended on every invocation | Resets on each call |
| **Persistent** | Cold / archival / index data | Extended explicitly or via `bump` | Decays unless bumped |

Archived payout histories are by definition read-only and rarely accessed, making persistent
storage the correct home for them.

---

## Impact on existing query paths

### `query_recipient_history`

Unchanged. The per-recipient inverted index (`DataKey::RecipientPayoutIndex`) was already in
persistent storage **before** this change.  It is written by `append_recipient_index` on every
payout and is unaffected by archival.  Callers of `query_recipient_history` continue to work
transparently for archived programs.

### `get_program_info_v2` / `ProgramData.payout_history`

After archival this field returns an empty `Vec`.  Consumers that need the full history of an
archived program should call `get_archived_program_payout_history` instead.

### `get_program_aggregate_stats`

This function reads from the legacy singleton `PROGRAM_DATA` key and its `payout_history` field.
If the singleton program is archived, `payout_history` will be empty post-migration.  The
persistent tier holds the authoritative history via `get_archived_program_payout_history`.

---

## Double-archival safety

`archive_program` guards the migration write with:

```rust
if !env.storage().persistent().has(&history_key) {
    env.storage().persistent().set(&history_key, &program_data.payout_history);
}
```

Calling `archive_program` a second time will not overwrite the already-migrated history with the
now-empty inline `Vec`.

---

## Security notes

- **Admin-only:** `archive_program` continues to require admin authorisation via `require_admin`.
- **Immutability:** The persistent history key is written exactly once and never mutated after
  archival. There is no code path that modifies `ArchivedPayoutHistory` post-write.
- **No trust escalation:** `get_archived_program_payout_history` is read-only and requires no
  authorisation — consistent with the existing `query_recipient_history` design (payout records
  are public on-chain data).
- **No data loss:** The migration copies the full `Vec<PayoutRecord>` to persistent storage
  before clearing the inline field. Tests confirm byte-for-byte equivalence.

---

## Tests

Tests live in `contracts/program-escrow/src/test_archival.rs`.

| Test | Description |
|------|-------------|
| `test_program_archival_success` | Basic archive flag and registry |
| `test_archive_migrates_payout_history_to_persistent_storage` | History present in persistent tier, cleared in instance tier |
| `test_get_archived_program_payout_history_returns_correct_records` | Query function returns correct records |
| `test_query_recipient_history_works_for_archived_program` | Per-recipient index unaffected by archival |
| `test_double_archival_is_idempotent_and_preserves_history` | Second `archive_program` call does not erase history |
| `test_archive_program_with_no_payouts` | Zero-payout programs archive cleanly |
| `test_archive_requires_admin` | Non-admin call panics |
| `test_archive_non_existent_program` | Missing program panics with `"Program not found"` |
| `test_instance_storage_footprint_shrinks_after_archival` | N archived programs have empty `payout_history` in instance storage; persistent tier holds all records |
| `test_program_archival_filtering` | `list_programs` filters archived entries |
| `test_no_data_loss_after_archival` | Total amount and per-recipient counts identical before and after archival |

Run with:

```sh
cargo test -p program-escrow
```

---

## Files changed

| File | Change |
|------|--------|
| `contracts/program-escrow/src/lib.rs` | Added `DataKey::ArchivedPayoutHistory`; updated `archive_program` to migrate history; added `get_archived_program_payout_history` |
| `contracts/program-escrow/src/test_archival.rs` | Comprehensive tests for migration, data integrity, and footprint |
| `docs/program-escrow-archival-storage-tiering.md` | This document |
