package didit

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

const BaseURL = "https://verification.didit.me/v2"

type Client struct {
	HTTP      *http.Client
	APIKey    string
	UserAgent string
}

func NewClient(apiKey string) *Client {
	return &Client{
		HTTP:      &http.Client{Timeout: 30 * time.Second},
		APIKey:    apiKey,
		UserAgent: "patchwork-backend",
	}
}

// CreateSessionRequest is the request body for creating a verification session
type CreateSessionRequest struct {
	WorkflowID string `json:"workflow_id"`
	VendorData string `json:"vendor_data,omitempty"` // User ID or other identifier
	Callback   string `json:"callback,omitempty"`    // Webhook callback URL
}

// CreateSessionResponse is the response from creating a session
type CreateSessionResponse struct {
	SessionID string `json:"session_id"`
	URL       string `json:"url"` // Verification link for user
}

// CreateSession creates a new KYC verification session
func (c *Client) CreateSession(ctx context.Context, req CreateSessionRequest) (CreateSessionResponse, error) {
	url := BaseURL + "/session/"
	
	body, err := json.Marshal(req)
	if err != nil {
		return CreateSessionResponse{}, fmt.Errorf("marshal request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return CreateSessionResponse{}, fmt.Errorf("create request: %w", err)
	}
	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Accept", "application/json")
	httpReq.Header.Set("x-api-key", c.APIKey)
	if c.UserAgent != "" {
		httpReq.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(httpReq)
	if err != nil {
		return CreateSessionResponse{}, fmt.Errorf("http request: %w", err)
	}
	defer resp.Body.Close()

	// Read the full response body for error details
	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return CreateSessionResponse{}, fmt.Errorf("read response body: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		// Try to parse error response
		var errBody struct {
			Error   string `json:"error"`
			Message string `json:"message"`
			Detail  string `json:"detail"`
		}
		_ = json.Unmarshal(bodyBytes, &errBody)
		
		// Build error message with all available information
		errMsg := errBody.Error
		if errMsg == "" {
			errMsg = errBody.Message
		}
		if errMsg == "" {
			errMsg = errBody.Detail
		}
		if errMsg == "" {
			errMsg = string(bodyBytes)
		}
		if errMsg == "" {
			errMsg = "unknown error"
		}
		
		return CreateSessionResponse{}, fmt.Errorf("didit create session failed: status %d, error: %s, body: %s", resp.StatusCode, errMsg, string(bodyBytes))
	}

	var result CreateSessionResponse
	if err := json.Unmarshal(bodyBytes, &result); err != nil {
		return CreateSessionResponse{}, fmt.Errorf("decode response: %w, body: %s", err, string(bodyBytes))
	}

	return result, nil
}

// SessionDecisionResponse contains the verification decision/result
type SessionDecisionResponse struct {
	Status      string                 `json:"status"` // approved, rejected, pending, etc.
	Decision    map[string]interface{} `json:"decision,omitempty"`
	Data        map[string]interface{} `json:"data,omitempty"`
	RawResponse string                 `json:"-"` // Raw JSON response for debugging
	// Capture any additional fields that might be in the response
	ExtraFields map[string]interface{} `json:"-"`
}

// GetSessionDecision retrieves the verification decision for a session
func (c *Client) GetSessionDecision(ctx context.Context, sessionID string) (SessionDecisionResponse, error) {
	url := fmt.Sprintf("%s/session/%s/decision/", BaseURL, sessionID)

	httpReq, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return SessionDecisionResponse{}, fmt.Errorf("create request: %w", err)
	}
	httpReq.Header.Set("Accept", "application/json")
	httpReq.Header.Set("x-api-key", c.APIKey)
	if c.UserAgent != "" {
		httpReq.Header.Set("User-Agent", c.UserAgent)
	}

	resp, err := c.HTTP.Do(httpReq)
	if err != nil {
		return SessionDecisionResponse{}, fmt.Errorf("http request: %w", err)
	}
	defer resp.Body.Close()

	// Read the raw response body for debugging
	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return SessionDecisionResponse{}, fmt.Errorf("read response body: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		var errBody struct {
			Error string `json:"error"`
		}
		_ = json.Unmarshal(bodyBytes, &errBody)
		return SessionDecisionResponse{}, fmt.Errorf("didit get decision failed: status %d, error: %s, body: %s", resp.StatusCode, errBody.Error, string(bodyBytes))
	}

	// First, unmarshal into a generic map to capture all fields
	var rawMap map[string]interface{}
	if err := json.Unmarshal(bodyBytes, &rawMap); err != nil {
		return SessionDecisionResponse{}, fmt.Errorf("decode response: %w, body: %s", err, string(bodyBytes))
	}

	// Extract known fields
	result := SessionDecisionResponse{
		RawResponse: string(bodyBytes),
		ExtraFields: make(map[string]interface{}),
	}

	if status, ok := rawMap["status"].(string); ok {
		result.Status = status
	}
	if decision, ok := rawMap["decision"].(map[string]interface{}); ok {
		result.Decision = decision
	}
	if data, ok := rawMap["data"].(map[string]interface{}); ok {
		result.Data = data
	}

	// Store any other fields that might contain rejection reasons
	for k, v := range rawMap {
		if k != "status" && k != "decision" && k != "data" {
			result.ExtraFields[k] = v
		}
	}

	return result, nil
}

