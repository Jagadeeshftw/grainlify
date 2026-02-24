package errors

import (
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// Authoritative discriminant lists (keep in sync with Rust source).
// ---------------------------------------------------------------------------

var expectedBountyEscrow = []uint32{
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, /* gap at 15 */ 16, 17, 18,
}

var expectedGovernance = []uint32{
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
}

var expectedCircuitBreaker = []uint32{0, 1001, 1002, 1003}

// ---------------------------------------------------------------------------
// Completeness: every on-chain error code is mapped.
// ---------------------------------------------------------------------------

func TestBountyEscrowCompleteness(t *testing.T) {
	for _, code := range expectedBountyEscrow {
		msg := ContractErrorMessage(BountyEscrow, code)
		if strings.HasPrefix(msg, "Unknown") {
			t.Errorf("bounty_escrow code %d is unmapped", code)
		}
		name := ContractErrorName(BountyEscrow, code)
		if strings.HasPrefix(name, "Unknown") {
			t.Errorf("bounty_escrow code %d has no name", code)
		}
	}
}

func TestGovernanceCompleteness(t *testing.T) {
	for _, code := range expectedGovernance {
		msg := ContractErrorMessage(Governance, code)
		if strings.HasPrefix(msg, "Unknown") {
			t.Errorf("governance code %d is unmapped", code)
		}
		name := ContractErrorName(Governance, code)
		if strings.HasPrefix(name, "Unknown") {
			t.Errorf("governance code %d has no name", code)
		}
	}
}

func TestCircuitBreakerCompleteness(t *testing.T) {
	for _, code := range expectedCircuitBreaker {
		msg := ContractErrorMessage(CircuitBreaker, code)
		if strings.HasPrefix(msg, "Unknown") {
			t.Errorf("circuit_breaker code %d is unmapped", code)
		}
	}
}

// ---------------------------------------------------------------------------
// Messages are non-empty and human-readable.
// ---------------------------------------------------------------------------

func TestAllMessagesNonEmpty(t *testing.T) {
	for _, kind := range []ContractKind{BountyEscrow, Governance, CircuitBreaker} {
		codes := AllCodes(kind)
		if len(codes) == 0 {
			t.Errorf("no codes registered for %s", kind)
		}
		for _, code := range codes {
			msg := ContractErrorMessage(kind, code)
			if msg == "" {
				t.Errorf("%s code %d has empty message", kind, code)
			}
			if len(msg) < 10 {
				t.Errorf("%s code %d message too short: %q", kind, code, msg)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Unknown codes return a descriptive fallback (never empty).
// ---------------------------------------------------------------------------

func TestUnknownCodeFallback(t *testing.T) {
	msg := ContractErrorMessage(BountyEscrow, 9999)
	if !strings.Contains(msg, "Unknown") {
		t.Errorf("expected fallback message for unknown code, got %q", msg)
	}
	if !strings.Contains(msg, "9999") {
		t.Errorf("expected code in fallback message, got %q", msg)
	}
}

func TestUnknownContractKind(t *testing.T) {
	msg := ContractErrorMessage("nonexistent", 1)
	if !strings.Contains(msg, "Unknown") {
		t.Errorf("expected fallback for unknown kind, got %q", msg)
	}
}

func TestUnknownNameFallback(t *testing.T) {
	name := ContractErrorName(BountyEscrow, 9999)
	if !strings.Contains(name, "Unknown") {
		t.Errorf("expected fallback name for unknown code, got %q", name)
	}
}

// ---------------------------------------------------------------------------
// Specific message spot-checks.
// ---------------------------------------------------------------------------

func TestBountyEscrowSpecificMessages(t *testing.T) {
	cases := []struct {
		code uint32
		want string
	}{
		{1, "already initialized"},
		{4, "Bounty not found"},
		{7, "not allowed"},
		{8, "Fee rate is invalid"},
		{13, "amount is invalid"},
		{16, "Insufficient funds"},
		{17, "not been approved"},
		{18, "paused"},
	}
	for _, tc := range cases {
		msg := ContractErrorMessage(BountyEscrow, tc.code)
		if !strings.Contains(strings.ToLower(msg), strings.ToLower(tc.want)) {
			t.Errorf("bounty_escrow %d: expected message containing %q, got %q", tc.code, tc.want, msg)
		}
	}
}

func TestGovernanceSpecificMessages(t *testing.T) {
	cases := []struct {
		code uint32
		want string
	}{
		{1, "not been initialized"},
		{6, "Proposal not found"},
		{9, "ended"},
		{11, "already voted"},
		{14, "expired"},
	}
	for _, tc := range cases {
		msg := ContractErrorMessage(Governance, tc.code)
		if !strings.Contains(strings.ToLower(msg), strings.ToLower(tc.want)) {
			t.Errorf("governance %d: expected message containing %q, got %q", tc.code, tc.want, msg)
		}
	}
}

func TestCircuitBreakerSpecificMessages(t *testing.T) {
	cases := []struct {
		code uint32
		want string
	}{
		{1001, "open"},
		{1002, "transfer failed"},
		{1003, "balance"},
	}
	for _, tc := range cases {
		msg := ContractErrorMessage(CircuitBreaker, tc.code)
		if !strings.Contains(strings.ToLower(msg), strings.ToLower(tc.want)) {
			t.Errorf("circuit_breaker %d: expected message containing %q, got %q", tc.code, tc.want, msg)
		}
	}
}

// ---------------------------------------------------------------------------
// Count guards: if a contract adds a new error, this test must be updated.
// ---------------------------------------------------------------------------

func TestRegistryCounts(t *testing.T) {
	if got := len(AllCodes(BountyEscrow)); got != 17 {
		t.Errorf("BountyEscrow: expected 17 error codes, got %d", got)
	}
	if got := len(AllCodes(Governance)); got != 14 {
		t.Errorf("Governance: expected 14 error codes, got %d", got)
	}
	if got := len(AllCodes(CircuitBreaker)); got != 4 {
		t.Errorf("CircuitBreaker: expected 4 error codes (including ERR_NONE), got %d", got)
	}
}
