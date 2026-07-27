# Program Escrow — Lifecycle Dwell-Time Metrics

## Overview

This feature extends the on-chain analytics of the Program Escrow contract to
record **per-status transition timestamps** for every program.  Ecosystem
operators can query `get_program_lifecycle_timeline` to see how long each
program spent in `Draft` or `Active` status before transitioning, enabling
detection of stalled or abandoned programs.

## Data Model

### `StatusTransition`

```rust
pub struct StatusTransition {
    pub from_status: ProgramStatus,
    pub to_status: ProgramStatus,
    pub timestamp: u64,
}
```

Each transition records:
- The status being transitioned **from**.
- The status being transitioned **to**.
- The **ledger timestamp** (seconds since Unix epoch) when the transition was
  recorded.

### `ProgramLifecycleTimeline`

```rust
pub struct ProgramLifecycleTimeline {
    pub transitions: Vec<StatusTransition>,
}
```

Stored under the separate storage key `DataKey::LifecycleTimeline(program_id)`,
a timeline is an ordered list of transitions (oldest first).  Because the
timeline lives under its own storage key, adding it **does not** alter the
field ordering of `ProgramData` or `Analytics`, preserving full storage
compatibility.

### Transitions Recorded

| Trigger                               | Transition     |
|---------------------------------------|----------------|
| `init_program` / `initialize_program` | Draft → Draft ¹ |
| `publish_program`                     | Draft → Active |
| `batch_initialize_programs`           | Draft → Draft ¹ |

¹ The initial Draft→Draft entry records the moment the program was created so
  that dwell time in Draft can be computed even if the program is never
  published.

## Query Function

### `get_program_lifecycle_timeline(env, program_id)`

```rust
pub fn get_program_lifecycle_timeline(
    env: Env,
    program_id: String,
) -> Vec<StatusTransition>
```

**Returns** the ordered list of status transitions for the given program.
Returns an empty `Vec` when:
- The program ID does not exist.
- The program was created before this feature was deployed (legacy program
  with no timeline stored).

## Computing Dwell Times

Clients compute dwell time by subtracting consecutive timestamps:

```
dwell_in_draft = transitions[1].timestamp - transitions[0].timestamp
```

Example:
```
StatusTransition { from: Draft, to: Draft,   timestamp: 1000 }   ← created
StatusTransition { from: Draft, to: Active, timestamp: 172_800 } ← published

Dwell in Draft = 172_800 - 1000 = 171_800 seconds (~2 days)
```

## Security Considerations

### Append-Only Semantics
New transitions are **appended** to the end of the vec.  No existing entry is
ever modified or removed.  This ensures that the timeline is an immutable
audit trail.

### No Replay Risk
The `record_status_transition` helper is called **after** the status change
has been persisted to `ProgramData`.  If the transition guard fails (e.g.
publishing an already‑published program), the function panics **before**
reaching the `record_status_transition` call, so no spurious transition is
recorded.

### Deterministic Timestamps
Timestamps come from `env.ledger().timestamp()`, which is part of the
deterministic Soroban host interface.  All validators agree on the same
value, so dwell-time computations are consensus-safe.

### Upgrade Safety
- `ProgramData` field ordering is **unchanged**.
- `Analytics` field ordering is **unchanged**.
- The timeline is stored under a new `DataKey::LifecycleTimeline` variant.
- Legacy programs that predate this feature return an empty vec — they do
  **not** panic.

## Test Coverage

The test suite in `test_lifecycle_dwell_time.rs` covers:

| Scenario                                                     | Assertion                                      |
|--------------------------------------------------------------|------------------------------------------------|
| `init_program` records initial Draft→Draft entry             | Timeline length = 1, timestamp matches ledger  |
| `publish_program` appends Draft→Active entry                 | Timeline length = 2, dwell > 0                 |
| Full lifecycle with non‑zero dwell time                      | dwell_seconds == expected delta                |
| `batch_initialize_programs` (via individual init)            | Each program has its own timeline              |
| Non‑existent / legacy program                                | Empty vec returned                             |
| Double‑publish panics without spurious transition            | Timeline unchanged after panic                 |
| Independent timelines for multiple programs                  | Timelines do not interfere                     |
| `StatusTransition` struct field correctness                  | Field getters return expected values           |

## Gas / Storage Impact

Each transition appends one `StatusTransition` (2 × enum discriminant +
1 × u64 ~ 50 bytes XDR) to the timeline vec.  For the common case (Draft →
Active → terminal), this adds ~150 bytes to persistent storage per program —
negligible compared to the existing `ProgramData` payload.

## Future Extensions

- **Terminal transitions**: `Completed` / `Cancelled` states can be added to
  `ProgramStatus` and wired into `record_status_transition` when those code
  paths are implemented.
- **Caller attribution**: An optional `triggered_by: Address` field could be
  added to `StatusTransition` in a future storage schema version.
- **Time-weighted metrics**: Off-chain indexers can compute average dwell
  times across all programs to surface stalled programs.
