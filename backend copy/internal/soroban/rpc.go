package soroban

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"time"
)

// RPCRequest represents a Soroban RPC JSON-RPC request
type RPCRequest struct {
	JSONRPC string      `json:"jsonrpc"`
	ID      int         `json:"id"`
	Method  string      `json:"method"`
	Params  interface{} `json:"params"`
}

// RPCResponse represents a Soroban RPC JSON-RPC response
type RPCResponse struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      int             `json:"id"`
	Result  json.RawMessage `json:"result,omitempty"`
	Error   *RPCError       `json:"error,omitempty"`
}

// RPCError represents a Soroban RPC error
type RPCError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
	Data    string `json:"data,omitempty"`
}

// Call makes a JSON-RPC call to the Soroban RPC endpoint
func (c *Client) Call(ctx context.Context, method string, params interface{}) (*RPCResponse, error) {
	req := RPCRequest{
		JSONRPC: "2.0",
		ID:      1,
		Method:  method,
		Params:  params,
	}

	reqBody, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, "POST", c.rpcURL, bytes.NewReader(reqBody))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	httpReq.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("RPC call failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("RPC call failed with status %d: %s", resp.StatusCode, string(body))
	}

	var rpcResp RPCResponse
	if err := json.NewDecoder(resp.Body).Decode(&rpcResp); err != nil {
		return nil, fmt.Errorf("failed to decode RPC response: %w", err)
	}

	if rpcResp.Error != nil {
		return nil, fmt.Errorf("RPC error: %s (code: %d)", rpcResp.Error.Message, rpcResp.Error.Code)
	}

	return &rpcResp, nil
}

// SimulateTransaction simulates a transaction using Soroban RPC
func (c *Client) SimulateTransaction(ctx context.Context, txEnvelopeXDR string) (map[string]interface{}, error) {
	params := map[string]interface{}{
		"transaction": txEnvelopeXDR,
	}

	resp, err := c.Call(ctx, "simulateTransaction", params)
	if err != nil {
		return nil, err
	}

	var result map[string]interface{}
	if err := json.Unmarshal(resp.Result, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal result: %w", err)
	}

	return result, nil
}

// SendTransaction sends a transaction using Soroban RPC
func (c *Client) SendTransaction(ctx context.Context, txEnvelopeXDR string) (string, error) {
	params := map[string]interface{}{
		"transaction": txEnvelopeXDR,
	}

	resp, err := c.Call(ctx, "sendTransaction", params)
	if err != nil {
		return "", err
	}

	var result map[string]interface{}
	if err := json.Unmarshal(resp.Result, &result); err != nil {
		return "", fmt.Errorf("failed to unmarshal result: %w", err)
	}

	hash, ok := result["transactionHash"].(string)
	if !ok {
		return "", fmt.Errorf("invalid response: missing transactionHash")
	}

	return hash, nil
}

// GetTransactionStatus gets the status of a transaction
func (c *Client) GetTransactionStatus(ctx context.Context, txHash string) (map[string]interface{}, error) {
	params := map[string]interface{}{
		"hash": txHash,
	}

	resp, err := c.Call(ctx, "getTransaction", params)
	if err != nil {
		return nil, err
	}

	var result map[string]interface{}
	if err := json.Unmarshal(resp.Result, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal result: %w", err)
	}

	return result, nil
}

// GetLatestLedger gets the latest ledger information
func (c *Client) GetLatestLedger(ctx context.Context) (map[string]interface{}, error) {
	resp, err := c.Call(ctx, "getLatestLedger", nil)
	if err != nil {
		return nil, err
	}

	var result map[string]interface{}
	if err := json.Unmarshal(resp.Result, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal result: %w", err)
	}

	return result, nil
}

// PollTransactionStatus polls for transaction status until confirmed or timeout
func (c *Client) PollTransactionStatus(ctx context.Context, txHash string, timeout time.Duration) (map[string]interface{}, error) {
	deadline := time.Now().Add(timeout)
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-ticker.C:
			if time.Now().After(deadline) {
				return nil, fmt.Errorf("timeout waiting for transaction: %s", txHash)
			}

			status, err := c.GetTransactionStatus(ctx, txHash)
			if err != nil {
				// Transaction not found yet, continue polling
				slog.Debug("transaction not found, continuing to poll",
					"tx_hash", txHash,
					"error", err,
				)
				continue
			}

			// Check status
			if statusVal, ok := status["status"].(string); ok {
				if statusVal == "SUCCESS" || statusVal == "FAILED" {
					slog.Info("transaction status determined",
						"tx_hash", txHash,
						"status", statusVal,
					)
					return status, nil
				}
			}

			// Transaction found but status not final, continue polling
			continue
		}
	}
}
