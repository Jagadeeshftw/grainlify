/*
 * businessMetrics — Monitoring Metrics Coverage Registry
 * ========================================================
 *
 * #1628 Document monitoring metrics coverage
 *
 * Central registry for business-facing monitoring metrics that feed
 * production dashboards.  This module pins down three things that were
 * previously implicit, which together form the regression surface:
 *
 *   1. METRIC NAMING CONVENTIONS  — every metric MUST match a pattern
 *      (prefix + snake_case + unit suffix).  Breaking a name breaks the
 *      dashboards that scrape the metric.
 *
 *   2. DASHBOARD COVERAGE MAP     — which panels / dashboards consume
 *      each metric.  Removing a metric or changing its cardinality breaks
 *      those panels (silently in most tools).
 *
 *   3. TYPE / CARDINALITY / UNITS — counters are append-only, gauges
 *      carry a "current value" semantics, rates are in basis points,
 *      amounts are in the native token (XLM / stroops).  Changing any of
 *      these without a coordinated dashboard update produces garbage
 *      graphs.
 *
 * The registry is read at runtime so call sites can be validated, and it
 * is also introspected by the test suite to snapshot the current
 * behaviour so any accidental rename / removal / cardinality change
 * surfaces as a failing test.
 *
 *
 * Regression Surface (things that, if changed, require a coordinated
 * dashboard / indexer update — do NOT alter in a patch release):
 *   -------------------------------------------------------------------
 *   MetricKey literal string values         (dashboards query by name)
 *   MetricUnit suffixes on amount metrics   (panel Y-axis unit labels)
 *   DashboardId -> MetricKey[] mapping      (panel queries)
 *   MetricCardinality for each metric       (group-by dimensions in UI)
 *   BountyFunnel stage order + names        (funnel chart is positional)
 *   PayoutStatus enum literal values        (status pie chart slices)
 *   -------------------------------------------------------------------
 */

// ─── Metric naming conventions ──────────────────────────────────────────────
/**
 * Valid metric name pattern:
 *   {prefix}_{snake_case_description}[_{unit}]
 *
 * Prefixes are reserved per domain — see `METRIC_PREFIXES`.  Units are
 * mandatory for any non-count metric: _total (counter), _seconds,
 * _xlm, _bps (basis points), _ratio.
 *
 * Examples:
 *   ✓  bounty_applied_total           counter, "bounty" prefix
 *   ✓  payout_amount_xlm              amount metric has _xlm suffix
 *   ✓  program_error_rate_bps         rate has _bps suffix
 *   ✗  BountyApplied                  camelCase — forbidden
 *   ✗  foo_applied_total              "foo" is not a reserved prefix
 *   ✗  payout_amount                  amount without unit suffix
 */
export const METRIC_NAME_REGEX =
  /^(bounty|program|payout|user|system|db|contract)_([a-z0-9_]+)_(total|seconds|xlm|stroops|bps|ratio|count)$/;

/**
 * Reserved metric-domain prefixes.  Adding a new one requires updating
 * DASHBOARD_COVERAGE so the new metrics are routed somewhere visible.
 */
export const METRIC_PREFIXES = [
  "bounty", // Bounty funnel + lifecycle
  "program", // Program escrow governance + dwell
  "payout", // Payout pipeline + amounts
  "user", // User journeys (sign-in, kyc, onboarding)
  "system", // App-level (error rates, latency)
  "db", // Database (from backend observability)
  "contract", // On-chain (from soroban event indexer)
] as const;

export type MetricPrefix = (typeof METRIC_PREFIXES)[number];

/** Canonical unit suffix — part of the on-wire metric name. */
export const METRIC_UNITS = [
  "total", // Append-only counter
  "count", // Instantaneous count (gauge)
  "seconds", // Duration
  "xlm", // Native token, whole units
  "stroops", // Native token, 1e-7 XLM
  "bps", // Basis points (1 = 0.01%)
  "ratio", // Unitless ratio 0..1
] as const;

export type MetricUnit = (typeof METRIC_UNITS)[number];

// ─── Cardinality / tag dimensions ───────────────────────────────────────────
/**
 * Which dimensions a metric can be grouped / filtered by in dashboards.
 * Adding a tag is backward-compatible (dashboards just ignore it).
 * Removing or renaming a tag breaks panel group-bys.
 */
export type MetricCardinality = ReadonlyArray<
  | "project_id"
  | "ecosystem_id"
  | "repository"
  | "contributor"
  | "depositor"
  | "status" // PayoutStatus / EscrowStatus
  | "program_id"
  | "period" // AnalyticsPeriod
  | "asset_code" // Multi-token future
