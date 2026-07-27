# Shared idempotency-key namespace

The `program-escrow` contract supports two idempotent payout entrypoints:

- [`single_payout_idempotent`]
- [`batch_payout_idempotent`]

Both entrypoints write their consumed keys into a **single, shared storage
namespace** so that a key consumed by one entrypoint is immediately visible
to the other.  This document explains the design and security model.

---

## Storage layout

| Namespace / key                        | Storage tier | Stored type              | Written by                        |
|----------------------------------------|--------------|--------------------------|-----------------------------------|
| `DataKey::IdempotencyKey(String)`      | `instance`   | `IdempotencyRecord`      | Both entrypoints                  |
| `DataKey::PayoutIdempotency(String)`   | `persistent` | `PayoutIdempotencyKey`   | `single_payout_idempotent` (leg.) |
| `PAYOUT_IDEM_KEYS`                     | `persistent` | `Vec<String>`            | `batch_payout_idempotent` (leg.)  |

The **primary** namespace is `DataKey::IdempotencyKey` stored via
[`store_idempotency_record`].  Both `single_payout_idempotent` and
`batch_payout_internal` (the inner function of `batch_payout_idempotent`)
write to this record, and both replay-detection paths check it before
executing any transfer.

The two legacy namespaces (`DataKey::PayoutIdempotency` and
`PAYOUT_IDEM_KEYS`) exist for backwards compatibility and are checked as
fallbacks.

---

## Replay-detection order

### `batch_payout_idempotent`

1. **Shared namespace** (`DataKey::IdempotencyKey`, instance storage).
2. **Legacy single namespace** (`DataKey::PayoutIdempotency`, persistent).
3. **Legacy batch set** (`PAYOUT_IDEM_KEYS`, persistent).

If any check finds the key, a `BatchPayoutReplayedEvent` is emitted and the
function returns the current `ProgramData` **without** touching any token
balance.

### `single_payout_idempotent`

1. **Shared namespace** (`DataKey::IdempotencyKey`, instance storage).
2. **Legacy namespace** (`DataKey::PayoutIdempotency`, persistent).

If any check finds the key, an `IdmReplay` event is emitted and the function
returns without executing.

---

## `is_payout_processed`

This view function checks both the shared and the legacy single namespace:

```rust
pub fn is_payout_processed(env: Env, idempotency_key: String) -> bool
```

Returns `true` if the key is found in either:
- `DataKey::IdempotencyKey` (instance storage)
- `DataKey::PayoutIdempotency` (persistent storage)

A key consumed by either entrypoint will therefore make this function return
`true`.

---

## Security model

### Deterministic rejection

Replay detection runs **before** any state mutation.  This ordering ensures
that even if the caller supplies different parameters on replay (e.g. a
different recipient or amount), the operation is rejected before any storage
is read or written.

### No double-payment

Because cross-entrypoint detection is implemented at the
`batch_payout_idempotent` / `single_payout_idempotent` level (not inside
`batch_payout_internal` / `single_payout_internal`), the replay guard
triggers before `remaining_balance` or recipient balances could change.

### Event-audit trail

| Event                       | Trigger                                       |
|-----------------------------|-----------------------------------------------|
| `BatchPayoutReplayedEvent`  | `batch_payout_idempotent` detects a replay    |
| `IdmReplay`                 | `single_payout_idempotent` detects a replay   |

Both events are published **before** the early return, so they appear in the
ledger even if the operation is a no-op.

### Backwards compatibility

The legacy namespaces are retained so that keys consumed before this shared
namespace was introduced are still recognised.

---

## Testing

The regression tests live in
`contracts/program-escrow/src/tests/cross_entrypoint_idempotency_tests.rs`.
They cover the six scenarios listed in the test file's module-level doc
comment.

```bash
# Run only the cross-entrypoint tests
cargo test -p program-escrow cross_entrypoint_idempotency -- --nocapture
```

[`single_payout_idempotent`]: ../contracts/program-escrow/src/lib.rs
[`batch_payout_idempotent`]: ../contracts/program-escrow/src/lib.rs
[`store_idempotency_record`]: ../contracts/program-escrow/src/lib.rs
