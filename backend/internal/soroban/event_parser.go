package soroban

import (
	"encoding/json"
	"fmt"
)

// EventCompatPayload is a minimal normalized representation used by indexers/SDK code.
// It intentionally keeps only required cross-version fields.
type EventCompatPayload struct {
	Version uint32 `json:"version"`
	Amount  int64  `json:"amount,omitempty"`
}

// ParseEventCompatPayload parses both legacy (v1, unversioned) and version-tagged payloads.
// Unknown/newer versions are accepted as long as required fields are present.
func ParseEventCompatPayload(raw []byte) (*EventCompatPayload, error) {
	var payload map[string]json.RawMessage
	if err := json.Unmarshal(raw, &payload); err != nil {
		return nil, fmt.Errorf("decode event payload: %w", err)
	}

	out := &EventCompatPayload{Version: 1}
	if rawVersion, ok := payload["version"]; ok {
		if err := json.Unmarshal(rawVersion, &out.Version); err != nil {
			return nil, fmt.Errorf("decode version: %w", err)
		}
	}

	if rawAmount, ok := payload["amount"]; ok {
		if err := json.Unmarshal(rawAmount, &out.Amount); err != nil {
			return nil, fmt.Errorf("decode amount: %w", err)
		}
	}

	if _, hasAmount := payload["amount"]; !hasAmount {
		return nil, fmt.Errorf("missing required field: amount")
	}

	return out, nil
}
