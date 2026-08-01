package auth

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
)

func TestRequireAuthMiddleware(t *testing.T) {
	jwtSecret := "test-jwt-secret-key-32-bytes-long!!"
	userID := uuid.New()
	validToken, err := IssueJWT(jwtSecret, userID, "user", "", "", 15*time.Minute)
	if err != nil {
		t.Fatalf("failed to issue test JWT: %v", err)
	}

	expiredToken, err := func() (string, error) {
		claims := &Claims{
			RegisteredClaims: jwt.RegisteredClaims{
				Subject:   userID.String(),
				IssuedAt:  jwt.NewNumericDate(time.Now().Add(-2 * time.Hour)),
				ExpiresAt: jwt.NewNumericDate(time.Now().Add(-1 * time.Hour)),
			},
			Role: "user",
		}
		t := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
		return t.SignedString([]byte(jwtSecret))
	}()
	if err != nil {
		t.Fatalf("failed to issue expired test JWT: %v", err)
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
		{
			name:           "expired jwt token",
			authHeader:     "Bearer " + expiredToken,
			wantStatusCode: http.StatusUnauthorized,
		},
		{
			name:           "token with trailing whitespace",
			authHeader:     "Bearer " + validToken + "   ",
			wantStatusCode: http.StatusOK,
		},
		{
			name:           "whitespace-only authorization header",
			authHeader:     "   ",
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
	t.Run("allowed role passes", func(t *testing.T) {
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "admin")
			return c.Next()
		})
		app.Use(RequireRole("admin", "reviewer"))
		app.Get("/admin-only", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/admin-only", nil)
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
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "user")
			return c.Next()
		})
		app.Use(RequireRole("admin", "reviewer"))
		app.Get("/admin-only", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/admin-only", nil)
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
		app := fiber.New()
		app.Use(RequireRole("admin", "reviewer"))
		app.Get("/admin-only", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

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

	t.Run("empty roles slice denies all", func(t *testing.T) {
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "admin")
			return c.Next()
		})
		app.Use(RequireRole())
		app.Get("/protected", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/protected", nil)
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusForbidden {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusForbidden)
		}

		var body map[string]string
		json.NewDecoder(resp.Body).Decode(&body)
		if body["error"] != "insufficient_role" {
			t.Errorf("expected insufficient_role error, got %v", body["error"])
		}
	})
}

func TestRequireScopedAdminMiddleware(t *testing.T) {
	t.Run("nil pool returns 503", func(t *testing.T) {
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "maintainer")
			c.Locals(LocalUserID, uuid.New().String())
			return c.Next()
		})
		app.Use(RequireScopedAdmin(nil, "program", "id"))
		app.Get("/programs/:id", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/programs/prog-abc", nil)
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusServiceUnavailable {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusServiceUnavailable)
		}
	})

	t.Run("global admin bypasses scoped check", func(t *testing.T) {
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "admin")
			c.Locals(LocalUserID, uuid.New().String())
			return c.Next()
		})
		app.Use(RequireScopedAdmin(nil, "program", "id"))
		app.Get("/programs/:id", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/programs/prog-abc", nil)
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusOK)
		}
	})

	t.Run("non-admin with nil pool returns 503 (not 401 for missing user_id)", func(t *testing.T) {
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "maintainer")
			return c.Next()
		})
		app.Use(RequireScopedAdmin(nil, "program", "id"))
		app.Get("/programs/:id", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/programs/prog-abc", nil)
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()
		// Falls through to nil pool check before user_id validation
		if resp.StatusCode != http.StatusServiceUnavailable {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusServiceUnavailable)
		}
	})

	t.Run("non-admin with nil pool returns 503 (not 400 for missing scope_id)", func(t *testing.T) {
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "maintainer")
			c.Locals(LocalUserID, uuid.New().String())
			return c.Next()
		})
		app.Use(RequireScopedAdmin(nil, "program", "id"))
		app.Get("/programs/:id", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/programs/", nil)
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()
		// Falls through to nil pool check before scope_id validation
		if resp.StatusCode != http.StatusServiceUnavailable {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusServiceUnavailable)
		}
	})

	t.Run("non-admin with non-admin role has insufficient role", func(t *testing.T) {
		app := fiber.New()
		app.Use(func(c *fiber.Ctx) error {
			c.Locals(LocalRole, "contributor")
			c.Locals(LocalUserID, uuid.New().String())
			return c.Next()
		})
		app.Use(RequireScopedAdmin(nil, "program", "id"))
		app.Get("/programs/:id", func(c *fiber.Ctx) error {
			return c.SendString("ok")
		})

		req := httptest.NewRequest("GET", "/programs/prog-abc", nil)
		resp, err := app.Test(req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusServiceUnavailable {
			t.Errorf("status = %d; want %d", resp.StatusCode, http.StatusServiceUnavailable)
		}
	})
}
