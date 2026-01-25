package syncjobs

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"os"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"golang.org/x/time/rate"

	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)

type Worker struct {
	cfg     config.Config
	pool    *pgxpool.Pool
	limiter *rate.Limiter
	gh      *github.Client
	workerID string
}

func New(cfg config.Config, pool *pgxpool.Pool) *Worker {
	return &Worker{
		cfg:      cfg,
		pool:     pool,
		limiter:  rate.NewLimiter(rate.Every(250*time.Millisecond), 2), // ~4 req/s, burst 2
		gh:       github.NewClient(),
		workerID: fmt.Sprintf("%s:%d", hostname(), os.Getpid()),
	}
}

func (w *Worker) Run(ctx context.Context) error {
	if w.pool == nil {
		return fmt.Errorf("db not configured")
	}
	t := time.NewTicker(1 * time.Second)
	defer t.Stop()

	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-t.C:
			if err := w.processOne(ctx); err != nil && !errors.Is(err, pgx.ErrNoRows) {
				slog.Error("sync worker error", "error", err)
			}
		}
	}
}

func (w *Worker) processOne(ctx context.Context) error {
	tx, err := w.pool.BeginTx(ctx, pgx.TxOptions{})
	if err != nil {
		return err
	}
	defer func() { _ = tx.Rollback(ctx) }()

	var jobID uuid.UUID
	var projectID uuid.UUID
	var jobType string
	err = tx.QueryRow(ctx, `
SELECT id, project_id, job_type
FROM sync_jobs
WHERE status = 'pending'
  AND run_at <= now()
ORDER BY run_at ASC
FOR UPDATE SKIP LOCKED
LIMIT 1
`).Scan(&jobID, &projectID, &jobType)
	if err != nil {
		return err
	}

	_, err = tx.Exec(ctx, `
UPDATE sync_jobs
SET status = 'running', locked_at = now(), locked_by = $2, updated_at = now()
WHERE id = $1
`, jobID, w.workerID)
	if err != nil {
		return err
	}

	if err := tx.Commit(ctx); err != nil {
		return err
	}

	runErr := w.runJob(ctx, jobID, projectID, jobType)

	status := "completed"
	lastErr := ""
	if runErr != nil {
		status = "failed"
		lastErr = runErr.Error()
	}

	_, _ = w.pool.Exec(ctx, `
UPDATE sync_jobs
SET status = $2, attempts = attempts + 1, last_error = NULLIF($3, ''), updated_at = now()
WHERE id = $1
`, jobID, status, lastErr)

	return nil
}

func (w *Worker) runJob(ctx context.Context, jobID uuid.UUID, projectID uuid.UUID, jobType string) error {
	// Load project + owner to get GitHub token.
	var fullName string
	var ownerUserID uuid.UUID
	err := w.pool.QueryRow(ctx, `
SELECT github_full_name, owner_user_id
FROM projects
WHERE id = $1
`, projectID).Scan(&fullName, &ownerUserID)
	if err != nil {
		slog.Error("sync job failed: project not found",
			"job_id", jobID,
			"project_id", projectID,
			"error", err,
		)
		return err
	}

	linked, err := github.GetLinkedAccount(ctx, w.pool, ownerUserID, w.cfg.TokenEncKeyB64)
	if err != nil {
		slog.Error("sync job failed: GitHub account not linked",
			"job_id", jobID,
			"project_id", projectID,
			"user_id", ownerUserID,
			"repo", fullName,
			"error", err,
			"hint", "User needs to link their GitHub account via OAuth",
		)
		return fmt.Errorf("github_not_linked: %w", err)
	}

	slog.Info("starting sync job",
		"job_id", jobID,
		"job_type", jobType,
		"project_id", projectID,
		"repo", fullName,
		"user_id", ownerUserID,
	)

	var syncErr error
	switch jobType {
	case "sync_issues":
		syncErr = w.syncIssues(ctx, projectID, fullName, linked.AccessToken)
	case "sync_prs":
		syncErr = w.syncPRs(ctx, projectID, fullName, linked.AccessToken)
	default:
		syncErr = fmt.Errorf("unknown job_type: %s", jobType)
	}

	if syncErr != nil {
		slog.Error("sync job failed",
			"job_id", jobID,
			"job_type", jobType,
			"project_id", projectID,
			"repo", fullName,
			"error", syncErr,
		)
		return syncErr
	}

	slog.Info("sync job completed successfully",
		"job_id", jobID,
		"job_type", jobType,
		"project_id", projectID,
		"repo", fullName,
	)
	return nil
}

