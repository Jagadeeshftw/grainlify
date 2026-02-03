package handlers

import (
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"strings"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)


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

		// Load repo + issue state, issue URL, and github_issue_id for dashboard deep link.
		var fullName, issueURL string
		var state string
		var authorLogin string
		var assigneesJSON []byte
		var githubIssueID int64
		if err := h.db.Pool.QueryRow(c.Context(), `
SELECT p.github_full_name, gi.state, gi.author_login, gi.assignees, COALESCE(gi.url, ''), gi.github_issue_id
FROM projects p
JOIN github_issues gi ON gi.project_id = p.id
WHERE p.id = $1 AND p.status = 'verified' AND p.deleted_at IS NULL
  AND gi.number = $2
LIMIT 1
`, projectID, issueNumber).Scan(&fullName, &state, &authorLogin, &assigneesJSON, &issueURL, &githubIssueID); err != nil {
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

		// Build Drips Wave–style template: header, blockquote for message, maintainer instructions with links.
		quotedLines := strings.Split(req.Message, "\n")
		for i := range quotedLines {
			quotedLines[i] = "> " + quotedLines[i]
		}
		quotedMsg := strings.Join(quotedLines, "\n")
		// Deep link to this issue in the dashboard so "review their application" opens the exact issue.
		base := strings.TrimSpace(strings.TrimRight(h.cfg.FrontendBaseURL, "/"))
		reviewURL := fmt.Sprintf("%s/dashboard?tab=browse&project=%s&issue=%d", base, projectID.String(), githubIssueID)
		if base == "" || !strings.HasPrefix(base, "http") {
			// Fallback: relative path only if FrontendBaseURL not configured (link will use current origin)
			reviewURL = fmt.Sprintf("/dashboard?tab=browse&project=%s&issue=%d", projectID.String(), githubIssueID)
		}
		if issueURL == "" {
			issueURL = fmt.Sprintf("https://github.com/%s/issues/%d", fullName, issueNumber)
		}
		commentBody := fmt.Sprintf("**📋 Grainlify Application**\n\n**@%s has applied to work on this issue as part of the Grainlify program.**\n\n%s\n\n---\n\n**Repo Maintainers:** To accept this application, [review their application](%s) or [assign @%s](%s) to this issue.",
			linked.Login, quotedMsg, reviewURL, linked.Login, issueURL)
		gh := github.NewClient()
		// Post as the applicant (user token) so the commenter is the user, not the bot (like Drips Wave: user + "with Drips Wave").
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

type botCommentRequest struct {
	Body string `json:"body"`
}

// PostBotComment posts a comment on a GitHub issue as the Grainlify GitHub App (bot).
// Requires project maintainer (owner) or admin. Project must have GitHub App installed.
func (h *IssueApplicationsHandler) PostBotComment() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if strings.TrimSpace(h.cfg.GitHubAppID) == "" || strings.TrimSpace(h.cfg.GitHubAppPrivateKey) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "github_app_not_configured"})
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
		role, _ := c.Locals(auth.LocalRole).(string)

		var req botCommentRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_body"})
		}
		req.Body = strings.TrimSpace(req.Body)
		if req.Body == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "body_required"})
		}
		if len(req.Body) > 32000 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "body_too_long"})
		}

		var owner uuid.UUID
		var fullName, installationID string
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT owner_user_id, github_full_name, COALESCE(github_app_installation_id, '')
FROM projects
WHERE id = $1 AND status = 'verified' AND deleted_at IS NULL
`, projectID).Scan(&owner, &fullName, &installationID)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
		}
		if owner != userID && role != "admin" {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "forbidden"})
		}
		if installationID == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "project_has_no_github_app_installation"})
		}

		appClient, err := github.NewGitHubAppClient(h.cfg.GitHubAppID, h.cfg.GitHubAppPrivateKey)
		if err != nil {
			slog.Error("failed to create GitHub App client for bot comment", "error", err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "github_app_client_failed"})
		}
		token, err := appClient.GetInstallationToken(c.Context(), installationID)
		if err != nil {
			slog.Warn("failed to get installation token for bot comment",
				"project_id", projectID.String(),
				"installation_id", installationID,
				"error", err,
			)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "installation_token_failed"})
		}

		gh := github.NewClient()
		ghComment, err := gh.CreateIssueComment(c.Context(), token, fullName, issueNumber, req.Body)
		if err != nil {
			slog.Warn("failed to post bot comment on GitHub",
				"project_id", projectID.String(),
				"issue_number", issueNumber,
				"github_full_name", fullName,
				"error", err,
			)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "github_comment_create_failed"})
		}

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

type withdrawRequest struct {
	CommentID int64 `json:"comment_id"`
}

// Withdraw removes the applicant's application by deleting their GitHub comment. Only the comment author can withdraw.
func (h *IssueApplicationsHandler) Withdraw() fiber.Handler {
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

		var req withdrawRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_body"})
		}
		if req.CommentID <= 0 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "comment_id_required"})
		}

		linked, err := github.GetLinkedAccount(c.Context(), h.db.Pool, userID, h.cfg.TokenEncKeyB64)
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "github_not_linked"})
		}

		var fullName string
		var commentsJSON []byte
		if err := h.db.Pool.QueryRow(c.Context(), `
