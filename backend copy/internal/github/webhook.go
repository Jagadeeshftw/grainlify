package github

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
)

type CreateWebhookRequest struct {
	URL    string
	Secret string
	Events []string
	Active bool
}

type Webhook struct {
	ID int64 `json:"id"`
}

func (c *Client) CreateWebhook(ctx context.Context, accessToken string, fullName string, req CreateWebhookRequest) (Webhook, error) {
	if req.URL == "" || req.Secret == "" {
		return Webhook{}, fmt.Errorf("webhook url and secret are required")
	}
	if len(req.Events) == 0 {
		req.Events = []string{"issues", "pull_request", "pull_request_review", "push"}
	}

	owner, repo, err := splitFullName(fullName)
	if err != nil {
		return Webhook{}, err
	}
	u := "https://api.github.com/repos/" + url.PathEscape(owner) + "/" + url.PathEscape(repo) + "/hooks"

	body := map[string]any{
		"name":   "web",
		"active": req.Active,
		"events": req.Events,
		"config": map[string]any{
			"url":          req.URL,
			"content_type": "json",
			"secret":       req.Secret,
			"insecure_ssl": "0",
		},
	}
	b, _ := json.Marshal(body)

	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, u, bytes.NewReader(b))
	if err != nil {
		return Webhook{}, err
	}
	httpReq.Header.Set("Authorization", "Bearer "+accessToken)
	httpReq.Header.Set("Accept", "application/vnd.github+json")
	httpReq.Header.Set("Content-Type", "application/json")
	if c.UserAgent != "" {
		httpReq.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(httpReq)
	if err != nil {
		return Webhook{}, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return Webhook{}, fmt.Errorf("github webhook create failed: status %d", resp.StatusCode)
	}

	var wh Webhook
	if err := json.NewDecoder(resp.Body).Decode(&wh); err != nil {
		return Webhook{}, err
	}
	if wh.ID == 0 {
		return Webhook{}, fmt.Errorf("invalid github webhook response")
	}
	return wh, nil
}


