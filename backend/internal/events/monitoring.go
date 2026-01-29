package events

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"sync"
	"time"
)

// EventMonitor provides real-time event monitoring and alerting
type EventMonitor struct {
	mu              sync.RWMutex
	listeners       map[string][]EventListener
	alertHandlers   []AlertHandler
	anomalyDetector *AnomalyDetector
}

// EventListener is a function that handles events
type EventListener func(event ContractEvent) error

// AlertHandler handles alerts
type AlertHandler func(alert Alert) error

// Alert represents a monitoring alert
type Alert struct {
	ID        string                 `json:"id"`
	Severity  string                 `json:"severity"` // "INFO", "WARNING", "CRITICAL"
	Message   string                 `json:"message"`
	EventType string                 `json:"event_type"`
	EventID   string                 `json:"event_id"`
	Data      map[string]interface{} `json:"data"`
	Timestamp int64                  `json:"timestamp"`
}

// NewEventMonitor creates a new event monitor
func NewEventMonitor() *EventMonitor {
	return &EventMonitor{
		listeners:       make(map[string][]EventListener),
		alertHandlers:   make([]AlertHandler, 0),
		anomalyDetector: NewAnomalyDetector(),
	}
}

// On registers a listener for a specific event type
func (em *EventMonitor) On(eventType string, listener EventListener) {
	em.mu.Lock()
	defer em.mu.Unlock()

	em.listeners[eventType] = append(em.listeners[eventType], listener)
}

// OnAlert registers an alert handler
func (em *EventMonitor) OnAlert(handler AlertHandler) {
	em.mu.Lock()
	defer em.mu.Unlock()

	em.alertHandlers = append(em.alertHandlers, handler)
}

// Emit emits an event to all registered listeners
func (em *EventMonitor) Emit(ctx context.Context, event ContractEvent) error {
	em.mu.RLock()
	listeners := em.listeners[event.EventType]
	em.mu.RUnlock()

	// Call all listeners
	for _, listener := range listeners {
		if err := listener(event); err != nil {
			slog.Error("event listener error", "event_type", event.EventType, "error", err)
		}
	}

	// Check for anomalies
	if anomalies := em.anomalyDetector.Detect(event); len(anomalies) > 0 {
		for _, anomaly := range anomalies {
			em.raiseAlert(ctx, anomaly)
		}
	}

	return nil
}

// raiseAlert raises an alert
func (em *EventMonitor) raiseAlert(ctx context.Context, alert Alert) {
	em.mu.RLock()
	handlers := em.alertHandlers
	em.mu.RUnlock()

	for _, handler := range handlers {
		if err := handler(alert); err != nil {
			slog.Error("alert handler error", "alert_id", alert.ID, "error", err)
		}
	}
}

// AnomalyDetector detects anomalies in events
type AnomalyDetector struct {
	mu              sync.RWMutex
	eventHistory    map[string][]ContractEvent
	thresholds      map[string]float64
	maxHistorySize  int
}

// NewAnomalyDetector creates a new anomaly detector
func NewAnomalyDetector() *AnomalyDetector {
	return &AnomalyDetector{
		eventHistory:   make(map[string][]ContractEvent),
		thresholds:     getDefaultThresholds(),
		maxHistorySize: 1000,
	}
}

// getDefaultThresholds returns default anomaly thresholds
func getDefaultThresholds() map[string]float64 {
	return map[string]float64{
		"FundsLocked":        3.0,  // 3x average
		"FundsReleased":      3.0,
		"FundsRefunded":      3.0,
		"ProgramFundsLocked": 3.0,
		"BatchPayout":        2.0,
	}
}

