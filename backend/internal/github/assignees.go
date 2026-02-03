package github

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
)

// AddIssueAssignees adds assignees to a GitHub issue. Requires repo write permission (maintainer).
func (c *Client) AddIssueAssignees(ctx context.Context, accessToken string, fullName string, issueNumber int, logins []string) error {
	if issueNumber <= 0 || len(logins) == 0 {
		return fmt.Errorf("invalid issue number or assignees")
	}
	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return err
	}

	u := "https://api.github.com/repos/" + url.PathEscape(owner) + "/" + url.PathEscape(repo) + "/issues/" + fmt.Sprintf("%d", issueNumber) + "/assignees"
	payload := map[string][]string{"assignees": logins}
	b, _ := json.Marshal(payload)

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, u, bytes.NewReader(b))
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("Content-Type", "application/json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return parseGitHubAPIError(resp)
	}
	return nil
}

// RemoveIssueAssignees removes assignees from a GitHub issue. Requires repo write permission.
func (c *Client) RemoveIssueAssignees(ctx context.Context, accessToken string, fullName string, issueNumber int, logins []string) error {
	if issueNumber <= 0 || len(logins) == 0 {
		return fmt.Errorf("invalid issue number or assignees")
	}
	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return err
	}

	u := "https://api.github.com/repos/" + url.PathEscape(owner) + "/" + url.PathEscape(repo) + "/issues/" + fmt.Sprintf("%d", issueNumber) + "/assignees"
	payload := map[string][]string{"assignees": logins}
	b, _ := json.Marshal(payload)

	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, u, bytes.NewReader(b))
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("Content-Type", "application/json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return parseGitHubAPIError(resp)
	}
	return nil
}
