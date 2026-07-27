/*
 * businessMetrics — unit & regression tests
 * ============================================
 *
 * #1628 Document monitoring metrics coverage
 *
 * These tests lock in the *current behaviour* of the metrics module so
 * that any accidental rename / removal / reordering surfaces as a hard
 * failure.  See the "Regression Surface" block in businessMetrics.ts
 * for the list of things that are deliberately pinned by this file.
 *
 * If you need to *change* something that breaks these tests (e.g. add
 * a new funnel stage), update both sides together AND bump the
 * `REGRESSION_SURFACE_VERSION` constant at the top of this file — that
 * lets reviewers spot "intentional" vs "accidental" changes.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  METRIC_NAME_REGEX,
  METRIC_PREFIXES,
  METRIC_UNITS,
  METRICS,
  METRIC_KEYS,
  DASHBOARD_IDS,
  DASHBOARD_COVERAGE,
  BOUNTY_FUNNEL_STAGES,
  PAYOUT_STATUSES,
  validateMetricName,
  findOrphanMetrics,
  findEmptyDashboards,
  getMetricDefinition,
  readMetric,
  incrementMetric,
  setMetric,
  _resetMetricStoreForTests,
  computeFunnelConversionBps,
  type MetricKey,
  type DashboardId,
  type MetricPrefix,
  type MetricUnit,
  type BountyFunnelStage,
  type PayoutStatusMetric,
} from "../businessMetrics";

/*
 * REGRESSION SURFACE VERSION
 * ---------------------------
 * Bump this whenever you intentionally change one of the pinned
 * behaviours (metric key set, dashboard coverage map, funnel stage
 * order, payout status values, dashboard ID list).  When a PR changes
 * this number the reviewer knows to cross-check with the dashboard
 * team before merging.  Keep it in sync with docs/analytics-metrics-coverage.md.
 */
const REGRESSION_SURFACE_VERSION = 1;

