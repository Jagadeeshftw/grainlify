package handlers

import (
	"crypto/rand"
	"encoding/base64"
	"errors"
	"log/slog"
	"net/url"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/cryptox"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)

type GitHubOAuthHandler struct {
	cfg config.Config
	db  *db.DB
}

func NewGitHubOAuthHandler(cfg config.Config, d *db.DB) *GitHubOAuthHandler {
	return &GitHubOAuthHandler{cfg: cfg, db: d}
}

func (h *GitHubOAuthHandler) Start() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if h.cfg.GitHubOAuthClientID == "" || effectiveGitHubRedirect(h.cfg) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "github_oauth_not_configured"})
		}

		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		state := randomState(32)
		expiresAt := time.Now().UTC().Add(10 * time.Minute)

		_, err = h.db.Pool.Exec(c.Context(), `
INSERT INTO oauth_states (state, user_id, kind, expires_at)
VALUES ($1, $2, 'github_link', $3)
`, state, userID, expiresAt)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "state_create_failed"})
		}

		// Scopes:
		// - read:user: link identity
		// - user:email: access user email addresses
		// - repo: access private repos + read repo metadata
		// - admin:repo_hook: create webhooks
		// - read:org: helps when dealing with org-owned repos
		authURL, err := github.AuthorizeURL(h.cfg.GitHubOAuthClientID, effectiveGitHubRedirect(h.cfg), state, []string{"read:user", "user:email", "repo", "admin:repo_hook", "read:org"})
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "auth_url_failed"})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"url": authURL})
	}
}

// LoginStart begins GitHub-only login/signup (no prior JWT required).
func (h *GitHubOAuthHandler) LoginStart() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if h.cfg.GitHubOAuthClientID == "" || effectiveGitHubRedirect(h.cfg) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "github_login_not_configured"})
		}

		state := randomState(32)
		expiresAt := time.Now().UTC().Add(10 * time.Minute)

		_, err := h.db.Pool.Exec(c.Context(), `
INSERT INTO oauth_states (state, user_id, kind, expires_at)
VALUES ($1, NULL, 'github_login', $2)
`, state, expiresAt)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "state_create_failed"})
		}

		// Login scopes: identity + email + repo access for later project verification.
		authURL, err := github.AuthorizeURL(h.cfg.GitHubOAuthClientID, effectiveGitHubRedirect(h.cfg), state, []string{"read:user", "user:email", "repo", "admin:repo_hook", "read:org"})
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "auth_url_failed"})
		}

		// Redirect user to GitHub OAuth page
		slog.Info("redirecting to github oauth", "url", authURL)
		return c.Redirect(authURL, fiber.StatusFound)
	}
}

