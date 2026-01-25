package github

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"strings"
)

type IssueListItem struct {
	ID      int64  `json:"id"`
	Number  int    `json:"number"`
	State   string `json:"state"`
	Title   string `json:"title"`
	Body    string `json:"body"`
	HTMLURL string `json:"html_url"`
	User    struct {
		Login string `json:"login"`
	} `json:"user"`
	Assignees []struct {
		Login string `json:"login"`
	} `json:"assignees"`
	Labels []struct {
		Name  string `json:"name"`
		Color string `json:"color"`
	} `json:"labels"`
	Comments int `json:"comments"` // Comments count
	CreatedAt *string `json:"created_at"`
	UpdatedAt *string `json:"updated_at"`
	ClosedAt  *string `json:"closed_at"`
	// If present, the item is a PR (GitHub "issues" API includes PRs).
	PullRequest any `json:"pull_request"`
}

type PRListItem struct {
	ID      int64  `json:"id"`
	Number  int    `json:"number"`
	State   string `json:"state"`
	Title   string `json:"title"`
	Body    string `json:"body"`
	HTMLURL string `json:"html_url"`
	User    struct {
		Login string `json:"login"`
	} `json:"user"`
	Merged   bool    `json:"merged"`
	MergedAt *string `json:"merged_at"`
	CreatedAt *string `json:"created_at"`
	UpdatedAt *string `json:"updated_at"`
	ClosedAt  *string `json:"closed_at"`
}

func (c *Client) ListIssuesPage(ctx context.Context, accessToken string, fullName string, page int) ([]IssueListItem, error) {
	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return nil, err
	}
	u, _ := url.Parse("https://api.github.com/repos/" + url.PathEscape(owner) + "/" + url.PathEscape(repo) + "/issues")
	q := u.Query()
	q.Set("state", "all")
	q.Set("per_page", "100")
	q.Set("page", strconv.Itoa(page))
	u.RawQuery = q.Encode()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u.String(), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("github list issues failed: status %d", resp.StatusCode)
	}

	var items []IssueListItem
	if err := json.NewDecoder(resp.Body).Decode(&items); err != nil {
		return nil, err
	}
	return items, nil
}

func (c *Client) ListPRsPage(ctx context.Context, accessToken string, fullName string, page int) ([]PRListItem, error) {
	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return nil, err
	}
	u, _ := url.Parse("https://api.github.com/repos/" + url.PathEscape(owner) + "/" + url.PathEscape(repo) + "/pulls")
	q := u.Query()
	q.Set("state", "all")
	q.Set("per_page", "100")
	q.Set("page", strconv.Itoa(page))
	u.RawQuery = q.Encode()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u.String(), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("github list prs failed: status %d", resp.StatusCode)
	}

	var items []PRListItem
	if err := json.NewDecoder(resp.Body).Decode(&items); err != nil {
		return nil, err
	}
	return items, nil
}

// IssueComment represents a comment on a GitHub issue.
type IssueComment struct {
	ID        int64  `json:"id"`
	Body      string `json:"body"`
	User      struct {
		Login string `json:"login"`
	} `json:"user"`
	CreatedAt string `json:"created_at"`
	UpdatedAt string `json:"updated_at"`
}

// ListIssueComments fetches all comments for a specific issue.
func (c *Client) ListIssueComments(ctx context.Context, accessToken string, fullName string, issueNumber int) ([]IssueComment, error) {
	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return nil, err
	}
	u, _ := url.Parse(fmt.Sprintf("https://api.github.com/repos/%s/%s/issues/%d/comments",
		url.PathEscape(owner), url.PathEscape(repo), issueNumber))

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u.String(), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("github list issue comments failed: status %d", resp.StatusCode)
	}

	var comments []IssueComment
	if err := json.NewDecoder(resp.Body).Decode(&comments); err != nil {
		return nil, err
	}
	return comments, nil
}

func looksLikeRFC3339(s string) bool {
	// cheap heuristic; actual parsing happens where stored.
	return strings.Contains(s, "T") && (strings.HasSuffix(s, "Z") || strings.Contains(s, "+") || strings.Contains(s, "-"))
}




