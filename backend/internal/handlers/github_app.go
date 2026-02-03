package handlers

import (
	"context"
	"encoding/json"
	"errors"
	"log/slog"
	"net/url"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)

type GitHubAppHandler struct {
	cfg config.Config
	db  *db.DB
}

func NewGitHubAppHandler(cfg config.Config, d *db.DB) *GitHubAppHandler {
	return &GitHubAppHandler{cfg: cfg, db: d}
}

// StartInstallation generates a GitHub App installation URL
func (h *GitHubAppHandler) StartInstallation() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		if h.cfg.GitHubAppID == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{
				"error":   "github_app_not_configured",
				"message": "GitHub App is not configured. Please contact support.",
			})
		}

		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		// Generate state for installation callback
		state := randomState(32)
		expiresAt := time.Now().UTC().Add(10 * time.Minute)

		_, err = h.db.Pool.Exec(c.Context(), `
INSERT INTO oauth_states (state, user_id, kind, expires_at)
VALUES ($1, $2, 'github_app_install', $3)
`, state, userID, expiresAt)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "state_create_failed"})
		}

		// Build GitHub App installation URL
		// Format: https://github.com/apps/{app-slug}/installations/new
		// Or: https://github.com/apps/{app-slug}/installations/new?state={state}
		appSlug := h.cfg.GitHubAppSlug
		if appSlug == "" {
			// Fallback: use app ID if slug not configured
			appSlug = h.cfg.GitHubAppID
		}

		installURL := "https://github.com/apps/" + appSlug + "/installations/new"
		if state != "" {
			// IMPORTANT:
			// Some GitHub App install callbacks don't reliably include the `state` query param.
			// To guarantee we can map installation -> user, embed state in `redirect_url`.
			u, err := url.Parse(installURL)
			if err == nil {
				q := u.Query()
				// keep state too (harmless if GitHub returns it)
				q.Set("state", state)
				// redirect back to our callback with state baked in
				cb := strings.TrimSuffix(h.cfg.PublicBaseURL, "/") + "/auth/github/app/install/callback"
				cbURL, cbErr := url.Parse(cb)
				if cbErr == nil {
					cbQ := cbURL.Query()
					cbQ.Set("state", state)
					cbURL.RawQuery = cbQ.Encode()
					q.Set("redirect_url", cbURL.String())
				}
				u.RawQuery = q.Encode()
				installURL = u.String()
			}
		}

		// Log installation start for debugging
		slog.Info("GitHub App installation started",
			"user_id", userID,
			"app_slug", appSlug,
			"app_id", h.cfg.GitHubAppID,
			"state", state,
			"install_url", installURL,
			"expected_callback_url", h.cfg.PublicBaseURL+"/auth/github/app/install/callback",
		)

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"install_url": installURL,
			"state":       state,
		})
	}
}

