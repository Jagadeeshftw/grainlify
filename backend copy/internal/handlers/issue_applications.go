package handlers

import (
	"encoding/json"
	"log/slog"
	"strings"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)

const grainlifyApplicationPrefix = "[grainlify application]"

type IssueApplicationsHandler struct {
	cfg config.Config
	db  *db.DB
}

func NewIssueApplicationsHandler(cfg config.Config, d *db.DB) *IssueApplicationsHandler {
	return &IssueApplicationsHandler{cfg: cfg, db: d}
}

type applyToIssueRequest struct {
	Message string `json:"message"`
}

func (h *IssueApplicationsHandler) Apply() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if strings.TrimSpace(h.cfg.TokenEncKeyB64) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "token_encryption_not_configured"})
		}

		projectID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_project_id"})
		}
		issueNumber, err := c.ParamsInt("number")
		if err != nil || issueNumber <= 0 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_issue_number"})
		}

		userIDStr, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(userIDStr)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		var req applyToIssueRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_body"})
		}
		req.Message = strings.TrimSpace(req.Message)
		if req.Message == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "message_required"})
		}
		if len(req.Message) > 5000 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "message_too_long"})
		}

		linked, err := github.GetLinkedAccount(c.Context(), h.db.Pool, userID, h.cfg.TokenEncKeyB64)
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "github_not_linked"})
		}

		// Load repo + issue state from DB.
		var fullName string
		var state string
		var authorLogin string
		var assigneesJSON []byte
		if err := h.db.Pool.QueryRow(c.Context(), `
SELECT p.github_full_name, gi.state, gi.author_login, gi.assignees
FROM projects p
JOIN github_issues gi ON gi.project_id = p.id
WHERE p.id = $1 AND p.status = 'verified' AND p.deleted_at IS NULL
  AND gi.number = $2
LIMIT 1
`, projectID, issueNumber).Scan(&fullName, &state, &authorLogin, &assigneesJSON); err != nil {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "issue_not_found"})
		}

		if strings.ToLower(strings.TrimSpace(state)) != "open" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "issue_not_open"})
		}
		if strings.EqualFold(strings.TrimSpace(authorLogin), strings.TrimSpace(linked.Login)) {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "cannot_apply_to_own_issue"})
		}

		// "yet to be assigned" => no assignees.
		var assignees []any
		_ = json.Unmarshal(assigneesJSON, &assignees)
		if len(assignees) > 0 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "issue_already_assigned"})
		}

		// Create GitHub comment as the applicant (OAuth token).
		commentBody := grainlifyApplicationPrefix + "\n\n" + req.Message
		gh := github.NewClient()
		ghComment, err := gh.CreateIssueComment(c.Context(), linked.AccessToken, fullName, issueNumber, commentBody)
		if err != nil {
			slog.Warn("failed to create github issue comment for application",
				"project_id", projectID.String(),
				"issue_number", issueNumber,
				"github_full_name", fullName,
				"user_id", userID.String(),
				"github_login", linked.Login,
				"error", err,
			)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "github_comment_create_failed"})
		}

		// Persist the comment into our DB so maintainers see it immediately.
		commentJSON, _ := json.Marshal(ghComment)
		_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE github_issues
SET comments = COALESCE(comments, '[]'::jsonb) || $3::jsonb,
    comments_count = COALESCE(comments_count, 0) + 1,
    updated_at_github = $4,
    last_seen_at = now()
WHERE project_id = $1 AND number = $2
`, projectID, issueNumber, commentJSON, ghComment.UpdatedAt)

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"ok": true,
			"comment": fiber.Map{
				"id": ghComment.ID,
				"body": ghComment.Body,
				"user": fiber.Map{"login": ghComment.User.Login},
				"created_at": ghComment.CreatedAt,
				"updated_at": ghComment.UpdatedAt,
			},
		})
	}
}


