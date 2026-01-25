package api

import (
	"context"

	"github.com/gofiber/fiber/v2"
)

func Shutdown(ctx context.Context, app *fiber.App) error {
	// Fiber's Shutdown doesn't accept a context; we use a goroutine + select to enforce it.
	errCh := make(chan error, 1)
	go func() {
		errCh <- app.Shutdown()
	}()

	select {
	case <-ctx.Done():
		return ctx.Err()
	case err := <-errCh:
		return err
	}
}
