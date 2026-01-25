package soroban

import (
	"context"
	"encoding/base64"
	"fmt"
	"log/slog"
	"time"

	"github.com/stellar/go/clients/horizonclient"
	"github.com/stellar/go/keypair"
	"github.com/stellar/go/txnbuild"
	"github.com/stellar/go/xdr"
)

// TransactionBuilder handles building, signing, and submitting Soroban transactions
type TransactionBuilder struct {
	client      *Client
	sourceKP    *keypair.Full
	retryConfig RetryConfig
}

// NewTransactionBuilder creates a new transaction builder
func NewTransactionBuilder(client *Client, sourceSecret string, retryConfig RetryConfig) (*TransactionBuilder, error) {
	sourceKP, err := keypair.ParseFull(sourceSecret)
	if err != nil {
		return nil, fmt.Errorf("invalid source secret: %w", err)
	}

	return &TransactionBuilder{
		client:      client,
		sourceKP:    sourceKP,
		retryConfig: retryConfig,
	}, nil
}

// BuildAndSubmit builds a transaction, signs it, and submits it to the network
func (tb *TransactionBuilder) BuildAndSubmit(ctx context.Context, operations []txnbuild.Operation) (*TransactionResult, error) {
	// Get account details
	accountRequest := horizonclient.AccountRequest{AccountID: tb.sourceKP.Address()}
	accountDetail, err := tb.client.GetHorizonClient().AccountDetail(accountRequest)
	if err != nil {
		return nil, fmt.Errorf("failed to get account details: %w", err)
	}

	// Build transaction
	tx, err := txnbuild.NewTransaction(
		txnbuild.TransactionParams{
			SourceAccount:        &accountDetail,
			IncrementSequenceNum: true,
			BaseFee:              txnbuild.MinBaseFee,
			Operations:           operations,
		},
	)
	if err != nil {
		return nil, fmt.Errorf("failed to build transaction: %w", err)
	}

	// Sign transaction
	tx, err = tx.Sign(tb.client.GetNetworkPassphrase(), tb.sourceKP)
	if err != nil {
		return nil, fmt.Errorf("failed to sign transaction: %w", err)
	}

	// Submit with retry
	return tb.submitWithRetry(ctx, tx)
}

// submitWithRetry submits a transaction with retry logic
func (tb *TransactionBuilder) submitWithRetry(ctx context.Context, tx *txnbuild.Transaction) (*TransactionResult, error) {
	var lastErr error
	delay := tb.retryConfig.InitialDelay

	for attempt := 0; attempt <= tb.retryConfig.MaxRetries; attempt++ {
		if attempt > 0 {
			slog.Info("retrying transaction submission",
				"attempt", attempt,
				"max_retries", tb.retryConfig.MaxRetries,
				"delay", delay,
			)
			select {
			case <-ctx.Done():
				return nil, ctx.Err()
			case <-time.After(delay):
			}
			delay = time.Duration(float64(delay) * tb.retryConfig.BackoffMultiplier)
			if delay > tb.retryConfig.MaxDelay {
				delay = tb.retryConfig.MaxDelay
			}
		}

		// Submit transaction
		resp, err := tb.client.GetHorizonClient().SubmitTransaction(tx)
		if err != nil {
			lastErr = err
			if herr, ok := err.(*horizonclient.Error); ok {
				slog.Warn("transaction submission failed",
					"attempt", attempt+1,
					"error", herr.Problem.Detail,
					"result_codes", herr.Problem.Extras,
				)
				// Don't retry on certain errors
				if isNonRetryableError(herr) {
					return nil, fmt.Errorf("non-retryable error: %w", err)
				}
			} else {
				slog.Warn("transaction submission failed",
					"attempt", attempt+1,
					"error", err,
				)
			}
			continue
		}

		// Success
		ledger := uint32(resp.Ledger)
		result := &TransactionResult{
			Hash:      resp.Hash,
			Ledger:    ledger,
			Status:    "pending",
			Submitted: time.Now(),
		}

		slog.Info("transaction submitted successfully",
			"tx_hash", resp.Hash,
			"ledger", resp.Ledger,
		)

		return result, nil
	}

	return nil, fmt.Errorf("transaction submission failed after %d attempts: %w", tb.retryConfig.MaxRetries+1, lastErr)
}

// isNonRetryableError checks if an error should not be retried
func isNonRetryableError(herr *horizonclient.Error) bool {
	// Check result codes
	if resultCodes, ok := herr.Problem.Extras["result_codes"].(map[string]interface{}); ok {
		if transactionCode, ok := resultCodes["transaction"].(string); ok {
			// These errors should not be retried
			nonRetryableCodes := []string{
				"tx_bad_auth",
				"tx_bad_seq",
				"tx_insufficient_balance",
				"tx_no_source_account",
			}
			for _, code := range nonRetryableCodes {
				if transactionCode == code {
					return true
				}
			}
		}
	}
	return false
}

// WaitForConfirmation polls for transaction confirmation
func (tb *TransactionBuilder) WaitForConfirmation(ctx context.Context, txHash string, timeout time.Duration) (*TransactionResult, error) {
	deadline := time.Now().Add(timeout)
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-ticker.C:
			if time.Now().After(deadline) {
				return nil, fmt.Errorf("timeout waiting for transaction confirmation: %s", txHash)
			}

			tx, err := tb.client.GetHorizonClient().TransactionDetail(txHash)
			if err != nil {
				// Transaction not found yet, continue polling
				continue
			}

			// Transaction found
			result := &TransactionResult{
				Hash:      txHash,
				Ledger:    uint32(tx.Ledger),
				Status:    "success",
				Submitted: time.Now(), // Approximate
				Confirmed: time.Now(),
			}

			slog.Info("transaction confirmed",
				"tx_hash", txHash,
				"ledger", tx.Ledger,
			)

			return result, nil
		}
	}
}

// EncodeContractAddress encodes a contract address to XDR
func EncodeContractAddress(contractID string) (xdr.ScAddress, error) {
	// Contract ID is typically a hex string (64 chars) or base64
	var hash xdr.Hash
	
	// Try hex first (64 hex chars = 32 bytes)
	if len(contractID) == 64 {
		// Parse hex string
		var err error
		for i := 0; i < 32; i++ {
			var b byte
			_, err = fmt.Sscanf(contractID[i*2:i*2+2], "%02x", &b)
			if err != nil {
				break
			}
			hash[i] = b
		}
		if err == nil {
			contractId := xdr.ContractId(hash)
			return xdr.ScAddress{
				Type:        xdr.ScAddressTypeScAddressTypeContract,
				ContractId: &contractId,
			}, nil
		}
	}

	// Try base64
	bytes, err := base64.StdEncoding.DecodeString(contractID)
	if err != nil {
		return xdr.ScAddress{}, fmt.Errorf("invalid contract ID format (expected hex or base64): %w", err)
	}
	if len(bytes) != 32 {
		return xdr.ScAddress{}, fmt.Errorf("contract ID must be 32 bytes, got %d", len(bytes))
	}
	copy(hash[:], bytes)

	contractId := xdr.ContractId(hash)
	return xdr.ScAddress{
		Type:        xdr.ScAddressTypeScAddressTypeContract,
		ContractId: &contractId,
	}, nil
}