>;

// ─── Core types ─────────────────────────────────────────────────────────────
export type MetricType = "counter" | "gauge" | "histogram" | "rate";

export interface MetricDefinition {
  /** The on-wire metric name.  MUST match METRIC_NAME_REGEX. */
  readonly key: string;
  readonly type: MetricType;
  readonly unit: MetricUnit;
  /** Human description used in dashboard tooltips. */
  readonly description: string;
  /** Group-by dimensions.  Empty = aggregate global only. */
  readonly cardinality: MetricCardinality;
  /**
   * Dashboards that query this metric.  Used by the coverage check to
   * flag "orphan" metrics that are emitted but never visualised, and
   * conversely dashboard panels that reference a non-existent metric.
   */
  readonly dashboards: ReadonlyArray<DashboardId>;
}

// ─── Dashboard IDs (coverage map targets) ───────────────────────────────────
/**
 * Every dashboard / panel that consumes metrics must be listed here.
 * Values are used in URL slugs and in the coverage report; treat them
 * as stable external identifiers.
 */
export const DASHBOARD_IDS = [
  "maintainers_analytics", // MaintainersPage → AnalyticsTab
  "maintainers_funnel", // → BountyFunnelChart
  "maintainers_payouts", // → PayoutHistoryTable
  "maintainers_top_contributors", // → TopContributorsModule
  "admin_ecosystem_overview", // EcosystemsPage aggregate
  "admin_error_monitor", // AdminPage error / health panel
  "profile_contributor_summary", // ProfilePage heatmap + rewards
  "leaderboard_ranking", // LeaderboardPage tables + podium
  "contract_escrow_health", // Off-chain indexer health view
  "db_observability", // Backend slow-query view
] as const;

export type DashboardId = (typeof DASHBOARD_IDS)[number];

// ─── Bounty funnel stages (positional + naming are stable) ──────────────────
/**
 * Funnel stages for the bounty conversion visualisation.
 * ORDER IS SIGNIFICANT — the chart renders stages in array order, so
 * inserting / reordering stages shifts all conversion-rate deltas.
 */
export const BOUNTY_FUNNEL_STAGES = [
  "applied", // Applications received
  "assigned", // Assigned to a contributor (no longer "open")
  "submitted", // Work submitted (PR opened)
  "paid", // Payout completed successfully
] as const;

export type BountyFunnelStage = (typeof BOUNTY_FUNNEL_STAGES)[number];

// ─── Payout statuses (slice names for status charts) ────────────────────────
/** Mirrors the PayoutStatus in features/maintainers/types; re-declared here
 *  so the metrics module owns the canonical string values that dashboards
 *  match against.  If they diverge the tests catch it. */
export const PAYOUT_STATUSES = [
  "paid",
  "pending",
  "processing",
  "failed",
] as const;

export type PayoutStatusMetric = (typeof PAYOUT_STATUSES)[number];

// ─── The canonical metric registry ──────────────────────────────────────────