// HandleInstallationCallback processes the GitHub App installation callback
func (h *GitHubAppHandler) HandleInstallationCallback() fiber.Handler {
	return func(c *fiber.Ctx) error {
		// Log immediately when callback is hit (even before DB check)
		slog.Info("=== GitHub App callback endpoint hit ===",
			"method", c.Method(),
			"path", c.Path(),
			"full_url", c.OriginalURL(),
			"remote_ip", c.IP(),
			"user_agent", c.Get("User-Agent"),
		)

		if h.db == nil || h.db.Pool == nil {
			slog.Error("callback received but DB not configured")
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		// Log all query parameters for debugging
		allParams := c.Queries()
		slog.Info("GitHub App installation callback received",
			"method", c.Method(),
			"path", c.Path(),
			"query_params", allParams,
			"raw_query", c.OriginalURL(),
		)

		// GitHub redirects with installation_id and setup_action
		installationID := c.Query("installation_id")
		state := c.Query("state")
		setupAction := c.Query("setup_action") // "install" or "update"

		// If installation_id is missing, user might have cancelled or accessed URL directly
		if installationID == "" {
			slog.Warn("GitHub App callback missing installation_id - user may have cancelled installation",
				"state", state,
				"setup_action", setupAction,
				"all_params", allParams,
			)

			// Redirect to frontend with cancellation message
			redirectURL := h.cfg.FrontendBaseURL
			if redirectURL == "" {
				redirectURL = "http://localhost:5173"
			}

			u, err := url.Parse(strings.TrimSuffix(redirectURL, "/") + "/dashboard")
			if err == nil {
				q := u.Query()
				q.Set("github_app_install", "cancelled")
				u.RawQuery = q.Encode()
				return c.Redirect(u.String(), fiber.StatusFound)
			}

			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{
				"error":   "missing_installation_id",
				"message": "Installation ID is missing. You may have cancelled the installation or accessed this URL directly.",
				"hint":    "Please try installing the GitHub App again from the dashboard.",
			})
		}

		// Verify state and get user ID
		var userID uuid.UUID
		if state != "" {
			var storedUserID *uuid.UUID
			var storedKind string
			err := h.db.Pool.QueryRow(c.Context(), `
SELECT user_id, kind
FROM oauth_states
WHERE state = $1
  AND expires_at > now()
  AND kind = 'github_app_install'
`, state).Scan(&storedUserID, &storedKind)
			if errors.Is(err, pgx.ErrNoRows) {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_or_expired_state"})
			}
			if err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "state_lookup_failed"})
			}

			if storedUserID != nil {
				userID = *storedUserID
			}

			// Clean up state
			_, _ = h.db.Pool.Exec(c.Context(), `DELETE FROM oauth_states WHERE state = $1`, state)
		}

		// If we don't have userID, we can't create projects - just redirect
		if userID == (uuid.UUID{}) {
			slog.Warn("GitHub App installation callback: no user ID found, skipping repository sync",
				"installation_id", installationID,
				"state", state,
			)
		} else {
			// Sync repositories in background (don't block redirect)
			go h.syncInstallationRepositories(c.Context(), userID, installationID)
		}

		// Redirect to frontend with success message
		redirectURL := h.cfg.FrontendBaseURL
		if redirectURL == "" {
			// Fallback for development
			redirectURL = "http://localhost:5173"
		}

		// Build redirect URL with query parameters
		u, err := url.Parse(strings.TrimSuffix(redirectURL, "/") + "/dashboard")
		if err != nil {
			slog.Error("failed to parse redirect URL", "error", err, "url", redirectURL)
			// Fallback: return JSON response
			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"ok":              true,
				"installation_id": installationID,
				"setup_action":    setupAction,
				"message":         "GitHub App installed successfully. Repositories will be synced shortly.",
				"redirect_url":    redirectURL + "/dashboard?github_app_installed=true&installation_id=" + installationID,
			})
		}

		q := u.Query()
		q.Set("github_app_installed", "true")
		q.Set("installation_id", installationID)
		if setupAction != "" {
			q.Set("setup_action", setupAction)
		}
		u.RawQuery = q.Encode()

		slog.Info("redirecting after GitHub App installation",
			"installation_id", installationID,
			"redirect_url", u.String(),
			"frontend_base_url", redirectURL,
		)

		return c.Redirect(u.String(), fiber.StatusFound)
	}
}

