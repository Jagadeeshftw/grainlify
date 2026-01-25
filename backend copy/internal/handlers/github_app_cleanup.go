package handlers

import (
	"context"
	"log/slog"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)

// GitHubAppCleanupHandler handles periodic cleanup of uninstalled GitHub Apps
type GitHubAppCleanupHandler struct {
	cfg  config.Config
	pool *pgxpool.Pool
}

func NewGitHubAppCleanupHandler(cfg config.Config, pool *pgxpool.Pool) *GitHubAppCleanupHandler {
	return &GitHubAppCleanupHandler{
		cfg:  cfg,
		pool: pool,
	}
}

// RunPeriodicCleanup runs a background task that periodically checks if installations are still active
// and marks projects as deleted if the installation no longer exists
func (h *GitHubAppCleanupHandler) RunPeriodicCleanup(ctx context.Context) {
	if h.cfg.GitHubAppID == "" || h.cfg.GitHubAppPrivateKey == "" {
		slog.Warn("GitHub App not configured, skipping periodic cleanup")
		return
	}

	ticker := time.NewTicker(5 * time.Minute) // Check every 5 minutes
	defer ticker.Stop()

	slog.Info("GitHub App periodic cleanup started")

	for {
		select {
		case <-ctx.Done():
			slog.Info("GitHub App periodic cleanup stopped")
			return
		case <-ticker.C:
			h.checkInstallations(ctx)
		}
	}
}

// checkInstallations checks all active installations and marks projects as deleted if installation is gone
func (h *GitHubAppCleanupHandler) checkInstallations(ctx context.Context) {
	if h.pool == nil {
		return
	}

	// Get all unique installation IDs from projects that aren't deleted
	rows, err := h.pool.Query(ctx, `
SELECT DISTINCT github_app_installation_id
FROM projects
WHERE github_app_installation_id IS NOT NULL
  AND deleted_at IS NULL
`)
	if err != nil {
		slog.Error("failed to query installations", "error", err)
		return
	}
	defer rows.Close()

	var installationIDs []string
	for rows.Next() {
		var installationID string
		if err := rows.Scan(&installationID); err != nil {
			continue
		}
		installationIDs = append(installationIDs, installationID)
	}

	if len(installationIDs) == 0 {
		return
	}

	slog.Info("checking installation status",
		"count", len(installationIDs),
	)

	// Create GitHub App client
	appClient, err := github.NewGitHubAppClient(h.cfg.GitHubAppID, h.cfg.GitHubAppPrivateKey)
	if err != nil {
		slog.Error("failed to create GitHub App client", "error", err)
		return
	}

	// Check each installation
	for _, installationID := range installationIDs {
		h.checkSingleInstallation(ctx, appClient, installationID)
	}
}

// checkSingleInstallation checks if a single installation is still active
func (h *GitHubAppCleanupHandler) checkSingleInstallation(ctx context.Context, appClient *github.GitHubAppClient, installationID string) {
	// Try to get an installation token - if this fails with 404, the installation was deleted
	_, err := appClient.GetInstallationToken(ctx, installationID)
	if err != nil {
		// Check if error is 404 (installation not found)
		errStr := err.Error()
		if contains(errStr, "404") || contains(errStr, "Not Found") || contains(errStr, "not found") {
			slog.Info("installation no longer exists, marking projects as deleted",
				"installation_id", installationID,
			)

			// Mark all projects from this installation as deleted
			result, err := h.pool.Exec(ctx, `
UPDATE projects
SET deleted_at = now(),
    status = 'rejected',
    updated_at = now()
WHERE github_app_installation_id = $1
  AND deleted_at IS NULL
`, installationID)
			if err != nil {
				slog.Error("failed to mark projects as deleted",
					"installation_id", installationID,
					"error", err,
				)
				return
			}

			rowsAffected := result.RowsAffected()
			slog.Info("marked projects as deleted",
				"installation_id", installationID,
				"rows_affected", rowsAffected,
			)
		} else {
			// Other error (network, auth, etc.) - log but don't delete
			slog.Warn("failed to check installation status",
				"installation_id", installationID,
				"error", err,
			)
		}
	}
	// If GetInstallationToken succeeds, installation is still active - do nothing
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && 
		(s == substr || 
		 (len(s) > len(substr) && 
		  (s[:len(substr)] == substr || 
		   s[len(s)-len(substr):] == substr || 
		   containsSubstring(s, substr))))
}

func containsSubstring(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