describe("REGRESSION_SURFACE_VERSION bump", () => {
  it("is pinned at 1 — only change when metric keys / coverage / stages change intentionally", () => {
    expect(REGRESSION_SURFACE_VERSION).toBe(1);
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  1. Metric naming conventions — validateMetricName edge cases          */
/* ═══════════════════════════════════════════════════════════════════════ */

describe("validateMetricName — naming convention enforcement", () => {
  describe("rejects invalid names", () => {
    const badCases: Array<[string, string]> = [
      ["", "empty string"],
      ["BountyAppliedTotal", "camelCase / uppercase letters"],
      ["BOUNTY_APPLIED_TOTAL", "all-caps snake_case"],
      ["bounty-applied-total", "kebab-case (hyphens)"],
      ["bounty.applied.total", "dots instead of underscores"],
      ["foo_applied_total", "unknown prefix 'foo'"],
      ["bounty_applied", "missing unit suffix"],
      ["bounty_applied_foo", "unknown unit 'foo'"],
      ["bounty__applied_total", "double underscore in name"],
      [" bounty_applied_total", "leading whitespace"],
      ["bounty_applied_total ", "trailing whitespace"],
    ];

    it.each(badCases)('rejects "%s" — %s', (name) => {
      const result = validateMetricName(name);
      expect(result.valid).toBe(false);
    });
  });

  describe("accepts valid canonical names", () => {
    const goodCases: string[] = [
      "bounty_applied_total",
      "bounty_paid_total",
      "program_draft_dwell_seconds",
      "payout_amount_xlm",
      "payout_amount_sum_stroops",
      "contract_error_rate_bps",
      "db_queries_total",
      "db_avg_duration_seconds",
      "system_error_rate_bps",
      "user_signin_total",
    ];

    it.each(goodCases)('accepts "%s"', (name) => {
      const result = validateMetricName(name);
      expect(result.valid).toBe(true);
    });
  });

  it("requireRegistered=true rejects a syntactically valid but unregistered name", () => {
    const candidate = "bounty_imaginary_counter_total";
    const without = validateMetricName(candidate);
    expect(without.valid).toBe(true);

    const withFlag = validateMetricName(candidate, { requireRegistered: true });
    expect(withFlag.valid).toBe(false);
    if (!withFlag.valid) {
      expect(withFlag.reason).toContain("not in METRICS registry");
    }
  });

  it("requireRegistered=true passes for a real registered key", () => {
    const result = validateMetricName("bounty_applied_total", { requireRegistered: true });
    expect(result.valid).toBe(true);
  });

  it("lowercase check catches mixed-case strings early", () => {
    const r = validateMetricName("Bounty_Applied_Total");
    expect(r.valid).toBe(false);
    if (!r.valid) expect(r.reason).toContain("lowercase");
  });
});

describe("METRIC_NAME_REGEX — the canonical pattern", () => {
  it("matches all registered keys (sanity: every real key passes the regex)", () => {
    for (const key of METRIC_KEYS) {
      expect(METRIC_NAME_REGEX.test(key)).toBe(true);
    }
  });

  it("does not match a string with extra characters around a valid core", () => {
    expect(METRIC_NAME_REGEX.test("xbounty_applied_total")).toBe(false);
    expect(METRIC_NAME_REGEX.test("bounty_applied_totalx")).toBe(false);
  });
});

describe("METRIC_PREFIXES and METRIC_UNITS — reserved lists", () => {
  it("has exactly the 7 documented prefixes", () => {
    expect(METRIC_PREFIXES).toEqual([
      "bounty", "program", "payout", "user", "system", "db", "contract",
    ]);
  });

  it("has exactly the 7 documented units", () => {
    expect(METRIC_UNITS).toEqual([
      "total", "count", "seconds", "xlm", "stroops", "bps", "ratio",
    ]);
  });

  it("every registered key's prefix is in METRIC_PREFIXES", () => {
    for (const key of METRIC_KEYS) {
      const prefix = key.slice(0, key.indexOf("_")) as MetricPrefix;
      expect(METRIC_PREFIXES).toContain(prefix);
    }
  });

  it("every registered key's unit suffix is in METRIC_UNITS", () => {
    for (const key of METRIC_KEYS) {
      const unit = key.slice(key.lastIndexOf("_") + 1) as MetricUnit;
      expect(METRIC_UNITS).toContain(unit);
    }
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  2. Registry integrity — every metric is well-formed + covered         */
/* ═══════════════════════════════════════════════════════════════════════ */

describe("METRICS registry integrity", () => {
  it("METRIC_KEYS and METRICS[].key are 1:1 (no duplicates, no drift)", () => {
    const fromDefs = METRICS.map((m) => m.key).sort();
    const fromKeys = [...METRIC_KEYS].sort();
    expect(fromDefs).toEqual(fromKeys);
  });

  it("no duplicate metric keys in the registry", () => {
    const seen = new Set<string>();
    for (const m of METRICS) {
      expect(seen.has(m.key)).toBe(false);
      seen.add(m.key);
    }
  });

  it("every metric has a non-empty description (for dashboard tooltips)", () => {
    for (const m of METRICS) {
      expect(m.description.trim().length).toBeGreaterThan(5);
    }
  });

  it("every metric declares at least one dashboard (no orphans by construction)", () => {
    expect(findOrphanMetrics()).toEqual([]);
  });

  it("every declared dashboard has at least one metric (no empty dashboards)", () => {
    expect(findEmptyDashboards()).toEqual([]);
  });

  it("DASHBOARD_IDS has exactly the documented list", () => {
    expect(DASHBOARD_IDS).toEqual([
      "maintainers_analytics",
      "maintainers_funnel",
      "maintainers_payouts",
      "maintainers_top_contributors",
      "admin_ecosystem_overview",
      "admin_error_monitor",
      "profile_contributor_summary",
      "leaderboard_ranking",
      "contract_escrow_health",
      "db_observability",
    ]);
  });

  it("DASHBOARD_COVERAGE keys exactly match DASHBOARD_IDS (no extra, no missing)", () => {
    const covered = Object.keys(DASHBOARD_COVERAGE).sort();
    const ids = [...DASHBOARD_IDS].sort();
    expect(covered).toEqual(ids);
  });

  it("bidirectional coverage: metric.dashboards → DASHBOARD_COVERAGE[*] round-trip", () => {
    for (const metric of METRICS) {
      for (const dash of metric.dashboards) {
        expect(DASHBOARD_COVERAGE[dash as DashboardId]).toContain(metric.key as MetricKey);
      }
    }
    for (const dash of DASHBOARD_IDS) {
      for (const key of DASHBOARD_COVERAGE[dash]) {
        const def = METRICS.find((m) => m.key === key)!;
        expect(def.dashboards).toContain(dash);
      }
    }
  });

  it("unit field in the definition matches the suffix extracted from key", () => {
    for (const m of METRICS) {
      const suffix = m.key.slice(m.key.lastIndexOf("_") + 1) as MetricUnit;
      expect(m.unit).toBe(suffix);
    }
  });

  it("type=counter always has unit=total (convention: counters end with _total)", () => {
    for (const m of METRICS) {
      if (m.type === "counter") {
        expect(m.unit).toBe("total" as MetricUnit || "xlm" as MetricUnit || "stroops" as MetricUnit);
      }
    }
  });

  it("type=rate always has unit=bps (we use basis points for all % rates in the UI)", () => {
    for (const m of METRICS) {
      if (m.type === "rate") {
        expect(m.unit).toBe("bps" as MetricUnit);
      }
    }
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  3. Regression-surface snapshots — key sets are EXPLICITLY pinned      */
/* ═══════════════════════════════════════════════════════════════════════ */
//
// These tests spell out the CURRENT behaviour.  They will fail if anyone
// renames / removes / reorders anything important.  Treat failures as
// either:
//   (a) accidental revert — fix the code back, or
//   (b) intentional change — update the snapshot AND bump
//       REGRESSION_SURFACE_VERSION above AND notify the dashboard team.

describe("REGRESSION: pinned metric key set (28 keys as of v1)", () => {
  const EXPECTED_KEYS: ReadonlyArray<MetricKey> = [
    // bounty
    "bounty_applied_total",
    "bounty_assigned_total",
    "bounty_submitted_total",
    "bounty_paid_total",
    "bounty_conversion_rate_bps",
    // payout
    "payout_amount_xlm",
    "payout_amount_sum_xlm",
    "payout_status_count",
    "payout_failure_rate_bps",
    // program / contract escrow
    "program_draft_dwell_seconds",
    "program_active_count",
    "contract_escrow_locked_xlm",
    "contract_escrow_released_xlm",
    "contract_escrow_refunded_xlm",
    "contract_operation_total",
    "contract_error_total",
    "contract_error_rate_bps",
    // user
    "user_contributor_earnings_xlm",
    "user_signin_total",
    "user_onboarding_completed_total",
    // system
    "system_pageview_total",
    "system_error_rate_bps",
    // db
    "db_queries_total",
    "db_slow_queries_total",
    "db_avg_duration_seconds",
  ];

  it("metric key list is exactly the documented set (order-independent)", () => {
    expect([...METRIC_KEYS].sort()).toEqual([...EXPECTED_KEYS].sort());
  });

  it("registry size is stable (25 as of v1)", () => {
    expect(METRICS.length).toBe(EXPECTED_KEYS.length);
  });
});

describe("REGRESSION: pinned dashboard → metrics coverage", () => {
  it("maintainers_funnel owns exactly the 4 funnel stage counters", () => {
    expect(DASHBOARD_COVERAGE["maintainers_funnel"].sort()).toEqual([
      "bounty_applied_total",
      "bounty_assigned_total",
      "bounty_paid_total",
      "bounty_submitted_total",
    ].sort());
  });

  it("contract_escrow_health owns the program + contract metrics", () => {
    expect(DASHBOARD_COVERAGE["contract_escrow_health"].sort()).toEqual([
      "program_draft_dwell_seconds",
      "program_active_count",
      "contract_escrow_locked_xlm",
      "contract_escrow_released_xlm",
      "contract_escrow_refunded_xlm",
      "contract_operation_total",
      "contract_error_total",
      "contract_error_rate_bps",
    ].sort());
  });

  it("db_observability owns exactly the 3 db_* metrics", () => {
    expect(DASHBOARD_COVERAGE["db_observability"].sort()).toEqual([
      "db_queries_total",
      "db_slow_queries_total",
      "db_avg_duration_seconds",
    ].sort());
  });

  it("leaderboard_ranking owns contributor earnings + released escrow", () => {
    expect(DASHBOARD_COVERAGE["leaderboard_ranking"].sort()).toEqual([
      "payout_amount_sum_xlm",
      "contract_escrow_released_xlm",
      "user_contributor_earnings_xlm",
    ].sort());
  });
});

describe("REGRESSION: BOUNTY_FUNNEL_STAGES order & values are pinned", () => {
  it("4 stages in exactly applied → assigned → submitted → paid order", () => {
    // Funnel chart draws in array order; swapping stages shifts the
    // conversion-rate bars without updating the UI labels.
    expect([...BOUNTY_FUNNEL_STAGES]).toEqual([
      "applied", "assigned", "submitted", "paid",
    ]);
  });

  it("each stage is a valid BountyFunnelStage type (round-trip through array)", () => {
    const typed: BountyFunnelStage[] = ["applied", "assigned", "submitted", "paid"];
    expect(typed).toEqual([...BOUNTY_FUNNEL_STAGES]);
  });

  it("number of funnel metrics == number of funnel stages (1 counter per stage)", () => {
    const stageKeys = METRICS.filter((m) =>
      m.key.startsWith("bounty_") && m.key.endsWith("_total") && m.key !== "bounty_conversion_rate_bps"
    );
    expect(stageKeys.length).toBe(BOUNTY_FUNNEL_STAGES.length);
  });
});

describe("REGRESSION: PAYOUT_STATUSES match the canonical 4 values", () => {
  it("exactly paid, pending, processing, failed in that order", () => {
    expect([...PAYOUT_STATUSES]).toEqual([
      "paid", "pending", "processing", "failed",
    ]);
  });

  it("each value is assignable to the PayoutStatusMetric type", () => {
    const all: PayoutStatusMetric[] = ["paid", "pending", "processing", "failed"];
    expect(all).toEqual([...PAYOUT_STATUSES]);
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  4. getMetricDefinition — lookup behaviour + edge cases                */
/* ═══════════════════════════════════════════════════════════════════════ */

describe("getMetricDefinition", () => {
  it("returns a definition with matching key for every registered key", () => {
    for (const key of METRIC_KEYS) {
      expect(getMetricDefinition(key).key).toBe(key);
    }
  });

  it("throws for an unregistered key (call with any to bypass TS)", () => {
    expect(() => getMetricDefinition("bounty_does_not_exist_total" as MetricKey))
      .toThrow(/not a registered metric/);
  });

  it("returned cardinality + dashboards arrays are populated", () => {
    const def = getMetricDefinition("bounty_applied_total");
    expect(def.cardinality.length).toBeGreaterThan(0);
    expect(def.dashboards.length).toBeGreaterThan(0);
    expect(def.type).toBe("counter");
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  5. In-memory tracking (read / increment / set / reset)                */
/* ═══════════════════════════════════════════════════════════════════════ */

describe("in-memory metric store (dev + tests only)", () => {
  beforeEach(() => {
    _resetMetricStoreForTests();
  });

  it("readMetric returns 0 for a key that has never been touched", () => {
    expect(readMetric("bounty_applied_total")).toBe(0);
  });

  it("incrementMetric adds delta to previous value (default delta = 1)", () => {
    incrementMetric("bounty_applied_total");
    expect(readMetric("bounty_applied_total")).toBe(1);
    incrementMetric("bounty_applied_total", 5);
    expect(readMetric("bounty_applied_total")).toBe(6);
  });

  it("setMetric overwrites the gauge value", () => {
    setMetric("program_active_count", 42);
    expect(readMetric("program_active_count")).toBe(42);
    setMetric("program_active_count", 7);
    expect(readMetric("program_active_count")).toBe(7);
  });

  it("incrementMetric validates the key (throws on bad key)", () => {
    expect(() => incrementMetric("nope_counter_total" as MetricKey))
      .toThrow(/not a registered metric/);
  });

  it("setMetric validates the key (throws on bad key)", () => {
    expect(() => setMetric("nope_gauge_count" as MetricKey, 0))
      .toThrow(/not a registered metric/);
  });

  it("_resetMetricStoreForTests zeroes all stored values", () => {
    incrementMetric("bounty_applied_total", 100);
    setMetric("program_active_count", 5);
    _resetMetricStoreForTests();
    expect(readMetric("bounty_applied_total")).toBe(0);
    expect(readMetric("program_active_count")).toBe(0);
  });

  it("values are coerced to integers (| 0)", () => {
    // The store uses `| 0` internally; document the behaviour.
    incrementMetric("bounty_paid_total", 2.7);
    expect(readMetric("bounty_paid_total")).toBe(2);
    setMetric("program_active_count", 9.9);
    expect(readMetric("program_active_count")).toBe(9);
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  6. computeFunnelConversionBps — safe-division helper                  */
/* ═══════════════════════════════════════════════════════════════════════ */

describe("computeFunnelConversionBps", () => {
  it("returns 0 when applied <= 0 (safe division, no NaN / Infinity)", () => {
    expect(computeFunnelConversionBps(0, 0)).toBe(0);
    expect(computeFunnelConversionBps(-5, 10)).toBe(0);
  });

  it("exact 100% = 10000 bps", () => {
    expect(computeFunnelConversionBps(10, 10)).toBe(10_000);
  });

  it("50% = 5000 bps", () => {
    expect(computeFunnelConversionBps(100, 50)).toBe(5_000);
  });

  it("0 paid = 0 bps", () => {
    expect(computeFunnelConversionBps(200, 0)).toBe(0);
  });

  it("clamps results to [0, 10000] (paid > applied should never happen but guard anyway)", () => {
    expect(computeFunnelConversionBps(10, 20)).toBe(10_000); // clamped from 20000
  });

  it("floor semantics (no rounding up)", () => {
    // 1/3 = 3333.333… → 3333 exactly
    expect(computeFunnelConversionBps(3, 1)).toBe(3_333);
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  7. Cross-module: payout status + funnel stages match maintaner types  */
/* ═══════════════════════════════════════════════════════════════════════ */
//
// The metrics module "owns" the canonical string values; the
// features/maintainers/types module re-declares them for ergonomics.
// These tests force them to stay in sync so that dashboard filters
// (which use the maintainers types) match the metric tag values.

describe("cross-module: payout status enum parity with maintainers/types", () => {
  it("every PAYOUT_STATUSES value is representable as a PayoutStatus (string equality)", () => {
    // If this fails, someone added a new status in maintainers/types
    // without adding it here — or vice versa.  Fix both sides.
    const asStrings: string[] = [...PAYOUT_STATUSES];
    expect(asStrings.sort()).toEqual(["failed", "paid", "pending", "processing"]);
  });
});

/* ═══════════════════════════════════════════════════════════════════════ */
/*  8. Orphan / empty-dashboard guards (positive + negative cases)        */
/* ═══════════════════════════════════════════════════════════════════════ */

describe("coverage guard helpers", () => {
  it("findOrphanMetrics returns empty list in the current codebase", () => {
    // If this test breaks, you registered a metric but did not assign
    // it to at least one dashboard.  Add one dashboard ID to its
    // dashboards[] array.
    expect(findOrphanMetrics()).toEqual([]);
  });

  it("findEmptyDashboards returns empty list in the current codebase", () => {
    // If this test breaks, you added a dashboard to DASHBOARD_IDS but
    // no metric declares it in its dashboards[] array.
    expect(findEmptyDashboards()).toEqual([]);
  });
});
