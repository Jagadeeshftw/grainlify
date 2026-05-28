# Idempotency Keys for Batch Payouts

## Overview

The `program-escrow` contract supports optional idempotency keys on batch payout
operations. An idempotency key is a caller-supplied string that uniquely identifies
a payout batch. The contract stores the key on-chain and rejects any subsequent
submission of the same key while it is still valid, preventing accidental double-pays.

## How It Works

### Submitting a payout with a key

Use `batch_payout_with_key` instead of `batch_payout_by`:

```
batch_payout_with_key(caller, idempotency_key, recipients, amounts)
```

On the first call the contract:
1. Checks that the key does not already exist in storage.
2. Stores a `PayoutIdempotencyKey` record with `expires_at = current_ledger + 100_000`.
3. Executes the payout normally.

### Rejection behaviour

| Scenario | Error |
|---|---|
| Key exists and `current_ledger <= expires_at` | `DuplicateIdempotencyKey` (1100) |
| Key exists and `current_ledger > expires_at` | `ExpiredIdempotencyKey` (1101) |

The two error codes are intentionally distinct so callers can tell whether a
rejection is a true duplicate (safe to discard) or an expired re-use (may need
investigation).

### TTL

Keys expire after **100,000 ledgers** (~7 days at 5 s/ledger). The constant is
exported as `IDEMPOTENCY_KEY_TTL_LEDGERS`.

## Storage Layout

| Key | Type | Description |
|---|---|---|
| `IdempotencyKey(program_id, key)` | `PayoutIdempotencyKey` | Per-key record |
| `IdempotencyKeyIndex(program_id)` | `Vec<String>` | Ordered list of keys for pruning |

Both use **instance storage** (same lifetime as the contract instance).

## Pruning Stale Keys

High-throughput programs accumulate expired key records over time. The admin can
call `prune_idempotency_keys` to reclaim storage:

```
prune_idempotency_keys(program_id, max_prune) -> u32
```

- `max_prune` caps the number of deletions per transaction to bound compute cost.
- Returns the number of keys actually removed.
- Only removes keys whose `expires_at` ledger has already passed.
- Emits an `IdemPrn` event with the count of pruned keys.

### Recommended pruning cadence

Call `prune_idempotency_keys` periodically (e.g., once per day) with a
`max_prune` of 50–200 depending on throughput. Multiple calls are safe and
idempotent.

## Security Notes

- Idempotency keys are **optional**. Callers using `batch_payout` or
  `batch_payout_by` are unaffected.
- The key check runs **after authorization**, so an unauthorized caller cannot
  probe the key store.
- Expired keys are rejected with a distinct error to prevent silent re-use after
  TTL rollover.
- The prune function is **admin-only** to prevent griefing.

## Error Codes

| Code | Name | Meaning |
|---|---|---|
| 1100 | `DuplicateIdempotencyKey` | Key already used and still within TTL |
| 1101 | `ExpiredIdempotencyKey` | Key was used before but TTL has passed |
