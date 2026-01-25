package handlers

import (
	"context"
	"time"

	"github.com/gofiber/fiber/v2"

	"github.com/jagadeesh/grainlify/backend/internal/db"
)

func Ready(d *db.DB) fiber.Handler {
	return func(c *fiber.Ctx) error {
		if d == nil || d.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{
				"ok":     false,
				"reason": "db_not_configured",
			})
		}

		ctx, cancel := context.WithTimeout(c.Context(), 1*time.Second)
		defer cancel()

		if err := d.Pool.Ping(ctx); err != nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{
				"ok":     false,
				"reason": "db_unreachable",
			})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"ok": true,
		})
	}
}





















