package handlers

import (
	"errors"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type AdminHandler struct {
	cfg config.Config
	db  *db.DB
}

func NewAdminHandler(cfg config.Config, d *db.DB) *AdminHandler {
	return &AdminHandler{cfg: cfg, db: d}
}

func (h *AdminHandler) ListUsers() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		rows, err := h.db.Pool.Query(c.Context(), `
SELECT id, role, github_user_id, created_at, updated_at
FROM users
ORDER BY created_at DESC
LIMIT 50
`)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "users_list_failed"})
		}
		defer rows.Close()

		var out []fiber.Map
		for rows.Next() {
			var id uuid.UUID
			var role string
			var ghID *int64
			var createdAt, updatedAt time.Time
			if err := rows.Scan(&id, &role, &ghID, &createdAt, &updatedAt); err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "users_list_failed"})
			}
			out = append(out, fiber.Map{
				"id":             id.String(),
				"role":           role,
				"github_user_id": ghID,
				"created_at":     createdAt,
				"updated_at":     updatedAt,
			})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"users": out})
	}
}

type setRoleRequest struct {
	Role string `json:"role"`
}

func (h *AdminHandler) SetUserRole() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		userID, err := uuid.Parse(c.Params("id"))
		if err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_user_id"})
		}
		var req setRoleRequest
		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}
		role := strings.TrimSpace(req.Role)
		if role != "contributor" && role != "maintainer" && role != "admin" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_role"})
		}
		ct, err := h.db.Pool.Exec(c.Context(), `
UPDATE users SET role = $2, updated_at = now()
WHERE id = $1
`, userID, role)
		if errors.Is(err, pgx.ErrNoRows) || ct.RowsAffected() == 0 {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "user_not_found"})
		}
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "role_update_failed"})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{"ok": true})
	}
}

// BootstrapAdmin promotes the currently authenticated user to admin if they know the bootstrap token.
// This allows any authenticated user with the correct bootstrap token to become an admin.
//
// Rules:
// - Requires ADMIN_BOOTSTRAP_TOKEN header match
// - If user is already an admin, returns a fresh JWT token
// - Otherwise, promotes the user to admin and returns a fresh JWT with the updated role
func (h *AdminHandler) BootstrapAdmin() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if h.cfg.AdminBootstrapToken == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "bootstrap_not_configured"})
		}
		if h.cfg.JWTSecret == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "jwt_not_configured"})
		}
		headerToken := strings.TrimSpace(c.Get("X-Admin-Bootstrap-Token"))
		configToken := strings.TrimSpace(h.cfg.AdminBootstrapToken)
		if headerToken != configToken {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_bootstrap_token"})
		}
		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		var currentRole string
		if err := h.db.Pool.QueryRow(c.Context(), `SELECT role FROM users WHERE id = $1`, userID).Scan(&currentRole); err != nil {
			if errors.Is(err, pgx.ErrNoRows) {
				return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "user_not_found"})
			}
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "bootstrap_failed"})
		}

		// If user is already an admin, no need to update
		if currentRole == "admin" {
			jwtToken, err := auth.IssueJWT(h.cfg.JWTSecret, userID, "admin", "", "", 60*time.Minute)
			if err != nil {
				return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "token_issue_failed"})
			}
			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"ok":    true,
				"token": jwtToken,
				"role":  "admin",
			})
		}

		// Promote user to admin if they have the correct bootstrap token
		_, err = h.db.Pool.Exec(c.Context(), `UPDATE users SET role = 'admin', updated_at = now() WHERE id = $1`, userID)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "bootstrap_failed"})
		}

		jwtToken, err := auth.IssueJWT(h.cfg.JWTSecret, userID, "admin", "", "", 60*time.Minute)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "token_issue_failed"})
		}
		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"ok":    true,
			"token": jwtToken,
			"role":  "admin",
		})
	}
}