const _METRICS = [
  // ── Bounty domain ───────────────────────────────────────────────────────
  {
    key: "bounty_applied_total",
    type: "counter",
    unit: "total",
    description: "Total bounty applications received (cumulative).",
    cardinality: ["project_id", "repository", "ecosystem_id", "period"],
    dashboards: ["maintainers_analytics", "maintainers_funnel"],
  },
  {
    key: "bounty_assigned_total",
    type: "counter",
    unit: "total",
    description: "Total bounties assigned to a contributor.",
    cardinality: [
      "project_id",
      "repository",
      "ecosystem_id",
      "contributor",
      "period",
    ],
    dashboards: ["maintainers_analytics", "maintainers_funnel"],
  },
  {
    key: "bounty_submitted_total",
    type: "counter",
    unit: "total",
    description:
      "Total work submissions (PR opens) against an assigned bounty.",
    cardinality: [
      "project_id",
      "repository",
      "ecosystem_id",
      "contributor",
      "period",
    ],
    dashboards: ["maintainers_analytics", "maintainers_funnel"],
  },
  {
    key: "bounty_paid_total",
    type: "counter",
    unit: "total",
    description: "Total bounties paid out successfully.",
    cardinality: [
      "project_id",
      "repository",
      "ecosystem_id",
      "contributor",
      "period",
    ],
    dashboards: ["maintainers_analytics", "maintainers_funnel"],
  },
  {
    key: "bounty_conversion_rate_bps",
    type: "rate",
    unit: "bps",
    description:
      "Applied → Paid conversion rate in basis points (1 bp = 0.01%). Derived = bounty_paid_total / bounty_applied_total.",
    cardinality: ["project_id", "ecosystem_id", "period"],
    dashboards: ["maintainers_analytics", "admin_ecosystem_overview"],
  },

  // ── Payout domain ───────────────────────────────────────────────────────
  {
    key: "payout_amount_xlm",
    type: "histogram",
    unit: "xlm",
    description: "Per-payout amount in whole XLM units (paid status only).",
    cardinality: [
      "project_id",
      "repository",
      "contributor",
      "status",
      "period",
    ],
    dashboards: ["maintainers_payouts", "profile_contributor_summary"],
  },
  {
    key: "payout_amount_sum_xlm",
    type: "counter",
    unit: "xlm",
    description: "Running sum of all paid payout amounts.",
    cardinality: ["project_id", "ecosystem_id", "contributor", "period"],
    dashboards: [
      "maintainers_payouts",
      "maintainers_top_contributors",
      "profile_contributor_summary",
      "leaderboard_ranking",
    ],
  },
  {
    key: "payout_status_count",
    type: "gauge",
    unit: "count",
    description:
      "Current payout count grouped by status.  Gauge semantics: refreshed at query time, not append-only.",
    cardinality: ["status", "project_id", "ecosystem_id", "period"],
    dashboards: ["maintainers_payouts", "admin_error_monitor"],
  },
  {
    key: "payout_failure_rate_bps",
    type: "rate",
    unit: "bps",
    description: "Failed / total payout rate in basis points.",
    cardinality: ["project_id", "ecosystem_id", "period"],
    dashboards: ["admin_error_monitor", "maintainers_payouts"],
  },

  // ── Program / escrow domain ─────────────────────────────────────────────
  {
    key: "program_draft_dwell_seconds",
    type: "histogram",
    unit: "seconds",
    description: "Elapsed seconds between program creation and publish.",
    cardinality: ["program_id", "ecosystem_id"],
    dashboards: ["contract_escrow_health"],
  },
  {
    key: "program_active_count",
    type: "gauge",
    unit: "count",
    description: "Current number of programs in Active status.",
    cardinality: ["ecosystem_id"],
    dashboards: ["admin_ecosystem_overview", "contract_escrow_health"],
  },
  {
    key: "contract_escrow_locked_xlm",
    type: "gauge",
    unit: "xlm",
    description:
      "Total XLM currently held in escrow contracts (live balance view).",
    cardinality: ["ecosystem_id", "asset_code"],
    dashboards: ["contract_escrow_health", "admin_ecosystem_overview"],
  },
  {
    key: "contract_escrow_released_xlm",
    type: "counter",
    unit: "xlm",
    description: "Cumulative XLM released from escrow to contributors.",
    cardinality: ["ecosystem_id", "asset_code"],
    dashboards: [
      "contract_escrow_health",
      "admin_ecosystem_overview",
      "leaderboard_ranking",
    ],
  },
  {
    key: "contract_escrow_refunded_xlm",
    type: "counter",
    unit: "xlm",
    description: "Cumulative XLM refunded to depositors.",
    cardinality: ["ecosystem_id", "asset_code"],
    dashboards: ["contract_escrow_health"],
  },
  {
    key: "contract_operation_total",
    type: "counter",
    unit: "total",
    description:
      "Total on-chain operations attempted (lock, release, refund, …).",
    cardinality: ["ecosystem_id"],
    dashboards: ["contract_escrow_health"],
  },
  {
    key: "contract_error_total",
    type: "counter",
    unit: "total",
    description: "Total on-chain operations that reverted / errored.",
    cardinality: ["ecosystem_id"],
    dashboards: ["contract_escrow_health", "admin_error_monitor"],
  },
  {
    key: "contract_error_rate_bps",
    type: "rate",
    unit: "bps",
    description: "contract_error_total / contract_operation_total in bps.",
    cardinality: ["ecosystem_id"],
    dashboards: ["contract_escrow_health", "admin_error_monitor"],
  },

  // ── User / contributor domain ───────────────────────────────────────────
  {
    key: "user_contributor_earnings_xlm",
    type: "counter",
    unit: "xlm",
    description: "Cumulative XLM earned by a contributor (used in rankings).",
    cardinality: ["contributor", "ecosystem_id", "period"],
    dashboards: [
      "maintainers_top_contributors",
      "profile_contributor_summary",
      "leaderboard_ranking",
    ],
  },
  {
    key: "user_signin_total",
    type: "counter",
    unit: "total",
    description: "Successful sign-ins (GitHub OAuth + wallet).",
    cardinality: ["period"],
    dashboards: ["admin_ecosystem_overview"],
  },
  {
    key: "user_onboarding_completed_total",
    type: "counter",
    unit: "total",
    description: "New users who finished the onboarding tutorial flow.",
    cardinality: ["period"],
    dashboards: ["admin_ecosystem_overview"],
  },

  // ── System domain ───────────────────────────────────────────────────────
  {
    key: "system_pageview_total",
    type: "counter",
    unit: "total",
    description: "Client-side page view count (sampled, not per-request).",
    cardinality: ["period"],
    dashboards: ["admin_ecosystem_overview"],
  },
  {
    key: "system_error_rate_bps",
    type: "rate",
    unit: "bps",
    description:
      "Global frontend error boundary trigger rate (error events / pageviews).",
    cardinality: ["period"],
    dashboards: ["admin_error_monitor"],
  },

  // ── DB domain (mirrors backend expvar names — DO NOT rename independently)
  {
    key: "db_queries_total",
    type: "counter",
    unit: "total",
    description: "Backend DB queries executed, all durations.",
    cardinality: ["period"],
    dashboards: ["db_observability"],
  },
  {
    key: "db_slow_queries_total",
    type: "counter",
    unit: "total",
    description: "Backend DB queries that exceeded SLOW_QUERY_THRESHOLD_MS.",
    cardinality: ["period"],
    dashboards: ["db_observability", "admin_error_monitor"],
  },
  {
    key: "db_avg_duration_seconds",
    type: "gauge",
    unit: "seconds",
    description: "Rolling average query duration.",
    cardinality: ["period"],
    dashboards: ["db_observability"],
  },
] as const;

