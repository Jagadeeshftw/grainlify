package db

import (
	"context"
	"expvar"
	"log/slog"
	"sync/atomic"
	"time"

	"github.com/jackc/pgx/v5"
)

// Metrics are exported via expvar at /debug/vars (no extra deps).
var (
	metricSlowQueryTotal    = expvar.NewInt("db_slow_queries_total")
	metricQueryDurationSum  = expvar.NewInt("db_query_duration_ms_total") // cumulative ms
	metricQueryCount        = expvar.NewInt("db_queries_total")
)

// queryKey is the context key used to pass start time between TraceQueryStart
// and TraceQueryEnd.
type queryKey struct{}

// SlowQueryTracer implements pgx.QueryTracer. It logs queries whose duration
// exceeds the configured threshold and updates in-process expvar counters.
// Raw SQL parameters are never logged to prevent sensitive-data leakage.
type SlowQueryTracer struct {
	// ThresholdMS is the minimum query duration (milliseconds) that triggers a
	// slow-query log entry. Zero or negative disables slow-query logging while
	// still accumulating metrics.
	ThresholdMS int64
	// disabled is set atomically when ThresholdMS <= 0 so the hot path avoids
	// any allocation.
	disabled atomic.Bool
}

// NewSlowQueryTracer returns a tracer ready for use. thresholdMS <= 0 disables
// slow-query log emission (metrics are still collected).
func NewSlowQueryTracer(thresholdMS int64) *SlowQueryTracer {
	t := &SlowQueryTracer{ThresholdMS: thresholdMS}
	if thresholdMS <= 0 {
		t.disabled.Store(true)
	}
	return t
}

type queryStartData struct {
	start time.Time
	sql   string // stored only for the log message; never includes args
}

// TraceQueryStart records the start time and SQL text (no args).
func (t *SlowQueryTracer) TraceQueryStart(ctx context.Context, _ *pgx.Conn, data pgx.TraceQueryStartData) context.Context {
	return context.WithValue(ctx, queryKey{}, queryStartData{
		start: time.Now(),
		sql:   data.SQL,
	})
}

// TraceQueryEnd measures duration, updates metrics, and emits a slow-query log
// when the threshold is exceeded.
func (t *SlowQueryTracer) TraceQueryEnd(ctx context.Context, _ *pgx.Conn, data pgx.TraceQueryEndData) {
	v := ctx.Value(queryKey{})
	if v == nil {
		return
	}
	sd := v.(queryStartData)
	dur := time.Since(sd.start)
	ms := dur.Milliseconds()

	metricQueryCount.Add(1)
	metricQueryDurationSum.Add(ms)

	if t.disabled.Load() {
		return
	}

	if ms >= t.ThresholdMS {
		metricSlowQueryTotal.Add(1)

		attrs := []any{
			slog.Int64("duration_ms", ms),
			slog.Int64("threshold_ms", t.ThresholdMS),
			slog.String("sql", sd.sql), // SQL text only – no bind parameters
		}
		if data.Err != nil {
			attrs = append(attrs, slog.String("error", data.Err.Error()))
		}
		slog.Warn("slow query detected", attrs...)
	}
}
