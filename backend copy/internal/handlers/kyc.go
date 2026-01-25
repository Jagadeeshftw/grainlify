package handlers

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/didit"
)

// extractKYCInfo extracts structured information from Didit response data
func extractKYCInfo(data map[string]interface{}) map[string]interface{} {
	extracted := make(map[string]interface{})

	// Extract personal information from id_verification
	if idVerification, ok := data["id_verification"].(map[string]interface{}); ok {
		if firstName, ok := idVerification["first_name"].(string); ok && firstName != "" {
			extracted["first_name"] = firstName
		}
		if lastName, ok := idVerification["last_name"].(string); ok && lastName != "" {
			extracted["last_name"] = lastName
		}
		if fullName, ok := idVerification["full_name"].(string); ok && fullName != "" {
			extracted["full_name"] = fullName
		}
		if address, ok := idVerification["address"].(string); ok && address != "" {
			extracted["address"] = address
		}
		if dob, ok := idVerification["date_of_birth"].(string); ok && dob != "" {
			extracted["date_of_birth"] = dob
		}
		if age, ok := idVerification["age"].(float64); ok {
			extracted["age"] = int(age)
		}
		if documentType, ok := idVerification["document_type"].(string); ok && documentType != "" {
			extracted["document_type"] = documentType
		}
		if documentNumber, ok := idVerification["document_number"].(string); ok && documentNumber != "" {
			extracted["document_number"] = documentNumber
		}
		if status, ok := idVerification["status"].(string); ok && status != "" {
			extracted["id_verification_status"] = status
		}
	}

	// Extract face match information
	if faceMatch, ok := data["face_match"].(map[string]interface{}); ok {
		if score, ok := faceMatch["score"].(float64); ok {
			extracted["face_match_score"] = score
		}
		if status, ok := faceMatch["status"].(string); ok && status != "" {
			extracted["face_match_status"] = status
		}
	}

	return extracted
}

// mapDiditStatus maps Didit status to our internal KYC status
// Production-ready mapping that preserves accurate status representation
// Status flow: not_started -> pending -> in_review -> verified/rejected/expired
func mapDiditStatus(diditStatus string) string {
	status := strings.ToLower(strings.TrimSpace(diditStatus))
	switch status {
	case "approved", "verified":
		return "verified"
	case "rejected", "declined":
		return "rejected"
	case "in review", "inreview":
		// Didit is actively reviewing the verification
		return "in_review"
	case "pending", "in_progress", "inprogress":
		// User has started verification process (clicked the link, submitted documents, etc.)
		// but Didit hasn't started reviewing yet
		return "pending"
	case "expired":
		return "expired"
	case "not started", "notstarted", "not_started":
		// Session exists but user hasn't clicked the verification link yet
		// This is distinct from "pending" - user hasn't begun verification
		return "not_started"
	default:
		// Unknown status - log as error for production monitoring
		slog.Error("unknown didit status - defaulting to not_started", "status", diditStatus, "original", diditStatus)
		return "not_started"
	}
}

type KYCHandler struct {
	cfg   config.Config
	db    *db.DB
	didit *didit.Client
}

func NewKYCHandler(cfg config.Config, d *db.DB) *KYCHandler {
	var diditClient *didit.Client
	if cfg.DiditAPIKey != "" {
		diditClient = didit.NewClient(cfg.DiditAPIKey)
	}
	return &KYCHandler{
		cfg:   cfg,
		db:    d,
		didit: diditClient,
	}
}

