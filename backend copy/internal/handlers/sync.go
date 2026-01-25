package handlers

import (
	"errors"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type SyncHandler struct {
	db *db.DB
}

func NewSyncHandler(d *db.DB) *SyncHandler {
	return &SyncHandler{db: d}
}

func (h *SyncHandler) EnqueueFullSync() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		projectID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_project_id"})
		}

		var owner uuid.UUID
		err = h.db.Pool.QueryRow(c.Context(), `SELECT owner_user_id FROM projects WHERE id = $1`, projectID).Scan(&owner)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
		}

		role, _ := c.Locals(auth.LocalRole).(string)
		if owner != userID && role != "admin" {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "forbidden"})
		}

		_, _ = h.db.Pool.Exec(c.Context(), `
INSERT INTO sync_jobs (project_id, job_type, status, run_at)
VALUES ($1, 'sync_issues', 'pending', now()),
       ($1, 'sync_prs', 'pending', now())
`, projectID)

		return c.Status(fiber.StatusAccepted).JSON(fiber.Map{"queued": true})
	}
}

func (h *SyncHandler) JobsForProject() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		projectID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_project_id"})
		}

		var owner uuid.UUID
		err = h.db.Pool.QueryRow(c.Context(), `SELECT owner_user_id FROM projects WHERE id = $1`, projectID).Scan(&owner)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
		}

		role, _ := c.Locals(auth.LocalRole).(string)
		if owner != userID && role != "admin" {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "forbidden"})
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT id, job_type, status, run_at, attempts, last_error, created_at, updated_at
FROM sync_jobs
WHERE project_id = $1
ORDER BY created_at DESC
LIMIT 50
`, projectID)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "jobs_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var id uuid.UUID
			var jobType, status string
			var runAt, createdAt, updatedAt time.Time
			var attempts int
			var lastErr *string
			if err := rows.Scan(&id, &jobType, &status, &runAt, &attempts, &lastErr, &createdAt, &updatedAt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "jobs_list_failed"})
			}
			out = append(out, fiber.Map{
				"id":         id.String(),
				"job_type":   jobType,
				"status":     status,
				"run_at":     runAt,
				"attempts":   attempts,
				"last_error": lastErr,
				"created_at": createdAt,
				"updated_at": updatedAt,
			})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"jobs": out})
	}
}





















