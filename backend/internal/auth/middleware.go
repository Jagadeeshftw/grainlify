package auth

import (
	"context"
	"log/slog"
	"strings"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

const (
	LocalUserID = "user_id"
	LocalRole   = "role"
)

func RequireAuth(jwtSecret string) fiber.Handler {
	return func(c *fiber.Ctx) error {
		h := strings.TrimSpace(c.Get("Authorization"))
		if h == "" || !strings.HasPrefix(strings.ToLower(h), "bearer ") {
			slog.Warn("auth middleware: missing or invalid Authorization header",
				"path", c.Path(),
				"method", c.Method(),
				"header_present", h != "",
				"header_prefix_ok", h != "" && strings.HasPrefix(strings.ToLower(h), "bearer "),
				"request_id", c.Locals("requestid"),
			)
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{
				"error": "missing_bearer_token",
			})
		}
		token := strings.TrimSpace(h[len("bearer "):])
		if token == "" {
			slog.Warn("auth middleware: empty token after 'bearer ' prefix",
				"path", c.Path(),
				"method", c.Method(),
				"request_id", c.Locals("requestid"),
			)
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{
				"error": "missing_bearer_token",
			})
		}
		claims, err := ParseJWT(jwtSecret, token)
		if err != nil {
			slog.Warn("auth middleware: JWT parse failed",
				"path", c.Path(),
				"method", c.Method(),
				"error", err,
				"token_length", len(token),
				"request_id", c.Locals("requestid"),
			)
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{
				"error": "invalid_token",
			})
		}

		c.Locals(LocalUserID, claims.Subject)
		c.Locals(LocalRole, claims.Role)
		return c.Next()
	}
}

func RequireRole(roles ...string) fiber.Handler {
	allowed := map[string]struct{}{}
	for _, r := range roles {
		allowed[r] = struct{}{}
	}
	return func(c *fiber.Ctx) error {
		role, _ := c.Locals(LocalRole).(string)
		if role == "" {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{
				"error": "missing_role",
			})
		}
		if len(allowed) == 0 {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{
				"error": "insufficient_role",
			})
		}
		if _, ok := allowed[role]; !ok {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{
				"error": "insufficient_role",
			})
		}
		return c.Next()
	}
}

// RequireScopedAdmin checks that the authenticated user is either a global admin
// OR holds a scoped admin role for the given scope_type + scope_id pair.
//
// scopeType: "program" | "escrow"
// scopeIDParam: the Fiber route param name that holds the scope ID (e.g. "id")
func RequireScopedAdmin(pool *pgxpool.Pool, scopeType, scopeIDParam string) fiber.Handler {
	return func(c *fiber.Ctx) error {
		role, _ := c.Locals(LocalRole).(string)

		// Global admins always pass before any other check.
		if role == "admin" {
			return c.Next()
		}

		if pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		sub, _ := c.Locals(LocalUserID).(string)
		if sub == "" {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		scopeID := strings.TrimSpace(c.Params(scopeIDParam))
		if scopeID == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "missing_scope_id"})
		}

		var exists bool
		err = pool.QueryRow(
			context.Background(),
			`SELECT EXISTS(
				SELECT 1 FROM program_admins
				WHERE user_id = $1 AND scope_type = $2 AND scope_id = $3
			)`,
			userID, scopeType, scopeID,
		).Scan(&exists)
		if err != nil {
			slog.Error("RequireScopedAdmin: db error", "error", err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "internal_error"})
		}
		if !exists {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "insufficient_role"})
		}
		return c.Next()
	}
}









