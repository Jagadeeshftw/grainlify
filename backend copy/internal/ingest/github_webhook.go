package ingest

import (
	"context"
	"encoding/json"
	"log/slog"
	"strings"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/jagadeesh/grainlify/backend/internal/events"
)

type GitHubWebhookIngestor struct {
	Pool *pgxpool.Pool
}

func (i *GitHubWebhookIngestor) Ingest(ctx context.Context, e events.GitHubWebhookReceived) error {
	if i == nil || i.Pool == nil {
		return nil
	}

	// Parse minimal envelope for mapping to project and snapshot upserts.
	var env ghWebhookEnvelope
	_ = json.Unmarshal(e.Payload, &env)

	repoFullName := strings.TrimSpace(e.RepoFullName)
	if repoFullName == "" && env.Repository != nil {
		repoFullName = strings.TrimSpace(env.Repository.FullName)
	}
	action := strings.TrimSpace(e.Action)
	if action == "" {
		action = strings.TrimSpace(env.Action)
	}

	var projectID *string
	if repoFullName != "" {
		var pid string
		if err := i.Pool.QueryRow(ctx, `SELECT id FROM projects WHERE github_full_name = $1`, repoFullName).Scan(&pid); err == nil {
			projectID = &pid
		}
	}

	// Auditable event record (idempotent via delivery_id primary key).
	if e.DeliveryID != "" {
		_, _ = i.Pool.Exec(ctx, `
INSERT INTO github_events (delivery_id, project_id, repo_full_name, event, action, payload)
VALUES ($1, $2::uuid, $3, $4, $5, $6::jsonb)
ON CONFLICT (delivery_id) DO NOTHING
`, e.DeliveryID, projectID, repoFullName, e.Event, nullIfEmpty(action), string(e.Payload))
	}

	// Snapshot upserts (idempotent).
	if projectID != nil {
		if e.Event == "issues" && env.Issue != nil {
			issue := env.Issue
			_, _ = i.Pool.Exec(ctx, `
INSERT INTO github_issues (project_id, github_issue_id, number, state, title, body, author_login, url, created_at_github, updated_at_github, closed_at_github, last_seen_at)
VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now())
ON CONFLICT (project_id, github_issue_id) DO UPDATE SET
  number = EXCLUDED.number,
  state = EXCLUDED.state,
  title = EXCLUDED.title,
  body = EXCLUDED.body,
  author_login = EXCLUDED.author_login,
  url = EXCLUDED.url,
  created_at_github = EXCLUDED.created_at_github,
  updated_at_github = EXCLUDED.updated_at_github,
  closed_at_github = EXCLUDED.closed_at_github,
  last_seen_at = now()
`, *projectID, issue.ID, issue.Number, issue.State, issue.Title, issue.Body, issue.User.Login, issue.HTMLURL, issue.CreatedAt, issue.UpdatedAt, issue.ClosedAt)
		}

		if (e.Event == "pull_request" || e.Event == "pull_request_review") && env.PullRequest != nil {
			pr := env.PullRequest
			_, _ = i.Pool.Exec(ctx, `
INSERT INTO github_pull_requests (project_id, github_pr_id, number, state, title, body, author_login, url, merged, merged_at_github, created_at_github, updated_at_github, closed_at_github, last_seen_at)
VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, now())
ON CONFLICT (project_id, github_pr_id) DO UPDATE SET
  number = EXCLUDED.number,
  state = EXCLUDED.state,
  title = EXCLUDED.title,
  body = EXCLUDED.body,
  author_login = EXCLUDED.author_login,
  url = EXCLUDED.url,
  merged = EXCLUDED.merged,
  merged_at_github = EXCLUDED.merged_at_github,
  created_at_github = EXCLUDED.created_at_github,
  updated_at_github = EXCLUDED.updated_at_github,
  closed_at_github = EXCLUDED.closed_at_github,
  last_seen_at = now()
`, *projectID, pr.ID, pr.Number, pr.State, pr.Title, pr.Body, pr.User.Login, pr.HTMLURL, pr.Merged, pr.MergedAt, pr.CreatedAt, pr.UpdatedAt, pr.ClosedAt)
		}
	}

	// Enqueue follow-up sync jobs (best-effort).
	if projectID != nil && (e.Event == "issues" || e.Event == "pull_request" || e.Event == "push") {
		_, _ = i.Pool.Exec(ctx, `
INSERT INTO sync_jobs (project_id, job_type, status, run_at)
VALUES ($1::uuid, 'sync_issues', 'pending', now()),
       ($1::uuid, 'sync_prs', 'pending', now())
`, *projectID)
	}

	// Handle GitHub App installation events
	if e.Event == "installation" || e.Event == "installation_repositories" {
		slog.Info("received installation webhook",
			"event", e.Event,
			"action", e.Action,
			"delivery_id", e.DeliveryID,
		)
		i.handleInstallationEvent(ctx, e, env)
	}

	return nil
}

