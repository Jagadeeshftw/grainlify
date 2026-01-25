package handlers

import "github.com/gofiber/fiber/v2"

func Health() fiber.Handler {
	return func(c *fiber.Ctx) error {
		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"ok":      true,
			"service": "patchwork-api",
		})
	}
}