// Detect detects anomalies in an event
func (ad *AnomalyDetector) Detect(event ContractEvent) []Alert {
	ad.mu.Lock()
	defer ad.mu.Unlock()

	var alerts []Alert

	// Add event to history
	ad.eventHistory[event.EventType] = append(ad.eventHistory[event.EventType], event)
	if len(ad.eventHistory[event.EventType]) > ad.maxHistorySize {
		ad.eventHistory[event.EventType] = ad.eventHistory[event.EventType][1:]
	}

	// Check for anomalies based on event type
	switch event.EventType {
	case "FundsLocked", "FundsReleased", "FundsRefunded", "ProgramFundsLocked":
		if alert := ad.detectAmountAnomaly(event); alert != nil {
			alerts = append(alerts, *alert)
		}
	case "OperationMetric":
		if alert := ad.detectOperationFailure(event); alert != nil {
			alerts = append(alerts, *alert)
		}
	case "PerformanceMetric":
		if alert := ad.detectPerformanceAnomaly(event); alert != nil {
			alerts = append(alerts, *alert)
		}
	}

	return alerts
}

// detectAmountAnomaly detects unusual transaction amounts
func (ad *AnomalyDetector) detectAmountAnomaly(event ContractEvent) *Alert {
	history := ad.eventHistory[event.EventType]
	if len(history) < 5 {
		return nil // Need at least 5 events for comparison
	}

	// Extract amount from event data
	var data map[string]interface{}
	if err := json.Unmarshal(event.Data, &data); err != nil {
		return nil
	}

	amount, ok := data["amount"].(float64)
	if !ok {
		return nil
	}

	// Calculate average
	var sum float64
	for _, e := range history[:len(history)-1] { // Exclude current event
		var d map[string]interface{}
		if err := json.Unmarshal(e.Data, &d); err != nil {
			continue
		}
		if a, ok := d["amount"].(float64); ok {
			sum += a
		}
	}

	avg := sum / float64(len(history)-1)
	threshold := ad.thresholds[event.EventType]

	if amount > avg*threshold {
		return &Alert{
			ID:        fmt.Sprintf("anomaly-%d", time.Now().UnixNano()),
			Severity:  "INFO",
			Message:   fmt.Sprintf("Unusual transaction amount: %.0f (avg: %.0f)", amount, avg),
			EventType: event.EventType,
			EventID:   event.ID,
			Data: map[string]interface{}{
				"amount":     amount,
				"average":    avg,
				"threshold":  threshold,
				"multiplier": amount / avg,
			},
			Timestamp: time.Now().Unix(),
		}
	}

	return nil
}

// detectOperationFailure detects operation failures
func (ad *AnomalyDetector) detectOperationFailure(event ContractEvent) *Alert {
	var data map[string]interface{}
	if err := json.Unmarshal(event.Data, &data); err != nil {
		return nil
	}

	success, ok := data["success"].(bool)
	if !ok || success {
		return nil // Only alert on failures
	}

	operation, _ := data["operation"].(string)
	caller, _ := data["caller"].(string)

	return &Alert{
		ID:        fmt.Sprintf("failure-%d", time.Now().UnixNano()),
		Severity:  "WARNING",
		Message:   fmt.Sprintf("Operation failed: %s", operation),
		EventType: event.EventType,
		EventID:   event.ID,
		Data: map[string]interface{}{
			"operation": operation,
			"caller":    caller,
		},
		Timestamp: time.Now().Unix(),
	}
}

// detectPerformanceAnomaly detects performance issues
func (ad *AnomalyDetector) detectPerformanceAnomaly(event ContractEvent) *Alert {
	var data map[string]interface{}
	if err := json.Unmarshal(event.Data, &data); err != nil {
		return nil
	}

	duration, ok := data["duration_ms"].(float64)
	if !ok {
		return nil
	}

	operation, _ := data["operation"].(string)

	// Define SLA thresholds (in milliseconds)
	slaThresholds := map[string]float64{
		"lock_funds":    1000,
		"release_funds": 1000,
		"refund_funds":  1000,
		"batch_payout":  5000,
	}

	sla, exists := slaThresholds[operation]
	if !exists {
		sla = 2000 // Default SLA
	}

	if duration > sla {
		return &Alert{
			ID:        fmt.Sprintf("perf-%d", time.Now().UnixNano()),
			Severity:  "WARNING",
			Message:   fmt.Sprintf("SLA violation: %s took %.0fms (SLA: %.0fms)", operation, duration, sla),
			EventType: event.EventType,
			EventID:   event.ID,
			Data: map[string]interface{}{
				"operation": operation,
				"duration":  duration,
				"sla":       sla,
				"exceeded":  duration - sla,
			},
			Timestamp: time.Now().Unix(),
		}
	}

	return nil
}