// Start initiates a KYC verification session for the authenticated user
func (h *KYCHandler) Start() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}
		if h.didit == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "kyc_not_configured", "message": "DIDIT_API_KEY and DIDIT_WORKFLOW_ID must be set"})
		}
		if h.cfg.DiditWorkflowID == "" {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "kyc_not_configured", "message": "DIDIT_WORKFLOW_ID must be set"})
		}

		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		// Check if user already has an active KYC session
		var existingSessionID *string
		var existingStatus *string
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT kyc_session_id, kyc_status
FROM users
WHERE id = $1
`, userID).Scan(&existingSessionID, &existingStatus)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "user_lookup_failed"})
		}

		// Only allow new session if:
		// 1. No session exists (status is NULL)
		// 2. Previous session was manually deleted in Didit dashboard and marked as 'expired'
		// Do NOT allow new session if status is: not_started, pending, in_review, verified, or rejected
		// Note: "not_started" means session exists but user hasn't clicked the link yet - still active
		if existingSessionID != nil && existingStatus != nil {
			// Get stored KYC data to find session URL
			var kycDataBytes []byte
			_ = h.db.Pool.QueryRow(c.Context(), `
SELECT kyc_data
FROM users
WHERE id = $1
`, userID).Scan(&kycDataBytes)

			var sessionURL string
			if len(kycDataBytes) > 0 {
				var kycDataMap map[string]interface{}
				if err := json.Unmarshal(kycDataBytes, &kycDataMap); err == nil {
					if url, ok := kycDataMap["session_url"].(string); ok && url != "" {
						sessionURL = url
					}
				}
			}

			// If no URL in stored data, construct it from session_id
			if sessionURL == "" && *existingSessionID != "" {
				// Construct URL: https://verify.didit.me/session/{short_id}
				// The session_id is UUID, but Didit uses a short ID in the URL
				// We'll try to get it from Didit API or construct a placeholder
				sessionURL = fmt.Sprintf("https://verify.didit.me/session/%s", *existingSessionID)
			}

			// Check if the existing session still exists in Didit
			// If it doesn't exist (404), it means admin deleted it - mark as expired and allow new session
			if h.didit != nil {
				decision, err := h.didit.GetSessionDecision(c.Context(), *existingSessionID)
				if err != nil {
					// Check if error indicates session not found/deleted
					errMsg := strings.ToLower(err.Error())
					if strings.Contains(errMsg, "404") ||
						strings.Contains(errMsg, "not found") ||
						strings.Contains(errMsg, "not_found") ||
						strings.Contains(errMsg, "invalid") ||
						strings.Contains(errMsg, "deleted") {
						// Session was deleted in Didit dashboard - mark as expired and allow new session
						_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE users
SET kyc_status = 'expired',
    kyc_session_id = NULL,
    updated_at = now()
WHERE id = $1
`, userID)
						slog.Info("session deleted in didit dashboard, marked as expired", "session_id", *existingSessionID, "user_id", userID)
						// Continue to create new session
					} else {
						// Session exists in Didit - don't allow new session, but return URL if we have it
						response := fiber.Map{
							"error":      "kyc_session_exists",
							"message":    fmt.Sprintf("You already have a KYC verification session (status: %s). Please complete it or contact admin to delete it.", *existingStatus),
							"session_id": *existingSessionID,
							"status":     *existingStatus,
						}
						if sessionURL != "" {
							response["url"] = sessionURL
						}
						return c.Status(fiber.StatusConflict).JSON(response)
					}
				} else {
					// Session exists in Didit - extract session_url from response if available
					if decision.ExtraFields != nil {
						if url, ok := decision.ExtraFields["session_url"].(string); ok && url != "" {
							sessionURL = url
						}
					}
					// Don't allow new session
					response := fiber.Map{
						"error":      "kyc_session_exists",
						"message":    fmt.Sprintf("You already have an active KYC verification session (status: %s). Please complete it or contact admin to delete it.", *existingStatus),
						"session_id": *existingSessionID,
						"status":     *existingStatus,
					}
					if sessionURL != "" {
						response["url"] = sessionURL
					}
					return c.Status(fiber.StatusConflict).JSON(response)
				}
			} else {
				// No Didit client - check status directly
				// Only allow new session if status is expired (session was deleted)
				if *existingStatus != "expired" {
					response := fiber.Map{
						"error":      "kyc_session_exists",
						"message":    fmt.Sprintf("You already have a KYC verification session (status: %s). Please complete it or contact admin to delete it.", *existingStatus),
						"session_id": *existingSessionID,
						"status":     *existingStatus,
					}
					if sessionURL != "" {
						response["url"] = sessionURL
					}
					return c.Status(fiber.StatusConflict).JSON(response)
				}
			}
		}

		// Build callback URL if public base URL is configured
		// Must be a full URL with protocol (https://)
		var callbackURL string
		if h.cfg.PublicBaseURL != "" {
			baseURL := strings.TrimRight(h.cfg.PublicBaseURL, "/")
			// Ensure it has a protocol
			if !strings.HasPrefix(baseURL, "http://") && !strings.HasPrefix(baseURL, "https://") {
				baseURL = "https://" + baseURL
			}
			callbackURL = fmt.Sprintf("%s/webhooks/didit", baseURL)
		}

		// Create Didit session
		slog.Info("creating didit session", "user_id", userID, "workflow_id", h.cfg.DiditWorkflowID, "callback", callbackURL)
		sessionResp, err := h.didit.CreateSession(c.Context(), didit.CreateSessionRequest{
			WorkflowID: h.cfg.DiditWorkflowID,
			VendorData: userID.String(),
			Callback:   callbackURL,
		})
		if err != nil {
			slog.Error("didit create session failed", "error", err, "user_id", userID, "workflow_id", h.cfg.DiditWorkflowID)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{
				"error":   "kyc_session_create_failed",
				"message": err.Error(),
			})
		}
		slog.Info("didit session created", "session_id", sessionResp.SessionID, "url", sessionResp.URL, "user_id", userID)

		// Store session ID and URL in database (replaces any existing session)
		// Store the URL in kyc_data so we can retrieve it later
		// Initial status should be 'not_started' since user hasn't clicked the link yet
		// The Status() endpoint will update it to 'pending' when user actually starts verification
		sessionDataJSON, _ := json.Marshal(map[string]interface{}{
			"session_url": sessionResp.URL,
		})

		slog.Info("storing kyc session in database", "user_id", userID, "session_id", sessionResp.SessionID, "status", "not_started")
		result, err := h.db.Pool.Exec(c.Context(), `
UPDATE users
SET kyc_session_id = $1,
    kyc_status = 'not_started',
    kyc_data = $2,
    updated_at = now()
WHERE id = $3
`, sessionResp.SessionID, sessionDataJSON, userID)
		if err != nil {
			slog.Error("failed to store kyc session in database",
				"error", err,
				"user_id", userID,
				"session_id", sessionResp.SessionID,
				"kyc_data_size", len(sessionDataJSON),
				"error_type", fmt.Sprintf("%T", err))
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{
				"error":   "kyc_session_store_failed",
				"message": err.Error(),
			})
		}

		rowsAffected := result.RowsAffected()
		slog.Info("stored new kyc session", "user_id", userID, "session_id", sessionResp.SessionID, "rows_affected", rowsAffected)

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"session_id": sessionResp.SessionID,
			"url":        sessionResp.URL,
		})
	}
}

