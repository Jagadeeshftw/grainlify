# `archive_program` — Interaction with Pending Release Schedules

**Issue:** [#1493](https://github.com/Jagadeeshftw/grainlify/issues/1493)  
**Contract:** `contracts/program-escrow/src/lib.rs`  
**Error code:** `ContractError::CannotArchiveWithPendingOps = 106`

---

## Problem Statement

`archive_program` marks a program as historical and read-only.  
`get_program_release_schedules` / `trigger_program_releases` manage timed fund
disbursements to recipients.

Before this change, calling `archive_program` on a program that still had
unreleased entries in `get_program_release_schedules` would silently succeed —
leaving those scheduled funds permanently stranded.  Once archived, the
contract returns an empty schedule list to `trigger_program_releases`, so those
amounts can never be released.

---

## Chosen Behavior: **Block archival** when pending schedules exist

The implementation adds a guard at the start of `archive_program` that
inspects the `SCHEDULES` storage key.  If any schedule has `released == false`
the call panics with:

```
"Cannot archive program with pending release schedules"
```

This maps to `ContractError::CannotArchiveWithPendingOps` (error code `106`).

### Why block instead of allow?

| Option | Pros | Cons |
|--------|------|------|
| **Block archival (chosen)** | No funds stranded. Explicit caller intent required. Secure by default. | Caller must drain schedules first. |
| Allow archival, honor post-archival triggers | Flexible | Requires `trigger_program_releases` to work on archived programs — creates contradictory state ("archived but still active"). |
| Allow archival, silently orphan schedules | Simpler | **Funds stuck forever.** Unacceptable for a financial contract. |

Blocking is the safest choice and aligns with the principle of *least
surprise*: a program that is "archived" should be fully settled.

---

## Required Caller Flow

Before calling `archive_program`, callers must ensure all release schedules are
in state `released == true`.  The standard flow is:

```
1.  create_program_release_schedule(recipient, amount, timestamp)
    …(wait for timestamp to pass)…
2.  trigger_program_releases()   ← marks each due schedule as released=true
3.  get_release_schedules()      ← verify all have released == true
4.  archive_program(program_id)  ← now succeeds
```

If a schedule was created by mistake and funds should be reclaimed without
releasing them, an admin can use the program's refund / emergency-withdraw
path to remove the reserved balance, but the schedule record itself must be
cleared (or a future `cancel_schedule` entrypoint, if implemented) before
archival is permitted.

---

## Guard Implementation

```rust
// In archive_program (lib.rs):
let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
    .storage()
    .instance()
    .get(&SCHEDULES)
    .unwrap_or_else(|| Vec::new(&env));

let has_pending = schedules.iter().any(|s| !s.released);
if has_pending {
    panic!("Cannot archive program with pending release schedules");
}
```

Key properties of this guard:

- **Timestamp-independent**: a schedule is considered pending as long as
  `released == false`, regardless of whether its `release_timestamp` is in the
  past or the future.  A past-due but un-triggered schedule is still pending.
- **Zero-overhead baseline**: programs with no schedules (`SCHEDULES` key
  absent) return an empty Vec and the guard passes immediately.
- **Idempotent**: archiving an already-archived program is a no-op (the history
  migration guard prevents data overwrite); the pending-schedule guard is also
  re-evaluated on the second call (there are no schedules on an already-archived
  program, so it passes).

---

## Test Coverage (test_archival.rs)

Seven new tests cover all edge cases for this behavior:

| Test | Purpose |
|------|---------|
| `test_archive_blocked_by_future_pending_schedule` | Future-dated pending schedule → archival panics |
| `test_archive_blocked_by_past_due_untriggered_schedule` | Past-due but un-triggered schedule → archival panics |
| `test_archive_succeeds_after_all_schedules_released` | All schedules triggered → archival succeeds |
| `test_archive_blocked_when_mixed_released_and_pending` | Mix of released + pending → archival panics |
| `test_archive_succeeds_with_zero_schedules` | No schedules at all → no regression, archival succeeds |
| `test_archive_blocked_after_partial_trigger` | Only some schedules triggered → archival still panics |
| `test_archive_succeeds_after_all_multi_schedules_released` | All schedules in a multi-schedule program released → archival succeeds, history preserved |

These tests complement the pre-existing archival tests (Tests 1–10) which cover
payout history migration, admin-only access, idempotency, and storage footprint.

---

## Security Notes

- **No silent fund loss**: the guard ensures funds allocated to pending
  schedules are explicitly settled before a program is made read-only.
- **Error code stability**: `CannotArchiveWithPendingOps = 106` is a stable
  error code in the `ContractError` enum.  SDK clients should handle this code
  and instruct users to trigger remaining schedules.
- **No cross-program leakage**: the `SCHEDULES` storage key is program-scoped
  via the per-program storage namespace; the guard only inspects the calling
  program's schedules.

---

## Inverse Case

A program with zero pending release schedules (either no schedules were ever
created, or all have been released) archives without any special handling —
confirming no regression in the common archival path.  This is verified by
`test_archive_succeeds_with_zero_schedules`.
