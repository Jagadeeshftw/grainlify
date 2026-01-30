package handlers

import (
	"errors"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type ProjectsAdminHandler struct {
	db *db.DB
}

func NewProjectsAdminHandler(d *db.DB) *ProjectsAdminHandler {
	return &ProjectsAdminHandler{db: d}
}

func (h *ProjectsAdminHandler) Delete() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		projectID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_project_id"})
		}

		ct, err := h.db.Pool.Exec(c.Context(), `
UPDATE projects
SET deleted_at = now(), updated_at = now()
WHERE id = $1 AND deleted_at IS NULL
`, projectID)
		if errors.Is(err, pgx.ErrNoRows) || ct.RowsAffected() == 0 {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_delete_failed"})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}
