package soroban

import "testing"

func TestParseEventCompatPayload_LegacyV1WithoutVersionTag(t *testing.T) {
	raw := []byte(`{"amount": 1500, "program_id": "hack-2026"}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed for v1 payload: %v", err)
	}
	if parsed.Version != 1 {
		t.Fatalf("expected default version 1 for legacy payload, got %d", parsed.Version)
	}
	if parsed.Amount != 1500 {
		t.Fatalf("expected amount 1500, got %d", parsed.Amount)
	}
}

func TestParseEventCompatPayload_VersionTaggedV2(t *testing.T) {
	raw := []byte(`{"version": 2, "amount": 4200, "program_id": "hack-2026", "extra":"ignored"}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed for v2 payload: %v", err)
	}
	if parsed.Version != 2 {
		t.Fatalf("expected version 2, got %d", parsed.Version)
	}
	if parsed.Amount != 4200 {
		t.Fatalf("expected amount 4200, got %d", parsed.Amount)
	}
}

func TestParseEventCompatPayload_NewerVersionStillParsesRequiredFields(t *testing.T) {
	raw := []byte(`{"version": 3, "amount": 999, "new_field":{"nested":true}}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed for newer payload: %v", err)
	}
	if parsed.Version != 3 {
		t.Fatalf("expected version 3, got %d", parsed.Version)
	}
	if parsed.Amount != 999 {
		t.Fatalf("expected amount 999, got %d", parsed.Amount)
	}
}