func (w *Worker) syncIssues(ctx context.Context, projectID uuid.UUID, fullName string, token string) error {
	totalIssues := 0
	for page := 1; page <= 50; page++ { // safety cap
		if err := w.limiter.Wait(ctx); err != nil {
			return err
		}
		items, err := w.gh.ListIssuesPage(ctx, token, fullName, page)
		if err != nil {
			return err
		}
		if len(items) == 0 {
			return nil
		}

		for _, it := range items {
			// Skip PRs from the issues endpoint.
			if it.PullRequest != nil {
				continue
			}
			totalIssues++
			// Convert assignees to JSONB (array of login strings)
			assigneesJSON, _ := json.Marshal(it.Assignees)
			// Convert labels to JSONB (array of {name, color} objects)
			labelsJSON, _ := json.Marshal(it.Labels)
			
			// Parse date strings from GitHub API
			var createdAt, updatedAt, closedAt *time.Time
			if it.CreatedAt != nil && *it.CreatedAt != "" {
				if t, err := time.Parse(time.RFC3339, *it.CreatedAt); err == nil {
					createdAt = &t
				} else {
					slog.Warn("failed to parse issue created_at",
						"project_id", projectID,
						"repo", fullName,
						"issue_id", it.ID,
						"created_at", *it.CreatedAt,
						"error", err,
					)
				}
			}
			if it.UpdatedAt != nil && *it.UpdatedAt != "" {
				if t, err := time.Parse(time.RFC3339, *it.UpdatedAt); err == nil {
					updatedAt = &t
				} else {
					slog.Warn("failed to parse issue updated_at",
						"project_id", projectID,
						"repo", fullName,
						"issue_id", it.ID,
						"updated_at", *it.UpdatedAt,
						"error", err,
					)
				}
			}
			if it.ClosedAt != nil && *it.ClosedAt != "" {
				if t, err := time.Parse(time.RFC3339, *it.ClosedAt); err == nil {
					closedAt = &t
				} else {
					slog.Warn("failed to parse issue closed_at",
						"project_id", projectID,
						"repo", fullName,
						"issue_id", it.ID,
						"closed_at", *it.ClosedAt,
						"error", err,
					)
				}
			}
			
			// Fetch comments for this issue (if comments_count > 0)
			var commentsJSON []byte = []byte("[]")
			if it.Comments > 0 {
				if err := w.limiter.Wait(ctx); err == nil {
					comments, err := w.gh.ListIssueComments(ctx, token, fullName, it.Number)
					if err == nil {
						commentsJSON, _ = json.Marshal(comments)
					}
				}
			}
			
			_, _ = w.pool.Exec(ctx, `
INSERT INTO github_issues (project_id, github_issue_id, number, state, title, body, author_login, url, assignees, labels, comments_count, comments, created_at_github, updated_at_github, closed_at_github, last_seen_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, now())
ON CONFLICT (project_id, github_issue_id) DO UPDATE SET
  number = EXCLUDED.number,
  state = EXCLUDED.state,
  title = EXCLUDED.title,
  body = EXCLUDED.body,
  author_login = EXCLUDED.author_login,
  url = EXCLUDED.url,
  assignees = EXCLUDED.assignees,
  labels = EXCLUDED.labels,
  comments_count = EXCLUDED.comments_count,
  comments = EXCLUDED.comments,
  created_at_github = COALESCE(EXCLUDED.created_at_github, github_issues.created_at_github),
  updated_at_github = COALESCE(EXCLUDED.updated_at_github, github_issues.updated_at_github),
  closed_at_github = COALESCE(EXCLUDED.closed_at_github, github_issues.closed_at_github),
  last_seen_at = now()
`, projectID, it.ID, it.Number, it.State, it.Title, it.Body, it.User.Login, it.HTMLURL, assigneesJSON, labelsJSON, it.Comments, commentsJSON, createdAt, updatedAt, closedAt)
		}
	}
	
	slog.Info("sync issues completed",
		"project_id", projectID,
		"repo", fullName,
		"total_issues", totalIssues,
	)
	return nil
}

func (w *Worker) syncPRs(ctx context.Context, projectID uuid.UUID, fullName string, token string) error {
	totalPRs := 0
	for page := 1; page <= 50; page++ { // safety cap
		if err := w.limiter.Wait(ctx); err != nil {
			return err
		}
		items, err := w.gh.ListPRsPage(ctx, token, fullName, page)
		if err != nil {
			slog.Error("failed to fetch PRs page",
				"project_id", projectID,
				"repo", fullName,
				"page", page,
				"error", err,
			)
			return err
		}
		if len(items) == 0 {
			slog.Info("sync PRs completed",
				"project_id", projectID,
				"repo", fullName,
				"total_prs", totalPRs,
			)
			return nil
		}

		for _, it := range items {
			totalPRs++
			
			// Parse date strings from GitHub API
			var createdAt, updatedAt, closedAt, mergedAt *time.Time
			if it.CreatedAt != nil && *it.CreatedAt != "" {
				if t, err := time.Parse(time.RFC3339, *it.CreatedAt); err == nil {
					createdAt = &t
				}
			}
			if it.UpdatedAt != nil && *it.UpdatedAt != "" {
				if t, err := time.Parse(time.RFC3339, *it.UpdatedAt); err == nil {
					updatedAt = &t
				}
			}
			if it.ClosedAt != nil && *it.ClosedAt != "" {
				if t, err := time.Parse(time.RFC3339, *it.ClosedAt); err == nil {
					closedAt = &t
				}
			}
			if it.MergedAt != nil && *it.MergedAt != "" {
				if t, err := time.Parse(time.RFC3339, *it.MergedAt); err == nil {
					mergedAt = &t
				}
			}
			
			_, _ = w.pool.Exec(ctx, `
INSERT INTO github_pull_requests (project_id, github_pr_id, number, state, title, body, author_login, url, merged, created_at_github, updated_at_github, closed_at_github, merged_at_github, last_seen_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, now())
ON CONFLICT (project_id, github_pr_id) DO UPDATE SET
  number = EXCLUDED.number,
  state = EXCLUDED.state,
  title = EXCLUDED.title,
  body = EXCLUDED.body,
  author_login = EXCLUDED.author_login,
  url = EXCLUDED.url,
  merged = EXCLUDED.merged,
  created_at_github = EXCLUDED.created_at_github,
  updated_at_github = EXCLUDED.updated_at_github,
  closed_at_github = EXCLUDED.closed_at_github,
  merged_at_github = EXCLUDED.merged_at_github,
  last_seen_at = now()
`, projectID, it.ID, it.Number, it.State, it.Title, it.Body, it.User.Login, it.HTMLURL, it.Merged, createdAt, updatedAt, closedAt, mergedAt)
		}
	}
	return nil
}

func hostname() string {
	h, _ := os.Hostname()
	if h == "" {
		return "unknown"
	}
	return h
}




