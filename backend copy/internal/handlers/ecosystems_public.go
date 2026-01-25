package handlers

import (
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type EcosystemsPublicHandler struct {
	db *db.DB
}

func NewEcosystemsPublicHandler(d *db.DB) *EcosystemsPublicHandler {
	return &EcosystemsPublicHandler{db: d}
}

// ListActive returns active ecosystems with computed counts:
// - project_count: number of projects assigned to the ecosystem
// - user_count: number of distinct project owners in the ecosystem
func (h *EcosystemsPublicHandler) ListActive() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT
  e.id,
  e.slug,
  e.name,
  e.description,
  e.website_url,
  e.status,
  e.created_at,
  e.updated_at,
  COUNT(p.id) AS project_count,
  COUNT(DISTINCT p.owner_user_id) AS user_count
FROM ecosystems e
LEFT JOIN projects p ON p.ecosystem_id = e.id
WHERE e.status = 'active'
GROUP BY e.id
ORDER BY e.created_at DESC
LIMIT 200
`)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystems_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var (
				id         uuid.UUID
				slug       string
				name       string
				status     string
				desc       *string
				website    *string
				createdAt  time.Time
				updatedAt  time.Time
				projectCnt int64
				userCnt    int64
			)
			if err := rows.Scan(&id, &slug, &name, &desc, &website, &status, &createdAt, &updatedAt, &projectCnt, &userCnt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystems_list_failed"})
			}
			out = append(out, fiber.Map{
				"id":            id.String(),
				"slug":          slug,
				"name":          name,
				"description":   desc,
				"website_url":   website,
				"status":        status,
				"created_at":    createdAt,
				"updated_at":    updatedAt,
				"project_count": projectCnt,
				"user_count":    userCnt,
			})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ecosystems": out})
	}
}
