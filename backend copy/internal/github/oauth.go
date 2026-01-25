package github

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"time"
)

type OAuthConfig struct {
	ClientID     string
	ClientSecret string
	RedirectURL  string
}

func AuthorizeURL(clientID string, redirectURL string, state string, scopes []string) (string, error) {
	if clientID == "" || redirectURL == "" {
		return "", fmt.Errorf("github oauth not configured")
	}
	u, _ := url.Parse("https://github.com/login/oauth/authorize")
	q := u.Query()
	q.Set("client_id", clientID)
	q.Set("redirect_uri", redirectURL)
	q.Set("state", state)
	if len(scopes) > 0 {
		// GitHub expects space-separated scopes
		q.Set("scope", joinScopes(scopes))
	}
	u.RawQuery = q.Encode()
	return u.String(), nil
}

func joinScopes(scopes []string) string {
	out := ""
	for i, s := range scopes {
		if i > 0 {
			out += " "
		}
		out += s
	}
	return out
}

type TokenResponse struct {
	AccessToken string `json:"access_token"`
	TokenType   string `json:"token_type"`
	Scope       string `json:"scope"`
}

func ExchangeCode(ctx context.Context, code string, cfg OAuthConfig) (TokenResponse, error) {
	if cfg.ClientID == "" || cfg.ClientSecret == "" || cfg.RedirectURL == "" {
		return TokenResponse{}, fmt.Errorf("github oauth not configured")
	}
	if code == "" {
		return TokenResponse{}, fmt.Errorf("code is required")
	}

	body := map[string]string{
		"client_id":     cfg.ClientID,
		"client_secret": cfg.ClientSecret,
		"code":          code,
		"redirect_uri":  cfg.RedirectURL,
	}
	b, _ := json.Marshal(body)

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, "https://github.com/login/oauth/access_token", bytes.NewReader(b))
	if err != nil {
		return TokenResponse{}, err
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 10 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return TokenResponse{}, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return TokenResponse{}, fmt.Errorf("token exchange failed: status %d", resp.StatusCode)
	}

	var tr TokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&tr); err != nil {
		return TokenResponse{}, err
	}
	if tr.AccessToken == "" {
		return TokenResponse{}, fmt.Errorf("token exchange returned empty token")
	}
	return tr, nil
}





















