package db

import (
	"context"
	"fmt"
	"log/slog"
	"strings"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

type DB struct {
	Pool *pgxpool.Pool
}

// Connect opens a pgxpool and attaches a SlowQueryTracer with the given
// threshold (milliseconds). Pass 0 to collect metrics without log emission.
func Connect(ctx context.Context, dbURL string, slowQueryThresholdMS int64) (*DB, error) {
	if dbURL == "" {
		return nil, fmt.Errorf("DB_URL is required")
	}

	// Log connection attempt (mask password in URL)
	maskedURL := maskDBURL(dbURL)
	slog.Info("parsing database URL", "db_url_masked", maskedURL)

	cfg, err := pgxpool.ParseConfig(dbURL)
	if err != nil {
		slog.Error("failed to parse database URL",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return nil, fmt.Errorf("parse DB_URL: %w", err)
	}

	slog.Info("database config parsed",
		"host", cfg.ConnConfig.Host,
		"port", cfg.ConnConfig.Port,
		"database", cfg.ConnConfig.Database,
		"user", cfg.ConnConfig.User,
	)

	cfg.MaxConns = 10
	cfg.MinConns = 0
	cfg.MaxConnLifetime = 30 * time.Minute
	cfg.MaxConnIdleTime = 5 * time.Minute
	cfg.HealthCheckPeriod = 30 * time.Second

	// Attach slow-query tracer.
	cfg.ConnConfig.Tracer = NewSlowQueryTracer(slowQueryThresholdMS)
	slog.Info("slow query tracer attached", "threshold_ms", slowQueryThresholdMS)

	slog.Info("creating database connection pool",
		"max_conns", cfg.MaxConns,
		"min_conns", cfg.MinConns,
	)

	pool, err := pgxpool.NewWithConfig(ctx, cfg)
	if err != nil {
		slog.Error("failed to create database connection pool",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return nil, fmt.Errorf("connect db: %w", err)
	}

	slog.Info("database connection pool created, testing connection")
	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		slog.Error("database ping failed",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return nil, fmt.Errorf("ping db: %w", err)
	}

	slog.Info("database connection successful")
	return &DB{Pool: pool}, nil
}

// maskDBURL masks the password in a database URL for logging.
// Format: scheme://user:password@host:port/db
func maskDBURL(dbURL string) string {
	if len(dbURL) < 20 {
		return "***"
	}
	// Skip past the scheme (e.g. "postgresql://") before searching for credentials.
	searchFrom := 0
	if idx := strings.Index(dbURL, "://"); idx >= 0 {
		searchFrom = idx + 3
	}

	atIdx := strings.Index(dbURL[searchFrom:], "@")
	if atIdx < 0 {
		return "***"
	}
	atIdx += searchFrom // absolute index

	colonIdx := strings.Index(dbURL[searchFrom:], ":")
	if colonIdx < 0 {
		return "***"
	}
	colonIdx += searchFrom // absolute index

	if colonIdx >= atIdx {
		// No password field (user@host form).
		return "***"
	}
	return dbURL[:colonIdx+1] + "***" + dbURL[atIdx:]
}

func (d *DB) Close() {
	if d == nil || d.Pool == nil {
		return
	}
	d.Pool.Close()
}




