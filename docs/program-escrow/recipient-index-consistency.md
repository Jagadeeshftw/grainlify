# Recipient Index Consistency

## Overview

Payout records are stored as a flat `Vec<PayoutRecord>` inside `ProgramData.payout_history`. There is **no separate inverted index** mapping recipients to their records. When `query_payouts_by_recipient` (or its alias `query_recipient_history`) is called, it performs an O(n) linear scan over the entire `payout_history` vector and returns records matching the requested recipient.

This design is deliberately simple and auditable: the ground truth is a single append-only vector. There is no secondary index that could desynchronize.

## Lazy Index Population Triggers

The `payout_history` vector is the "lazy recipient index" — it is populated **only** when a payout actually executes:

### Append paths (add a `PayoutRecord`)

| Function | File | Line | Condition |
|---|---|---|---|
| `single_payout_internal` | `lib.rs` | ~6063 | Amount validated, transfer executed, no duplicate idempotency key |
| `batch_payout_internal` | `lib.rs` | ~5710 | Per-recipient loop after each transfer |
| `trigger_scheduled_releases` | `lib.rs` | ~6467 | Per-released schedule |
| `payout_splits` | `payout_splits.rs` | ~325 | Per-split payout |

### Non-append paths (no `PayoutRecord` added)

| Code path | Reason |
|---|---|
| `single_payout_idempotent` / `batch_payout_idempotent` key replay | Returns stored state early; no re-execution |
| `single_payout_internal` / `batch_payout_internal` with existing idempotency key | `handle_idempotency` returns existing record before transfer |
| Failed payout (insufficient balance, paused, dispute, circuit breaker) | Validation panics before `push_back` |
| Input validation failure (zero amount, invalid recipient, pagination error) | Panics before any state mutation |

### Key invariant
**Exactly one `PayoutRecord` is appended per successful payout execution.** Idempotent replays, failed/aborted transactions, and validation errors never add records. This means the total `payout_history.len()` equals the count of unique non-replayed payout operations.

## Guarding Against Index Drift

Because there is no secondary index, the dominant consistency risk is code changes that:

1. Add a new payout entrypoint without appending to `payout_history`
2. Append to `payout_history` in an existing path that was previously a no-op
3. Modify `paginate_filtered` or the query predicates

### Checklist for new payout paths

When adding a new payout entrypoint (e.g. `delegated_payout` or `scheduled_payout_v2`):

- [ ] Does the function append a `PayoutRecord` to `program_data.payout_history`?
- [ ] Is the append **after** the token transfer (or atomic with it)?
- [ ] Does an idempotency check exist, and does it skip the append on replay?
- [ ] Does `query_payouts_by_recipient` / `query_recipient_history` return the new records?
- [ ] Have the tests in `recipient_index_tests.rs` been updated to include the new path?

## Test Coverage

The test suite in `recipient_index_tests.rs` verifies:

| Test | Checks |
|---|---|
| `test_interleaved_payouts_same_recipient` | All 4 payout types interleaved; count matches after each call; idempotent replay does not add |
| `test_idempotent_replay_no_duplicate` | Both `single_payout_idempotent` and `batch_payout_idempotent` replay guards |
| `test_payout_history_chronological_order` | Records appear in FIFO insertion order |
| `test_pagination_consistency_after_interleaved` | Paginated scan matches full unfiltered scan |
| `test_query_equals_get` | `query_payouts_by_recipient` matches `get_payouts_by_recipient` |
| `test_multi_program_isolation` | Separate contract instances produce separate histories |
| `test_unknown_recipient_returns_empty` | Querying an address with no payouts returns empty |
| `test_batch_payout_multiple_recipients_query_isolation` | Single batch correctly attributes to each recipient |
| `test_query_pagination_edge_cases_with_recipient_history` | Offset-beyond-end returns empty; partial last page works |

### Facade proxy test

The `escrow-view-facade` contract provides `query_recipient_history` which proxies to
`program-escrow::query_payouts_by_recipient`. Because the proxy is a thin pass-through
with no transformation or caching, it introduces no additional drift. The cross-contract
test is in `contracts/escrow-view-facade/src/`.

## Security Properties

- **Append-only**: `payout_history` only grows; records are never removed or modified
- **Atomic per transaction**: The `push_back` and `remaining_balance` update happen in the same `set` call
- **Idempotent-safe**: Replayed keys do not append, preventing history inflation
- **Deterministic ordering**: Records are in chronological push-back order (FIFO)