// syncInstallationRepositories syncs repositories from a GitHub App installation
func (h *GitHubAppHandler) syncInstallationRepositories(ctx context.Context, userID uuid.UUID, installationID string) {
	ctx, cancel := context.WithTimeout(ctx, 60*time.Second)
	defer cancel()

	slog.Info("starting repository sync for GitHub App installation",
		"user_id", userID,
		"installation_id", installationID,
	)

	// Check if GitHub App is configured
	if h.cfg.GitHubAppID == "" || h.cfg.GitHubAppPrivateKey == "" {
		slog.Error("GitHub App not configured, cannot sync repositories",
			"app_id_set", h.cfg.GitHubAppID != "",
			"private_key_set", h.cfg.GitHubAppPrivateKey != "",
		)
		return
	}

	// Create GitHub App client
	appClient, err := github.NewGitHubAppClient(h.cfg.GitHubAppID, h.cfg.GitHubAppPrivateKey)
	if err != nil {
		slog.Error("failed to create GitHub App client", "error", err)
		return
	}

	// Get installation token
	installationToken, err := appClient.GetInstallationToken(ctx, installationID)
	if err != nil {
		slog.Error("failed to get installation token", "error", err, "installation_id", installationID)
		return
	}

	// List repositories
	repos, err := appClient.ListInstallationRepositories(ctx, installationToken)
	if err != nil {
		slog.Error("failed to list installation repositories", "error", err)
		return
	}

	slog.Info("found repositories in installation",
		"count", len(repos),
		"installation_id", installationID,
	)

	// Get default ecosystem (or use a fallback)
	var defaultEcosystemID uuid.UUID
	err = h.db.Pool.QueryRow(ctx, `
SELECT id FROM ecosystems WHERE status = 'active' ORDER BY created_at ASC LIMIT 1
`).Scan(&defaultEcosystemID)
	if err != nil {
		slog.Warn("no active ecosystem found, repositories will be created without ecosystem",
			"error", err,
		)
	}

	// Create projects for each repository (never add or restore private repos)
	createdCount := 0
	updatedCount := 0
	for _, repo := range repos {
		if repo.Private {
			// Never show or consider private repos anywhere in the dashboard
			var existingID uuid.UUID
			err := h.db.Pool.QueryRow(ctx, `SELECT id FROM projects WHERE github_full_name = $1`, repo.FullName).Scan(&existingID)
			if err == nil {
				_, _ = h.db.Pool.Exec(ctx, `UPDATE projects SET deleted_at = now(), updated_at = now() WHERE id = $1`, existingID)
				slog.Info("marked private repo as deleted, excluded from dashboard",
					"project_id", existingID,
					"repo", repo.FullName,
				)
			}
			continue
		}

		// Check if project already exists
		var existingID uuid.UUID
		var existingStatus string
		err := h.db.Pool.QueryRow(ctx, `
SELECT id, status FROM projects WHERE github_full_name = $1
`, repo.FullName).Scan(&existingID, &existingStatus)
		
		if err == nil {
			// Repository already exists - verify and enqueue sync if needed (public only)
			projectID := existingID
			
			// Always verify the project (update github_repo_id and status, restore if deleted)
			_, _ = h.db.Pool.Exec(ctx, `
UPDATE projects
SET github_repo_id = $2,
    status = 'verified',
    verified_at = COALESCE(verified_at, now()),
    verification_error = NULL,
    github_app_installation_id = $3,
    deleted_at = NULL,
    updated_at = now()
WHERE id = $1
`, projectID, repo.ID, installationID)
			
			slog.Info("verified existing project from GitHub App installation",
				"project_id", projectID,
				"repo", repo.FullName,
				"old_status", existingStatus,
			)
			
			// Always enqueue sync jobs (they will be deduplicated by the worker if already running)
			_, _ = h.db.Pool.Exec(ctx, `
INSERT INTO sync_jobs (project_id, job_type, status, run_at)
VALUES ($1, 'sync_issues', 'pending', now()),
       ($1, 'sync_prs', 'pending', now())
`, projectID)
			
			slog.Info("enqueued sync jobs for existing project",
				"project_id", projectID,
				"repo", repo.FullName,
			)
			
			updatedCount++
			continue
		}

		// Prepare tags from topics
		var tagsJSON []byte = []byte("[]")
		if len(repo.Topics) > 0 {
			tagsJSON, _ = json.Marshal(repo.Topics)
		}

		// Insert project
		var projectID uuid.UUID
		var ecosystemID *uuid.UUID
		if defaultEcosystemID != (uuid.UUID{}) {
			ecosystemID = &defaultEcosystemID
		}

		// Only insert public repos; private repos are never added
		err = h.db.Pool.QueryRow(ctx, `
INSERT INTO projects (owner_user_id, github_full_name, ecosystem_id, language, tags, status, github_app_installation_id, needs_metadata)
VALUES ($1, $2, $3, $4, $5, 'pending_verification', $6, true)
ON CONFLICT (github_full_name) DO UPDATE SET
  owner_user_id = EXCLUDED.owner_user_id,
  github_app_installation_id = EXCLUDED.github_app_installation_id,
  deleted_at = NULL,
  updated_at = now()
RETURNING id
`, userID, repo.FullName, ecosystemID, repo.Language, tagsJSON, installationID).Scan(&projectID)
		if err != nil {
			slog.Error("failed to create project",
				"error", err,
				"repo", repo.FullName,
			)
			continue
		}

		createdCount++
		slog.Info("created project from GitHub App installation",
			"project_id", projectID,
			"repo", repo.FullName,
		)

		// Automatically verify the project since we have installation access
		// Set github_repo_id and mark as verified
		_, _ = h.db.Pool.Exec(ctx, `
UPDATE projects
SET github_repo_id = $2,
    status = 'verified',
    verified_at = now(),
    verification_error = NULL,
    github_app_installation_id = $3,
    deleted_at = NULL,
    updated_at = now()
WHERE id = $1
`, projectID, repo.ID, installationID)

		// Enqueue sync jobs for issues and PRs
		_, _ = h.db.Pool.Exec(ctx, `
INSERT INTO sync_jobs (project_id, job_type, status, run_at)
VALUES ($1, 'sync_issues', 'pending', now()),
       ($1, 'sync_prs', 'pending', now())
`, projectID)

		slog.Info("verified project and enqueued sync jobs",
			"project_id", projectID,
			"repo", repo.FullName,
		)
	}

	slog.Info("completed repository sync",
		"total_repos", len(repos),
		"created", createdCount,
		"updated", updatedCount,
		"skipped", len(repos)-createdCount-updatedCount,
		"installation_id", installationID,
	)
}

