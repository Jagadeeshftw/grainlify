# Gas Budget — Production Enforcement Gap

**Contract:** `contracts/bounty_escrow/contracts/escrow`
**Module:** `src/gas_budget.rs`
**Severity:** Informational (by design; Soroban platform constraint)

---

## Summary

`GasBudgetConfig` caps are **stored, observable, and enforced in test builds**
but are **advisory-only on the live Stellar network**. No CPU or memory
measurement occurs at runtime in production WASM because `env.budget()` is
gated behind the `testutils` feature and is unconditionally absent from
contracts deployed on-chain.

Operators and auditors MUST NOT rely on configured caps as active production
safeguards against runaway resource consumption.

---

## Root Cause

Soroban's `env.budget()` API (`cpu_instruction_cost()`,
`memory_bytes_cost()`) is available only when the crate is compiled with the
`testutils` feature of `soroban-sdk`. This feature is never present in the
WASM binary that runs on-chain.

As a result, the measurement and enforcement blocks in `gas_budget::check()`
are conditionally compiled:

```rust
#[cfg(any(test, feature = "testutils"))]
pub fn capture(env: &Env) -> BudgetSnapshot { ... }

#[cfg(any(test, feature = "testutils"))]
pub fn check(env, op_name, budget, snapshot, enforce) -> Result<()> { ... }
```

On a production deployment these functions do not exist in the binary. The
call sites in `lib.rs` are also guarded by the same `cfg` attribute, so the
production WASM never attempts measurement.

---

## Enforcement Matrix

| Build | `capture()` / `check()` present? | Caps enforced? | `caps_enforced_in_production` |
|-------|----------------------------------|---------------|-------------------------------|
| `cargo test` (testutils) | ✅ Yes | ✅ Yes | `true` |
| `cargo build --release` (WASM) | ❌ No | ❌ No | `false` |
| Stellar mainnet | ❌ No | ❌ No | `false` |

---

## Detecting the Gap at Runtime

### `get_gas_budget_advisory_status`

The canonical on-chain query for this gap. Returns a
`GasBudgetAdvisoryStatus` struct:

```rust
pub struct GasBudgetAdvisoryStatus {
    /// Always false in production WASM.
    pub caps_enforced_in_production: bool,
    /// True when any operation has a non-zero cap configured.
    pub caps_configured: bool,
    /// Reflects GasBudgetConfig::enforce as stored.
    pub enforce_flag_set: bool,
    /// Full snapshot of current caps.
    pub config: GasBudgetConfig,
}
```

Call this function from operator tooling, monitoring dashboards, or audit
scripts to confirm the enforcement state without inspecting source code.

### `"gas_adv"` Advisory Event

When `caps_configured = true`, calling `get_gas_budget_advisory_status`
emits a `GasBudgetAdvisoryNotice` event with topic `"gas_adv"`. This event
appears in the on-chain event stream and is queryable via Horizon/RPC:

```json
{
  "topics": ["gas_adv"],
  "data": {
    "caps_enforced_in_production": false,
    "caps_configured": true,
    "enforce_flag_set": true,
    "timestamp": 1234567890
  }
}
```

Monitoring systems can subscribe to this topic to alert operators whenever a
deployment has configured caps that are not being enforced.

---

## Indirect Production Mitigations

Although per-operation gas caps cannot be measured, the following contract
controls partially substitute:

### 1. `MAX_BATCH_SIZE` (default: 20)

Batch operations (`batch_lock_funds`, `batch_release_funds`) are hard-capped
at `MAX_BATCH_SIZE` items per call. Since each item involves predictable
storage reads/writes and one token transfer, this bounds the worst-case CPU
cost of a single call by construction — no runtime measurement needed.

```rust
const MAX_BATCH_SIZE: u32 = 20;
```

Operators can tighten this limit via `set_batch_size_caps` (both `lock_cap`
and `release_cap` must satisfy `1 ≤ cap ≤ MAX_BATCH_SIZE`).

### 2. Soroban Network-Level Hard Limits

The Stellar network enforces absolute resource limits **independent of contract
logic**:

| Resource | Network Limit |
|----------|--------------|
| CPU instructions | ~100 billion per transaction |
| Memory bytes | ~40 MB per transaction |

Transactions exceeding either limit are rejected before execution completes.
These limits are set by validators and updated via Stellar Core configuration,
not by contract code.

### 3. Protocol Fee Caps

Lock and release operations have a maximum configurable fee rate (50%), which
implicitly bounds the token-math complexity per call.

### 4. Per-Bounty Amount Validation

`lock_funds` validates `amount > 0` and rejects calls with invalid amounts
before any storage writes occur, bounding the computation in the common
failure case.

---

## What Operators Should Do

1. **Do not treat configured caps as production safeguards.** Use them as:
   - Documentation of expected resource usage (derived from profiling).
   - Off-chain tooling reference for setting transaction `fee` values.
   - Test-environment enforcement to catch regressions.

2. **Run `get_gas_budget_advisory_status` from your monitoring stack** after
   deployment to verify the advisory status and confirm caps are documented.

3. **Use `MAX_BATCH_SIZE` as your primary batch-operation safety control.**
   Set it conservatively via `set_batch_size_caps` based on the profiling
   data in `GAS_TESTS.md`.

4. **Profile regularly.** Run `cargo test gas_profile_scaling_summary --
   --nocapture` after every Soroban SDK version bump or significant code
   change. Commit the updated `GAS_TESTS.md`.

5. **Subscribe to `"gas_adv"` events** in your Horizon/RPC monitoring
   configuration to detect advisory-only deployments.

---

## What Auditors Should Note

> See also: `external-audit-checklist.md` §Gas Budget section.

1. `GasBudgetConfig::enforce = true` has **no runtime effect** in production.
   A contract configured with `enforce = true` and non-zero caps is
   **functionally uncapped** on mainnet.

2. `get_gas_budget()` returns the stored policy values — not measured limits.
   Do not interpret the presence of non-zero cap values as proof of enforcement.

3. `get_gas_budget_advisory_status().caps_enforced_in_production` is a
   **compile-time constant** (`false`). It cannot be spoofed. This is the
   authoritative indicator.

4. The network-level limits (§ Indirect Mitigations) are the only guaranteed
   hard caps on mainnet resource consumption.

---

## Files Changed / Added

| File | Change |
|------|--------|
| `src/gas_budget.rs` | Extended module doc; added `GasBudgetAdvisoryStatus`, `is_any_cap_configured`, `advisory_status`, `emit_advisory_notice_if_needed` |
| `src/events.rs` | Added `GasBudgetAdvisoryNotice` event struct |
| `src/lib.rs` | Added `get_gas_budget_advisory_status` public function; updated `get_gas_budget` doc |
| `src/test_gas_budget.rs` | Added advisory status tests and `is_any_cap_configured` unit tests |
| `docs/security/gas-budget-production-gap.md` | This document |
| `docs/security/external-audit-checklist.md` | Created with gas-budget gap cross-reference |

---

## References

- `src/gas_budget.rs` — implementation with full NatSpec doc comments
- `src/test_gas_budget.rs` — test coverage for advisory status
- `GAS_TESTS.md` — profiling workflow and baseline measurements
- [Soroban Resource Limits](https://developers.stellar.org/docs/networks/resource-limits-fees)
- [Soroban SDK `testutils` feature](https://docs.rs/soroban-sdk/latest/soroban_sdk/testutils/index.html)
