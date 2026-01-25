package handlers

import (
	"errors"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type OpenSourceWeekHandler struct {
	db *db.DB
}

func NewOpenSourceWeekHandler(d *db.DB) *OpenSourceWeekHandler {
	return &OpenSourceWeekHandler{db: d}
}

// ListPublic returns events that are not draft (upcoming/running/completed).
func (h *OpenSourceWeekHandler) ListPublic() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT id, title, description, location, status, start_at, end_at, created_at, updated_at
FROM open_source_week_events
WHERE status <> 'draft'
ORDER BY start_at DESC
LIMIT 100
`)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "osw_events_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var id uuid.UUID
			var title, status string
			var desc, location *string
			var startAt, endAt, createdAt, updatedAt time.Time
			if err := rows.Scan(&id, &title, &desc, &location, &status, &startAt, &endAt, &createdAt, &updatedAt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "osw_events_list_failed"})
			}
			out = append(out, fiber.Map{
				"id":          id.String(),
				"title":       title,
				"description": desc,
				"location":    location,
				"status":      status,
				"start_at":    startAt,
				"end_at":      endAt,
				"created_at":  createdAt,
				"updated_at":  updatedAt,
			})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"events": out})
	}
}

func (h *OpenSourceWeekHandler) GetPublic() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		evID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_event_id"})
		}

		var title, status string
		var desc, location *string
		var startAt, endAt, createdAt, updatedAt time.Time
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT title, description, location, status, start_at, end_at, created_at, updated_at
FROM open_source_week_events
WHERE id = $1 AND status <> 'draft'
`, evID).Scan(&title, &desc, &location, &status, &startAt, &endAt, &createdAt, &updatedAt)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "event_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "osw_event_get_failed"})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"event": fiber.Map{
				"id":          evID.String(),
				"title":       title,
				"description": desc,
				"location":    location,
				"status":      status,
				"start_at":    startAt,
				"end_at":      endAt,
				"created_at":  createdAt,
				"updated_at":  updatedAt,
			},
		})
	}
}

type OpenSourceWeekAdminHandler struct {
	db *db.DB
}

func NewOpenSourceWeekAdminHandler(d *db.DB) *OpenSourceWeekAdminHandler {
	return &OpenSourceWeekAdminHandler{db: d}
}

func (h *OpenSourceWeekAdminHandler) List() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		rows, err := h.db.Pool.Query(c.Context(), `
SELECT id, title, description, location, status, start_at, end_at, created_at, updated_at
FROM open_source_week_events
ORDER BY start_at DESC
LIMIT 200
`)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "osw_events_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var id uuid.UUID
			var title, status string
			var desc, location *string
			var startAt, endAt, createdAt, updatedAt time.Time
			if err := rows.Scan(&id, &title, &desc, &location, &status, &startAt, &endAt, &createdAt, &updatedAt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "osw_events_list_failed"})
			}
			out = append(out, fiber.Map{
				"id":          id.String(),
				"title":       title,
				"description": desc,
				"location":    location,
				"status":      status,
				"start_at":    startAt,
				"end_at":      endAt,
				"created_at":  createdAt,
				"updated_at":  updatedAt,
			})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"events": out})
	}
}

type oswCreateRequest struct {
	Title       string `json:"title"`
	Description string `json:"description"`
	Location    string `json:"location"`
	Status      string `json:"status"`   // upcoming|running|completed|draft
	StartAt     string `json:"start_at"` // RFC3339
	EndAt       string `json:"end_at"`   // RFC3339
}

func (h *OpenSourceWeekAdminHandler) Create() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		var req oswCreateRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}

		title := strings.TrimSpace(req.Title)
		if title == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "title_required"})
		}
		status := strings.TrimSpace(req.Status)
		if status == "" {
			status = "upcoming"
		}
		if status != "upcoming" && status != "running" && status != "completed" && status != "draft" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_status"})
		}

		startAt, err := time.Parse(time.RFC3339, strings.TrimSpace(req.StartAt))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_start_at"})
		}
		endAt, err := time.Parse(time.RFC3339, strings.TrimSpace(req.EndAt))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_end_at"})
		}
		if !endAt.After(startAt) {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "end_at_must_be_after_start_at"})
		}

		var id uuid.UUID
		err = h.db.Pool.QueryRow(c.Context(), `
INSERT INTO open_source_week_events (title, description, location, status, start_at, end_at)
VALUES ($1, NULLIF($2,''), NULLIF($3,''), $4, $5, $6)
RETURNING id
`, title, strings.TrimSpace(req.Description), strings.TrimSpace(req.Location), status, startAt, endAt).Scan(&id)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "osw_event_create_failed"})
		}

		return c.Status(fiber.StatusCreated).JSON(fiber.Map{"id": id.String()})
	}
}

func (h *OpenSourceWeekAdminHandler) Delete() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		evID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_event_id"})
		}
		ct, err := h.db.Pool.Exec(c.Context(), `DELETE FROM open_source_week_events WHERE id = $1`, evID)
		if errors.Is(err, pgx.ErrNoRows) || ct.RowsAffected() == 0 {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "event_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "osw_event_delete_failed"})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}


