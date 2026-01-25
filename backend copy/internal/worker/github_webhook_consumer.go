package worker

import (
	"context"
	"encoding/json"
	"log/slog"

	"github.com/nats-io/nats.go"

	"github.com/jagadeesh/grainlify/backend/internal/events"
	"github.com/jagadeesh/grainlify/backend/internal/ingest"
)

type GitHubWebhookConsumer struct {
	Sub    *nats.Subscription
	Ingest *ingest.GitHubWebhookIngestor
}

func (c *GitHubWebhookConsumer) Subscribe(ctx context.Context, nc *nats.Conn, queue string) error {
	if nc == nil {
		return nil
	}
	if queue == "" {
		queue = "patchwork-workers"
	}

	sub, err := nc.QueueSubscribe(events.SubjectGitHubWebhookReceived, queue, func(msg *nats.Msg) {
		var e events.GitHubWebhookReceived
		if err := json.Unmarshal(msg.Data, &e); err != nil {
			slog.Error("bad github webhook event", "error", err)
			return
		}
		if c.Ingest != nil {
			if err := c.Ingest.Ingest(context.Background(), e); err != nil {
				slog.Error("webhook ingest failed", "error", err)
			}
		}
	})
	if err != nil {
		return err
	}
	c.Sub = sub

	go func() {
		<-ctx.Done()
		_ = sub.Unsubscribe()
	}()

	return nil
}





















