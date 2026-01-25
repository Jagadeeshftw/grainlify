package config

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/joho/godotenv"
)

// LoadDotenv loads environment variables from a local .env file if present.
//
// It does NOT override already-exported environment variables.
// This is meant for local development convenience.
func LoadDotenv() {
	// Allow an explicit env file path (or comma-separated list).
	if v := strings.TrimSpace(os.Getenv("ENV_FILE")); v != "" {
		parts := strings.Split(v, ",")
		for i := range parts {
			parts[i] = strings.TrimSpace(parts[i])
		}
		_ = godotenv.Load(parts...)
		return
	}

	// Default: try a few common locations, so starting the process from repo root or backend/ works.
	// We keep it silent and best-effort.
	candidates := []string{".env"}

	if wd, err := os.Getwd(); err == nil {
		// If running from repo root.
		candidates = append(candidates, filepath.Join(wd, "backend", ".env"))
		// If running from backend/ already or from backend/cmd.
		candidates = append(candidates, filepath.Join(wd, ".env"))
		candidates = append(candidates, filepath.Join(wd, "..", "backend", ".env"))
	}

	for _, p := range candidates {
		if strings.TrimSpace(p) == "" {
			continue
		}
		if _, err := os.Stat(p); err == nil {
			_ = godotenv.Load(p)
			return
		}
	}
}