// handleInstallationEvent handles GitHub App installation/uninstallation events
func (i *GitHubWebhookIngestor) handleInstallationEvent(ctx context.Context, e events.GitHubWebhookReceived, env ghWebhookEnvelope) {
	var installationPayload ghInstallationPayload
	if err := json.Unmarshal(e.Payload, &installationPayload); err != nil {
		slog.Error("failed to parse installation webhook payload", "error", err)
		return
	}

	action := strings.ToLower(strings.TrimSpace(installationPayload.Action))
	installationID := installationPayload.Installation.ID.String() // Convert json.Number to string

	slog.Info("handling installation event",
		"event", e.Event,
		"action", action,
		"installation_id", installationID,
	)

	if action == "deleted" {
		// Installation was completely uninstalled - mark all projects from this installation as deleted
		result, err := i.Pool.Exec(ctx, `
UPDATE projects
SET deleted_at = now(),
    status = 'rejected',
    updated_at = now()
WHERE github_app_installation_id = $1
  AND deleted_at IS NULL
`, installationID)
		if err != nil {
			slog.Error("failed to delete projects for installation", "installation_id", installationID, "error", err)
			return
		}
		rowsAffected := result.RowsAffected()
		slog.Info("marked projects as deleted for installation",
			"installation_id", installationID,
			"rows_affected", rowsAffected,
		)
	} else if action == "removed" && e.Event == "installation_repositories" {
		// Specific repositories were removed from installation
		if installationPayload.RepositoriesRemoved != nil {
			slog.Info("removing repositories from installation",
				"count", len(installationPayload.RepositoriesRemoved),
				"installation_id", installationID,
			)
			for _, repo := range installationPayload.RepositoriesRemoved {
				repoFullName := strings.TrimSpace(repo.FullName)
				if repoFullName != "" {
					result, err := i.Pool.Exec(ctx, `
UPDATE projects
SET deleted_at = now(),
    status = 'rejected',
    updated_at = now()
WHERE github_full_name = $1
  AND (github_app_installation_id = $2 OR github_app_installation_id IS NULL)
  AND deleted_at IS NULL
`, repoFullName, installationID)
					if err != nil {
						slog.Error("failed to delete project", "repo", repoFullName, "error", err)
						continue
					}
					rowsAffected := result.RowsAffected()
					if rowsAffected > 0 {
						slog.Info("marked project as deleted",
							"repo", repoFullName,
							"installation_id", installationID,
						)
					} else {
						slog.Warn("no project found to delete",
							"repo", repoFullName,
							"installation_id", installationID,
						)
					}
				}
			}
		}
	} else if action == "added" && e.Event == "installation_repositories" {
		// Repositories were added back to installation - restore them
		if installationPayload.RepositoriesAdded != nil {
			for _, repo := range installationPayload.RepositoriesAdded {
				repoFullName := strings.TrimSpace(repo.FullName)
				if repoFullName != "" {
					_, _ = i.Pool.Exec(ctx, `
UPDATE projects
SET deleted_at = NULL,
    status = 'verified',
    updated_at = now()
WHERE github_full_name = $1
  AND github_app_installation_id = $2
  AND deleted_at IS NOT NULL
`, repoFullName, installationID)
				}
			}
		}
	}
}

type ghWebhookEnvelope struct {
	Action      string               `json:"action"`
	Repository  *ghRepoPayload       `json:"repository"`
	Issue       *ghIssuePayload      `json:"issue"`
	PullRequest *ghPullRequestPayload `json:"pull_request"`
}

type ghRepoPayload struct {
	FullName string `json:"full_name"`
}

type ghUserPayload struct {
	Login string `json:"login"`
}

type ghIssuePayload struct {
	ID        int64         `json:"id"`
	Number    int           `json:"number"`
	State     string        `json:"state"`
	Title     string        `json:"title"`
	Body      string        `json:"body"`
	HTMLURL   string        `json:"html_url"`
	User      ghUserPayload `json:"user"`
	CreatedAt *time.Time    `json:"created_at"`
	UpdatedAt *time.Time    `json:"updated_at"`
	ClosedAt  *time.Time    `json:"closed_at"`
}

type ghPullRequestPayload struct {
	ID        int64         `json:"id"`
	Number    int           `json:"number"`
	State     string        `json:"state"`
	Title     string        `json:"title"`
	Body      string        `json:"body"`
	HTMLURL   string        `json:"html_url"`
	User      ghUserPayload `json:"user"`
	Merged    bool          `json:"merged"`
	MergedAt  *time.Time    `json:"merged_at"`
	CreatedAt *time.Time    `json:"created_at"`
	UpdatedAt *time.Time    `json:"updated_at"`
	ClosedAt  *time.Time    `json:"closed_at"`
}

type ghInstallationPayload struct {
	Action                string                    `json:"action"`
	Installation           ghInstallationInfo        `json:"installation"`
	RepositoriesRemoved    []ghRepoPayload           `json:"repositories_removed,omitempty"`
	RepositoriesAdded      []ghRepoPayload           `json:"repositories_added,omitempty"`
	RepositorySelection    string                    `json:"repository_selection,omitempty"`
}

type ghInstallationInfo struct {
	ID json.Number `json:"id"` // GitHub returns installation ID as a number
}

func nullIfEmpty(s string) any {
	if strings.TrimSpace(s) == "" {
		return nil
	}
	return s
}







