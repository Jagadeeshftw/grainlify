# Program Escrow Event Ordering Guarantee

## Overview

Soroban executes contract host functions sequentially within a transaction. Each call to `env.events().publish()` appends an event to the transaction's ordered event log. This means event topic order is **deterministic and matches the exact call order** of the invoking functions.

Off-chain indexers rely on this ordering to reconstruct an accurate, ordered activity feed for a program.

## Payout Event Topics

| Function | Event Topic | Struct |
|---|---|---|
| `single_payout` / `single_payout_by` | `Payout` | `PayoutEvent` |
| `batch_payout` / `batch_payout_by` | `BatchPay` | `BatchPayoutEvent` |

## Pause State-Change Event Topics

| Function | Event Topics | Struct |
|---|---|---|
| `set_paused` | `PauseSt` (v1 legacy) | `PauseStateChanged` |
| | `PauseStV2` (v2) | `PauseStateChangedV2` |

Each call to `set_paused` emits **two events per mode toggled**: a legacy `PauseSt` event followed by a `PauseStV2` event. If multiple modes are toggled in a single call (e.g., `lock=true, release=true`), four events are emitted in lock-then-release order.

## Ordering Guarantee

Within a single Soroban transaction:

1. Events are appended to the log in the exact order the host function `publish()` is called.
2. There is no reordering, batching, or deferred emission.
3. When `single_payout` and `batch_payout` (or their `_by` variants) are invoked in sequence, the resulting `Payout` / `BatchPay` events appear in the precise call order.
4. Pause state-change events emitted by `set_paused` between payout calls are interleaved at their exact call position.

### Example

```
single_payout(r1, 1000)       → emits Payout
set_paused(lock=true)         → emits PauseSt, PauseStV2
batch_payout([r2], [2000])    → emits BatchPay
set_paused(lock=false)        → emits PauseSt, PauseStV2
```

Event log (in order):
1. `Payout`
2. `PauseSt` (lock)
3. `PauseStV2` (lock)
4. `BatchPay`
5. `PauseSt` (lock unpause)
6. `PauseStV2` (lock unpause)

## Implications for Indexers

- Events can be safely ordered by their position in the transaction event log.
- No secondary sort key (e.g., timestamp) is needed for within-transaction ordering.
- The `version` field (currently `2`) on payout and pause V2 events enables schema evolution without breaking ordering assumptions.

## Test Coverage

The deterministic event ordering for interleaved payout and pause calls is verified in:

- `contracts/program-escrow/src/test_event_ordering.rs`

Tests cover:
- Single-then-batch and batch-then-single ordering
- Four-call alternating sequence (single, batch, single, batch)
- Pause events interleaved between payouts
- Multi-mode pause (lock + release) between payouts
- Cross-run determinism (identical topic order across independent runs)
- V2 version tag presence on all payout events
