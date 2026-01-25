package natsbus

import (
	"context"
	"fmt"
	"log/slog"
	"time"

	"github.com/nats-io/nats.go"
)

type Bus struct {
	nc *nats.Conn
}

func Connect(url string) (*Bus, error) {
	if url == "" {
		return nil, fmt.Errorf("NATS_URL is required")
	}
	
	// Mask URL for logging (don't expose credentials)
	maskedURL := maskNATSURL(url)
	slog.Info("connecting to NATS",
		"url_masked", maskedURL,
		"timeout", "5s",
		"max_reconnects", 5,
	)
	
	nc, err := nats.Connect(url,
		nats.Name("grainlify-api"),
		nats.Timeout(5*time.Second),
		nats.RetryOnFailedConnect(true),
		nats.MaxReconnects(5),
		nats.ReconnectWait(500*time.Millisecond),
	)
	if err != nil {
		slog.Error("failed to connect to NATS",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return nil, err
	}
	
	slog.Info("NATS connection established",
		"status", nc.Status().String(),
		"connected_url", nc.ConnectedUrl(),
	)
	
	return &Bus{nc: nc}, nil
}

// maskNATSURL masks credentials in NATS URL for logging
func maskNATSURL(url string) string {
	// Format: nats://user:pass@host:port
	// Simple masking: replace password with ***
	if len(url) < 10 {
		return "***"
	}
	atIdx := -1
	colonIdx := -1
	for i, r := range url {
		if r == '@' {
			atIdx = i
			break
		}
		if r == ':' && colonIdx == -1 {
			colonIdx = i
		}
	}
	if atIdx > 0 && colonIdx > 0 && colonIdx < atIdx {
		return url[:colonIdx+1] + "***" + url[atIdx:]
	}
	return "***"
}

func (b *Bus) Publish(ctx context.Context, subject string, data []byte) error {
	if b == nil || b.nc == nil {
		return fmt.Errorf("nats not connected")
	}
	// nats.go Publish is fast; respect ctx only for cancellation before send.
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}
	return b.nc.Publish(subject, data)
}

func (b *Bus) Close() {
	if b == nil || b.nc == nil {
		return
	}
	slog.Info("closing NATS connection")
	b.nc.Drain()
	b.nc.Close()
	slog.Info("NATS connection closed")
}

func (b *Bus) Conn() *nats.Conn { return b.nc }




