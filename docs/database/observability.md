# Database Observability

## Slow Query Logging

The backend instruments every PostgreSQL query via a `pgx.QueryTracer` attached to the connection pool. When a query's wall-clock duration exceeds the configured threshold a structured `WARN` log entry is emitted.

### Configuration

| Environment variable | Type | Default | Description |
|---|---|---|---|
| `SLOW_QUERY_THRESHOLD_MS` | integer (ms) | `500` | Minimum duration that triggers a slow-query log. Set to `0` to disable log emission while keeping metrics active. |

### Log format

```json
{
  "time": "2026-04-24T12:00:00Z",
  "level": "WARN",
  "msg": "slow query detected",
  "duration_ms": 823,
  "threshold_ms": 500,
  "sql": "SELECT * FROM contributions WHERE user_id = $1"
}
```

- `sql` — the query template only. **Bind parameters are never logged** to prevent sensitive data (PII, secrets) from appearing in log streams.
- `error` — present only when the query also returned an error.

### Security assumptions

- Query arguments (`Args`) are captured by pgx in `TraceQueryStartData` but the tracer discards them immediately; they are never stored or forwarded.
- The SQL template itself may contain table/column names. Avoid embedding literal values directly in SQL strings.
- `maskDBURL` redacts the password from the connection string before it is logged at startup.

## Metrics

In-process counters are exported via Go's standard `expvar` package and are available at `GET /debug/vars` (net/http default mux, enabled in dev; gate behind auth in production).

| Metric | Type | Description |
|---|---|---|
| `db_queries_total` | counter | Total queries executed (all durations). |
| `db_slow_queries_total` | counter | Queries that exceeded the threshold. |
| `db_query_duration_ms_total` | counter | Cumulative query duration in milliseconds. |

Derived rate (example with any scraper):

```
slow_query_rate = db_slow_queries_total / db_queries_total
avg_duration_ms = db_query_duration_ms_total / db_queries_total
```

### Disabling slow-query logging (metrics only)

```bash
SLOW_QUERY_THRESHOLD_MS=0
```

All three counters still increment; only the `WARN` log is suppressed.

## Implementation notes

- **Zero extra dependencies** — uses `expvar` (stdlib) and `log/slog` (stdlib).
- **pgx tracer interface** — `SlowQueryTracer` implements `pgx.QueryTracer` (`TraceQueryStart` / `TraceQueryEnd`). It is attached via `pgxpool.Config.ConnConfig.Tracer`.
- **Atomic disabled flag** — the hot path checks an `atomic.Bool` to avoid allocations when logging is disabled.
- **Context propagation** — start time is stored in the request context between the two tracer calls; no global state per query.