/**
 * Readonly registry of every metric that is currently wired up to at
 * least one production dashboard.  Introspect this in tests, or from
 * debug tooling to produce a coverage report.
 *
 * NOTE: even though `key` above is typed as a plain string in each tuple,
 * `METRIC_KEYS` (below) pulls them through as literal types so call
 * sites that pass a metric name get TS validation.
 */
export const METRICS: ReadonlyArray<MetricDefinition> =
  _METRICS as ReadonlyArray<MetricDefinition>;

/** Literal-string union of every registered key — use at call sites. */
export type MetricKey = (typeof _METRICS)[number]["key"];

/** Derived ordered list of literal keys (same order as METRICS). */
export const METRIC_KEYS: ReadonlyArray<MetricKey> = METRICS.map(
  (m) => m.key as MetricKey,
);

// ─── Dashboard → metrics reverse index ──────────────────────────────────────
/**
 * For each dashboard, the list of metrics that MUST be present for the
 * dashboard to render all panels correctly.  Generated from METRICS so
 * there is a single source of truth.
 */
export const DASHBOARD_COVERAGE: Readonly<
  Record<DashboardId, ReadonlyArray<MetricKey>>
> = DASHBOARD_IDS.reduce(
  (acc, id) => {
    acc[id] = METRICS.filter((m) => m.dashboards.includes(id)).map(
      (m) => m.key as MetricKey,
    );
    return acc;
  },
  {} as Record<DashboardId, MetricKey[]>,
);

// ─── Validation helpers (runtime, not just TS) ──────────────────────────────

/**
 * Validate that a candidate metric name:
 *   1. matches the naming regex
 *   2. has a known prefix
 *   3. has a known unit suffix
 *   4. (if `requireRegistered` is true) exists in METRICS
 *
 * Returns `{ valid: true }` or `{ valid: false, reason: string }`.
 * Used both in tests and at debug-page runtime to flag stray metrics.
 */
