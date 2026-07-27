# Business Metrics — Naming & Dashboard Coverage

Issue #1628: Document monitoring metrics coverage  
Regression surface version: **1**

---

## Purpose

This document pins down the *implicit* contracts around the Grainlify
business metrics pipeline that production dashboards depend on.  The
three contracts are:

1. **Metric naming** — every metric has a predictable, validateable name.
2. **Dashboard coverage map** — which panels consume which metrics.
3. **Type / unit / cardinality** — counters append-only, rates in bps,
   amounts in XLM, stable tag-dimensions for group-by.

Breaking any of these contracts (e.g. renaming a metric, changing the
suffix unit, removing a tag dimension) silently breaks production
dashboards.  The test suite in
[businessMetrics.test.ts](file:///c:/Users/Admin/Documents/Drips/grainlify/frontend/src/metrics/__tests__/businessMetrics.test.ts)
flags any such change as a failing test.  If you *intentionally* need
to change one of the pinned behaviours, bump
`REGRESSION_SURFACE_VERSION` in that file at the same time, and notify
the dashboard / observability team.

---

## 1. Metric Naming Convention

Every on-wire metric name MUST match:

```
/{prefix}_{snake_case_description}_{unit}/
```

Validated at runtime by `validateMetricName()` in
[businessMetrics.ts](file:///c:/Users/Admin/Documents/Drips/grainlify/frontend/src/metrics/businessMetrics.ts).

### 1.1 Reserved Prefixes

Adding a new prefix?  Register it in `METRIC_PREFIXES` and assign at
least one dashboard to its first metric so coverage stays tight.

| Prefix     | Domain                                           |
|------------|--------------------------------------------------|
| `bounty`   | Bounty conversion funnel + lifecycle             |
| `program`  | Program escrow governance + dwell-time           |
| `payout`   | Payout pipeline amounts, statuses, failure rates |
| `user`     | Contributor earnings, sign-ins, onboarding       |
| `system`   | App-level error rates, sampled page-views        |
| `db`       | Backend PostgreSQL observability                 |
| `contract` | On-chain Soroban escrow operations + amounts     |

### 1.2 Unit Suffixes

Mandatory — every non-count metric must declare its unit *as part of
the name* so dashboards can label Y-axes without extra metadata.

| Suffix     | Meaning                                            |
|------------|----------------------------------------------------|
| `_total`   | Append-only counter (monotonic)                    |
| `_count`   | Instantaneous gauge count (can go up and down)     |
| `_seconds` | Duration in wall-clock seconds                     |
| `_xlm`     | Amount in whole XLM tokens                         |
| `_stroops` | Amount in stroops (1 stroop = 1e-7 XLM)           |
| `_bps`     | Rate in basis points (1 bps = 0.01%)               |
| `_ratio`   | Unitless ratio in [0, 1]                           |

### 1.3 Examples

```
✅ bounty_applied_total              counter + bounty prefix
✅ payout_amount_sum_xlm             amount metric has _xlm
✅ contract_error_rate_bps           rate has _bps

❌ BountyApplied                    camelCase forbidden
❌ foo_applied_total                "foo" is not a reserved prefix
❌ payout_amount                    amount without unit suffix
❌ bounty-paid-total                hyphens forbidden (use underscore)
```

### 1.4 Validation helpers exposed

- `validateMetricName(name)` — syntactic check only.
- `validateMetricName(name, { requireRegistered: true })` — also checks
  the name exists in the `METRICS` registry.

---

## 2. Dashboard Coverage Map

Every registered metric is claimed by at least one dashboard.  Every
dashboard ID in `DASHBOARD_IDS` owns at least one metric.  Run
`findOrphanMetrics()` + `findEmptyDashboards()` in the test suite or
debug page to verify.

| Dashboard ID                      | UI Panel (approx)                         |
|-----------------------------------|-------------------------------------------|
| `maintainers_analytics`           | MaintainersPage → AnalyticsTab (aggregate) |
| `maintainers_funnel`              | → BountyFunnelChart                       |
| `maintainers_payouts`             | → PayoutHistoryTable + status pills       |
| `maintainers_top_contributors`    | → TopContributorsModule                   |
| `admin_ecosystem_overview`        | EcosystemsPage aggregate tiles            |
| `admin_error_monitor`             | AdminPage → health / errors panel         |
| `profile_contributor_summary`     | ProfilePage → RewardsChart + heatmap      |
| `leaderboard_ranking`             | LeaderboardPage (contributors + projects) |
| `contract_escrow_health`          | Off-chain indexer health view             |
| `db_observability`                | Backend slow-query / query-rate view      |

### 2.1 Coverage for every metric (current set = 25 keys)

| Key                                       | Dashboards                                                        |
|-------------------------------------------|-------------------------------------------------------------------|
| `bounty_applied_total`                    | maintainers_analytics, maintainers_funnel                        |
| `bounty_assigned_total`                   | maintainers_analytics, maintainers_funnel                        |
| `bounty_submitted_total`                  | maintainers_analytics, maintainers_funnel                        |
| `bounty_paid_total`                       | maintainers_analytics, maintainers_funnel                        |
| `bounty_conversion_rate_bps`              | maintainers_analytics, admin_ecosystem_overview                  |
| `payout_amount_xlm`                       | maintainers_payouts, profile_contributor_summary                 |
| `payout_amount_sum_xlm`                   | maintainers_payouts, maintainers_top_contributors, profile_contributor_summary, leaderboard_ranking |
| `payout_status_count`                     | maintainers_payouts, admin_error_monitor                         |
| `payout_failure_rate_bps`                 | admin_error_monitor, maintainers_payouts                         |
| `program_draft_dwell_seconds`             | contract_escrow_health                                           |
| `program_active_count`                    | admin_ecosystem_overview, contract_escrow_health                 |
| `contract_escrow_locked_xlm`              | contract_escrow_health, admin_ecosystem_overview                 |
| `contract_escrow_released_xlm`            | contract_escrow_health, admin_ecosystem_overview, leaderboard_ranking |
| `contract_escrow_refunded_xlm`            | contract_escrow_health                                           |
| `contract_operation_total`                | contract_escrow_health                                           |
| `contract_error_total`                    | contract_escrow_health, admin_error_monitor                      |
| `contract_error_rate_bps`                 | contract_escrow_health, admin_error_monitor                      |
| `user_contributor_earnings_xlm`           | maintainers_top_contributors, profile_contributor_summary, leaderboard_ranking |
| `user_signin_total`                       | admin_ecosystem_overview                                         |
| `user_onboarding_completed_total`         | admin_ecosystem_overview                                         |
| `system_pageview_total`                   | admin_ecosystem_overview                                         |
| `system_error_rate_bps`                   | admin_error_monitor                                              |
| `db_queries_total`                        | db_observability                                                 |
| `db_slow_queries_total`                   | db_observability, admin_error_monitor                            |
| `db_avg_duration_seconds`                 | db_observability                                                 |

### 2.2 Per-dashboard metric list

Use `DASHBOARD_COVERAGE[id]` in code.  This is the source of truth for
which metrics a dashboard scraper should pull.

| Dashboard                      | Metric keys                                                                                                                               |
|--------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| `maintainers_analytics`        | bounty_applied_total, bounty_assigned_total, bounty_submitted_total, bounty_paid_total, bounty_conversion_rate_bps                        |
| `maintainers_funnel`           | bounty_applied_total, bounty_assigned_total, bounty_submitted_total, bounty_paid_total                                                    |
| `maintainers_payouts`          | payout_amount_xlm, payout_amount_sum_xlm, payout_status_count, payout_failure_rate_bps                                                   |
| `maintainers_top_contributors` | payout_amount_sum_xlm, user_contributor_earnings_xlm                                                                                     |
| `admin_ecosystem_overview`     | bounty_conversion_rate_bps, program_active_count, contract_escrow_locked_xlm, contract_escrow_released_xlm, user_signin_total, user_onboarding_completed_total, system_pageview_total |
| `admin_error_monitor`          | payout_status_count, payout_failure_rate_bps, contract_error_total, contract_error_rate_bps, system_error_rate_bps, db_slow_queries_total |
| `profile_contributor_summary`  | payout_amount_xlm, payout_amount_sum_xlm, user_contributor_earnings_xlm                                                                  |
| `leaderboard_ranking`          | payout_amount_sum_xlm, contract_escrow_released_xlm, user_contributor_earnings_xlm                                                       |
| `contract_escrow_health`       | program_draft_dwell_seconds, program_active_count, contract_escrow_locked_xlm, contract_escrow_released_xlm, contract_escrow_refunded_xlm, contract_operation_total, contract_error_total, contract_error_rate_bps |
| `db_observability`             | db_queries_total, db_slow_queries_total, db_avg_duration_seconds                                                                         |

---

## 3. Regression Surface (what NOT to patch-release)

The following are the *implicit* contracts between the codebase and the
production dashboards.  Changing any of them without coordinating a
dashboard update == broken graphs.  The test suite pins them all via
snapshot-style assertions.

| Thing pinned                     | Why it is pinned                                        |
|----------------------------------|---------------------------------------------------------|
| Literal `MetricKey` strings      | Dashboards scrape by exact name match.                  |
| Unit suffixes on amount metrics  | Panel Y-axis labels assume _xlm / _stroops / _bps.      |
| DashboardId → MetricKey[] map    | Each panel's PromQL / SQL query uses this list.         |
| Metric cardinality per key       | `GROUP BY` clauses in dashboards assume these tags.     |
| BOUNTY_FUNNEL_STAGES order       | Funnel chart is positional; swapping stages flips bars. |
| PAYOUT_STATUSES string values    | Status pie-chart slices match on exact string.          |
| BPS range = [0, 10000]           | Y-axes clamped to 0-100 (%).  clamp in compute helper.  |
| Counters are monotonic           | Derivative queries (`rate()`) produce garbage otherwise.|

### 3.1 Bounty Funnel Stages (positional!)

```
applied → assigned → submitted → paid
```
*Number = 4.  Order = exactly as shown.*  The conversion-rate labels
between stages are computed by subtracting neighbours; reordering =
wrong conversion rates in the legend.

### 3.2 Payout Statuses

```
paid, pending, processing, failed
```

These MUST match the `PayoutStatus` type in
`features/maintainers/types/index.ts` string-for-string.  The test
"cross-module: payout status enum parity with maintainers/types" locks
them together.

### 3.3 BPS rate semantics

All rates use **basis points**.  `1 bps = 0.01 %` so the range is
`[0, 10000]`.  Derived helpers:

- `computeFunnelConversionBps(applied, paid)` → safe-division,
  floor-clamped to `[0, 10000]`.
- Backend `error_rate_bps` uses `(error_count * 10000 / operation_count)`
  — keep the two implementations in sync.

### 3.4 Cardinality / valid tag dimensions

Metrics can only be grouped / filtered by these tags.  Adding a tag is
safe (dashboards ignore unknown tags).  *Removing* a tag breaks panel
`GROUP BY`s.  Valid tag keys:

`project_id`, `ecosystem_id`, `repository`, `contributor`, `depositor`,
`status`, `program_id`, `period`, `asset_code`

---

## 4. Coverage Report (how to check locally)

From the frontend dir:

```bash
cd frontend
npm test -- src/metrics/__tests__/businessMetrics.test.ts
```

The suite includes:
- All 25 keys exist exactly as documented.
- All 10 dashboards have ≥ 1 metric.
- All 25 metrics have ≥ 1 dashboard (no orphans).
- Funnel stages + payout statuses are exactly the pinned values.
- BPS clamp + safe-division edge cases (0 applied, paid > applied,
  floor semantics).

---

## 5. Adding a new metric (checklist)

1. Pick a **reserved prefix** or propose a new one (add to
   `METRIC_PREFIXES`).
2. Name it: `{prefix}_{description}_{unit}` matching
   `METRIC_NAME_REGEX`.
3. Add entry to `METRICS` array in `businessMetrics.ts`:
   - `type`, `unit`, `description`, `cardinality`, **≥ 1 dashboard**.
4. If the new metric belongs to a *new* dashboard, add its ID to
   `DASHBOARD_IDS` (the test suite will yell otherwise).
5. Run the tests — registry integrity, coverage guards, and
   name-regex checks all run automatically.
6. Bump `REGRESSION_SURFACE_VERSION` in the test file *only if* this
   change is visible to dashboards (additions are fine without a bump,
   renames / removals need one).

---

## 6. Backend ↔ Frontend Metric Parity

The backend's Go `expvar` layer exports:

- `db_queries_total`
- `db_slow_queries_total`
- (cumulative ms captured internally; frontend sees
  `db_avg_duration_seconds` derived)

See [database/observability.md](file:///c:/Users/Admin/Documents/Drips/grainlify/docs/database/observability.md).
The *names* of the shared keys (`db_queries_total`, `db_slow_queries_total`)
are declared in both places.  Do not rename independently — the test
suite pins those exact strings via the `METRIC_KEYS` snapshot.

Contract event-indexer keys (`contract_escrow_locked_xlm` etc.) mirror
the names of the on-chain views (`get_balance`, `get_aggregate_stats`,
`health_check`); see the on-chain monitoring module for the canonical
source on the program-escrow side.
