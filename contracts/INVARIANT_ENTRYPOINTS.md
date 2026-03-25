# Invariant Entrypoints

This repository exposes explicit on-chain invariant checks so auditors and monitoring services can poll contract health without mutating state.

## Bounty Escrow (`contracts/bounty_escrow/contracts/escrow`)

- `check_invariants() -> InvariantCheckResult`
  - Detailed report with:
  - `healthy`, `initialized`, `config_sane`
  - `sum_remaining`, `token_balance`
  - `per_escrow_failures`, `orphaned_index_entries`, `refund_inconsistencies`, `violation_count`
- `verify_all_invariants() -> bool`
  - Lightweight boolean verdict suitable for high-frequency polling.

Checks include:

- Balance consistency: sum of active escrow balances equals contract token balance.
- Per-escrow sanity: non-negative amounts, remaining <= amount, status/remaining consistency.
- Index consistency: every indexed escrow id has a backing escrow record.
- Config sanity: admin/token presence, fee bounds, amount policy bounds, multisig threshold bounds.

## Grainlify Core (`contracts/grainlify-core`)

- `check_invariants() -> InvariantReport`
  - Detailed report with:
  - `healthy`, `config_sane`, `metrics_sane`
  - `admin_set`, `version_set`, `version`
  - `operation_count`, `unique_users`, `error_count`, `violation_count`
- `verify_invariants() -> bool`
  - Lightweight boolean verdict suitable for high-frequency polling.
- `health_check() -> HealthStatus`
  - Bounded operator-facing summary with:
  - `is_healthy`: mirrors the invariant verdict and returns `false` before initialization.
  - `last_operation`: timestamp of the most recent tracked operation, or `0` on empty state.
  - `total_operations`: current aggregate operation count.
  - `contract_version`: semantic version string derived from the stored numeric version.
- `get_analytics() -> Analytics`
  - Bounded aggregate counters with:
  - `operation_count`: total monitored operations.
  - `unique_users`: distinct callers observed through monitoring.
  - `error_count`: monitored failed operations.
  - `error_rate`: failure rate in basis points, or `0` when no operations were recorded.

Checks include:

- Metrics consistency: `error_count <= operation_count`, `unique_users <= operation_count`.
- Zero-activity consistency: if operations are zero then users/errors must also be zero.
- Config sanity: admin/version presence, valid version, previous-version ordering, chain/network pair consistency.

Monitoring view notes:

- `health_check` and `get_analytics` are read-only and safe to poll against empty state.
- Empty state is explicit rather than exceptional: counters return `0`, and health reflects the invariant/config verdict.
- `error_rate` is intentionally coarse; use emitted events for higher-fidelity off-chain analytics.

## Integrator Guidance

- Poll `verify_*` for cheap liveness checks.
- Poll `check_invariants` on a slower cadence (for richer diagnostics and alert payloads).
- Trigger alerts immediately when `healthy == false`.
- Include the returned counters in telemetry to speed incident triage.