SELECT p.github_full_name, COALESCE(gi.comments, '[]'::jsonb)
FROM projects p
JOIN github_issues gi ON gi.project_id = p.id
WHERE p.id = $1 AND p.status = 'verified' AND p.deleted_at IS NULL AND gi.number = $2
`, projectID, issueNumber).Scan(&fullName, &commentsJSON); err != nil {
			if errors.Is(err, pgx.ErrNoRows) {
				return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "issue_not_found"})
			}
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
		}

		// Verify the comment exists and belongs to the current user before calling GitHub (avoids 403/502)
		var comments []struct {
			ID   int64  `json:"id"`
			Body string `json:"body"`
			User struct {
				Login string `json:"login"`
			} `json:"user"`
		}
		if err := json.Unmarshal(commentsJSON, &comments); err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "comments_parse_failed"})
		}
		var commentOwned bool
		for _, com := range comments {
			if com.ID == req.CommentID {
				if !strings.EqualFold(strings.TrimSpace(com.User.Login), strings.TrimSpace(linked.Login)) {
					return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "you_can_only_withdraw_your_own_application"})
				}
				commentOwned = true
				break
			}
		}
		if !commentOwned {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "comment_not_found"})
		}

		gh := github.NewClient()
		if err := gh.DeleteIssueComment(c.Context(), linked.AccessToken, fullName, req.CommentID); err != nil {
			var ghErr *github.GitHubAPIError
			if errors.As(err, &ghErr) {
				if ghErr.StatusCode == 403 {
					return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "cannot_delete_comment_forbidden"})
				}
				if ghErr.StatusCode == 404 {
					return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "comment_not_found"})
				}
			}
			slog.Warn("failed to delete github comment for withdraw",
				"project_id", projectID.String(), "issue_number", issueNumber, "comment_id", req.CommentID,
				"user_id", userID.String(), "error", err)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "github_comment_delete_failed"})
		}

		_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE github_issues
SET comments = (
  SELECT COALESCE(jsonb_agg(elem), '[]'::jsonb)
  FROM jsonb_array_elements(COALESCE(comments, '[]'::jsonb)) AS elem
  WHERE (elem->>'id')::bigint != $3
),
comments_count = GREATEST(0, COALESCE(comments_count, 0) - 1),
last_seen_at = now()
WHERE project_id = $1 AND number = $2
`, projectID, issueNumber, req.CommentID)

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}

type assignRequest struct {
	Assignee string `json:"assignee"`
}

