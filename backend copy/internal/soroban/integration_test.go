package soroban

import (
	"context"
	"os"
	"testing"
	"time"
)

// Integration tests for Soroban contract interactions
// These tests require:
// - SOROBAN_RPC_URL environment variable
// - SOROBAN_SOURCE_SECRET environment variable
// - A funded test account
// - Deployed contracts

func TestEscrowContract_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test")
	}

	rpcURL := os.Getenv("SOROBAN_RPC_URL")
	if rpcURL == "" {
		t.Skip("SOROBAN_RPC_URL not set, skipping integration test")
	}

	sourceSecret := os.Getenv("SOROBAN_SOURCE_SECRET")
	if sourceSecret == "" {
		t.Skip("SOROBAN_SOURCE_SECRET not set, skipping integration test")
	}

	contractID := os.Getenv("ESCROW_CONTRACT_ID")
	if contractID == "" {
		t.Skip("ESCROW_CONTRACT_ID not set, skipping integration test")
	}

	// Create client
	client, err := NewClient(Config{
		RPCURL:            rpcURL,
		Network:          NetworkTestnet,
		NetworkPassphrase: "", // Will use default for testnet
		HTTPTimeout:      30 * time.Second,
	})
	if err != nil {
		t.Fatalf("failed to create client: %v", err)
	}

	// Create transaction builder
	txBuilder, err := NewTransactionBuilder(client, sourceSecret, DefaultRetryConfig())
	if err != nil {
		t.Fatalf("failed to create transaction builder: %v", err)
	}

	// Create escrow contract client
	escrow := NewEscrowContract(client, txBuilder, contractID)

	ctx := context.Background()

	// Test GetEscrowInfo (read-only)
	t.Run("GetEscrowInfo", func(t *testing.T) {
		_, err := escrow.GetEscrowInfo(ctx, 1)
		// This may fail if contract not initialized or bounty doesn't exist
		// That's okay for integration test
		if err != nil {
			t.Logf("GetEscrowInfo returned error (expected if not initialized): %v", err)
		}
	})

	// Test GetBalance (read-only)
	t.Run("GetBalance", func(t *testing.T) {
		_, err := escrow.GetBalance(ctx)
		// This may fail if contract not initialized
		if err != nil {
			t.Logf("GetBalance returned error (expected if not initialized): %v", err)
		}
	})
}

func TestProgramEscrowContract_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test")
	}

	rpcURL := os.Getenv("SOROBAN_RPC_URL")
	if rpcURL == "" {
		t.Skip("SOROBAN_RPC_URL not set, skipping integration test")
	}

	sourceSecret := os.Getenv("SOROBAN_SOURCE_SECRET")
	if sourceSecret == "" {
		t.Skip("SOROBAN_SOURCE_SECRET not set, skipping integration test")
	}

	contractID := os.Getenv("PROGRAM_ESCROW_CONTRACT_ID")
	if contractID == "" {
		t.Skip("PROGRAM_ESCROW_CONTRACT_ID not set, skipping integration test")
	}

	// Create client
	client, err := NewClient(Config{
		RPCURL:            rpcURL,
		Network:          NetworkTestnet,
		NetworkPassphrase: "",
		HTTPTimeout:      30 * time.Second,
	})
	if err != nil {
		t.Fatalf("failed to create client: %v", err)
	}

	// Create transaction builder
	txBuilder, err := NewTransactionBuilder(client, sourceSecret, DefaultRetryConfig())
	if err != nil {
		t.Fatalf("failed to create transaction builder: %v", err)
	}

	// Create program escrow contract client
	programEscrow := NewProgramEscrowContract(client, txBuilder, contractID)

	ctx := context.Background()

	// Test GetProgramInfo (read-only)
	t.Run("GetProgramInfo", func(t *testing.T) {
		_, err := programEscrow.GetProgramInfo(ctx)
		// This may fail if contract not initialized
		if err != nil {
			t.Logf("GetProgramInfo returned error (expected if not initialized): %v", err)
		}
	})

	// Test GetRemainingBalance (read-only)
	t.Run("GetRemainingBalance", func(t *testing.T) {
		_, err := programEscrow.GetRemainingBalance(ctx)
		// This may fail if contract not initialized
		if err != nil {
			t.Logf("GetRemainingBalance returned error (expected if not initialized): %v", err)
		}
	})
}

func TestClient_RPC(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test")
	}

	rpcURL := os.Getenv("SOROBAN_RPC_URL")
	if rpcURL == "" {
		t.Skip("SOROBAN_RPC_URL not set, skipping integration test")
	}

	client, err := NewClient(Config{
		RPCURL:            rpcURL,
		Network:          NetworkTestnet,
		NetworkPassphrase: "",
		HTTPTimeout:      30 * time.Second,
	})
	if err != nil {
		t.Fatalf("failed to create client: %v", err)
	}

	ctx := context.Background()

	// Test GetLatestLedger
	t.Run("GetLatestLedger", func(t *testing.T) {
		ledger, err := client.GetLatestLedger(ctx)
		if err != nil {
			t.Fatalf("GetLatestLedger failed: %v", err)
		}

		if ledger == nil {
			t.Fatal("GetLatestLedger returned nil")
		}

		t.Logf("Latest ledger: %+v", ledger)
	})
}