// CallbackUnified finishes either:
// - github_login: GitHub-only login/signup (issues JWT)
// - github_link: link/re-authorize GitHub for an existing user
//
// Recommended for production: configure ONE GitHub OAuth callback URL and point it to this handler.
func (h *GitHubOAuthHandler) CallbackUnified() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if h.cfg.GitHubOAuthClientID == "" || h.cfg.GitHubOAuthClientSecret == "" || effectiveGitHubRedirect(h.cfg) == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "github_oauth_not_configured"})
		}
		if h.cfg.JWTSecret == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "jwt_not_configured"})
		}

		code := c.Query("code")
		state := c.Query("state")
		if code == "" || state == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "missing_code_or_state"})
		}

		var storedKind string
		var stateUserID *uuid.UUID
		err := h.db.Pool.QueryRow(c.Context(), `
SELECT kind, user_id
FROM oauth_states
WHERE state = $1
  AND expires_at > now()
`, state).Scan(&storedKind, &stateUserID)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_or_expired_state"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "state_lookup_failed"})
		}

		_, _ = h.db.Pool.Exec(c.Context(), `DELETE FROM oauth_states WHERE state = $1`, state)

		tr, err := github.ExchangeCode(c.Context(), code, github.OAuthConfig{
			ClientID:     h.cfg.GitHubOAuthClientID,
			ClientSecret: h.cfg.GitHubOAuthClientSecret,
			RedirectURL:  effectiveGitHubRedirect(h.cfg),
		})
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "token_exchange_failed"})
		}

		encKey, err := cryptox.KeyFromB64(h.cfg.TokenEncKeyB64)
		if err != nil {
			slog.Error("failed to load encryption key", "error", err, "config_len", len(h.cfg.TokenEncKeyB64))
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "token_encryption_not_configured"})
		}
		encToken, err := cryptox.EncryptAESGCM(encKey, []byte(tr.AccessToken))
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "token_encrypt_failed"})
		}

		gh := github.NewClient()
		u, err := gh.GetUser(c.Context(), tr.AccessToken)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "github_user_fetch_failed"})
		}

		var userID uuid.UUID
		var role string
		switch storedKind {
		case "github_login":
			// Create-or-find user by github_user_id.
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT id, role
FROM users
WHERE github_user_id = $1
`, u.ID).Scan(&userID, &role)
			if errors.Is(err, pgx.ErrNoRows) {
				err = h.db.Pool.QueryRow(c.Context(), `
INSERT INTO users (github_user_id) VALUES ($1)
RETURNING id, role
`, u.ID).Scan(&userID, &role)
			}
			if err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "user_upsert_failed"})
			}
		case "github_link":
			if stateUserID == nil {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_state_user"})
			}
			userID = *stateUserID
			// Fetch role for JWT issuance.
			if err := h.db.Pool.QueryRow(c.Context(), `SELECT role FROM users WHERE id = $1`, userID).Scan(&role); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "user_lookup_failed"})
			}
		default:
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "wrong_state_kind"})
		}

		_, err = h.db.Pool.Exec(c.Context(), `
INSERT INTO github_accounts (user_id, github_user_id, login, avatar_url, access_token, token_type, scope)
VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (user_id) DO UPDATE SET
  github_user_id = EXCLUDED.github_user_id,
  login = EXCLUDED.login,
  avatar_url = EXCLUDED.avatar_url,
  access_token = EXCLUDED.access_token,
  token_type = EXCLUDED.token_type,
  scope = EXCLUDED.scope,
  updated_at = now()
`, userID, u.ID, u.Login, u.AvatarURL, encToken, tr.TokenType, tr.Scope)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "github_account_upsert_failed"})
		}

		// Ensure users.github_user_id is set (idempotent).
		_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE users SET github_user_id = $2, updated_at = now() WHERE id = $1
`, userID, u.ID)

		// For login: issue JWT. For link: we can optionally redirect without token.
		if storedKind == "github_login" {
			jwtToken, err := auth.IssueJWT(h.cfg.JWTSecret, userID, role, "", "", 60*time.Minute)
			if err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "token_issue_failed"})
			}

			// Determine redirect URL: use config if set, otherwise construct from FrontendBaseURL
			redirectURL := h.cfg.GitHubLoginSuccessRedirectURL
			if redirectURL == "" && h.cfg.FrontendBaseURL != "" {
				redirectURL = strings.TrimSuffix(h.cfg.FrontendBaseURL, "/") + "/auth/callback"
			}

			if redirectURL != "" {
				ru, err := url.Parse(redirectURL)
				if err == nil {
					q := ru.Query()
					q.Set("token", jwtToken)
					q.Set("github", u.Login)
					ru.RawQuery = q.Encode()
					return c.Redirect(ru.String(), fiber.StatusFound)
				}
			}

			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"token": jwtToken,
				"user": fiber.Map{
					"id":   userID.String(),
					"role": role,
				},
				"github": fiber.Map{
					"id":         u.ID,
					"login":      u.Login,
					"avatar_url": u.AvatarURL,
				},
			})
		}

		// github_link behavior (no new token required).
		if h.cfg.GitHubOAuthSuccessRedirectURL != "" {
			ru, err := url.Parse(h.cfg.GitHubOAuthSuccessRedirectURL)
			if err == nil {
				q := ru.Query()
				q.Set("linked", "true")
				q.Set("github", u.Login)
				ru.RawQuery = q.Encode()
				return c.Redirect(ru.String(), fiber.StatusFound)
			}
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"ok": true,
			"github": fiber.Map{
				"id":         u.ID,
				"login":      u.Login,
				"avatar_url": u.AvatarURL,
			},
		})
	}
}

func effectiveGitHubRedirect(cfg config.Config) string {
	// Recommended: set GITHUB_OAUTH_REDIRECT_URL to the full callback URL
	// Example: http://localhost:8080/auth/github/login/callback
	// This must match exactly what's registered in your GitHub OAuth app settings
	if strings.TrimSpace(cfg.GitHubOAuthRedirectURL) != "" {
		return strings.TrimSpace(cfg.GitHubOAuthRedirectURL)
	}
	// Fallback to GitHubLoginRedirectURL for backwards compatibility
	if strings.TrimSpace(cfg.GitHubLoginRedirectURL) != "" {
		return strings.TrimSpace(cfg.GitHubLoginRedirectURL)
	}
	// If neither is set and we have PublicBaseURL, construct it
	if cfg.PublicBaseURL != "" {
		baseURL := strings.TrimSuffix(cfg.PublicBaseURL, "/")
		return baseURL + "/auth/github/login/callback"
	}
	return ""
}

func (h *GitHubOAuthHandler) Status() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		var githubUserID int64
		var login string
		var avatarURL *string
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT github_user_id, login, avatar_url
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubUserID, &login, &avatarURL)
		if errors.Is(err, pgx.ErrNoRows) {
			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"linked": false,
			})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "status_failed"})
		}

		githubMap := fiber.Map{
			"id":    githubUserID,
			"login": login,
		}
		if avatarURL != nil && *avatarURL != "" {
			githubMap["avatar_url"] = *avatarURL
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"linked": true,
			"github": githubMap,
		})
	}
}

func randomState(n int) string {
	b := make([]byte, n)
	_, _ = rand.Read(b)
	return base64.RawURLEncoding.EncodeToString(b)
}