// Assign adds the applicant as assignee on GitHub and posts a congratulations bot comment. Maintainer only.
func (h *IssueApplicationsHandler) Assign() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if strings.TrimSpace(h.cfg.GitHubAppID) == "" || strings.TrimSpace(h.cfg.GitHubAppPrivateKey) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "github_app_not_configured"})
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
		role, _ := c.Locals(auth.LocalRole).(string)

		var req assignRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_body"})
		}
		req.Assignee = strings.TrimSpace(req.Assignee)
		if req.Assignee == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "assignee_required"})
		}

		var owner uuid.UUID
		var fullName, installationID string
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT owner_user_id, github_full_name, COALESCE(github_app_installation_id, '')
FROM projects
WHERE id = $1 AND status = 'verified' AND deleted_at IS NULL
`, projectID).Scan(&owner, &fullName, &installationID)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
		}
		if owner != userID && role != "admin" {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "forbidden"})
		}
		if installationID == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "project_has_no_github_app_installation"})
		}

		appClient, err := github.NewGitHubAppClient(h.cfg.GitHubAppID, h.cfg.GitHubAppPrivateKey)
		if err != nil {
			slog.Error("failed to create GitHub App client for assign", "error", err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "github_app_client_failed"})
		}
		token, err := appClient.GetInstallationToken(c.Context(), installationID)
		if err != nil {
			slog.Warn("failed to get installation token for assign", "project_id", projectID.String(), "error", err)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "installation_token_failed"})
		}

		gh := github.NewClient()
		if err := gh.AddIssueAssignees(c.Context(), token, fullName, issueNumber, []string{req.Assignee}); err != nil {
			slog.Warn("failed to add assignee on GitHub", "project_id", projectID.String(), "issue_number", issueNumber, "assignee", req.Assignee, "error", err)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "github_assign_failed"})
		}

		assigneesJSON, _ := json.Marshal([]map[string]string{{"login": req.Assignee}})
		_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE github_issues SET assignees = $3, last_seen_at = now()
WHERE project_id = $1 AND number = $2
`, projectID, issueNumber, assigneesJSON)

		var githubIssueID int64
		_ = h.db.Pool.QueryRow(c.Context(), `SELECT github_issue_id FROM github_issues WHERE project_id = $1 AND number = $2`, projectID, issueNumber).Scan(&githubIssueID)
		base := strings.TrimSpace(strings.TrimRight(h.cfg.FrontendBaseURL, "/"))
		manageURL := base + "/dashboard?tab=browse&project=" + projectID.String() + "&issue=" + fmt.Sprintf("%d", githubIssueID)
		if base == "" || !strings.HasPrefix(base, "http") {
			manageURL = "/dashboard?tab=browse&project=" + projectID.String() + "&issue=" + fmt.Sprintf("%d", githubIssueID)
		}
		botBody := fmt.Sprintf("Congratulations, **@%s**! 🎉 Your application was accepted by the repo's maintainers.\n\n"+
			"Please resolve the issue such that the repo's maintainers have enough time to review your contribution.\n\n"+
			"> ⚠️ **Warning:** When opening a PR, please link it to this issue to ensure it gets tracked accurately.\n\n"+
			"**Repo maintainers:** You can manage this issue, including adjusting complexity and points, [here](%s).",
			req.Assignee, manageURL)

		ghComment, err := gh.CreateIssueComment(c.Context(), token, fullName, issueNumber, botBody)
		if err != nil {
			slog.Warn("assign: bot congratulations comment failed", "error", err)
		} else {
			commentJSON, _ := json.Marshal(ghComment)
			_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE github_issues SET comments = COALESCE(comments, '[]'::jsonb) || $3::jsonb,
  comments_count = COALESCE(comments_count, 0) + 1, updated_at_github = $4, last_seen_at = now()
WHERE project_id = $1 AND number = $2
`, projectID, issueNumber, commentJSON, ghComment.UpdatedAt)
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}

// Unassign removes the current assignee(s) from the GitHub issue and posts a bot comment. Maintainer only.
func (h *IssueApplicationsHandler) Unassign() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if strings.TrimSpace(h.cfg.GitHubAppID) == "" || strings.TrimSpace(h.cfg.GitHubAppPrivateKey) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "github_app_not_configured"})
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
		role, _ := c.Locals(auth.LocalRole).(string)

		var owner uuid.UUID
		var fullName, installationID string
		var assigneesJSON []byte
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT p.owner_user_id, p.github_full_name, COALESCE(p.github_app_installation_id, ''), COALESCE(gi.assignees, '[]'::jsonb)
FROM projects p
JOIN github_issues gi ON gi.project_id = p.id
WHERE p.id = $1 AND p.status = 'verified' AND p.deleted_at IS NULL AND gi.number = $2
`, projectID, issueNumber).Scan(&owner, &fullName, &installationID, &assigneesJSON)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "issue_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
		}
		if owner != userID && role != "admin" {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "forbidden"})
		}
		if installationID == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "project_has_no_github_app_installation"})
		}

		var assignees []struct {
			Login string `json:"login"`
		}
		_ = json.Unmarshal(assigneesJSON, &assignees)
		if len(assignees) == 0 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "issue_has_no_assignees"})
		}
		logins := make([]string, 0, len(assignees))
		for _, a := range assignees {
			if a.Login != "" {
				logins = append(logins, a.Login)
			}
		}
		if len(logins) == 0 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "issue_has_no_assignees"})
		}

		appClient, err := github.NewGitHubAppClient(h.cfg.GitHubAppID, h.cfg.GitHubAppPrivateKey)
		if err != nil {
			slog.Error("failed to create GitHub App client for unassign", "error", err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "github_app_client_failed"})
		}
		token, err := appClient.GetInstallationToken(c.Context(), installationID)
		if err != nil {
			slog.Warn("failed to get installation token for unassign", "project_id", projectID.String(), "error", err)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "installation_token_failed"})
		}

		gh := github.NewClient()
		if err := gh.RemoveIssueAssignees(c.Context(), token, fullName, issueNumber, logins); err != nil {
			slog.Warn("failed to remove assignees on GitHub", "project_id", projectID.String(), "issue_number", issueNumber, "error", err)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "github_unassign_failed"})
		}

		_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE github_issues SET assignees = '[]'::jsonb, last_seen_at = now()
