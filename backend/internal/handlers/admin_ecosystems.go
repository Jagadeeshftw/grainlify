package handlers

import (
	"encoding/json"
	"errors"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type EcosystemsAdminHandler struct {
	db *db.DB
}

func NewEcosystemsAdminHandler(d *db.DB) *EcosystemsAdminHandler {
	return &EcosystemsAdminHandler{db: d}
}

func (h *EcosystemsAdminHandler) List() fiber.Handler {
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
  e.short_description,
  e.website_url,
  e.status,
  e.languages,
  e.key_areas,
  e.technologies,
  e.created_at,
  e.updated_at,
  COUNT(p.id) AS project_count,
  COUNT(DISTINCT p.owner_user_id) AS user_count
FROM ecosystems e
LEFT JOIN projects p ON p.ecosystem_id = e.id
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
			var id uuid.UUID
			var slug, name, status string
			var desc, shortDesc, website *string
			var languagesJSON, keyAreasJSON, technologiesJSON []byte
			var createdAt, updatedAt time.Time
			var projectCnt int64
			var userCnt int64
			if err := rows.Scan(&id, &slug, &name, &desc, &shortDesc, &website, &status, &languagesJSON, &keyAreasJSON, &technologiesJSON, &createdAt, &updatedAt, &projectCnt, &userCnt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystems_list_failed"})
			}

			// Parse JSONB fields
			var languages []Language
			var keyAreas []KeyArea
			var technologies []string
			if len(languagesJSON) > 0 {
				json.Unmarshal(languagesJSON, &languages)
			}
			if len(keyAreasJSON) > 0 {
				json.Unmarshal(keyAreasJSON, &keyAreas)
			}
			if len(technologiesJSON) > 0 {
				json.Unmarshal(technologiesJSON, &technologies)
			}

			out = append(out, fiber.Map{
				"id":                id.String(),
				"slug":              slug,
				"name":              name,
				"description":       desc,
				"short_description": shortDesc,
				"website_url":       website,
				"status":            status,
				"languages":         languages,
				"key_areas":         keyAreas,
				"technologies":      technologies,
				"created_at":        createdAt,
				"updated_at":        updatedAt,
				"project_count":     projectCnt,
				"user_count":        userCnt,
			})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ecosystems": out})
	}
}

type Language struct {
	Name       string  `json:"name"`
	Percentage float64 `json:"percentage"`
}

type KeyArea struct {
	Title       string `json:"title"`
	Description string `json:"description"`
}

type ecosystemUpsertRequest struct {
	Slug             string     `json:"slug"`
	Name             string     `json:"name"`
	Description      string     `json:"description"`
	ShortDescription string     `json:"short_description"`
	WebsiteURL       string     `json:"website_url"`
	Status           string     `json:"status"` // active|inactive
	Languages        []Language `json:"languages"`
	KeyAreas         []KeyArea  `json:"key_areas"`
	Technologies     []string   `json:"technologies"`
}

func (h *EcosystemsAdminHandler) Create() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		var req ecosystemUpsertRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}
		name := strings.TrimSpace(req.Name)
		if name == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "name_required"})
		}
		// Auto-generate slug from name (users never see/type slug)
		slug := normalizeSlug(name)
		if slug == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "name_must_contain_valid_characters"})
		}
		status := strings.TrimSpace(req.Status)
		if status == "" {
			status = "active"
		}
		if status != "active" && status != "inactive" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_status"})
		}

		// Marshal JSONB fields
		languagesJSON, _ := json.Marshal(req.Languages)
		keyAreasJSON, _ := json.Marshal(req.KeyAreas)
		technologiesJSON, _ := json.Marshal(req.Technologies)

		var id uuid.UUID
		err := h.db.Pool.QueryRow(c.Context(), `
INSERT INTO ecosystems (slug, name, description, short_description, website_url, status, languages, key_areas, technologies)
VALUES ($1, $2, NULLIF($3,''), NULLIF($4,''), NULLIF($5,''), $6, $7, $8, $9)
RETURNING id
`, slug, name, strings.TrimSpace(req.Description), strings.TrimSpace(req.ShortDescription), strings.TrimSpace(req.WebsiteURL), status, languagesJSON, keyAreasJSON, technologiesJSON).Scan(&id)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystem_create_failed"})
		}
		return c.Status(fiber.StatusCreated).JSON(fiber.Map{"id": id.String()})
	}
}

func (h *EcosystemsAdminHandler) Update() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		ecoID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_ecosystem_id"})
		}
		var req ecosystemUpsertRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}

		name := strings.TrimSpace(req.Name)
		status := strings.TrimSpace(req.Status)

		if status != "" && status != "active" && status != "inactive" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_status"})
		}

		// Auto-generate slug from name if name is provided
		var slugVal *string
		if name != "" {
			slug := normalizeSlug(name)
			if slug == "" {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "name_must_contain_valid_characters"})
			}
			slugVal = &slug
		}

		// Marshal JSONB fields
		languagesJSON, _ := json.Marshal(req.Languages)
		keyAreasJSON, _ := json.Marshal(req.KeyAreas)
		technologiesJSON, _ := json.Marshal(req.Technologies)

		ct, err := h.db.Pool.Exec(c.Context(), `
UPDATE ecosystems
SET slug = COALESCE($2, slug),
    name = COALESCE(NULLIF($3,''), name),
    description = COALESCE(NULLIF($4,''), description),
    short_description = COALESCE(NULLIF($5,''), short_description),
    website_url = COALESCE(NULLIF($6,''), website_url),
    status = COALESCE(NULLIF($7,''), status),
    languages = COALESCE($8, languages),
    key_areas = COALESCE($9, key_areas),
    technologies = COALESCE($10, technologies),
    updated_at = now()
WHERE id = $1
`, ecoID, slugVal, name, strings.TrimSpace(req.Description), strings.TrimSpace(req.ShortDescription), strings.TrimSpace(req.WebsiteURL), status, languagesJSON, keyAreasJSON, technologiesJSON)
		if errors.Is(err, pgx.ErrNoRows) || ct.RowsAffected() == 0 {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "ecosystem_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystem_update_failed"})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}

func (h *EcosystemsAdminHandler) Delete() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		ecoID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_ecosystem_id"})
		}

		// Check if ecosystem has any projects
		var projectCount int64
		if err := h.db.Pool.QueryRow(c.Context(), `SELECT COUNT(*) FROM projects WHERE ecosystem_id = $1`, ecoID).Scan(&projectCount); err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystem_delete_check_failed"})
		}
		if projectCount > 0 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "ecosystem_has_projects", "message": "Cannot delete ecosystem with existing projects"})
		}

		ct, err := h.db.Pool.Exec(c.Context(), `DELETE FROM ecosystems WHERE id = $1`, ecoID)
		if errors.Is(err, pgx.ErrNoRows) || ct.RowsAffected() == 0 {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "ecosystem_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystem_delete_failed"})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}

func normalizeSlug(s string) string {
	v := strings.ToLower(strings.TrimSpace(s))
	v = strings.ReplaceAll(v, " ", "-")
	// allow: a-z 0-9 - _
	out := make([]rune, 0, len(v))
	for _, r := range v {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') || r == '-' || r == '_' {
			out = append(out, r)
		}
	}
	return strings.Trim(string(out), "-")
}


