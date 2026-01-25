package main

import (
	"context"
	"log/slog"
	"os"
	"time"

	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/migrate"
)

func main() {
	config.LoadDotenv()
	cfg := config.Load()

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: cfg.LogLevel(),
	}))
	slog.SetDefault(logger)

	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	d, err := db.Connect(ctx, cfg.DBURL)
	if err != nil {
		slog.Error("db connect failed", "error", err)
		os.Exit(1)
	}
	defer d.Close()

	if err := migrate.Up(ctx, d.Pool); err != nil {
		slog.Error("migrate up failed", "error", err)
		os.Exit(1)
	}

	slog.Info("migrations applied")
}


