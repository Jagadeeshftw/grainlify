package db

import (
	"context"
	"fmt"
	"log/slog"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

type DB struct {
	Pool *pgxpool.Pool
}

func Connect(ctx context.Context, dbURL string) (*DB, error) {
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

// maskDBURL masks the password in a database URL for logging
func maskDBURL(dbURL string) string {
	// Simple masking: replace password with ***
	// Format: postgresql://user:password@host:port/db
	if len(dbURL) < 20 {
		return "***"
	}
	// Find @ symbol and mask everything between : and @
	atIdx := -1
	colonIdx := -1
	for i, r := range dbURL {
		if r == '@' {
			atIdx = i
			break
		}
		if r == ':' && colonIdx == -1 {
			colonIdx = i
		}
	}
	if atIdx > 0 && colonIdx > 0 && colonIdx < atIdx {
		return dbURL[:colonIdx+1] + "***" + dbURL[atIdx:]
	}
	return "***"
}

func (d *DB) Close() {
	if d == nil || d.Pool == nil {
		return
	}
	d.Pool.Close()
}




