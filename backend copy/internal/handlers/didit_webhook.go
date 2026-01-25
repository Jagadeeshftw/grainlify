package handlers

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/didit"
)

type DiditWebhookHandler struct {
	cfg   config.Config
	db    *db.DB
	didit *didit.Client
}

func NewDiditWebhookHandler(cfg config.Config, d *db.DB) *DiditWebhookHandler {
	var diditClient *didit.Client
	if cfg.DiditAPIKey != "" {
		diditClient = didit.NewClient(cfg.DiditAPIKey)
	}
	return &DiditWebhookHandler{
		cfg:   cfg,
		db:    d,
		didit: diditClient,
	}
}

// WebhookEvent represents a Didit webhook event
type WebhookEvent struct {
	Event     string                 `json:"event"` // e.g., "status.updated", "data.updated"
	SessionID string                 `json:"session_id"`
	Data      map[string]interface{} `json:"data,omitempty"`
	Status    string                 `json:"status,omitempty"`
}

// Receive handles incoming Didit webhook events and callback redirects
// Supports both:
// - GET requests with query params (callback redirect from Didit)
// - POST requests with JSON body (webhook events from Didit)
func (h *DiditWebhookHandler) Receive() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		var sessionID string
		var status string

		// Handle GET request (callback redirect from Didit)
		if c.Method() == "GET" {
			sessionID = c.Query("verificationSessionId")
			status = c.Query("status")
			
			if sessionID == "" {
				// Try alternative query param name
				sessionID = c.Query("session_id")
			}
		} else {
			// Handle POST request (webhook event from Didit)
			var event WebhookEvent
			if err := c.BodyParser(&event); err != nil {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
			}
			sessionID = event.SessionID
			status = event.Status
		}

		if sessionID == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "missing_session_id"})
		}

		// Find user by session ID
		var userID uuid.UUID
		err := h.db.Pool.QueryRow(c.Context(), `
SELECT id
FROM users
WHERE kyc_session_id = $1
`, sessionID).Scan(&userID)
		if err != nil {
			// Session not found - might be from another system or invalid
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "session_not_found"})
		}

		// Process status update
		// Fetch latest decision from Didit API if available
		var kycStatus string
		var decisionData map[string]interface{}
		
		if h.didit != nil {
			decision, err := h.didit.GetSessionDecision(c.Context(), sessionID)
			if err != nil {
				// If API call fails, use status from query/body
				kycStatus = mapDiditStatus(status)
			} else {
				// Map Didit status to our KYC status
				kycStatus = mapDiditStatus(decision.Status)
				// Store both Decision and Data from Didit response
				decisionData = map[string]interface{}{
					"decision": decision.Decision,
					"data":     decision.Data,
				}
			}
		} else {
			// If no Didit client, use status from query/body
			kycStatus = mapDiditStatus(status)
		}

		// Store decision data as JSONB (includes both Decision and Data)
		decisionJSON, _ := json.Marshal(decisionData)

		// Update user KYC status
		_, err = h.db.Pool.Exec(c.Context(), `
UPDATE users
SET kyc_status = $1,
    kyc_data = $2,
    kyc_verified_at = CASE WHEN $1 = 'verified' THEN now() ELSE kyc_verified_at END,
    updated_at = now()
WHERE id = $3
`, kycStatus, decisionJSON, userID)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "kyc_update_failed"})
		}

		// For GET requests (callback redirect), redirect to success page
		if c.Method() == "GET" {
			// Redirect to frontend with success message
			successURL := h.cfg.GitHubOAuthSuccessRedirectURL
			if successURL == "" && h.cfg.FrontendBaseURL != "" {
				successURL = strings.TrimSuffix(h.cfg.FrontendBaseURL, "/")
			}
			if successURL != "" {
				// Add query params to indicate success
				redirectURL := fmt.Sprintf("%s?kyc=verified&session_id=%s", successURL, sessionID)
				return c.Redirect(redirectURL, fiber.StatusFound)
			}
		}

		// For POST requests (webhook), return JSON
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true, "status": kycStatus})
	}
}

