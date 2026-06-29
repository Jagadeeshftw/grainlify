# Payout History Recipient Index

**Issue:** #1384  
**Status:** Implemented  

## Problem

`PayoutRecord` entries were stored only in a sequential list on `ProgramData.payout_history` (instance storage). Recipient-specific lookups required a full scan of all payout records — O(n) in the total number of payouts across all recipients.

## Solution

A lazy-initialized inverted index in **persistent storage**:

```
DataKey::RecipientPayoutIndex(program_id: String, recipient: Address)
    → Vec<PayoutRecord>
```

The key is only created on the **first payout to that recipient**. Programs with no payouts to a given address pay zero cold-storage cost (the key simply does not exist).

## Storage Choice

| Aspect | Decision |
|---|---|
| Storage tier | `persistent` — survives instance TTL eviction |
| Key type | `DataKey::RecipientPayoutIndex(program_id, recipient)` |
| Value type | `Vec<PayoutRecord>` — appended on every payout |
| Init cost | Zero until first payout to each recipient |

## Write Paths

Both payout execution paths now write to the index **after** the token transfer and sequential log update, in the same transaction:

- `single_payout_internal` — calls `append_recipient_index` once
- `batch_payout_internal` — calls `append_recipient_index` once per recipient in the loop

`trigger_program_releases` (scheduled releases) is intentionally out of scope for this issue.

## Read Path

### program-escrow: `query_recipient_history`

```rust
pub fn query_recipient_history(
    env: Env,
    program_id: String,
    recipient: Address,
) -> Vec<PayoutRecord>
```

O(1) lookup — reads `DataKey::RecipientPayoutIndex(program_id, recipient)` from persistent storage. Returns an empty `Vec` for unknown recipients.

### view-facade: `query_recipient_history`

```rust
pub fn query_recipient_history(
    env: Env,
    escrow: Address,
    program_id: String,
    recipient: Address,
) -> Vec<PayoutRecord>
```

Delegates to the program-escrow contract at `escrow` via cross-contract call. The caller is responsible for supplying a registered `ProgramEscrow` address (verifiable via `ViewFacade::get_contract`).

## Complexity

| Operation | Before | After |
|---|---|---|
| Lookup by recipient | O(n) full scan | O(1) index read |
| Write (single payout) | O(1) append to list | O(1) + 1 persistent write |
| Write (batch, k recipients) | O(k) appends | O(k) + k persistent writes |

## Security Notes

- The index is written exclusively by authenticated payout paths — an unprivileged caller cannot inject fake records.
- `query_recipient_history` is read-only; no authorization required.
- The `program_id` parameter in the view-facade call is caller-supplied but cannot forge records: the key written during a payout uses the program_id stored in `ProgramData`, not a caller-controlled value.
- Persistent storage entries for the index are separate from instance storage, so they survive contract upgrades.

## Tests

`contracts/program-escrow/src/recipient_index_tests.rs` — 9 tests:

1. Unknown recipient returns empty (lazy init)
2. `single_payout` writes index
3. Multiple `single_payout` calls accumulate in order
4. `batch_payout` writes index for each recipient
5. Mixed single + batch accumulate correctly
6. `query_recipient_history` agrees with legacy `query_payouts_by_recipient`
7. Index is scoped to `program_id` (no cross-program leakage)
8. Unrelated recipients not affected
9. Timestamps are recorded
