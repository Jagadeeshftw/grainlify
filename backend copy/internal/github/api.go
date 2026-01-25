package github

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"
)

type Client struct {
	HTTP      *http.Client
	UserAgent string
}

func NewClient() *Client {
	return &Client{
		HTTP:      &http.Client{Timeout: 10 * time.Second},
		UserAgent: "patchwork-backend",
	}
}

type User struct {
	ID        int64  `json:"id"`
	Login     string `json:"login"`
	AvatarURL string `json:"avatar_url"`
	Name      string `json:"name"`
	Email     string `json:"email"`
	Location  string `json:"location"`
	Bio       string `json:"bio"`
	Blog      string `json:"blog"` // Website URL
}

type Email struct {
	Email      string `json:"email"`
	Primary    bool   `json:"primary"`
	Verified   bool   `json:"verified"`
	Visibility string `json:"visibility"`
}

func (c *Client) GetUser(ctx context.Context, accessToken string) (User, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "https://api.github.com/user", nil)
	if err != nil {
		return User{}, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github+json")
	if c.UserAgent != "" {
		req.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return User{}, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return User{}, fmt.Errorf("github /user failed: status %d", resp.StatusCode)
	}

	var u User
	if err := json.NewDecoder(resp.Body).Decode(&u); err != nil {
		return User{}, err
	}
	if u.ID == 0 || u.Login == "" {
		return User{}, fmt.Errorf("invalid github user response")
	}
	return u, nil
}

// GetUserEmails fetches the user's email addresses from GitHub
// Requires user:email scope
func (c *Client) GetUserEmails(ctx context.Context, accessToken string) ([]Email, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "https://api.github.com/user/emails", nil)
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
		return nil, fmt.Errorf("github /user/emails failed: status %d", resp.StatusCode)
	}

	var emails []Email
	if err := json.NewDecoder(resp.Body).Decode(&emails); err != nil {
		return nil, err
	}
	return emails, nil
}

// GetPrimaryEmail gets the primary email from the user's emails list
func (c *Client) GetPrimaryEmail(ctx context.Context, accessToken string) (string, error) {
	emails, err := c.GetUserEmails(ctx, accessToken)
	if err != nil {
		return "", err
	}
	
	// Find primary email
	for _, email := range emails {
		if email.Primary && email.Verified {
			return email.Email, nil
		}
	}
	
	// If no primary verified email, return first verified email
	for _, email := range emails {
		if email.Verified {
			return email.Email, nil
		}
	}
	
	// If no verified email, return first email
	if len(emails) > 0 {
		return emails[0].Email, nil
	}
	
	return "", fmt.Errorf("no email found")
}

