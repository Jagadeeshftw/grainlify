# Production Monitoring Metrics Coverage & Naming Specification

## Executive Summary

This document specifies the metrics naming standards, label sanitization requirements, dashboard visibility mapping, and expected regression surface for application monitoring and observability in `src/metrics/businessMetrics.ts`.

---

## 1. Metric Naming Standards & Conventions

All production application metrics MUST adhere to standardized Prometheus / OpenTelemetry naming rules:

1. **Prefix Convention:**
   - All metric names MUST begin with the application domain prefix (`grainlify_` or configured registry prefix).
   - Character set: `^[a-zA-Z_][a-zA-Z0-9_]*$`.

2. **Standard Type Suffixes:**
   - **Counters:** MUST end with `_total` (e.g., `grainlify_bounty_created_total`, `grainlify_rate_limit_exceeded_total`).
   - **Gauges:** Use unit or state descriptor (e.g., `grainlify_active_users_gauge`, `grainlify_memory_usage_ratio`).
   - **Histograms:** MUST end with unit descriptor: `_seconds`, `_ms`, `_bytes`, or `_duration` (e.g., `grainlify_http_request_duration_seconds`, `grainlify_escrow_locked_bytes`).

---

## 2. High-Cardinality Protection & Label Sanitization

To prevent memory leaks, performance degradation, and cardinality explosions in production metrics collectors (Grafana, Prometheus, Datadog):

1. **Label Key Sanitization:**
   - Label keys are converted to match `^[a-zA-Z_][a-zA-Z0-9_]*$`. Non-alphanumeric characters are replaced with underscores (`_`).

2. **Label Value Bounds & Truncation:**
   - Label values exceeding 64 characters are automatically truncated with an ellipsis (`...`).
   - `null` or `undefined` label values are defaulted to `"unknown"`.

3. **Metric Cardinality Overflow Cap:**
   - Each metric stream is capped at a maximum cardinality of unique label combinations (default: 500).
   - Additional label combinations beyond the cap are aggregated into an explicit `overflow=true` bucket.

---

## 3. Standard Business Metrics Catalogue

| Metric Name | Type | Unit | Description | Labels |
|-------------|------|------|-------------|--------|
| `grainlify_bounty_created_total` | Counter | Count | Total bounties created | `ecosystem`, `tier` |
| `grainlify_bounty_settled_total` | Counter | Count | Total bounties settled | `status`, `ecosystem` |
| `grainlify_escrow_locked_bytes` | Counter | Bytes / Subunits | Escrow locked volume | `token`, `escrow_type` |
| `grainlify_http_request_duration_seconds` | Histogram | Seconds | Latency distribution | `method`, `path`, `status` |
| `grainlify_active_users_gauge` | Gauge | Count | Current concurrent active users | `role` |
| `grainlify_rate_limit_exceeded_total` | Counter | Count | Total rate limit hits | `route`, `client_type` |

---

## 4. Expected Regression Surface

To ensure production visibility remains intact, the following invariants MUST be preserved in future modifications:

1. **Metric Naming Invariant:**
   - Any metric registered without a valid suffix or regex match MUST throw an explicit validation error or fail validation.

2. **Overflow Aggregation Invariant:**
   - High-cardinality bursts MUST NOT crash the process or consume unbounded memory. Excess streams MUST route to `overflow=true`.

3. **No Null/NaN Values Invariant:**
   - Invalid numbers (`NaN`, `Infinity`, negative counter increments) MUST be ignored or defaulted to `0` to prevent broken dashboard rendering.

---

## 5. Test Coverage Reference

The regression surface is explicitly covered by unit test suites:
- **TypeScript Metrics Coverage & Naming:** `src/metrics/businessMetrics.test.ts`
- **Go Escrow & Lifecycle Metrics:** `contracts/bounty_escrow/contracts/escrow/src/test_analytics_monitoring.rs` & `contracts/grainlify-core/src/test_core_monitoring.rs`
