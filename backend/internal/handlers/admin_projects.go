package handlers

import (
	"log/slog"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type AdminProjectsHandler struct {
	cfg config.Config
	db  *db.DB
}

func NewAdminProjectsHandler(cfg config.Config, d *db.DB) *AdminProjectsHandler {
	return &AdminProjectsHandler{cfg: cfg, db: d}
}

func (h *AdminProjectsHandler) List() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT 
  p.id, 
  p.github_full_name, 
  p.status, 
  p.github_repo_id, 
  p.stars_count,
  p.forks_count,
  (
    SELECT COUNT(*)
    FROM github_issues gi
    WHERE gi.project_id = p.id AND gi.state = 'open'
  ) AS open_issues_count,
  (
    SELECT COUNT(*)
    FROM github_pull_requests gpr
    WHERE gpr.project_id = p.id AND gpr.state = 'open'
  ) AS open_prs_count,
  (
    SELECT COUNT(DISTINCT a.author_login)
    FROM (
      SELECT author_login FROM github_issues WHERE project_id = p.id AND author_login IS NOT NULL AND author_login != ''
      UNION
      SELECT author_login FROM github_pull_requests WHERE project_id = p.id AND author_login IS NOT NULL AND author_login != ''
    ) a
  ) AS contributors_count,
  p.created_at, 
  p.updated_at,
  e.name AS ecosystem_name,
  p.language,
  p.tags,
  p.category
FROM projects p
LEFT JOIN ecosystems e ON p.ecosystem_id = e.id
WHERE p.deleted_at IS NULL
ORDER BY p.created_at DESC
`)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "projects_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var id uuid.UUID
			var fullName, status string
			var repoID *int64
			var starsCount, forksCount *int
			var openIssuesCount, openPRsCount, contributorsCount int
			var createdAt, updatedAt time.Time
			var ecosystemName *string
			var language *string
			var tags []string
			var category *string

			if err := rows.Scan(&id, &fullName, &status, &repoID, &starsCount, &forksCount, &openIssuesCount, &openPRsCount, &contributorsCount, &createdAt, &updatedAt, &ecosystemName, &language, &tags, &category); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "projects_list_failed"})
			}

			out = append(out, fiber.Map{
				"id":                 id.String(),
				"github_full_name":   fullName,
				"status":             status,
				"github_repo_id":     repoID,
				"stars_count":        starsCount,
				"forks_count":        forksCount,
				"open_issues_count":  openIssuesCount,
				"open_prs_count":     openPRsCount,
				"contributors_count": contributorsCount,
				"created_at":         createdAt,
				"updated_at":         updatedAt,
				"ecosystem_name":     ecosystemName,
				"language":           language,
				"tags":               tags,
				"category":           category,
			})
		}

		if out == nil {
			out = []fiber.Map{}
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"projects": out,
		})
	}
}

func (h *AdminProjectsHandler) Delete() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		projectID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_project_id"})
		}

		result, err := h.db.Pool.Exec(c.Context(), `
UPDATE projects
SET deleted_at = now()
WHERE id = $1 AND deleted_at IS NULL
`, projectID)
		if err != nil {
			slog.Error("failed to delete project", "error", err, "project_id", projectID)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_delete_failed"})
		}

		if result.RowsAffected() == 0 {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
		}

		return c.SendStatus(fiber.StatusNoContent)
	}
}