// SetThreshold sets an anomaly detection threshold
func (ad *AnomalyDetector) SetThreshold(eventType string, threshold float64) {
	ad.mu.Lock()
	defer ad.mu.Unlock()

	ad.thresholds[eventType] = threshold
}

// EventFilter provides filtering capabilities for events
type EventFilter struct {
	EventTypes    []string
	MinAmount     float64
	MaxAmount     float64
	StartTime     int64
	EndTime       int64
	CorrelationID string
}

// Matches checks if an event matches the filter
func (ef *EventFilter) Matches(event ContractEvent) bool {
	// Check event type
	if len(ef.EventTypes) > 0 {
		found := false
		for _, et := range ef.EventTypes {
			if et == event.EventType {
				found = true
				break
			}
		}
		if !found {
			return false
		}
	}

	// Check time range
	if ef.StartTime > 0 && event.Timestamp < ef.StartTime {
		return false
	}
	if ef.EndTime > 0 && event.Timestamp > ef.EndTime {
		return false
	}

	// Check correlation ID
	if ef.CorrelationID != "" && event.CorrelationID != ef.CorrelationID {
		return false
	}

	// Check amount range
	if ef.MinAmount > 0 || ef.MaxAmount > 0 {
		var data map[string]interface{}
		if err := json.Unmarshal(event.Data, &data); err != nil {
			return false
		}

		amount, ok := data["amount"].(float64)
		if !ok {
			return false
		}

		if ef.MinAmount > 0 && amount < ef.MinAmount {
			return false
		}
		if ef.MaxAmount > 0 && amount > ef.MaxAmount {
			return false
		}
	}

	return true
}

// EventAggregator aggregates events for reporting
type EventAggregator struct {
	mu     sync.RWMutex
	events []ContractEvent
}

// NewEventAggregator creates a new event aggregator
func NewEventAggregator() *EventAggregator {
	return &EventAggregator{
		events: make([]ContractEvent, 0),
	}
}

// Add adds an event to the aggregator
func (ea *EventAggregator) Add(event ContractEvent) {
	ea.mu.Lock()
	defer ea.mu.Unlock()

	ea.events = append(ea.events, event)
}

// GetStats returns aggregated statistics
type AggregatedStats struct {
	TotalEvents      int
	EventsByType     map[string]int
	TotalAmount      float64
	AverageAmount    float64
	MinAmount        float64
	MaxAmount        float64
	TimeRange        [2]int64
	UniqueContracts  int
}

// GetStats returns aggregated statistics
func (ea *EventAggregator) GetStats() *AggregatedStats {
	ea.mu.RLock()
	defer ea.mu.RUnlock()

	stats := &AggregatedStats{
		TotalEvents:     len(ea.events),
		EventsByType:    make(map[string]int),
		MinAmount:       float64(^uint64(0) >> 1), // Max float64
		UniqueContracts: 0,
	}

	contracts := make(map[string]bool)

	for _, event := range ea.events {
		stats.EventsByType[event.EventType]++
		contracts[event.ContractID] = true

		// Extract amount if present
		var data map[string]interface{}
		if err := json.Unmarshal(event.Data, &data); err != nil {
			continue
		}

		if amount, ok := data["amount"].(float64); ok {
			stats.TotalAmount += amount
			if amount < stats.MinAmount {
				stats.MinAmount = amount
			}
			if amount > stats.MaxAmount {
				stats.MaxAmount = amount
			}
		}

		// Track time range
		if stats.TimeRange[0] == 0 || event.Timestamp < stats.TimeRange[0] {
			stats.TimeRange[0] = event.Timestamp
		}
		if event.Timestamp > stats.TimeRange[1] {
			stats.TimeRange[1] = event.Timestamp
		}
	}

	if stats.TotalEvents > 0 {
		stats.AverageAmount = stats.TotalAmount / float64(stats.TotalEvents)
	}

	stats.UniqueContracts = len(contracts)

	return stats
}
