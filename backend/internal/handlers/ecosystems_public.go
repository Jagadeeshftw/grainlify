package handlers

import (
	"encoding/json"
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

// GetByID returns one ecosystem by ID with full detail (about, links, key_areas, technologies) and computed stats.
func (h *EcosystemsPublicHandler) GetByID() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		ecoID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_ecosystem_id"})
		}

		var id uuid.UUID
		var slug, name, status string
		var desc, website, logoURL, about *string
		var linksJSON, keyAreasJSON, technologiesJSON []byte
		var createdAt, updatedAt time.Time
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT e.id, e.slug, e.name, e.description, e.website_url, e.logo_url, e.status, e.created_at, e.updated_at,
       e.about, e.links, e.key_areas, e.technologies
FROM ecosystems e
WHERE e.id = $1 AND e.status = 'active'
`, ecoID).Scan(&id, &slug, &name, &desc, &website, &logoURL, &status, &createdAt, &updatedAt, &about, &linksJSON, &keyAreasJSON, &technologiesJSON)
		if err != nil {
			if err.Error() == "no rows in result set" {
				return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "ecosystem_not_found"})
			}
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystem_lookup_failed"})
		}

		var links, keyAreas, technologies interface{}
		if len(linksJSON) > 0 {
			_ = json.Unmarshal(linksJSON, &links)
		}
		if len(keyAreasJSON) > 0 {
			_ = json.Unmarshal(keyAreasJSON, &keyAreas)
		}
		if len(technologiesJSON) > 0 {
			_ = json.Unmarshal(technologiesJSON, &technologies)
		}

		// Count only verified projects (same as public projects list) so Overview matches Projects tab
		var projectCount int64
		var contributorsCount int64
		var openIssuesCount int64
		var openPRsCount int64
		_ = h.db.Pool.QueryRow(c.Context(), `
SELECT
  (SELECT COUNT(*) FROM projects p WHERE p.ecosystem_id = $1 AND p.deleted_at IS NULL AND p.status = 'verified' AND p.needs_metadata = false),
  COALESCE((
    SELECT COUNT(DISTINCT a.author_login)
    FROM (
      SELECT author_login FROM github_issues WHERE project_id IN (SELECT id FROM projects WHERE ecosystem_id = $1 AND deleted_at IS NULL AND status = 'verified' AND needs_metadata = false) AND author_login IS NOT NULL AND author_login != ''
      UNION
      SELECT author_login FROM github_pull_requests WHERE project_id IN (SELECT id FROM projects WHERE ecosystem_id = $1 AND deleted_at IS NULL AND status = 'verified' AND needs_metadata = false) AND author_login IS NOT NULL AND author_login != ''
    ) a
  ), 0),
  COALESCE((SELECT COUNT(*) FROM github_issues gi INNER JOIN projects p ON p.id = gi.project_id WHERE p.ecosystem_id = $1 AND p.deleted_at IS NULL AND p.status = 'verified' AND p.needs_metadata = false AND gi.state = 'open'), 0),
  COALESCE((SELECT COUNT(*) FROM github_pull_requests gpr INNER JOIN projects p ON p.id = gpr.project_id WHERE p.ecosystem_id = $1 AND p.deleted_at IS NULL AND p.status = 'verified' AND p.needs_metadata = false AND gpr.state = 'open'), 0)
`, ecoID, ecoID, ecoID, ecoID).Scan(&projectCount, &contributorsCount, &openIssuesCount, &openPRsCount)

		out := fiber.Map{
			"id":                   id.String(),
			"slug":                 slug,
			"name":                 name,
			"description":          desc,
			"website_url":          website,
			"logo_url":             logoURL,
			"status":               status,
			"created_at":           createdAt,
			"updated_at":           updatedAt,
			"about":                about,
			"links":                links,
			"key_areas":            keyAreas,
			"technologies":         technologies,
			"project_count":        projectCount,
			"contributors_count":   contributorsCount,
			"open_issues_count":    openIssuesCount,
			"open_prs_count":       openPRsCount,
		}
		return c.Status(fiber.StatusOK).JSON(out)
	}
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
  e.logo_url,
  e.status,
  e.created_at,
  e.updated_at,
  COUNT(p.id) AS project_count,
  COUNT(DISTINCT p.owner_user_id) AS user_count
FROM ecosystems e
LEFT JOIN projects p ON p.ecosystem_id = e.id AND p.deleted_at IS NULL
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
				logoURL    *string
				createdAt  time.Time
				updatedAt  time.Time
				projectCnt int64
				userCnt    int64
			)
			if err := rows.Scan(&id, &slug, &name, &desc, &website, &logoURL, &status, &createdAt, &updatedAt, &projectCnt, &userCnt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystems_list_failed"})
			}
			out = append(out, fiber.Map{
				"id":            id.String(),
				"slug":          slug,
				"name":          name,
				"description":   desc,
				"website_url":   website,
				"logo_url":      logoURL,
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
