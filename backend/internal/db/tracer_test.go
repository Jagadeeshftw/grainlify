package db

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"expvar"
	"fmt"
	"log/slog"
	"strings"
	"testing"
	"time"

	"github.com/jackc/pgx/v5"
)

// newCaptureLogger returns a JSON slog logger and the buffer it writes to.
func newCaptureLogger() (*slog.Logger, *bytes.Buffer) {
	buf := &bytes.Buffer{}
	h := slog.NewJSONHandler(buf, &slog.HandlerOptions{Level: slog.LevelDebug})
	return slog.New(h), buf
}

// logLines parses newline-delimited JSON log output into a slice of maps.
func logLines(buf *bytes.Buffer) []map[string]any {
	var out []map[string]any
	for _, line := range strings.Split(buf.String(), "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		var m map[string]any
		if err := json.Unmarshal([]byte(line), &m); err == nil {
			out = append(out, m)
		}
	}
	return out
}

// simulateQuery drives the tracer through a full start→sleep→end cycle.
func simulateQuery(tr *SlowQueryTracer, sql string, sleep time.Duration, queryErr error) {
	ctx := tr.TraceQueryStart(context.Background(), nil, pgx.TraceQueryStartData{SQL: sql})
	if sleep > 0 {
		time.Sleep(sleep)
	}
	tr.TraceQueryEnd(ctx, nil, pgx.TraceQueryEndData{Err: queryErr})
}

// resetMetrics zeroes the shared expvar counters so tests are independent.
func resetMetrics() {
	metricSlowQueryTotal.Set(0)
	metricQueryDurationSum.Set(0)
	metricQueryCount.Set(0)
}

// --- constructor tests ---

func TestNewSlowQueryTracer_EnabledWhenPositive(t *testing.T) {
	tr := NewSlowQueryTracer(500)
	if tr.disabled.Load() {
		t.Fatal("tracer should be enabled when threshold > 0")
	}
}

func TestNewSlowQueryTracer_DisabledWhenZero(t *testing.T) {
	tr := NewSlowQueryTracer(0)
	if !tr.disabled.Load() {
		t.Fatal("tracer should be disabled when threshold == 0")
	}
}

func TestNewSlowQueryTracer_DisabledWhenNegative(t *testing.T) {
	tr := NewSlowQueryTracer(-1)
	if !tr.disabled.Load() {
		t.Fatal("tracer should be disabled when threshold < 0")
	}
}

// --- slow-query log emission ---

func TestSlowQuery_LogEmittedAboveThreshold(t *testing.T) {
	resetMetrics()
	logger, buf := newCaptureLogger()
	slog.SetDefault(logger)

	tr := NewSlowQueryTracer(1) // 1 ms – easy to exceed
	simulateQuery(tr, "SELECT 1", 5*time.Millisecond, nil)

	found := false
	for _, l := range logLines(buf) {
		if msg, _ := l["msg"].(string); strings.Contains(msg, "slow query") {
			found = true
			if l["sql"] == nil {
				t.Error("log entry missing 'sql' field")
			}
		}
	}
	if !found {
		t.Errorf("slow query log not found; output:\n%s", buf.String())
	}
}

func TestSlowQuery_NoLogBelowThreshold(t *testing.T) {
	resetMetrics()
	logger, buf := newCaptureLogger()
	slog.SetDefault(logger)

	tr := NewSlowQueryTracer(60_000) // 60 s – will never be hit
	simulateQuery(tr, "SELECT 1", 0, nil)

	for _, l := range logLines(buf) {
		if msg, _ := l["msg"].(string); strings.Contains(msg, "slow query") {
			t.Errorf("unexpected slow query log: %v", l)
		}
	}
}

func TestSlowQuery_DisabledThreshold_NoLog(t *testing.T) {
	resetMetrics()
	logger, buf := newCaptureLogger()
	slog.SetDefault(logger)

	tr := NewSlowQueryTracer(0) // disabled
	simulateQuery(tr, "SELECT secret_data", 5*time.Millisecond, nil)

	for _, l := range logLines(buf) {
		if msg, _ := l["msg"].(string); strings.Contains(msg, "slow query") {
			t.Errorf("slow query log emitted even though threshold is disabled: %v", l)
		}
	}
}

// --- metrics ---

