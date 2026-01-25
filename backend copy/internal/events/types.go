package events

import "encoding/json"

const (
	SubjectGitHubWebhookReceived = "github.webhook.received"
)

type GitHubWebhookReceived struct {
	DeliveryID   string          `json:"delivery_id"`
	Event        string          `json:"event"`
	Action       string          `json:"action,omitempty"`
	RepoFullName string          `json:"repo_full_name,omitempty"`
	Payload      json.RawMessage `json:"payload"`
}









