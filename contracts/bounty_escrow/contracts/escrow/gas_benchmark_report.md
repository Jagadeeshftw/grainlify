# Gas & Resource Benchmark Gate Report — bounty-escrow

- **Overall Gate Status**: ❌ FAILED (REGRESSION)
- **Execution Mode**: `strict (enforcing default)`
- **Stated Tolerance**: 1000 bps (10.0%)
- **Committed Baseline File**: `gas_baseline.json`
- **WASM Size Budget Ceiling**: 260000 bytes

| Operation | Dimension | Actual Cost | Recorded Baseline | Delta | Allowed Ceiling (+tolerance) | Gate Status |
|:---|:---:|---:|---:|---:|---:|:---:|
| `batch_lock_n20_on_60_index` | CPU | 19897599 | 19808371 | +0.45% (+45 bps) | 21789208 | PASS |
| `batch_lock_n20_on_60_index` | MEM | 5355808 | 5350430 | +0.10% (+10 bps) | 5885473 | PASS |
| `batch_release_n20_on_60_index` | CPU | 15367288 | 15231318 | +0.89% (+89 bps) | 16754449 | PASS |
| `batch_release_n20_on_60_index` | MEM | 3230737 | 3212659 | +0.56% (+56 bps) | 3533924 | PASS |
| `create_lock_index60_large_amount` | CPU | 2632899 | 2614652 | +0.69% (+69 bps) | 2876117 | PASS |
| `create_lock_index60_large_amount` | MEM | 475976 | 472494 | +0.73% (+73 bps) | 519743 | PASS |
| `test_validation_regression` | CPU | 1150000 | 1000000 | +15.00% (+1500 bps) | 1100000 | **FAIL (REGRESSION)** |
| `test_validation_pass` | CPU | 1050000 | 1000000 | +5.00% (+500 bps) | 1100000 | PASS |
| `migration_set_deprecation_target` | CPU | 121978 | 121978 | +0.00% (+0 bps) | 134175 | PASS |
| `migration_set_deprecation_target` | MEM | 19447 | 19447 | +0.00% (+0 bps) | 21391 | PASS |
| `migration_simulate_upgrade_60_escrows` | CPU | 13407860 | 13393590 | +0.10% (+10 bps) | 14732949 | PASS |
| `migration_simulate_upgrade_60_escrows` | MEM | 3181821 | 3179105 | +0.08% (+8 bps) | 3497015 | PASS |
| `pagination_aggregate_stats_60` | CPU | 1299626 | 1293614 | +0.46% (+46 bps) | 1422975 | PASS |
| `pagination_aggregate_stats_60` | MEM | 111688 | 110252 | +1.30% (+130 bps) | 121277 | PASS |
| `pagination_whitelist_60total_limit50` | CPU | 468160 | 456266 | +2.60% (+260 bps) | 501892 | PASS |
| `pagination_whitelist_60total_limit50` | MEM | 92259 | 90263 | +2.21% (+221 bps) | 99289 | PASS |
| `payout_partial_release_5k_from_escrow60` | CPU | 2223114 | 2205092 | +0.81% (+81 bps) | 2425601 | PASS |
| `payout_partial_release_5k_from_escrow60` | MEM | 350496 | 347038 | +0.99% (+99 bps) | 381741 | PASS |
| `payout_refund_escrow60_deadline` | CPU | 2706657 | 2688146 | +0.68% (+68 bps) | 2956960 | PASS |
| `payout_refund_escrow60_deadline` | MEM | 483256 | 479774 | +0.72% (+72 bps) | 527751 | PASS |
| `payout_release_escrow60` | CPU | 1320545 | 1303289 | +1.32% (+132 bps) | 1433617 | PASS |
| `payout_release_escrow60` | MEM | 345804 | 342322 | +1.01% (+101 bps) | 376554 | PASS |

---
*Report published as a CI artifact on every run.*
