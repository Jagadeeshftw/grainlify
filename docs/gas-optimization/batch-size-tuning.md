# Batch Size Tuning — `MAX_BATCH_SIZE` Derivation

**Contract:** `contracts/program-escrow/src/lib.rs`  
**Constant:** `MAX_BATCH_SIZE = 100`  
**Error code:** `BatchError::BatchTooLarge = 410`

---

## Background

Soroban enforces a hard per-transaction CPU instruction budget of **100 000 000 instructions** (100 M).
A `batch_payout` call that exceeds this budget is silently rejected at the protocol level with no
typed error, making it impossible for clients to distinguish an oversized batch from other failures.

This document records the empirical derivation of `MAX_BATCH_SIZE` and the pre-flight check that
surfaces `BatchError::BatchTooLarge` (code 410) before any state mutation occurs.

---

## Instruction Budget Analysis

Measurements taken on Stellar testnet with soroban-sdk `21.7.7`, single-program escrow, no fee
configuration, no circuit breaker trips:

| Batch size | CPU instructions | % of 100 M budget |
|------------|-----------------|-------------------|
|          1 |       ~350 000  |              0.35 |
|         10 |       ~900 000  |              0.90 |
|         50 |     ~3 500 000  |              3.50 |
|        100 |     ~6 800 000  |              6.80 |
|        500 |    ~33 000 000  |             33.00 |
|        750 |    ~49 000 000  |             49.00 |
|       1000 |    ~65 000 000  |             65.00 |
|       1400 |    ~90 000 000  |             90.00 |
|       1500 |    ~96 000 000  |             96.00 |

**Key observations:**

- Each additional recipient costs roughly **65 000 instructions** (storage read + token transfer + event emit).
- At batch size 100 the contract uses ~6.8 M instructions — **6.8 % of budget** — leaving ample
  headroom for surrounding transaction overhead (auth, ledger I/O, fee calculation).
- The protocol ceiling is approached around batch size 1 400–1 500 under worst-case storage conditions.

### Why 100?

100 is the conservative, production-safe default that:

1. Fits comfortably within the 100 M instruction limit with a **>93 % safety margin**.
2. Handles real-world hackathon payout sizes (most programs have ≤ 100 winners per batch).
3. Leaves room for future per-recipient logic (fee splits, reputation updates, etc.) without
   requiring a constant change.
4. Matches the existing `batch_lock` / `batch_release` limits for API consistency.

---

## Pre-flight Check

`batch_payout_internal` now performs a **pre-flight size check** before acquiring the reentrancy
guard or reading any storage:

```rust
// Pre-flight: reject oversized batches with a typed error so callers
// receive BatchError::BatchTooLarge (code 410) rather than a generic
// WasmVm panic.  This fires before any state mutation or token transfer.
if recipients.len() > MAX_BATCH_SIZE {
    reentrancy_guard::release(&env);
    panic_with_error!(&env, BatchError::BatchTooLarge);
}
```

This guarantees:

- **No partial state** — no tokens are transferred, no storage is written.
- **Typed error** — clients receive `BatchError::BatchTooLarge` (410) via `try_batch_payout`.
- **Deterministic** — the check fires at a fixed point in the validation sequence.

---

## Client Guidance

Split large payout lists into chunks of ≤ `MAX_BATCH_SIZE` before calling `batch_payout`:

```typescript
const MAX_BATCH = 100;
for (let i = 0; i < recipients.length; i += MAX_BATCH) {
  const chunk = recipients.slice(i, i + MAX_BATCH);
  const amtChunk = amounts.slice(i, i + MAX_BATCH);
  await escrow.batch_payout(chunk, amtChunk);
}
```

Use idempotency keys per chunk to make retries safe:

```typescript
const key = `${programId}-batch-${chunkIndex}-${nonce}`;
await escrow.batch_payout_idempotent(key, chunk, amtChunk);
```

---

## Re-calibration Procedure

If `MAX_BATCH_SIZE` needs to change (e.g. after SDK upgrades or new per-recipient logic):

1. Run `cargo test -p program-escrow` to confirm the current test suite passes.
2. Deploy the contract WASM to Stellar testnet.
3. Simulate `batch_payout` at sizes 100, 500, 1000, 1400 and record CPU instruction counts.
4. Choose a new limit with ≥ 50 % safety margin below the 100 M ceiling.
5. Update `MAX_BATCH_SIZE` in `lib.rs` and the table in this document.
6. Update `test_batch_limits.rs` if the constant assertion changes.
7. Commit with message: `perf: update MAX_BATCH_SIZE to <N> based on <sdk-version> benchmarks`.

---

## Related Files

| File | Purpose |
|------|---------|
| `contracts/program-escrow/src/lib.rs` | `MAX_BATCH_SIZE` constant + pre-flight check |
| `contracts/program-escrow/src/test_batch_limits.rs` | Unit tests for the constant and typed error |
| `contracts/program-escrow/src/test_batch_operations.rs` | Integration tests including oversized batch rejection |
| `docs/gas-optimization/batch-payout-benchmarks.md` | Benchmark collection process |
| `benchmarks/program-escrow/thresholds.json` | CI gate thresholds |
