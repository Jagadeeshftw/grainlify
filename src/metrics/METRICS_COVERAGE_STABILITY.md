# Monitoring Metrics Coverage - Stability and Regression Safety

This document describes the stabilized behavior of the monitoring metrics coverage system in `src/metrics/businessMetrics.ts`. It documents edge cases, deterministic behavior guarantees, and regression-safe patterns.

## Deterministic Output Guarantees

The `BusinessMetricsRegistry.getMetrics()` method now returns metrics in a deterministic order:

1. **Metric names are sorted alphabetically** - All counters, gauges, and histograms are exported in sorted order by their full metric name
2. **Label combinations are sorted alphabetically** - Within each metric, label combinations are sorted by their serialized label string
3. **Label keys are sorted during serialization** - When serializing labels, keys are sorted alphabetically to ensure consistent ordering

This ensures that:
- Multiple calls to `getMetrics()` return the same array order for the same registry state
- Test snapshots remain stable across reruns
- Dashboard queries receive consistent ordering for visualization

## Edge Case Behavior

### Auto-Registration

When a metric is recorded (incremented, set, or histogram-recorded) without prior registration:

- The metric is automatically registered with a default help text: `"<type> for <name>"`
- The metric name is validated against naming conventions
- For counters: must end with `_total`
- For histograms: must end with `_seconds`, `_ms`, `_bytes`, or `_duration`
- Gauges have no suffix requirement

**Example:**
```typescript
registry.incrementCounter('custom_metric_total', 5);
// Auto-registers with help: "Counter for custom_metric_total"
```

### Prefix Handling

The registry prefix is applied only if the metric name does not already start with the prefix:

```typescript
const registry = new BusinessMetricsRegistry({ prefix: 'app_' });
registry.incrementCounter('test_total', 1);           // Becomes 'app_test_total'
registry.incrementCounter('app_test_total', 1);      // Stays 'app_test_total' (no double prefix)
```

### Label Value Truncation

Label values longer than `maxLabelValueLength` (default: 64) are truncated with an ellipsis suffix:

```typescript
registry.incrementCounter('test_total', 1, { long: 'x'.repeat(100) });
// Label value becomes: 'xxxxxxxx...(64 chars)...'
```

### Cardinality Overflow Protection

When the number of unique label combinations for a metric exceeds `maxCardinalityPerMetric` (default: 500):

- **Counters**: New combinations are routed to an overflow bucket with label `overflow=true`
- **Gauges**: New combinations are silently dropped
- **Histograms**: New combinations are silently dropped

The overflow key format is hardcoded as `'overflow=true'` for consistency.

### Empty Label Sets

Empty label objects `{}` and no labels argument are treated identically:

```typescript
registry.incrementCounter('test_total', 1, {});
registry.incrementCounter('test_total', 1);
// Both increment the same metric with empty labels
```

### Label Key Sanitization

Label keys are sanitized to match `^[a-zA-Z_][a-zA-Z0-9_]*$`:

- Special characters are replaced with underscores
- Keys starting with numbers are prefixed with underscore
- Empty keys become `'unknown'`

**Examples:**
```typescript
'user-id'        → 'user_id'
'http.status'    → 'http_status'
'123key'         → '_123key'
''               → 'unknown'
```

### Label Value Handling

Label values are sanitized as follows:

- `null` and `undefined` → `'unknown'`
- Empty string → `'unknown'`
- Non-string values → converted to string via `String(val)`
- Whitespace-only strings → trimmed, becomes `'unknown'` if empty after trim

### Singleton Registry State

The `defaultBusinessRegistry` is a singleton that persists state across the application lifecycle.

**For test environments:**
- Use `resetDefaultRegistry()` to clear state between tests
- This function only resets when `NODE_ENV === 'test'`
- In production, calling `resetDefaultRegistry()` has no effect

**Example:**
```typescript
import { resetDefaultRegistry } from './businessMetrics';

beforeEach(() => {
  process.env.NODE_ENV = 'test';
  resetDefaultRegistry();
});
```

## Naming Conventions

### Counter Metrics
- Must end with `_total` suffix
- Regex: `^[a-zA-Z_][a-zA-Z0-9_]*_total$`
- Examples: `grainlify_bounty_created_total`, `grainlify_rate_limit_exceeded_total`

### Histogram Metrics
- Must end with one of: `_seconds`, `_ms`, `_bytes`, `_duration`
- Regex: `^[a-zA-Z_][a-zA-Z0-9_]*(_seconds|_ms|_bytes|_duration)$`
- Examples: `grainlify_http_request_duration_seconds`, `grainlify_payload_size_bytes`

### Gauge Metrics
- No required suffix
- Regex: `^[a-zA-Z_][a-zA-Z0-9_]*$`
- Examples: `grainlify_active_users_gauge`, `grainlify_memory_usage_ratio`

## Pre-Registered Metrics

The default registry pre-registers the following application metrics:

1. `grainlify_bounty_created_total` - Total bounties created
2. `grainlify_bounty_settled_total` - Total bounties settled
3. `grainlify_escrow_locked_bytes` - Total escrow funds locked in bytes/subunits
4. `grainlify_http_request_duration_seconds` - HTTP request duration in seconds
5. `grainlify_active_users_gauge` - Current active users count
6. `grainlify_rate_limit_exceeded_total` - Total rate limit exceeded events

## Regression Safety

### What Changes Would Break Backward Compatibility?

1. **Changing the overflow key format** - Any change from `'overflow=true'` would break dashboard queries expecting this format
2. **Removing auto-registration** - Code relying on implicit registration would fail
3. **Changing label serialization order** - Would break deterministic output guarantees
4. **Changing prefix handling logic** - Could cause duplicate prefixes or missing prefixes
5. **Changing default bucket values** - Would affect histogram bucket boundaries

### Safe Changes

The following can be changed without breaking backward compatibility:

1. **Adding new pre-registered metrics** - Existing metrics remain unchanged
2. **Increasing default cardinality limits** - Only affects overflow behavior at higher scales
3. **Changing default help text** - Help text is not used in metric serialization
4. **Adding new metric types** - Existing types continue to work

## Testing Coverage

The test suite in `businessMetrics.test.ts` covers:

- Naming convention validation for all metric types
- Label key and value sanitization
- Cardinality overflow protection
- Deterministic output ordering
- Auto-registration behavior
- Prefix handling edge cases
- Singleton registry reset behavior
- NaN/Infinity/negative value handling
- Registry reset functionality

## Production Visibility

For dashboard coverage:

1. **Counter metrics** - Use for cumulative counts (events, totals)
2. **Gauge metrics** - Use for point-in-time values (current users, memory)
3. **Histogram metrics** - Use for distributions (latency, payload sizes)

All metrics exported by `getMetrics()` include:
- `name`: Full metric name with prefix
- `type`: Metric type (counter/gauge/histogram)
- `value`: Numeric value (or sum/count/buckets for histograms)
- `labels`: Sanitized label key-value pairs
- `timestamp`: Unix timestamp in milliseconds

## Migration Guide

If you need to change metric names or labels:

1. **Add new metric** - Register with new name
2. **Dual-write** - Record to both old and new metrics temporarily
3. **Update dashboards** - Migrate queries to use new metric
4. **Remove old metric** - After dashboard migration is complete
