package github

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"time"
)

type issueCommentCreateResponse struct {
	ID   int64  `json:"id"`
	Body string `json:"body"`
	User struct {
		Login string `json:"login"`
	} `json:"user"`
	HTMLURL   string    `json:"html_url"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

func (c *Client) CreateIssueComment(ctx context.Context, accessToken string, fullName string, issueNumber int, body string) (IssueComment, error) {
	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return IssueComment{}, err
	}
	if strings.TrimSpace(accessToken) == "" {
		return IssueComment{}, fmt.Errorf("missing github access token")
	}
	if issueNumber <= 0 {
		return IssueComment{}, fmt.Errorf("invalid issue number")
	}
	if strings.TrimSpace(body) == "" {
		return IssueComment{}, fmt.Errorf("comment body is required")
	}

	u := "https://api.github.com/repos/" + url.PathEscape(owner) + "/" + url.PathEscape(repo) + "/issues/" + fmt.Sprintf("%d", issueNumber) + "/comments"
	payload := map[string]string{"body": body}
	b, _ := json.Marshal(payload)

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, u, bytes.NewReader(b))
	if err != nil {
		return IssueComment{}, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("Content-Type", "application/json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return IssueComment{}, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return IssueComment{}, parseGitHubAPIError(resp)
	}

	var out issueCommentCreateResponse
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return IssueComment{}, err
	}
	if out.ID == 0 {
		return IssueComment{}, fmt.Errorf("invalid github comment response")
	}
	// Reuse the existing IssueComment type used by ListIssueComments.
	return IssueComment{
		ID:   out.ID,
		Body: out.Body,
		User: struct {
			Login string `json:"login"`
		}{Login: out.User.Login},
		CreatedAt: out.CreatedAt.UTC().Format(time.RFC3339),
		UpdatedAt: out.UpdatedAt.UTC().Format(time.RFC3339),
	}, nil
}

// DeleteIssueComment deletes a comment on a GitHub issue. The accessToken must belong to the comment author or a repo admin.
func (c *Client) DeleteIssueComment(ctx context.Context, accessToken string, fullName string, commentID int64) error {
	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return err
	}
	if commentID <= 0 {
		return fmt.Errorf("invalid comment id")
	}

	u := "https://api.github.com/repos/" + url.PathEscape(owner) + "/" + url.PathEscape(repo) + "/issues/comments/" + fmt.Sprintf("%d", commentID)
	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, u, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != 204 && (resp.StatusCode < 200 || resp.StatusCode >= 300) {
		return parseGitHubAPIError(resp)
	}
	return nil
}