WHERE project_id = $1 AND number = $2
`, projectID, issueNumber)

		who := "@" + logins[0]
		if len(logins) > 1 {
			who = "@" + strings.Join(logins, ", @")
		}
		botBody := fmt.Sprintf("%s has been unassigned from this issue. The maintainer may assign another contributor.", who)

		ghComment, err := gh.CreateIssueComment(c.Context(), token, fullName, issueNumber, botBody)
		if err != nil {
			slog.Warn("unassign: bot comment failed", "error", err)
		} else {
			commentJSON, _ := json.Marshal(ghComment)
			_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE github_issues SET comments = COALESCE(comments, '[]'::jsonb) || $3::jsonb,
  comments_count = COALESCE(comments_count, 0) + 1, updated_at_github = $4, last_seen_at = now()
WHERE project_id = $1 AND number = $2
`, projectID, issueNumber, commentJSON, ghComment.UpdatedAt)
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}

type rejectRequest struct {
	Assignee string `json:"assignee"`
}

// Reject posts a bot comment that the applicant's application was not accepted. Maintainer only.
func (h *IssueApplicationsHandler) Reject() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if strings.TrimSpace(h.cfg.GitHubAppID) == "" || strings.TrimSpace(h.cfg.GitHubAppPrivateKey) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "github_app_not_configured"})
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
		role, _ := c.Locals(auth.LocalRole).(string)

		var req rejectRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_body"})
		}
		req.Assignee = strings.TrimSpace(req.Assignee)
		if req.Assignee == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "assignee_required"})
		}

		var owner uuid.UUID
		var fullName, installationID string
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT owner_user_id, github_full_name, COALESCE(github_app_installation_id, '')
FROM projects
WHERE id = $1 AND status = 'verified' AND deleted_at IS NULL
`, projectID).Scan(&owner, &fullName, &installationID)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "project_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_lookup_failed"})
		}
		if owner != userID && role != "admin" {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "forbidden"})
		}
		if installationID == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "project_has_no_github_app_installation"})
		}

		appClient, err := github.NewGitHubAppClient(h.cfg.GitHubAppID, h.cfg.GitHubAppPrivateKey)
		if err != nil {
			slog.Error("failed to create GitHub App client for reject", "error", err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "github_app_client_failed"})
		}
		token, err := appClient.GetInstallationToken(c.Context(), installationID)
		if err != nil {
			slog.Warn("failed to get installation token for reject", "project_id", projectID.String(), "error", err)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "installation_token_failed"})
		}

		botBody := fmt.Sprintf("@%s your application was not accepted for this issue. The maintainer may assign another contributor.", req.Assignee)
		gh := github.NewClient()
		ghComment, err := gh.CreateIssueComment(c.Context(), token, fullName, issueNumber, botBody)
		if err != nil {
			slog.Warn("reject: bot comment failed", "error", err)
			return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "github_comment_create_failed"})
		}
		commentJSON, _ := json.Marshal(ghComment)
		_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE github_issues SET comments = COALESCE(comments, '[]'::jsonb) || $3::jsonb,
  comments_count = COALESCE(comments_count, 0) + 1, updated_at_github = $4, last_seen_at = now()
WHERE project_id = $1 AND number = $2
`, projectID, issueNumber, commentJSON, ghComment.UpdatedAt)

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}