// Status returns the current KYC verification status for the authenticated user
// If status is pending and we have a session_id, fetches latest status from Didit API
func (h *KYCHandler) Status() fiber.Handler {
	return func(c *fiber.Ctx) error {
		slog.Info("kyc status request started", "path", c.Path(), "method", c.Method())

		if h.db == nil || h.db.Pool == nil {
			slog.Error("db not configured in kyc status handler")
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		sub, _ := c.Locals(auth.LocalUserID).(string)
		if sub == "" {
			slog.Error("no user id in context")
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		userID, err := uuid.Parse(sub)
		if err != nil {
			slog.Error("failed to parse user id", "sub", sub, "error", err)
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		slog.Info("fetching kyc status from database", "user_id", userID)

		var kycStatus *string
		var kycSessionID *string
		var kycVerifiedAt *time.Time
		var kycData []byte

		err = h.db.Pool.QueryRow(c.Context(), `
SELECT kyc_status, kyc_session_id, kyc_verified_at, kyc_data
FROM users
WHERE id = $1
`, userID).Scan(&kycStatus, &kycSessionID, &kycVerifiedAt, &kycData)
		if err != nil {
			slog.Error("failed to fetch kyc status from database", "user_id", userID, "error", err, "error_type", fmt.Sprintf("%T", err))
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{
				"error":   "kyc_status_fetch_failed",
				"message": err.Error(),
			})
		}

		// Log actual values, not pointers
		statusStr := "nil"
		if kycStatus != nil {
			statusStr = *kycStatus
		}
		sessionIDStr := "nil"
		if kycSessionID != nil {
			sessionIDStr = *kycSessionID
		}
		verifiedAtLogStr := "nil"
		if kycVerifiedAt != nil {
			verifiedAtLogStr = kycVerifiedAt.Format(time.RFC3339)
		}

		slog.Info("fetched kyc status from database",
			"user_id", userID,
			"kyc_status", statusStr,
			"kyc_session_id", sessionIDStr,
			"kyc_verified_at", verifiedAtLogStr,
			"kyc_data_size", len(kycData))

		// If we have a session ID, always fetch latest status from Didit API
		// This ensures we detect if the session was deleted in Didit dashboard
		// and get accurate status updates (including not_started -> pending transitions)
		if kycSessionID != nil && *kycSessionID != "" && h.didit != nil {
			currentStatusStr := "nil"
			if kycStatus != nil {
				currentStatusStr = *kycStatus
			}
			slog.Info("checking session with didit api", "session_id", *kycSessionID, "current_status", currentStatusStr)
			// Always fetch to check if session still exists (especially for pending status)
			decision, err := h.didit.GetSessionDecision(c.Context(), *kycSessionID)
			if err != nil {
				// If API call fails, check if it's because session was deleted
				errMsg := strings.ToLower(err.Error())
				currentStatusStr := "nil"
				if kycStatus != nil {
					currentStatusStr = *kycStatus
				}
				slog.Warn("didit api call failed",
					"session_id", *kycSessionID,
					"error", err.Error(),
					"current_status", currentStatusStr,
					"error_type", fmt.Sprintf("%T", err))

				// Check if error indicates session not found, deleted, or invalid
				// Check for various error patterns that indicate session doesn't exist
				// The error format from Didit client is: "didit get decision failed: status 404, error: ..., body: ..."
				isDeleted := strings.Contains(errMsg, "status 404") ||
					strings.Contains(errMsg, "status: 404") ||
					strings.Contains(errMsg, "404") ||
					strings.Contains(errMsg, "not found") ||
					strings.Contains(errMsg, "not_found") ||
					strings.Contains(errMsg, "invalid") ||
					strings.Contains(errMsg, "deleted") ||
					strings.Contains(errMsg, "does not exist") ||
					strings.Contains(errMsg, "doesn't exist") ||
					strings.Contains(errMsg, "no such") ||
					strings.Contains(errMsg, "not available")

				if isDeleted {
					previousStatusStr := "nil"
					if kycStatus != nil {
						previousStatusStr = *kycStatus
					}
					slog.Info("session deleted in didit - marking as expired",
						"session_id", *kycSessionID,
						"user_id", userID,
						"previous_status", previousStatusStr)
					// Session was deleted in Didit dashboard - mark as expired
					expiredStatus := "expired"
					// Store the session ID before clearing it for logging
					deletedSessionID := *kycSessionID
					_, updateErr := h.db.Pool.Exec(c.Context(), `
UPDATE users
SET kyc_status = $1,
    kyc_session_id = NULL,
    updated_at = now()
WHERE id = $2
`, expiredStatus, userID)
					if updateErr != nil {
						slog.Error("failed to mark session as expired in database",
							"error", updateErr,
							"user_id", userID,
							"session_id", deletedSessionID,
							"error_type", fmt.Sprintf("%T", updateErr))
						// Don't return error - continue with existing status
					} else {
						kycStatus = &expiredStatus
						kycSessionID = nil // Clear session ID since it's invalid
						previousStatusStr := "nil"
						if kycStatus != nil {
							previousStatusStr = *kycStatus
						}
						slog.Info("marked session as expired - deleted in didit dashboard",
							"session_id", deletedSessionID,
							"user_id", userID,
							"previous_status", previousStatusStr,
							"new_status", expiredStatus)
					}
				} else {
					// For other errors (network, timeout, etc.), log but keep existing status
					currentStatusStr := "nil"
					if kycStatus != nil {
						currentStatusStr = *kycStatus
					}
					slog.Warn("didit api error but session may still exist",
						"session_id", *kycSessionID,
						"error", err.Error(),
						"current_status", currentStatusStr)
				}
			} else {
				// Session exists in Didit - update status based on Didit response
				newStatus := mapDiditStatus(decision.Status)

				// Log the full decision structure for debugging
				decisionJSONDebug, _ := json.Marshal(decision.Decision)
				dataJSONDebug, _ := json.Marshal(decision.Data)
				extraFieldsJSON, _ := json.Marshal(decision.ExtraFields)
				currentStatusStr := "nil"
				if kycStatus != nil {
					currentStatusStr = *kycStatus
				}
				slog.Info("fetched didit status",
					"session_id", *kycSessionID,
					"didit_status", decision.Status,
					"mapped_status", newStatus,
					"current_db_status", currentStatusStr,
					"decision", string(decisionJSONDebug),
					"data", string(dataJSONDebug),
					"extra_fields", string(extraFieldsJSON))

				// Store Decision, Data, and any extra fields from Didit response
				combinedData := map[string]interface{}{
					"decision": decision.Decision,
					"data":     decision.Data,
				}
				// Include any extra fields (like session_url)
				for k, v := range decision.ExtraFields {
					combinedData[k] = v
				}

				// Extract structured information from the response
				extractedInfo := extractKYCInfo(combinedData)
				if len(extractedInfo) > 0 {
					combinedData["extracted"] = extractedInfo
				}

				decisionJSON, _ := json.Marshal(combinedData)

				// Update database if status changed (including not_started -> pending transitions)
				// Always update to ensure accurate status representation
				statusChanged := kycStatus == nil || *kycStatus != newStatus
				if statusChanged || *kycStatus == "rejected" {
					oldStatusStr := "nil"
					if kycStatus != nil {
						oldStatusStr = *kycStatus
					}
					_, updateErr := h.db.Pool.Exec(c.Context(), `
UPDATE users
SET kyc_status = $1,
    kyc_data = $2,
    kyc_verified_at = CASE WHEN $1 = 'verified' THEN now() ELSE kyc_verified_at END,
    updated_at = now()
WHERE id = $3
`, newStatus, decisionJSON, userID)
					if updateErr != nil {
						slog.Error("failed to update kyc status", "error", updateErr, "user_id", userID, "old_status", oldStatusStr, "new_status", newStatus)
					} else {
						kycStatus = &newStatus
						// Update kycData with latest decision data
						kycData = decisionJSON
						if statusChanged {
							slog.Info("kyc status changed", "user_id", userID, "old_status", oldStatusStr, "new_status", newStatus, "didit_status", decision.Status)
						}
					}
				} else {
					// Status hasn't changed, but still update kyc_data if we have new info
					_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE users
SET kyc_data = $1,
    updated_at = now()
WHERE id = $2
`, decisionJSON, userID)
					kycData = decisionJSON
				}
			}
		}

		var kycDataMap map[string]interface{}
		if len(kycData) > 0 {
			_ = json.Unmarshal(kycData, &kycDataMap)
		}

		// Extract rejection reasons and get extracted info
		var extractedInfo map[string]interface{}
		var rejectionReason interface{}

		if kycDataMap != nil {
			// Get extracted info if it exists, otherwise extract it now
			if extracted, ok := kycDataMap["extracted"].(map[string]interface{}); ok {
				extractedInfo = extracted
			} else {
				// Extract info if not already extracted
				extractedInfo = extractKYCInfo(kycDataMap)
				if len(extractedInfo) > 0 {
					// Store extracted info
					mergedData := make(map[string]interface{})
					if len(kycData) > 0 {
						_ = json.Unmarshal(kycData, &mergedData)
					}
					mergedData["extracted"] = extractedInfo
					mergedJSON, _ := json.Marshal(mergedData)

					_, _ = h.db.Pool.Exec(c.Context(), `
UPDATE users
SET kyc_data = $1,
    updated_at = now()
WHERE id = $2
`, mergedJSON, userID)
				}
			}

			// Extract rejection reasons from warnings
			var rejectionReasons []string

			// Check face_match warnings
			if faceMatch, ok := kycDataMap["face_match"].(map[string]interface{}); ok {
				if warnings, ok := faceMatch["warnings"].([]interface{}); ok {
					for _, warning := range warnings {
						if w, ok := warning.(map[string]interface{}); ok {
							if longDesc, ok := w["long_description"].(string); ok && longDesc != "" {
								rejectionReasons = append(rejectionReasons, longDesc)
							} else if shortDesc, ok := w["short_description"].(string); ok && shortDesc != "" {
								rejectionReasons = append(rejectionReasons, shortDesc)
							}
						}
					}
				}
			}

			// Check other feature warnings (id_verification, liveness, etc.)
			featuresToCheck := []string{"id_verification", "liveness", "ip_analysis"}
			for _, featureName := range featuresToCheck {
				if feature, ok := kycDataMap[featureName].(map[string]interface{}); ok {
					if warnings, ok := feature["warnings"].([]interface{}); ok {
						for _, warning := range warnings {
							if w, ok := warning.(map[string]interface{}); ok {
								if longDesc, ok := w["long_description"].(string); ok && longDesc != "" {
									rejectionReasons = append(rejectionReasons, longDesc)
								} else if shortDesc, ok := w["short_description"].(string); ok && shortDesc != "" {
									rejectionReasons = append(rejectionReasons, shortDesc)
								}
							}
						}
					}
				}
			}

			// If rejected, set rejection reason
			if kycStatus != nil && *kycStatus == "rejected" {
				if len(rejectionReasons) > 0 {
					rejectionReason = strings.Join(rejectionReasons, "; ")
					if extractedInfo == nil {
						extractedInfo = make(map[string]interface{})
					}
					extractedInfo["rejection_reasons"] = rejectionReasons
				} else {
					// Fallback: check for any status fields that indicate rejection
					rejectionReason = "Verification declined"
				}
			}
		}

		// Format verified_at as ISO8601 string for JSON response
		var verifiedAtStr *string
		if kycVerifiedAt != nil {
			formatted := kycVerifiedAt.Format(time.RFC3339)
			verifiedAtStr = &formatted
		}

		response := fiber.Map{
			"status":      kycStatus,
			"session_id":  kycSessionID,
			"verified_at": verifiedAtStr,
			"data":        kycDataMap,
		}

		// Add extracted information if available
		if extractedInfo != nil && len(extractedInfo) > 0 {
			response["extracted"] = extractedInfo
		}

		// Add rejection reason if available
		if rejectionReason != nil {
			response["rejection_reason"] = rejectionReason
		}

		// Log actual status values for debugging
		responseStatusStr := "nil"
		if kycStatus != nil {
			responseStatusStr = *kycStatus
		}
		responseSessionIDStr := "nil"
		if kycSessionID != nil {
			responseSessionIDStr = *kycSessionID
		}
		responseVerifiedAtLogStr := "nil"
		if verifiedAtStr != nil {
			responseVerifiedAtLogStr = *verifiedAtStr
		}

		slog.Info("returning kyc status response",
			"user_id", userID,
			"status", responseStatusStr,
			"session_id", responseSessionIDStr,
			"verified_at", responseVerifiedAtLogStr,
			"has_extracted", extractedInfo != nil && len(extractedInfo) > 0,
			"has_rejection_reason", rejectionReason != nil)

		return c.Status(fiber.StatusOK).JSON(response)
	}
}
