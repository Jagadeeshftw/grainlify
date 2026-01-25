package github

import (
	"context"
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// GitHubAppClient handles GitHub App API calls
type GitHubAppClient struct {
	AppID      string
	PrivateKey *rsa.PrivateKey
	HTTP       *http.Client
	UserAgent  string
}

// NewGitHubAppClient creates a new GitHub App client
func NewGitHubAppClient(appID string, privateKeyPEM string) (*GitHubAppClient, error) {
	// Try to decode base64 private key first, fallback to raw PEM
	keyBytes := []byte(privateKeyPEM)
	decoded, err := base64.StdEncoding.DecodeString(privateKeyPEM)
	if err == nil {
		// Successfully decoded from base64
		keyBytes = decoded
	}
	// If base64 decode fails, use the raw string (it's already PEM format)

	// Parse RSA private key
	privateKey, err := jwt.ParseRSAPrivateKeyFromPEM(keyBytes)
	if err != nil {
		return nil, fmt.Errorf("failed to parse private key: %w", err)
	}

	return &GitHubAppClient{
		AppID:      appID,
		PrivateKey: privateKey,
		HTTP:       &http.Client{Timeout: 10 * time.Second},
		UserAgent:  "grainlify-backend",
	}, nil
}

// GenerateJWT generates a JWT token for GitHub App authentication
func (c *GitHubAppClient) GenerateJWT() (string, error) {
	now := time.Now()
	claims := jwt.MapClaims{
		"iat": now.Add(-60 * time.Second).Unix(), // Issued at time (allow 60s clock skew)
		"exp": now.Add(10 * time.Minute).Unix(),   // Expires in 10 minutes
		"iss": c.AppID,                            // Issuer is the App ID
	}

	token := jwt.NewWithClaims(jwt.SigningMethodRS256, claims)
	tokenString, err := token.SignedString(c.PrivateKey)
	if err != nil {
		return "", fmt.Errorf("failed to sign JWT: %w", err)
	}

	return tokenString, nil
}

// InstallationTokenResponse represents the response from GitHub's installation token endpoint
type InstallationTokenResponse struct {
	Token     string    `json:"token"`
	ExpiresAt time.Time `json:"expires_at"`
}

// GetInstallationToken gets an installation access token for a specific installation
func (c *GitHubAppClient) GetInstallationToken(ctx context.Context, installationID string) (string, error) {
	jwtToken, err := c.GenerateJWT()
	if err != nil {
		return "", fmt.Errorf("failed to generate JWT: %w", err)
	}

	url := fmt.Sprintf("https://api.github.com/app/installations/%s/access_tokens", installationID)
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, nil)
	if err != nil {
		return "", err
	}

	req.Header.Set("Authorization", "Bearer "+jwtToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		var errBody map[string]interface{}
		json.NewDecoder(resp.Body).Decode(&errBody)
		return "", fmt.Errorf("failed to get installation token: status %d, error: %v", resp.StatusCode, errBody)
	}

	var tokenResp InstallationTokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&tokenResp); err != nil {
		return "", err
	}

	return tokenResp.Token, nil
}

// InstallationRepository represents a repository in a GitHub App installation
type InstallationRepository struct {
	ID       int64  `json:"id"`
	FullName string `json:"full_name"`
	Name     string `json:"name"`
	Private  bool   `json:"private"`
	Owner    struct {
		ID    int64  `json:"id"`
		Login string `json:"login"`
		Type  string `json:"type"` // "User" or "Organization"
	} `json:"owner"`
	Language    *string `json:"language"`
	Description *string `json:"description"`
	Topics      []string `json:"topics"`
}

// ListInstallationRepositories lists all repositories accessible to an installation
func (c *GitHubAppClient) ListInstallationRepositories(ctx context.Context, installationToken string) ([]InstallationRepository, error) {
	url := "https://api.github.com/installation/repositories"
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}

	req.Header.Set("Authorization", "Bearer "+installationToken)
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
		var errBody map[string]interface{}
		json.NewDecoder(resp.Body).Decode(&errBody)
		return nil, fmt.Errorf("failed to list repositories: status %d, error: %v", resp.StatusCode, errBody)
	}

	var result struct {
		Repositories []InstallationRepository `json:"repositories"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}

	return result.Repositories, nil
}

