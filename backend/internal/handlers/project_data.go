package handlers

import (
	"encoding/json"
	"errors"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type ProjectDataHandler struct {
	db *db.DB
}

func NewProjectDataHandler(d *db.DB) *ProjectDataHandler {
	return &ProjectDataHandler{db: d}
}

// projectIDForRead returns project ID if the user is authenticated and the project exists (verified).
// Any authenticated user can read project issues/PRs/events (e.g. contributors browsing issues).
func (h *ProjectDataHandler) projectIDForRead(c *fiber.Ctx) (uuid.UUID, error) {
	if h.db == nil || h.db.Pool == nil {
		return uuid.Nil, c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
	}
	if _, ok := c.Locals(auth.LocalUserID).(string); !ok {
		return uuid.Nil, c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
	}
	projectID, err := uuid.Parse(c.Params("id"))
	if err != nil {
		return uuid.Nil, c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_project_id"})
	}
	var exists bool
	err = h.db.Pool.QueryRow(c.Context(), `
SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND status = 'verified' AND deleted_at IS NULL)
`, projectID).Scan(&exists)
	if err != nil || !exists {
		return uuid.Nil, c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
	}
	return projectID, nil
}

func (h *ProjectDataHandler) Issues() fiber.Handler {
	return func(c *fiber.Ctx) error {
		projectID, err := h.projectIDForRead(c)
		if err != nil {
			return err
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT github_issue_id, number, state, title, body, author_login, url, assignees, labels, comments_count, comments, updated_at_github, last_seen_at
FROM github_issues
WHERE project_id = $1
ORDER BY COALESCE(updated_at_github, last_seen_at) DESC
LIMIT 50
`, projectID)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "issues_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var gid int64
			var number int
			var state, title, author, url string
			var body *string
			var assigneesJSON, labelsJSON, commentsJSON []byte
			var commentsCount int
			var updated *time.Time
			var lastSeen time.Time
			if err := rows.Scan(&gid, &number, &state, &title, &body, &author, &url, &assigneesJSON, &labelsJSON, &commentsCount, &commentsJSON, &updated, &lastSeen); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "issues_list_failed"})
			}
			
			// Parse JSONB fields
			var assignees []any
			var labels []any
			var comments []any
			if len(assigneesJSON) > 0 {
				_ = json.Unmarshal(assigneesJSON, &assignees)
			}
			if len(labelsJSON) > 0 {
				_ = json.Unmarshal(labelsJSON, &labels)
			}
			if len(commentsJSON) > 0 {
				_ = json.Unmarshal(commentsJSON, &comments)
			}
			
			out = append(out, fiber.Map{
				"github_issue_id": gid,
				"number":          number,
				"state":           state,
				"title":           title,
				"description":     body, // GitHub issue body/description
				"author_login":    author,
				"assignees":       assignees,
				"labels":          labels,
				"comments_count": commentsCount,
				"comments":        comments, // Actual comments array
				"url":             url,
				"updated_at":      updated,
				"last_seen_at":    lastSeen,
			})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"issues": out})
	}
}

func (h *ProjectDataHandler) PRs() fiber.Handler {
	return func(c *fiber.Ctx) error {
		projectID, err := h.projectIDForRead(c)
		if err != nil {
			return err
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT github_pr_id, number, state, title, author_login, url, merged, 
       created_at_github, updated_at_github, closed_at_github, merged_at_github, last_seen_at
FROM github_pull_requests
WHERE project_id = $1
ORDER BY COALESCE(updated_at_github, last_seen_at) DESC
LIMIT 50
`, projectID)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "prs_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var gid int64
			var number int
			var state, title, author, url string
			var merged bool
			var createdAt, updated, closedAt, mergedAt *time.Time
			var lastSeen time.Time
			if err := rows.Scan(&gid, &number, &state, &title, &author, &url, &merged, &createdAt, &updated, &closedAt, &mergedAt, &lastSeen); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "prs_list_failed"})
			}
			out = append(out, fiber.Map{
				"github_pr_id":    gid,
				"number":          number,
				"state":           state,
				"title":           title,
				"author_login":    author,
				"url":             url,
				"merged":          merged,
				"created_at":       createdAt,
				"updated_at":      updated,
				"closed_at":       closedAt,
				"merged_at":       mergedAt,
				"last_seen_at":    lastSeen,
			})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"prs": out})
	}
}

func (h *ProjectDataHandler) Events() fiber.Handler {
	return func(c *fiber.Ctx) error {
		projectID, err := h.projectIDForRead(c)
		if err != nil {
			return err
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT delivery_id, event, action, received_at
FROM github_events
WHERE project_id = $1
ORDER BY received_at DESC
LIMIT 50
`, projectID)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "events_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var deliveryID string
			var event string
			var action *string
			var receivedAt time.Time
			if err := rows.Scan(&deliveryID, &event, &action, &receivedAt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "events_list_failed"})
			}
			out = append(out, fiber.Map{
				"delivery_id":  deliveryID,
				"event":        event,
				"action":       action,
				"received_at":  receivedAt,
			})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"events": out})
	}
}

func (h *ProjectDataHandler) authorizeProject(c *fiber.Ctx) (uuid.UUID, bool, error) {
	if h.db == nil || h.db.Pool == nil {
		return uuid.Nil, false, c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
	}
	sub, _ := c.Locals(auth.LocalUserID).(string)
	userID, err := uuid.Parse(sub)
	if err != nil {
		return uuid.Nil, false, c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
	}
	projectID, err := uuid.Parse(c.Params("id"))
	if err != nil {
		return uuid.Nil, false, c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_project_id"})
	}

	var owner uuid.UUID
	err = h.db.Pool.QueryRow(c.Context(), `SELECT owner_user_id FROM projects WHERE id = $1`, projectID).Scan(&owner)
	if errors.Is(err, pgx.ErrNoRows) {
		return uuid.Nil, false, c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
	}
	if err != nil {
		return uuid.Nil, false, c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
	}

	role, _ := c.Locals(auth.LocalRole).(string)
	ownerOK := owner == userID || role == "admin"
	return projectID, ownerOK, nil
}