func TestMetrics_QueryCountAlwaysIncrements(t *testing.T) {
	resetMetrics()
	slog.SetDefault(slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	tr := NewSlowQueryTracer(0) // logging disabled, metrics still run
	simulateQuery(tr, "SELECT 1", 0, nil)
	simulateQuery(tr, "SELECT 2", 0, nil)

	if got := metricQueryCount.Value(); got != 2 {
		t.Errorf("db_queries_total = %d, want 2", got)
	}
}

func TestMetrics_SlowQueryCounterIncrements(t *testing.T) {
	resetMetrics()
	slog.SetDefault(slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	tr := NewSlowQueryTracer(1)
	simulateQuery(tr, "SELECT slow", 5*time.Millisecond, nil)

	if got := metricSlowQueryTotal.Value(); got < 1 {
		t.Errorf("db_slow_queries_total = %d, want >= 1", got)
	}
}

func TestMetrics_DurationSumPositive(t *testing.T) {
	resetMetrics()
	slog.SetDefault(slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	tr := NewSlowQueryTracer(1)
	simulateQuery(tr, "SELECT 1", 2*time.Millisecond, nil)

	if got := metricQueryDurationSum.Value(); got <= 0 {
		t.Errorf("db_query_duration_ms_total = %d, want > 0", got)
	}
}

func TestMetrics_DisabledTracer_SlowCounterNotIncremented(t *testing.T) {
	resetMetrics()
	slog.SetDefault(slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	tr := NewSlowQueryTracer(0) // disabled
	simulateQuery(tr, "SELECT 1", 5*time.Millisecond, nil)

	if got := metricSlowQueryTotal.Value(); got != 0 {
		t.Errorf("db_slow_queries_total = %d, want 0 when tracer disabled", got)
	}
}

// --- security: no args in logs ---

func TestSlowQuery_NoArgsInLog(t *testing.T) {
	resetMetrics()
	logger, buf := newCaptureLogger()
	slog.SetDefault(logger)

	tr := NewSlowQueryTracer(1)
	ctx := tr.TraceQueryStart(context.Background(), nil, pgx.TraceQueryStartData{
		SQL:  "SELECT * FROM users WHERE email = $1",
		Args: []any{"user@example.com"}, // sensitive – must NOT be logged
	})
	time.Sleep(5 * time.Millisecond)
	tr.TraceQueryEnd(ctx, nil, pgx.TraceQueryEndData{})

	raw := buf.String()
	if strings.Contains(raw, "user@example.com") {
		t.Error("sensitive query argument leaked into log output")
	}
	if strings.Contains(raw, `"args"`) {
		t.Error("'args' field must not appear in log output")
	}
}

// --- error field ---

func TestSlowQuery_ErrorIncludedInLog(t *testing.T) {
	resetMetrics()
	logger, buf := newCaptureLogger()
	slog.SetDefault(logger)

	tr := NewSlowQueryTracer(1)
	ctx := tr.TraceQueryStart(context.Background(), nil, pgx.TraceQueryStartData{SQL: "SELECT bad"})
	time.Sleep(5 * time.Millisecond)
	tr.TraceQueryEnd(ctx, nil, pgx.TraceQueryEndData{Err: errors.New("syntax error")})

	found := false
	for _, l := range logLines(buf) {
		if msg, _ := l["msg"].(string); strings.Contains(msg, "slow query") {
			found = true
			if l["error"] == nil {
				t.Error("expected 'error' field in slow query log when query fails")
			}
		}
	}
	if !found {
		t.Error("slow query log not found for failed query")
	}
}

// --- edge cases ---

func TestSlowQuery_MissingContextKey_NoPanic(t *testing.T) {
	tr := NewSlowQueryTracer(1)
	// TraceQueryEnd with a bare context (no queryKey) must not panic.
	tr.TraceQueryEnd(context.Background(), nil, pgx.TraceQueryEndData{})
}

func TestExpvarMetricsRegisteredOnce(t *testing.T) {
	// expvar panics on duplicate registration; creating multiple tracers must
	// not re-register the same vars.
	defer func() {
		if r := recover(); r != nil {
			t.Errorf("expvar re-registration panic: %v", r)
		}
	}()
	_ = NewSlowQueryTracer(100)
	_ = NewSlowQueryTracer(200)
}

func TestExpvarMetricsAccessible(t *testing.T) {
	for _, name := range []string{
		"db_slow_queries_total",
		"db_query_duration_ms_total",
		"db_queries_total",
	} {
		if expvar.Get(name) == nil {
			t.Errorf("expvar %q not registered", name)
		}
	}
}

func TestMaskDBURL_MasksPassword(t *testing.T) {
	cases := []struct {
		in   string
		want string // must not contain the literal password
	}{
		{"postgresql://user:secret@host:5432/db", "postgresql://user:***@host:5432/db"},
		{"short", "***"},
		{"postgresql://user@host/db", "***"}, // no colon before @
	}
	for _, tc := range cases {
		got := maskDBURL(tc.in)
		if strings.Contains(got, "secret") {
			t.Errorf("maskDBURL(%q) = %q; password not masked", tc.in, got)
		}
		if tc.want != "" && got != tc.want {
			t.Errorf("maskDBURL(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}
}

// Ensure the package compiles and fmt is used.
var _ = fmt.Sprintf
