# Recipient Payout Index Consistency

## Problem

`query_payouts_by_recipient` and `get_payouts_by_recipient` scan the entire `payout_history` vector with a per-record filter (`record.recipient == target`). This is O(n) in the total number of payouts across all recipients, degrading as history grows.

## Solution: Lazy Inverted Index

A new `DataKey::RecipientPayoutIndex(String, Address)` → `Vec<PayoutRecord>` entry in persistent storage:

- Keyed by `(program_id, recipient)` so each recipient has its own compact list
- Written by `append_recipient_index()` called inside `single_payout_internal` and `batch_payout_internal`
- Lazily initialized — no storage entry exists until the first payout to that recipient
- Read by `query_recipient_history(env, program_id, recipient)` which is O(1)

## Consistency Guarantees

| Property | Mechanism |
|---|---|
| At-most-once per payout | Index written inside the same atomic execution as the payout transfer; a replay via idempotency key returns early with no duplicate append |
| Insertion order | `append_recipient_index` pushes to the back of the Vec; both `single_payout_internal` and `batch_payout_internal` execute payouts in chronological order |
| Cross-program isolation | Key includes `program_id`, so payouts in program A never leak into program B's index |
| Parity with legacy query | `query_recipient_history` returns the exact same records as `query_payouts_by_recipient` filtered to that recipient, confirmed by test `test_index_matches_legacy_filtered_query` |

## Test Coverage

All tests live in `contracts/program-escrow/src/recipient_index_tests.rs`:

1. **`test_unknown_recipient_returns_empty`** — lazy init, no panic
2. **`test_single_payout_writes_index`** — basic single payout
3. **`test_single_payout_accumulates_in_order`** — multiple singles, same recipient
4. **`test_batch_payout_writes_index_for_each_recipient`** — batch, multiple recipients
5. **`test_single_and_batch_payout_accumulate`** — interleaved single + batch
6. **`test_index_matches_legacy_filtered_query`** — parity with `query_payouts_by_recipient`
7. **`test_index_scoped_to_program_id`** — no cross-program leakage
8. **`test_unrelated_recipient_index_stays_empty`** — unrelated recipient unaffected
9. **`test_index_records_timestamp`** — timestamp captured in index entry
10. **`test_idempotent_replay_does_not_duplicate_index`** — idempotent replay safety
11. **`test_batch_idempotent_replay_does_not_duplicate_index`** — batch idempotent replay safety
12. **`test_index_returns_all_records_beyond_limit`** — index returns full history beyond pagination window
13. **`test_interleaved_payouts_across_recipients`** — complex interleaved payout sequences
14. **`test_multiple_batches_to_same_recipient`** — repeated batch payouts to same recipient
15. **`test_all_entrypoints_interleaved_to_same_recipient`** — all four entrypoints (single, batch, idem-single, idem-batch) interleaved to the same recipient, with idempotent replay verification

## Facade Proxy

`EscrowViewFacade::query_recipient_history` in `contracts/escrow-view-facade/` exposes the read to off-chain consumers:

```rust
pub fn query_recipient_history(
    env: Env,
    program_contract: Address,
    program_id: String,
    recipient: Address,
) -> Vec<PayoutRecord>
```

This follows the existing facade pattern: it returns an empty `Vec` on cross-contract error rather than trapping, making it safe for frontend consumption.
