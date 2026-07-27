# Program escrow reputation and dust-payout gaming

This document describes how `get_program_reputation` behaves under dust-sized payout spam and which fields integrators should trust for “program health” signals.

## Formula (on-chain)

`get_program_reputation` reads the active program’s schedules and `payout_history`. Derived fields:

| Field | Meaning | Dust spam effect |
| --- | --- | --- |
| `total_payouts` | Length of `payout_history` | Increases one per call (informational only) |
| `qualified_payout_count` | Payouts with `amount >= REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT` (1_000 base units) | Unaffected by 1-unit dust |
| `total_funds_distributed` | Sum of payout amounts | Grows linearly with dust total, not per-call |
| `payout_fulfillment_rate_bps` | `(distributed / locked) * 10_000` | Stays near zero unless meaningful value moves |
| `completion_rate_bps` | Released schedules / total schedules | Independent of payout count |
| `overall_score_bps` | `0` if any overdue schedule; else `60% * completion + 40% fulfillment` | **Value-weighted**, not operation-count-based |

Constants are defined in `contracts/program-escrow/src/reputation.rs` and re-exported from the contract crate root.

## Threat model

**Attack:** A program locks a large prize pool and repeatedly calls `single_payout` with the minimum allowed amount (1 base unit) to cooperating addresses, inflating visible activity cheaply.

**Impact on score:** Because `overall_score_bps` weights **fulfillment by value**, dust leaves almost all funds locked and keeps fulfillment (and thus overall score) low. Spamming calls does not bypass the locked-pool denominator.

**Residual limitation:** Off-chain dashboards that rank programs by `total_payouts` alone remain gameable. Prefer `qualified_payout_count`, `total_funds_distributed`, or `overall_score_bps`.

## Resistance properties

1. **Value-primary scoring** — `overall_score_bps` does not use raw payout count.
2. **Qualifying activity floor** — `qualified_payout_count` excludes payouts below 1_000 base units.
3. **Overdue penalty** — Any overdue release forces `overall_score_bps = 0` regardless of payout activity.

## Benchmark (N = 25, typical payout = 10_000)

Locked pool: `N * 10_000`. Compare:

- **Dust path:** `N` payouts of `1` → fulfillment ≈ `N / (N * 10_000) * 10_000` ≈ **1 bps** → overall ≈ **6_000 bps** (perfect schedules assumed).
- **Typical path:** `N` payouts of `10_000` → full distribution → fulfillment **10_000 bps** → overall **10_000 bps**.

The pure helper `reputation::benchmark_overall_scores_dust_vs_typical` mirrors this math; `test_reputation_benchmark_dust_vs_typical_on_chain` asserts the contract matches the helper.

## Security notes

- Dust gaming does not drain escrow faster than the summed dust amounts; economic cost is still bounded by locked liquidity and authorization on `single_payout`.
- Reputation is a **snapshot** of on-chain history, not proof of off-chain deliverables.
- Future changes to weights or qualifying thresholds must remain backward-compatible for indexers documenting field semantics.

## Tests

Run:

```bash
cargo test -p program-escrow test_reputation
```

Key cases: `test_reputation_dust_payouts_gaming_resistance`, `test_reputation_benchmark_dust_vs_typical_on_chain`, `test_reputation_qualifying_threshold_boundary`.
