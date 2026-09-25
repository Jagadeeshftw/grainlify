# Batch Lock and Batch Release — `bounty_escrow/escrow`

> **Module**: `bounty_escrow/contracts/escrow/src/lib.rs`  
> **Test file**: `bounty_escrow/contracts/escrow/src/test_batch_failure_modes.rs`  
> **Soroban SDK**: Stellar smart contract platform

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture & Design Principles](#architecture--design-principles)
3. [Data Types](#data-types)
4. [Public Interface](#public-interface)
   - [`batch_lock_funds`](#batch_lock_funds)
   - [`batch_release_funds`](#batch_release_funds)
5. [Ordering Guarantee](#ordering-guarantee)
6. [Fee Handling in Batch Operations](#fee-handling-in-batch-operations)
7. [CEI Pattern (Checks–Effects–Interactions)](#cei-pattern-checkseffectsinteractions)
8. [Reentrancy Protection](#reentrancy-protection)
9. [Events](#events)
10. [Error Reference](#error-reference)
11. [Security Assumptions & Threat Model](#security-assumptions--threat-model)
12. [Test Coverage Summary](#test-coverage-summary)
13. [Integration Guide](#integration-guide)
14. [Limitations & Known Constraints](#limitations--known-constraints)

---

## Overview

The batch operations surface area provides two contract entry points that allow an admin or depositor to lock or release funds for multiple independent escrows in a **single Soroban transaction**. This reduces on-chain overhead, simplifies client-side orchestration, and provides stronger atomicity guarantees than calling the single-item variants in a loop.

```
┌──────────────────────────────────────────────────────────────┐
│  batch_lock_funds([item₁, item₂, …, itemₙ])                 │
│                                                              │
│  Validation pass (all items)                                 │
│  ├─ participant filter check (each depositor)                │
│  ├─ duplicate bounty_id detection                            │
│  ├─ amount > 0 check                                         │
│  └─ bounty does not already exist                            │
│                                                              │
│  Sort items by ascending bounty_id                           │
│                                                              │
│  EFFECTS pass (write all escrow records + indices)           │
│                                                              │
│  INTERACTIONS pass (token transfers + events)                │
│                                                              │
│  Emit BatchFundsLocked                                       │
└──────────────────────────────────────────────────────────────┘
```

---

## Architecture & Design Principles

### Atomicity

Both batch operations follow an **all-or-nothing** model: if any item in the batch fails validation, the entire transaction is aborted and **no state changes** are committed. Soroban's transaction model ensures this at the VM level — a panic or returned error rolls back the entire ledger footprint modification.

### CEI (Checks–Effects–Interactions)

Both functions strictly adhere to the **Checks → Effects → Interactions** pattern to prevent reentrancy exploits:

1. **Checks** — all validation logic runs first before any state is written.
2. **Effects** — all storage writes happen in a dedicated loop before any external calls.
3. **Interactions** — token transfers (external calls) execute only after all state is finalized.

### Deterministic Ordering

All batch items are reordered by ascending `bounty_id` before processing. This eliminates front-running via input reordering and ensures the canonical on-chain execution order is always predictable.

### Shared Reentrancy Guard

Both functions acquire the shared reentrancy guard (`reentrancy_guard::acquire`) at entry and release it (`reentrancy_guard::release`) at exit. This prevents any reentrant call from another contract invoking these functions while a batch is in flight.

---

## Data Types

### `LockFundsItem`

Represents a single escrow to lock within a batch.

```rust
pub struct LockFundsItem {
    /// Unique identifier for the bounty. Must not already exist in storage.
    pub bounty_id: u64,

    /// Address of the depositor. Funds are transferred from this address.
    /// `require_auth()` is called once per unique depositor address.
    pub depositor: Address,

    /// Gross amount to lock (in token base units / stroops).
    /// Must be > 0. Subject to the configured AmountPolicy (min/max).
    pub amount: i128,

    /// Unix timestamp (seconds) after which the escrow is eligible for refund
    /// without admin approval.
    pub deadline: u64,
}
```

### `ReleaseFundsItem`

Represents a single escrow to release within a batch.

```rust
pub struct ReleaseFundsItem {
    /// ID of the bounty to release. Must exist and be in `Locked` status.
    pub bounty_id: u64,

    /// Address that will receive the released funds.
    pub contributor: Address,
}
```

---

## Public Interface

### `batch_lock_funds`

```rust
pub fn batch_lock_funds(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error>
```

Locks funds for multiple bounties in a single atomic transaction.

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `items` | `Vec<LockFundsItem>` | 1–`MAX_BATCH_SIZE` lock requests. Processed in ascending `bounty_id` order. |

**Returns**

`Ok(u32)` — the number of bounties successfully locked (always equals `items.len()` on success, since the operation is atomic).

**Pre-conditions (all must hold or the call panics / returns `Err`)**

| Condition | Error on failure |
|-----------|-----------------|
| Contract has been initialized via `init` | `NotInitialized` |
| Lock operation is not paused | `FundsPaused` |
| Contract is not deprecated | `ContractDeprecated` |
| `1 ≤ items.len() ≤ MAX_BATCH_SIZE` | `InvalidBatchSize` |
| No `bounty_id` is duplicated within the batch | `DuplicateBountyId` |
| No `bounty_id` already exists in persistent storage | `BountyExists` |
| All amounts are `> 0` | `InvalidAmount` |
| Each depositor passes participant filter (blocklist / allowlist) | `ParticipantBlocked` / `ParticipantNotAllowed` |

**Post-conditions (guaranteed on `Ok`)**

- An `Escrow` record exists in persistent storage for every `bounty_id` in `items`.
- Each escrow has `status = EscrowStatus::Locked`.
- `remaining_amount == amount` for every escrow.
- The EscrowIndex and DepositorIndex are updated for every item.
- Token funds equal to `item.amount` have been transferred from each depositor to the contract.
- One `FundsLocked` event per item has been emitted.
- One `BatchFundsLocked` event has been emitted with aggregate totals.

**Example (off-chain SDK call)**

```typescript
const items = [
  { bounty_id: 1n, depositor: alice, amount: 5_000_000n, deadline: futureTs },
  { bounty_id: 2n, depositor: bob,   amount: 2_500_000n, deadline: futureTs },
];
const lockedCount = await contract.batch_lock_funds({ items });
// lockedCount === 2
```

---

### `batch_release_funds`

```rust
pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error>
```

Releases funds from multiple locked escrows to their respective contributors in a single atomic transaction. Only the admin may call this function.

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `items` | `Vec<ReleaseFundsItem>` | 1–`MAX_BATCH_SIZE` release requests. Processed in ascending `bounty_id` order. |

**Returns**

`Ok(u32)` — the number of bounties successfully released.

**Pre-conditions**

| Condition | Error on failure |
|-----------|-----------------|
| Release operation is not paused | `FundsPaused` |
| Contract is initialized | `NotInitialized` |
| `1 ≤ items.len() ≤ MAX_BATCH_SIZE` | `InvalidBatchSize` |
| Caller is the admin (`require_auth`) | `Unauthorized` (auth failure) |
| No `bounty_id` duplicated within the batch | `DuplicateBountyId` |
| Every `bounty_id` exists in persistent storage | `BountyNotFound` |
| Every escrow has `status == EscrowStatus::Locked` | `FundsNotLocked` |

**Post-conditions (guaranteed on `Ok`)**

- Every referenced escrow has `status = EscrowStatus::Released`.
- `remaining_amount == 0` for every released escrow.
- Token funds equal to each escrow's `amount` have been transferred to the respective `contributor`.
- One `FundsReleased` event per item has been emitted.
- One `BatchFundsReleased` event has been emitted with aggregate totals.

**Example (off-chain SDK call)**

```typescript
const items = [
  { bounty_id: 1n, contributor: alice },
  { bounty_id: 2n, contributor: bob },
];
const releasedCount = await contract.batch_release_funds({ items });
// releasedCount === 2
```

---

## Ordering Guarantee

Both functions internally sort the input vector by `bounty_id` in ascending order before processing. The sort uses a simple insertion-sort-style pass (`order_batch_lock_items` / `order_batch_release_items`) that is `O(n²)` but acceptable given `MAX_BATCH_SIZE = 20`.

**Why this matters:**

- **Front-running mitigation**: A caller cannot influence execution order by reordering the input slice.
- **Deterministic events**: On-chain event streams always appear in canonical ID order, simplifying indexers.
- **Lock ordering**: Consistent ID ordering prevents deadlocks in any future multi-contract scenarios.

The caller's input order is irrelevant — items are always executed in ascending `bounty_id` order regardless.

---

## Fee Handling in Batch Operations

`batch_lock_funds` currently does **not** deduct lock fees per item (unlike the single-item `lock_funds` path which applies `resolve_fee_config` and ceiling-division fee deduction). The stored `amount` equals the gross deposited amount. This is an intentional design trade-off for batch efficiency — if per-item fees are required, use the single-item `lock_funds` path or configure fees to zero for batch flows.

`batch_release_funds` similarly does not apply the release fee rate. The full `escrow.amount` is transferred to the contributor.

> **Note for integrators**: If your deployment relies on fee collection, be aware that batch paths bypass fee logic. Review your fee configuration accordingly.

---

## CEI Pattern (Checks–Effects–Interactions)

The implementations follow the industry-standard CEI ordering to prevent reentrancy attacks:

### `batch_lock_funds` CEI breakdown

```
CHECKS ────────────────────────────────────────────────────────
  reentrancy_guard::acquire()
  check paused / deprecated
  validate batch size (1..=MAX_BATCH_SIZE)
  for each item:
    check_participant_filter(depositor)
    assert bounty_id not in storage
    assert amount > 0
    assert no duplicate bounty_ids in batch

EFFECTS ───────────────────────────────────────────────────────
  for each item (sorted by bounty_id asc):
    write Escrow { status: Locked, ... } to persistent storage
    append to EscrowIndex
    append to DepositorIndex(depositor)

INTERACTIONS ──────────────────────────────────────────────────
  for each item (sorted):
    token.transfer(depositor → contract, amount)
    emit FundsLocked(...)
  emit BatchFundsLocked(count, total_amount, timestamp)
  reentrancy_guard::release()
```

### `batch_release_funds` CEI breakdown

```
CHECKS ────────────────────────────────────────────────────────
  check paused
  reentrancy_guard::acquire()
  validate batch size
  admin.require_auth()
  for each item:
    assert bounty_id exists
    assert escrow.status == Locked
    assert no duplicate bounty_ids in batch

EFFECTS ───────────────────────────────────────────────────────
  for each item (sorted by bounty_id asc):
    escrow.status ← Released
    escrow.remaining_amount ← 0
    write back to persistent storage
    record (contributor, amount) in release_pairs Vec

INTERACTIONS ──────────────────────────────────────────────────
  for each item (sorted):
    token.transfer(contract → contributor, amount)
    emit FundsReleased(...)
  emit BatchFundsReleased(count, total_amount, timestamp)
  reentrancy_guard::release()
```

---

## Reentrancy Protection

Both batch functions use the **shared reentrancy guard** module (`reentrancy_guard`). This guard sets a flag in instance storage at entry and clears it at exit. Any reentrant invocation — whether from a malicious token contract callback or a cross-contract call — will `panic!("Reentrancy detected")` when it attempts to acquire the already-held guard.

The guard is acquired **before** any state reads or writes, and released **after** all state writes and external transfers complete. This means the guard window covers the entire function body.

```
batch_lock_funds entry → acquire guard
   ... validation ...
   ... write effects ...
   ... token transfers ...
batch_lock_funds exit → release guard
```

If a token transfer callback somehow invokes `batch_lock_funds` again before the first call completes, the second invocation will panic immediately, rolling back the entire transaction.

---

## Events

### `FundsLocked` (emitted once per item in `batch_lock_funds`)

```rust
pub struct FundsLocked {
    pub version: u32,       // EVENT_VERSION_V2
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
}
```

Soroban event topics: `("funds", "locked")`

### `BatchFundsLocked` (emitted once per `batch_lock_funds` call)

```rust
pub struct BatchFundsLocked {
    pub count: u32,         // Number of escrows created
    pub total_amount: i128, // Sum of all item amounts
    pub timestamp: u64,     // Ledger timestamp at call time
}
```

Soroban event topics: `("batch", "locked")`

### `FundsReleased` (emitted once per item in `batch_release_funds`)

```rust
pub struct FundsReleased {
    pub version: u32,       // EVENT_VERSION_V2
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}
```

Soroban event topics: `("funds", "released")`

### `BatchFundsReleased` (emitted once per `batch_release_funds` call)

```rust
pub struct BatchFundsReleased {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}
```

Soroban event topics: `("batch", "released")`

All events are emitted via the `events` module helper functions (`emit_batch_funds_locked`, `emit_batch_funds_released`, `emit_funds_locked`, `emit_funds_released`), which ensure consistent topic and data encoding.

---

## Error Reference

| Error | Code | Batch context |
|-------|------|---------------|
| `NotInitialized` | 2 | Contract `init` was never called |
| `BountyExists` | 3 | A `bounty_id` in the batch already exists in storage |
| `FundsNotLocked` | 5 | A bounty in a release batch is not in `Locked` status |
| `Unauthorized` | 7 | `batch_release_funds` caller is not the admin |
| `InvalidBatchSize` | 10 | Batch length is 0 or exceeds `MAX_BATCH_SIZE` |
| `DuplicateBountyId` | 12 | Two items share the same `bounty_id` within the batch |
| `InvalidAmount` | 13 | An item's `amount` is ≤ 0 |
| `FundsPaused` | 18 | Lock or release operation is currently paused |
| `ParticipantBlocked` | 35 | Depositor is blocklisted (BlocklistOnly mode) |
| `ParticipantNotAllowed` | 36 | Depositor is not allowlisted (AllowlistOnly mode) |
| `ContractDeprecated` | 34 | New locks are blocked because contract is deprecated |
| `BountyNotFound` | 4 | A `bounty_id` in a release batch does not exist |

---

## Security Assumptions & Threat Model

### 1. Atomicity / Rollback Safety

**Assumption**: Soroban transactions are atomic — any panic or returned error causes a full state rollback. No partial state can be committed.

**Implication**: An attacker cannot craft a batch where the first `n` items succeed and write state while item `n+1` panics. The all-or-nothing property is enforced by the runtime, not by the contract code.

### 2. Authorization

**`batch_lock_funds`**: Each unique depositor address is required to authorize the transaction (via `require_auth`). The contract collects unique depositors first and calls `require_auth` once per unique address, preventing the "double-auth" error while ensuring no depositor can be drained without their explicit approval.

**`batch_release_funds`**: Only the admin address may call this function. A single `admin.require_auth()` covers the entire batch. The admin is assumed to be a trusted, access-controlled key (e.g., a multisig or backend service account).

### 3. Duplicate ID Detection

Within-batch duplicate detection is O(n²) but bounded by `MAX_BATCH_SIZE = 20`, making the worst case 400 comparisons — negligible. Between-batch conflicts are prevented by checking persistent storage before any writes.

### 4. Integer Overflow in `total_amount`

The batch release function accumulates `total_amount` using `checked_add` with an `ok_or(Error::InvalidAmount)` guard. If the sum of all escrow amounts would overflow `i128`, the function returns an error rather than panicking or silently wrapping.

### 5. Reentrancy via Malicious Token

The CEI ordering combined with the reentrancy guard means that even if the escrow token is replaced by a malicious contract that attempts to call back into `batch_lock_funds` or `batch_release_funds` during a transfer, the reentrant call will fail because:

- The reentrancy guard is already held (panic path), AND
- All state writes have already been committed (effects are done before interactions), so the reentrant call would fail duplicate-existence checks anyway.

### 6. Participant Filtering Bypass

The participant filter is checked per-item during the validation pass. An attacker cannot smuggle a blocked address into the middle of a batch — every depositor is checked before any funds are moved.

### 7. Rate Limiting

Rate limiting applies per element across every batch entry point. Both `batch_lock_funds` and `batch_release_funds` call `anti_abuse::check_rate_limit` for each item's designated address (depositor for lock, contributor for release). The check respects whitelist bypass — addresses on the whitelist skip the limit. A batch where any individual element would exceed the per-element rate limit is rejected by the existing `panic!("Rate limit exceeded")` from `check_rate_limit`.

High-throughput batch users should be placed on the whitelist via `set_whitelist_entry` to bypass rate limiting.

### 8. MAX_BATCH_SIZE Constant

`MAX_BATCH_SIZE = 20` is enforced as a hard constant. This limits CPU and ledger-byte usage per transaction, preventing denial-of-service via oversized batches. Raising this value requires careful benchmarking of instruction budgets on the Soroban platform.

---

## Test Coverage Summary

The file `test_batch_failure_modes.rs` provides exhaustive negative-path coverage. Below is a map of test cases to the scenarios they verify.

### `batch_lock_funds` tests

| Test name | Scenario covered |
|-----------|-----------------|
| `batch_lock_empty_batch_fails` | Zero-length batch → `InvalidBatchSize` |
| `batch_lock_single_item_succeeds` | Minimum valid batch (1 item) |
| `batch_lock_exceeds_max_batch_size_fails` | 101 items → `InvalidBatchSize` |
| `batch_lock_exactly_max_batch_size_succeeds` | 100 items (boundary) → succeeds |
| `batch_lock_duplicate_bounty_id_within_batch_fails` | Two items same `bounty_id` → `DuplicateBountyId` |
| `batch_lock_bounty_id_already_exists_in_storage_fails` | Pre-existing bounty → `BountyExists` |
| `batch_lock_invalid_amount_in_second_item_rolls_back_first` | Zero amount in item 2 → atomicity check |
| `batch_lock_duplicate_in_last_item_rolls_back_all_previous` | Last item dup → all previous not persisted |
| `batch_lock_not_initialized_fails` | No `init` call → `NotInitialized` |
| `batch_lock_same_depositor_multiple_bounties_succeeds` | One depositor, 3 bounties → succeeds |
| `batch_lock_zero_amount_fails` | Amount = 0 → `InvalidAmount` |

### `batch_release_funds` tests

| Test name | Scenario covered |
|-----------|-----------------|
| `batch_release_empty_batch_fails` | Zero-length batch → `InvalidBatchSize` |
| `batch_release_single_item_succeeds` | Minimum valid batch (1 item) |
| `batch_release_exceeds_max_batch_size_fails` | 101 items → `InvalidBatchSize` |
| `batch_release_duplicate_bounty_id_within_batch_fails` | Duplicate ID → `DuplicateBountyId` |
| `batch_release_nonexistent_bounty_fails` | Never-locked bounty → `BountyNotFound` |
| `batch_release_nonexistent_second_item_rolls_back_first` | Item 2 missing → item 1 not released |
| `batch_release_already_released_bounty_fails` | Already-released → `FundsNotLocked` |
| `batch_release_mixed_locked_and_refunded_fails_atomically` | One locked + one refunded → `FundsNotLocked`, locked escrow unchanged |
| `batch_release_not_initialized_fails` | No `init` call → `NotInitialized` |

### Cross-cutting atomicity tests

| Test name | Scenario covered |
|-----------|-----------------|
| `batch_release_partial_failure_does_not_release_any` | Second item already released → first item stays Locked |
| `batch_release_exactly_max_batch_size_succeeds` | 100 items → all released |

---

## Integration Guide

### Checking preconditions before submitting a batch

Before submitting a batch transaction, off-chain clients should verify:

1. The contract is initialized (`get_deprecation_status().deprecated == false`).
2. The lock or release operation is not paused (`get_pause_flags()`).
3. All `bounty_id`s in the batch are unique and do not already exist (query `get_escrow_info` for each).
4. All depositor addresses pass the current filter mode (`get_filter_mode()`).
5. All amounts satisfy the current `AmountPolicy` (`get_amount_policy()` if exposed).

### Simulating before submitting

Soroban supports simulation via `simulateTransaction`. Use simulation to estimate instruction units and verify the transaction will succeed before paying fees.

### Handling partial-but-rolled-back scenarios

Because batches are atomic, if a batch fails you can safely retry with a corrected item list. There is no risk of double-locking or double-releasing because the failed transaction leaves no trace in storage.

### Recommended batch size

While `MAX_BATCH_SIZE = 20` is the hard ceiling, practical instruction budgets on Stellar Mainnet may further constrain effective batch sizes depending on ledger congestion and per-item complexity. Benchmark your specific use case and start conservatively (batches of 5–10) before scaling up.

---

## Limitations & Known Constraints

| Limitation | Details |
|------------|---------|
| No per-item fee deduction | Batch lock/release bypass the fee logic present in single-item `lock_funds` / `release_funds`. |
| No rate limiting per item | `anti_abuse::check_rate_limit` is not called per depositor in `batch_lock_funds`. Use whitelisting for high-throughput depositors. |
| `MAX_BATCH_SIZE = 20` | Hard-coded constant. Raising it requires instruction-budget validation. |
| O(n²) duplicate detection | Acceptable at n ≤ 20 but not suitable for larger batches without a sort-based approach. |
| No partial success | The entire batch succeeds or the entire batch fails. There is no "best-effort" mode. |
| Single token | Both batch functions operate on the single token configured at `init`. Multi-token escrows require separate contract instances. |
| Anonymous escrows excluded | `batch_lock_funds` only creates standard (non-anonymous) escrows. Use `lock_funds_anonymous` for commitment-scheme deposits. |

---

*Last updated: based on `lib.rs` implementation and `test_batch_failure_modes.rs` test suite.*