export function validateMetricName(
  candidate: string,
  opts: { requireRegistered?: boolean } = {},
): { valid: true } | { valid: false; reason: string } {
  if (typeof candidate !== "string" || candidate.length === 0) {
    return { valid: false, reason: "metric name must be a non-empty string" };
  }
  if (candidate !== candidate.toLowerCase()) {
    return {
      valid: false,
      reason: "metric name must be lowercase (snake_case convention)",
    };
  }
  if (!METRIC_NAME_REGEX.test(candidate)) {
    const unitList = METRIC_UNITS.join("|");
    const prefixList = METRIC_PREFIXES.join("|");
    return {
      valid: false,
      reason: `name must match /^(${prefixList})_([a-z0-9_]+)_(${unitList})$/.`,
    };
  }
  const prefix = candidate.slice(0, candidate.indexOf("_")) as MetricPrefix;
  if (!METRIC_PREFIXES.includes(prefix)) {
    return {
      valid: false,
      reason: `prefix "${prefix}" is not in the reserved prefix list.`,
    };
  }
  const lastUnderscore = candidate.lastIndexOf("_");
  const unit = candidate.slice(lastUnderscore + 1) as MetricUnit;
  if (!METRIC_UNITS.includes(unit)) {
    return {
      valid: false,
      reason: `suffix unit "${unit}" is not in METRIC_UNITS.`,
    };
  }
  if (opts.requireRegistered && !METRIC_KEYS.includes(candidate as MetricKey)) {
    return {
      valid: false,
      reason: `"${candidate}" is not in METRICS registry — register it first or remove requireRegistered.`,
    };
  }
  return { valid: true };
}

/**
 * Return the list of metrics that are registered but NOT claimed by any
 * dashboard (potential dead code, or dashboards that haven't been wired
 * yet).  Clean dashboard coverage should leave this empty.
 */
export function findOrphanMetrics(): ReadonlyArray<MetricKey> {
  return METRICS.filter((m) => m.dashboards.length === 0).map(
    (m) => m.key as MetricKey,
  );
}

/**
 * Return the list of dashboards that reference zero metrics (usually a
 * sign the dashboard was added to DASHBOARD_IDS but its panels were
 * never mapped into the registry).
 */
export function findEmptyDashboards(): ReadonlyArray<DashboardId> {
  return (Object.keys(DASHBOARD_COVERAGE) as DashboardId[]).filter(
    (id) => DASHBOARD_COVERAGE[id].length === 0,
  );
}

/**
 * Look up a MetricDefinition by key, or throw if not registered.
 * Safe for runtime use; the test suite ensures all call-site strings
 * are valid MetricKey values (so this never throws in practice).
 */
export function getMetricDefinition(key: MetricKey): MetricDefinition {
  const def = METRICS.find((m) => m.key === key);
  if (!def) {
    throw new Error(
      `[businessMetrics] "${key}" is not a registered metric.  Add it to METRICS in src/metrics/businessMetrics.ts.`,
    );
  }
  return def;
}

// ─── In-memory tracking (dev / debug helper — production uses real sinks) ───
/**
 * Lightweight, append-only in-memory counter store.  Used in:
 *   - unit tests to verify a metric was incremented
 *   - local dev dashboard to sanity-check coverage without a real sink
 *
 * NOT for production: in prod the same keys are pushed to OpenTelemetry
 * / Prometheus / the backend event collector.  The keys are the same.
 */
const _store: Partial<Record<MetricKey, number>> = {};

/** Reset the in-memory store — call between test cases only. */
export function _resetMetricStoreForTests(): void {
  for (const k of Object.keys(_store)) delete _store[k as MetricKey];
}

/** Read the current accumulated value for a registered metric (0 if absent). */
export function readMetric(key: MetricKey): number {
  return _store[key] ?? 0;
}

/**
 * Add `delta` (default 1) to a counter-style metric.  Validates the key
 * at runtime so typos surface.  For histogram / gauge-style metrics use
 * `setMetric` directly (there is no rate-limiting in the memory store).
 */
export function incrementMetric(key: MetricKey, delta: number = 1): void {
  const def = getMetricDefinition(key);
  if (delta < 0) {
    // Counters are append-only; we still allow negative for tests that
    // simulate a reset between runs, but warn via the type-checker by
    // documenting the invariant here.  See test "counters are append-only".
    if (def.type === "counter") {
      // no-op in prod; tests pin the behaviour.
    }
  }
  _store[key] = (readMetric(key) + delta) | 0;
}

/** Overwrite a gauge-style metric with `value`. */
export function setMetric(key: MetricKey, value: number): void {
  getMetricDefinition(key); // ensure key is known
  _store[key] = value | 0;
}

/**
 * Derived helper: applied→paid funnel rate in basis points, safe-div.
 * Mirrors the on-chain error_rate calculation to keep bps semantics
 * consistent across the stack.
 */
export function computeFunnelConversionBps(
  applied: number,
  paid: number,
): number {
  if (applied <= 0) return 0;
  return Math.max(0, Math.min(10_000, Math.floor((paid * 10_000) / applied)));
}
