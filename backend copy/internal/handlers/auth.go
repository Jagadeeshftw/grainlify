package handlers

import (
	"log/slog"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)

type AuthHandler struct {
	cfg config.Config
	db  *db.DB
}

func NewAuthHandler(cfg config.Config, d *db.DB) *AuthHandler {
	return &AuthHandler{cfg: cfg, db: d}
}

type nonceRequest struct {
	WalletType string `json:"wallet_type"`
	Address    string `json:"address"`
}

func (h *AuthHandler) Nonce() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		var req nonceRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}

		wType, err := auth.NormalizeWalletType(req.WalletType)
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_wallet_type"})
		}
		addr, err := auth.NormalizeAddress(wType, req.Address)
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_address"})
		}

		n, err := auth.CreateNonce(c.Context(), h.db.Pool, wType, addr, 10*time.Minute)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "nonce_create_failed"})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"nonce":      n.Nonce,
			"message":    auth.LoginMessage(n.Nonce),
			"expires_at": n.ExpiresAt,
		})
	}
}

type verifyRequest struct {
	WalletType string `json:"wallet_type"`
	Address    string `json:"address"`
	Nonce      string `json:"nonce"`
	Signature  string `json:"signature"`
	PublicKey  string `json:"public_key,omitempty"`
}

func (h *AuthHandler) Verify() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if h.cfg.JWTSecret == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "jwt_not_configured"})
		}

		var req verifyRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}

		wType, err := auth.NormalizeWalletType(req.WalletType)
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_wallet_type"})
		}
		addr, err := auth.NormalizeAddress(wType, req.Address)
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_address"})
		}
		if req.Nonce == "" || req.Signature == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "missing_nonce_or_signature"})
		}

		// Be tolerant during early dev: accept both the current canonical message and the
		// legacy newline message (so signing tools that copied `\n` vs newline don't block you).
		msgs := []string{
			auth.LoginMessage(req.Nonce),
			auth.LegacyLoginMessage(req.Nonce),
		}
		var sigOK bool
		for _, msg := range msgs {
			if err := auth.VerifySignature(wType, addr, msg, req.Signature, req.PublicKey); err == nil {
				sigOK = true
				break
			}
		}
		if !sigOK {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_signature"})
		}

		res, err := auth.ConsumeNonceAndUpsertUser(c.Context(), h.db.Pool, wType, addr, req.Nonce, req.PublicKey)
		if err != nil {
			if err.Error() == "invalid_or_expired_nonce" {
				return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_or_expired_nonce"})
			}
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "auth_failed"})
		}

		token, err := auth.IssueJWT(h.cfg.JWTSecret, res.User.ID, res.User.Role, res.Wallet.WalletType, res.Wallet.Address, 15*time.Minute)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "token_issue_failed"})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"token": token,
			"user":  res.User,
			"wallet": fiber.Map{
				"wallet_type": res.Wallet.WalletType,
				"address":     res.Wallet.Address,
			},
		})
	}
}

func (h *AuthHandler) Me() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		userIDStr, _ := c.Locals(auth.LocalUserID).(string)
		role, _ := c.Locals(auth.LocalRole).(string)
		userID, err := uuid.Parse(userIDStr)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		// Get user profile fields from database
		var firstName, lastName, location, website, bio, avatarURL, telegram, linkedin, whatsapp, twitter, discord *string
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT first_name, last_name, location, website, bio, avatar_url, telegram, linkedin, whatsapp, twitter, discord
FROM users
WHERE id = $1
`, userID).Scan(&firstName, &lastName, &location, &website, &bio, &avatarURL, &telegram, &linkedin, &whatsapp, &twitter, &discord)
		if err != nil {
			slog.Warn("failed to fetch user profile fields", "error", err, "user_id", userID)
		}

		response := fiber.Map{
			"id":   userIDStr,
			"role": role,
		}

		// Try to get GitHub access token and fetch full profile
		linkedAccount, err := github.GetLinkedAccount(c.Context(), h.db.Pool, userID, h.cfg.TokenEncKeyB64)
		if err == nil {
			// Fetch full GitHub user profile
			gh := github.NewClient()
			ghUser, err := gh.GetUser(c.Context(), linkedAccount.AccessToken)
			if err == nil {
				githubMap := fiber.Map{
					"login": ghUser.Login,
				}
				// Use database avatar_url if set, otherwise use GitHub avatar
				if avatarURL != nil && *avatarURL != "" {
					githubMap["avatar_url"] = *avatarURL
				} else {
					githubMap["avatar_url"] = ghUser.AvatarURL
				}
				// Add optional fields if available
				if ghUser.Name != "" {
					githubMap["name"] = ghUser.Name
				}
				// Try to get email from GitHub emails endpoint (more reliable)
				email, err := gh.GetPrimaryEmail(c.Context(), linkedAccount.AccessToken)
				if err == nil && email != "" {
					githubMap["email"] = email
				} else if ghUser.Email != "" {
					// Fallback to email from /user endpoint
					githubMap["email"] = ghUser.Email
				}
				// Use database location if set, otherwise use GitHub location
				if location != nil && *location != "" {
					githubMap["location"] = *location
				} else if ghUser.Location != "" {
					githubMap["location"] = ghUser.Location
				}
				// Use database bio if set, otherwise use GitHub bio
				if bio != nil && *bio != "" {
					githubMap["bio"] = *bio
				} else if ghUser.Bio != "" {
					githubMap["bio"] = ghUser.Bio
				}
				// Use database website if set, otherwise use GitHub blog
				if website != nil && *website != "" {
					githubMap["website"] = *website
				} else if ghUser.Blog != "" {
					githubMap["website"] = ghUser.Blog
				}
				response["github"] = githubMap
			} else {
				// Fallback to database values if GitHub API fails
				var githubLogin *string
				var githubAvatarURL *string
				_ = h.db.Pool.QueryRow(c.Context(), `
