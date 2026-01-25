package handlers

import (
	"crypto/hmac"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/json"
	"log/slog"
	"strings"

	"github.com/gofiber/fiber/v2"

	"github.com/jagadeesh/grainlify/backend/internal/bus"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/events"
	"github.com/jagadeesh/grainlify/backend/internal/ingest"
)

type GitHubWebhooksHandler struct {
	cfg config.Config
	db  *db.DB
	bus bus.Bus
	ing *ingest.GitHubWebhookIngestor
}

func NewGitHubWebhooksHandler(cfg config.Config, d *db.DB, b bus.Bus) *GitHubWebhooksHandler {
	var ingestor *ingest.GitHubWebhookIngestor
	if d != nil && d.Pool != nil {
		ingestor = &ingest.GitHubWebhookIngestor{Pool: d.Pool}
	}
	return &GitHubWebhooksHandler{cfg: cfg, db: d, bus: b, ing: ingestor}
}

func (h *GitHubWebhooksHandler) Receive() fiber.Handler {
	return func(c *fiber.Ctx) error {
		// Handle CORS preflight requests
		if c.Method() == "OPTIONS" {
			slog.Info("GitHub webhook OPTIONS preflight request",
				"path", c.Path(),
				"remote_ip", c.IP(),
			)
			return c.SendStatus(fiber.StatusOK)
		}

		// Capture all request details for detailed logging
		body := c.Body()
		bodySize := len(body)
		delivery := strings.TrimSpace(c.Get("X-GitHub-Delivery"))
		event := strings.TrimSpace(c.Get("X-GitHub-Event"))
		sig := strings.TrimSpace(c.Get("X-Hub-Signature-256"))
		sigSha1 := strings.TrimSpace(c.Get("X-Hub-Signature"))
		hookID := strings.TrimSpace(c.Get("X-GitHub-Hook-ID"))
		hookInstallationTargetID := strings.TrimSpace(c.Get("X-GitHub-Hook-Installation-Target-ID"))
		hookInstallationTargetType := strings.TrimSpace(c.Get("X-GitHub-Hook-Installation-Target-Type"))

		// Detailed logging of incoming webhook request
		slog.Info("=== GitHub Webhook POST Request Received ===",
			"method", c.Method(),
			"path", c.Path(),
			"original_url", c.OriginalURL(),
			"remote_ip", c.IP(),
			"user_agent", c.Get("User-Agent"),
			"content_type", c.Get("Content-Type"),
			"content_length", c.Get("Content-Length"),
			"body_size_bytes", bodySize,
			"x_github_delivery", delivery,
			"x_github_event", event,
			"x_github_hook_id", hookID,
			"x_github_hook_installation_target_id", hookInstallationTargetID,
			"x_github_hook_installation_target_type", hookInstallationTargetType,
			"x_hub_signature_256_present", sig != "",
			"x_hub_signature_present", sigSha1 != "",
			"accept_header", c.Get("Accept"),
		)

		// Log first 500 chars of body for debugging (truncate if too long)
		bodyPreview := string(body)
		if len(bodyPreview) > 500 {
			bodyPreview = bodyPreview[:500] + "... (truncated)"
		}
		slog.Info("GitHub webhook request body preview",
			"delivery_id", delivery,
			"body_preview", bodyPreview,
			"body_size", bodySize,
		)

		if h.cfg.GitHubWebhookSecret == "" {
			slog.Error("GitHub webhook secret not configured - rejecting request",
				"delivery_id", delivery,
				"event", event,
			)
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "webhook_secret_not_configured"})
		}

		slog.Info("GitHub webhook secret configured, proceeding with signature verification",
			"delivery_id", delivery,
			"event", event,
		)

		// Prepare signature preview for logging
		sigPreview := sig
		if len(sigPreview) > 20 {
			sigPreview = sigPreview[:20] + "..."
		}

		if !verifyGitHubSignature(h.cfg.GitHubWebhookSecret, body, sig) {
			slog.Warn("GitHub webhook signature verification FAILED",
				"delivery_id", delivery,
				"event", event,
				"has_signature_256", sig != "",
				"signature_256_preview", sigPreview,
				"body_size", bodySize,
			)
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_signature"})
		}

		slog.Info("GitHub webhook signature verification SUCCESS",
			"delivery_id", delivery,
			"event", event,
		)

		var repoFullName string
		var action string

		var env ghWebhookEnvelope
		if err := json.Unmarshal(body, &env); err == nil {
			if env.Repository != nil {
				repoFullName = strings.TrimSpace(env.Repository.FullName)
			}
			action = strings.TrimSpace(env.Action)
		}

		ev := events.GitHubWebhookReceived{
			DeliveryID:   delivery,
			Event:        event,
			Action:       action,
			RepoFullName: repoFullName,
			Payload:      body,
		}

		slog.Info("GitHub webhook event parsed",
			"delivery_id", delivery,
			"event", event,
			"action", action,
			"repo_full_name", repoFullName,
		)

		// Preferred path: publish to NATS and return immediately (no heavy work in request path).
		if h.bus != nil {
			slog.Info("Publishing GitHub webhook to NATS event bus",
				"delivery_id", delivery,
				"event", event,
				"subject", events.SubjectGitHubWebhookReceived,
			)
			b, err := json.Marshal(ev)
			if err != nil {
				slog.Error("Failed to marshal webhook event for NATS",
					"delivery_id", delivery,
					"error", err,
				)
			} else {
				if pubErr := h.bus.Publish(c.Context(), events.SubjectGitHubWebhookReceived, b); pubErr != nil {
					slog.Error("Failed to publish webhook event to NATS",
						"delivery_id", delivery,
						"error", pubErr,
					)
				} else {
					slog.Info("Successfully published GitHub webhook to NATS",
						"delivery_id", delivery,
						"event", event,
					)
				}
			}
			slog.Info("=== GitHub Webhook Request Completed (NATS) ===",
				"delivery_id", delivery,
				"event", event,
				"status", "200 OK",
			)
			return c.SendStatus(fiber.StatusOK)
		}

		// Fallback path (no NATS): ingest inline (still no external calls).
		if h.ing != nil {
			slog.Info("Processing GitHub webhook inline (no NATS configured)",
				"delivery_id", delivery,
				"event", event,
			)
			if err := h.ing.Ingest(c.Context(), ev); err != nil {
				slog.Error("Failed to ingest GitHub webhook",
					"delivery_id", delivery,
					"event", event,
					"error", err,
				)
			} else {
				slog.Info("Successfully ingested GitHub webhook",
					"delivery_id", delivery,
					"event", event,
				)
			}
		} else {
			slog.Warn("No webhook ingestor configured - webhook received but not processed",
				"delivery_id", delivery,
				"event", event,
			)
		}

		slog.Info("=== GitHub Webhook Request Completed (Inline) ===",
			"delivery_id", delivery,
			"event", event,
			"status", "200 OK",
		)
		return c.SendStatus(fiber.StatusOK)
	}
}

func verifyGitHubSignature(secret string, body []byte, header string) bool {
	// GitHub uses: X-Hub-Signature-256: sha256=<hex>
	if !strings.HasPrefix(header, "sha256=") {
		return false
	}
	gotHex := strings.ToLower(strings.TrimPrefix(header, "sha256="))
	mac := hmac.New(sha256.New, []byte(secret))
	_, _ = mac.Write(body)
	want := mac.Sum(nil)
	wantHex := hexEncodeLower(want)
	return subtle.ConstantTimeCompare([]byte(gotHex), []byte(wantHex)) == 1
}

func hexEncodeLower(b []byte) string {
	const hextable = "0123456789abcdef"
	out := make([]byte, len(b)*2)
	for i, v := range b {
		out[i*2] = hextable[v>>4]
		out[i*2+1] = hextable[v&0x0f]
	}
	return string(out)
}

type ghWebhookEnvelope struct {
	Action     string         `json:"action"`
	Repository *ghRepoPayload `json:"repository"`
}

type ghRepoPayload struct {
	FullName string `json:"full_name"`
}

 


