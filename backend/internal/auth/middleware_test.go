package auth

import (
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
)

func TestRequireAuthMiddleware(t *testing.T) {
	jwtSecret := "test-jwt-secret-key-32-bytes-long!!"
	userID := uuid.New()
	validToken, err := IssueJWT(jwtSecret, userID, "user", "", "", 15*time.Minute)
	if err != nil {
		t.Fatalf("failed to issue test JWT: %v", err)
	}

	app := fiber.New()
	app.Use(RequireAuth(jwtSecret))
	app.Get("/protected", func(c *fiber.Ctx) error {
		uid, _ := c.Locals(LocalUserID).(string)
		role, _ := c.Locals(LocalRole).(string)
		return c.JSON(fiber.Map{
			"user_id": uid,
			"role":    role,
		})
	})

	tests := []struct {
		name           string
		authHeader     string
		wantStatusCode int
	}{
		{
			name:           "valid bearer token",
			authHeader:     "Bearer " + validToken,
			wantStatusCode: http.StatusOK,
		},
		{
			name:           "valid bearer token lowercase scheme",
			authHeader:     "bearer " + validToken,
			wantStatusCode: http.StatusOK,
		},
		{
			name:           "missing authorization header",
			authHeader:     "",
			wantStatusCode: http.StatusUnauthorized,
		},
		{
			name:           "non-bearer scheme",
			authHeader:     "Basic dXNlcjpwYXNz",
			wantStatusCode: http.StatusUnauthorized,
		},
		{
			name:           "empty token after bearer",
			authHeader:     "Bearer ",
			wantStatusCode: http.StatusUnauthorized,
		},
		{
			name:           "invalid jwt token string",
			authHeader:     "Bearer invalid.jwt.token",
			wantStatusCode: http.StatusUnauthorized,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/protected", nil)
			if tt.authHeader != "" {
				req.Header.Set("Authorization", tt.authHeader)
			}
			resp, err := app.Test(req)
			if err != nil {
				t.Fatalf("unexpected error making test request: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != tt.wantStatusCode {
				t.Errorf("status code = %d; want %d", resp.StatusCode, tt.wantStatusCode)
			}
		})
	}
}

func TestRequireRoleMiddleware(t *testing.T) {
	app := fiber.New()
	app.Use(func(c *fiber.Ctx) error {
		roleHeader := c.Get("X-Test-Role")
		if roleHeader != "" {
			c.Locals(LocalRole, roleHeader)
		}
		return c.Next()
	})
	app.Use(RequireRole("admin", "reviewer"))
	app.Get("/admin-only", func(c *fiber.Ctx) error {
		return c.SendString("ok")
	})

	t.Run("allowed role passes", func(t *testing.T) {
		req := httptest.NewRequest("GET", "/admin-only", nil)
		req.Header.Set("X-Test-Role", "admin")
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusOK)
		}
	})

	t.Run("disallowed role forbidden", func(t *testing.T) {
		req := httptest.NewRequest("GET", "/admin-only", nil)
		req.Header.Set("X-Test-Role", "user")
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusForbidden {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusForbidden)
		}

		body, _ := io.ReadAll(resp.Body)
		if string(body) == "ok" {
			t.Errorf("expected access to be forbidden")
		}
	})

	t.Run("missing role forbidden", func(t *testing.T) {
		req := httptest.NewRequest("GET", "/admin-only", nil)
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusForbidden {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusForbidden)
		}
	})
}
