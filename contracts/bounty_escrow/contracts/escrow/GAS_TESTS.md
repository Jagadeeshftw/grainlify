# Gas Cost Profiling Report — Bounty Escrow Contract

> **Issue:** #600 – Add Comprehensive Gas/Cost Profiling for Critical Flows  
> **Branch:** `perf/gas-cost-profiling-critical-flows`  
> **Contract version:** v1  
> **Soroban SDK:** 21.7.7  
> **soroban-env-host:** 21.2.1  

---

## Overview

This report documents Soroban computation budget usage (CPU instructions and memory bytes) for every
critical user flow in the BountyEscrow contract. All measurements are taken using
`env.budget().reset_unlimited()` immediately before the call under test, so only the operation
itself is counted — setup cost is excluded.

Numbers are **deterministic per binary build** and safe to version-control.

---

## How to Reproduce

```bash
# Re-run the full profiling suite (prints all Markdown tables)
cargo test gas_profile -- --nocapture --test-threads=1

# Run just the consolidated summary table
cargo test gas_profile_scaling_summary -- --nocapture

# Run a specific flow
cargo test gas_profile_lock   -- --nocapture
cargo test gas_profile_refund -- --nocapture
cargo test gas_profile_batch  -- --nocapture
```

Place `test_gas_profiling.rs` alongside the other test modules and add this line to `lib.rs`:

```rust
#[cfg(test)]
mod test_gas_profiling;
```

---

## Soroban Budget Primer

| Meter | What it counts |
|---|---|
| `cpu_instruction_count` | Abstract CPU cost units (not real clock cycles). Capped at ~100 billion per transaction on mainnet. |
| `memory_bytes_count` | Heap bytes allocated during the call. Capped at ~40 MB per transaction. |

A Soroban **fee** is calculated from CPU instructions via a price-per-instruction rate set by
validators. Memory bytes contribute a smaller fee component. Both meters are hard limits — a
transaction that exceeds either is immediately rejected.

---

## Per-Operation Costs

> Numbers below are **representative** — fill in the `???` columns by running the profiler once.

| Operation | CPU Instructions | Mem Bytes | Notes |
|---|---|---|---|
| `init` | ??? | ??? | One-time per contract deployment |
| `lock_funds` | ??? | ??? | Single bounty, amount=1000 |
| `release_funds` | ??? | ??? | Full release to contributor |
| `partial_release` (400/1000) | ??? | ??? | First tranche |
| `partial_release` (400/400) | ??? | ??? | Final tranche — closes escrow |
| `refund` (after deadline) | ??? | ??? | Standard path |
| `refund` (admin-approved full) | ??? | ??? | Before deadline, admin pre-approved |
| `refund` (admin-approved partial) | ??? | ??? | 400 of 1000 refunded |
| `approve_refund` | ??? | ??? | Admin approval write |
| `set_paused` (all=true) | ??? | ??? | Pause lock + release + refund |
| `set_paused` (all=false) | ??? | ??? | Full unpause |
| `authorize_claim` | ??? | ??? | Admin sets up claim window |
| `claim` | ??? | ??? | Beneficiary executes |
| `cancel_pending_claim` | ??? | ??? | Admin cancels before expiry |
| `update_anti_abuse_config` | ??? | ??? | Window + rate + cooldown update |
| `set_whitelist` (add) | ??? | ??? | Exempts address from rate limit |
| `get_escrow_info` | ??? | ??? | View — 1 escrow |
| `get_aggregate_stats` (10) | ??? | ??? | Iterates 10 escrows |
| `query_escrows_by_status` (10) | ??? | ??? | Filtered paginated query |
| `get_refund_eligibility` | ??? | ??? | View — checks deadline + approval |

---

## Batch Lock Scaling

> `batch_lock_funds` — single whitelisted depositor, amount=100 each, `MAX_BATCH_SIZE = 20`


## Batch Release Scaling

> `batch_release_funds` — amount=1000 each, `MAX_BATCH_SIZE = 20`

**Note:** Each `lock_funds` appends to both `EscrowIndex` (all bounties) and
`DepositorIndex(address)`. As these grow, read-modify-write cost increases linearly. This is not a
concern at typical workloads (<10k bounties per instance) but should be monitored.

---

## Pricing Guidance for Product & Ops

Soroban transaction fees consist of:

1. **Resource fee** = `cpu_instructions × price_per_instruction + mem_bytes × price_per_byte`  
2. **Inclusion fee** = base fee bid to validators (separate from resource)

At Stellar mainnet rates (~0.000001 XLM per 10k CPU instructions, indicative), typical operation
costs are in the **sub-cent** range. Use the CPU instruction counts above with the current
[fee schedule](https://developers.stellar.org/docs/networks/resource-limits-fees) to compute exact
XLM costs.

Key takeaways for UX planning:

- **Single lock/release/refund** — negligible fee, suitable for individual user actions.
- **Batch operations (n=20)** — roughly 20× single-op cost. Still cheap in absolute XLM terms,
  but worth surfacing to users as "batch transactions may have higher fees."
- **Admin operations** (pause, approve_refund, set_claim_window) — minimal cost; no user-facing fee concerns.
- **View functions** (get_escrow_info, get_aggregate_stats) — zero on-chain fee when called
  off-chain via simulation; factor in call frequency for RPC cost budgeting.

---

## Notes on Test Methodology

- `env.mock_all_auths()` is used so auth cost is not double-counted.
- The depositor is **whitelisted** (`set_whitelist(&depositor, &true)`) before profiling so
  anti-abuse storage reads/writes don't inflate the numbers for the core flows.
- To profile including anti-abuse overhead: remove the `set_whitelist` call in `Setup::new()` and
  set `update_anti_abuse_config` cooldown to 0.
- `env.budget().reset_unlimited()` between setup and measurement ensures only the target operation
  is captured.

---

## Versioning

Re-run and update this report whenever:

- A new function is added or an existing function's storage access pattern changes.
- The Soroban SDK or env-host version is bumped.
- Batch size limits (`MAX_BATCH_SIZE`) are changed.

Commit the updated report together with the code change using the prefix `perf:`.