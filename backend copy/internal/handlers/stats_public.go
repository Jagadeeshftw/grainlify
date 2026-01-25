package handlers

import (
	"log/slog"

	"github.com/gofiber/fiber/v2"

	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type LandingStatsHandler struct {
	db *db.DB
}

func NewLandingStatsHandler(d *db.DB) *LandingStatsHandler {
	return &LandingStatsHandler{db: d}
}

type LandingStatsResponse struct {
	ActiveProjects       int64 `json:"active_projects"`
	Contributors         int64 `json:"contributors"`
	GrantsDistributedUSD int64 `json:"grants_distributed_usd"`
}

// Get returns high-level landing page stats.
//
// Notes:
// - Active projects are verified projects that aren't soft-deleted.
// - Contributors are distinct GitHub author logins across issues/PRs in verified projects.
// - Grants distributed is currently 0 (no payouts table implemented yet).
func (h *LandingStatsHandler) Get() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		var resp LandingStatsResponse
		err := h.db.Pool.QueryRow(c.Context(), `
WITH verified_projects AS (
  SELECT id
  FROM projects
  WHERE status = 'verified' AND deleted_at IS NULL
),
all_contributors AS (
  SELECT gi.author_login AS login
  FROM github_issues gi
  INNER JOIN verified_projects vp ON vp.id = gi.project_id
  WHERE gi.author_login IS NOT NULL AND gi.author_login != ''
  UNION
  SELECT gpr.author_login AS login
  FROM github_pull_requests gpr
  INNER JOIN verified_projects vp ON vp.id = gpr.project_id
  WHERE gpr.author_login IS NOT NULL AND gpr.author_login != ''
)
SELECT
  (SELECT COUNT(*) FROM verified_projects) AS active_projects,
  (SELECT COUNT(DISTINCT LOWER(login)) FROM all_contributors) AS contributors
`).Scan(&resp.ActiveProjects, &resp.Contributors)
		if err != nil {
			slog.Error("failed to fetch landing stats", "error", err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "stats_fetch_failed"})
		}

		// No payouts/grants table exists yet in the schema.
		resp.GrantsDistributedUSD = 0

		return c.Status(fiber.StatusOK).JSON(resp)
	}
}