SELECT login, avatar_url
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubLogin, &githubAvatarURL)
				if githubLogin != nil {
					githubMap := fiber.Map{
						"login": *githubLogin,
					}
					// Use database avatar_url if set, otherwise use GitHub account avatar
					if avatarURL != nil && *avatarURL != "" {
						githubMap["avatar_url"] = *avatarURL
					} else if githubAvatarURL != nil && *githubAvatarURL != "" {
						githubMap["avatar_url"] = *githubAvatarURL
					}
					// Add profile fields from database
					if location != nil && *location != "" {
						githubMap["location"] = *location
					}
					if bio != nil && *bio != "" {
						githubMap["bio"] = *bio
					}
					if website != nil && *website != "" {
						githubMap["website"] = *website
					}
					response["github"] = githubMap
				}
			}
		} else {
			// No GitHub account linked, try to get from database anyway
			var githubLogin *string
			var githubAvatarURL *string
			_ = h.db.Pool.QueryRow(c.Context(), `
SELECT login, avatar_url
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubLogin, &githubAvatarURL)
			if githubLogin != nil {
				githubMap := fiber.Map{
					"login": *githubLogin,
				}
				// Use database avatar_url if set, otherwise use GitHub account avatar
				if avatarURL != nil && *avatarURL != "" {
					githubMap["avatar_url"] = *avatarURL
				} else if githubAvatarURL != nil && *githubAvatarURL != "" {
					githubMap["avatar_url"] = *githubAvatarURL
				}
				// Add profile fields from database
				if location != nil && *location != "" {
					githubMap["location"] = *location
				}
				if bio != nil && *bio != "" {
					githubMap["bio"] = *bio
				}
				if website != nil && *website != "" {
					githubMap["website"] = *website
				}
				response["github"] = githubMap
			}
		}

		// Add user profile fields to response (for first_name, last_name, social links)
		if firstName != nil && *firstName != "" {
			response["first_name"] = *firstName
		}
		if lastName != nil && *lastName != "" {
			response["last_name"] = *lastName
		}
		if telegram != nil && *telegram != "" {
			response["telegram"] = *telegram
		}
		if linkedin != nil && *linkedin != "" {
			response["linkedin"] = *linkedin
		}
		if whatsapp != nil && *whatsapp != "" {
			response["whatsapp"] = *whatsapp
		}
		if twitter != nil && *twitter != "" {
			response["twitter"] = *twitter
		}
		if discord != nil && *discord != "" {
			response["discord"] = *discord
		}

		return c.Status(fiber.StatusOK).JSON(response)
	}
}

// ResyncGitHubProfile fetches fresh GitHub profile data including email
func (h *AuthHandler) ResyncGitHubProfile() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		userIDStr, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(userIDStr)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		// Get GitHub access token
		linkedAccount, err := github.GetLinkedAccount(c.Context(), h.db.Pool, userID, h.cfg.TokenEncKeyB64)
		if err != nil {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "github_not_linked"})
		}

		// Fetch fresh GitHub user profile
		gh := github.NewClient()
		ghUser, err := gh.GetUser(c.Context(), linkedAccount.AccessToken)
		if err != nil {
			slog.Error("failed to fetch GitHub user", "error", err, "user_id", userID)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "github_fetch_failed"})
		}

		// Get primary email from GitHub
		email, err := gh.GetPrimaryEmail(c.Context(), linkedAccount.AccessToken)
		if err != nil {
			slog.Warn("failed to fetch GitHub email", "error", err, "user_id", userID)
			// Continue without email if email fetch fails
		}

		// Update github_accounts table with fresh data
		_, err = h.db.Pool.Exec(c.Context(), `
UPDATE github_accounts
SET login = $1, avatar_url = $2, updated_at = now()
WHERE user_id = $3
`, ghUser.Login, ghUser.AvatarURL, userID)
		if err != nil {
			slog.Error("failed to update github_accounts", "error", err, "user_id", userID)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "update_failed"})
		}

		// Return fresh GitHub data
		githubMap := fiber.Map{
			"login":      ghUser.Login,
			"avatar_url": ghUser.AvatarURL,
		}
		if ghUser.Name != "" {
			githubMap["name"] = ghUser.Name
		}
		if email != "" {
			githubMap["email"] = email
		} else if ghUser.Email != "" {
			githubMap["email"] = ghUser.Email
		}
		if ghUser.Location != "" {
			githubMap["location"] = ghUser.Location
		}
		if ghUser.Bio != "" {
			githubMap["bio"] = ghUser.Bio
		}
		if ghUser.Blog != "" {
			githubMap["website"] = ghUser.Blog
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"github": githubMap,
		})
	}
}


