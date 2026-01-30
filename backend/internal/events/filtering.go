package events

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"time"
)

// FilterBuilder provides a fluent API for building event filters
type FilterBuilder struct {
	filter *EventFilter
}

// NewFilterBuilder creates a new filter builder
func NewFilterBuilder() *FilterBuilder {
	return &FilterBuilder{
		filter: &EventFilter{},
	}
}

// WithEventTypes adds event types to filter
func (fb *FilterBuilder) WithEventTypes(types ...string) *FilterBuilder {
	fb.filter.EventTypes = append(fb.filter.EventTypes, types...)
	return fb
}

// WithMinAmount sets minimum amount filter
func (fb *FilterBuilder) WithMinAmount(amount float64) *FilterBuilder {
	fb.filter.MinAmount = amount
	return fb
}

// WithMaxAmount sets maximum amount filter
func (fb *FilterBuilder) WithMaxAmount(amount float64) *FilterBuilder {
	fb.filter.MaxAmount = amount
	return fb
}

// WithTimeRange sets time range filter
func (fb *FilterBuilder) WithTimeRange(start, end time.Time) *FilterBuilder {
	fb.filter.StartTime = start.Unix()
	fb.filter.EndTime = end.Unix()
	return fb
}

// WithCorrelationID sets correlation ID filter
func (fb *FilterBuilder) WithCorrelationID(id string) *FilterBuilder {
	fb.filter.CorrelationID = id
	return fb
}

// Build returns the built filter
func (fb *FilterBuilder) Build() *EventFilter {
	return fb.filter
}

// EventFilterChain allows chaining multiple filters
type EventFilterChain struct {
	filters []*EventFilter
}

// NewEventFilterChain creates a new filter chain
func NewEventFilterChain() *EventFilterChain {
	return &EventFilterChain{
		filters: make([]*EventFilter, 0),
	}
}

// Add adds a filter to the chain
func (efc *EventFilterChain) Add(filter *EventFilter) *EventFilterChain {
	efc.filters = append(efc.filters, filter)
	return efc
}

// Matches checks if an event matches all filters in the chain
func (efc *EventFilterChain) Matches(event ContractEvent) bool {
	for _, filter := range efc.filters {
		if !filter.Matches(event) {
			return false
		}
	}
	return true
}

// EventFilterPresets provides common filter presets
type EventFilterPresets struct{}

// LargeTransactions returns a filter for large transactions
func (efp *EventFilterPresets) LargeTransactions(threshold float64) *EventFilter {
	return &EventFilter{
		EventTypes: []string{"FundsLocked", "FundsReleased", "ProgramFundsReleased"},
		MinAmount:  threshold,
	}
}

// RecentEvents returns a filter for recent events
func (efp *EventFilterPresets) RecentEvents(hours int) *EventFilter {
	now := time.Now()
	return &EventFilter{
		StartTime: now.Add(-time.Duration(hours) * time.Hour).Unix(),
		EndTime:   now.Unix(),
	}
}

// FailedOperations returns a filter for failed operations
func (efp *EventFilterPresets) FailedOperations() *EventFilter {
	return &EventFilter{
		EventTypes: []string{"OperationMetric"},
	}
}

// PerformanceIssues returns a filter for performance issues
func (efp *EventFilterPresets) PerformanceIssues(thresholdMs int64) *EventFilter {
	return &EventFilter{
		EventTypes: []string{"PerformanceMetric"},
	}
}

// EventFilterValidator validates filter parameters
type EventFilterValidator struct{}

// Validate validates a filter
func (efv *EventFilterValidator) Validate(filter *EventFilter) error {
	if filter == nil {
		return fmt.Errorf("filter cannot be nil")
	}

	if filter.StartTime > 0 && filter.EndTime > 0 && filter.StartTime > filter.EndTime {
		return fmt.Errorf("start time cannot be after end time")
	}

	if filter.MinAmount > 0 && filter.MaxAmount > 0 && filter.MinAmount > filter.MaxAmount {
		return fmt.Errorf("min amount cannot be greater than max amount")
	}

	if filter.MinAmount < 0 || filter.MaxAmount < 0 {
		return fmt.Errorf("amounts cannot be negative")
	}

	return nil
}

// EventFilterOptimizer optimizes filters for query performance
type EventFilterOptimizer struct{}

// Optimize optimizes a filter for better query performance
func (efo *EventFilterOptimizer) Optimize(filter *EventFilter) *EventFilter {
	optimized := &EventFilter{
		EventTypes:    filter.EventTypes,
		MinAmount:     filter.MinAmount,
		MaxAmount:     filter.MaxAmount,
		StartTime:     filter.StartTime,
		EndTime:       filter.EndTime,
		CorrelationID: filter.CorrelationID,
	}

	// Remove empty event types
	if len(optimized.EventTypes) == 0 {
		optimized.EventTypes = nil
	}

	// Set reasonable defaults for time range
	if optimized.StartTime == 0 && optimized.EndTime == 0 {
		// Default to last 30 days
		now := time.Now()
		optimized.StartTime = now.Add(-30 * 24 * time.Hour).Unix()
		optimized.EndTime = now.Unix()
	}

	return optimized
}

// AdvancedEventFilter provides advanced filtering capabilities
type AdvancedEventFilter struct {
	BaseFilter *EventFilter
	DataFilter map[string]interface{}
	Operators  map[string]string // "eq", "gt", "lt", "contains", etc.
}

// NewAdvancedEventFilter creates a new advanced filter
func NewAdvancedEventFilter(baseFilter *EventFilter) *AdvancedEventFilter {
	return &AdvancedEventFilter{
		BaseFilter: baseFilter,
		DataFilter: make(map[string]interface{}),
		Operators:  make(map[string]string),
	}
}

// WithDataFilter adds a data field filter
func (aef *AdvancedEventFilter) WithDataFilter(field string, value interface{}, operator string) *AdvancedEventFilter {
	aef.DataFilter[field] = value
	aef.Operators[field] = operator
	return aef
}

// Matches checks if an event matches the advanced filter
func (aef *AdvancedEventFilter) Matches(event ContractEvent) bool {
	// Check base filter
	if !aef.BaseFilter.Matches(event) {
		return false
	}

	// Check data filters
	var data map[string]interface{}
	if err := json.Unmarshal(event.Data, &data); err != nil {
		slog.Error("failed to unmarshal event data", "error", err)
		return false
	}

	for field, expectedValue := range aef.DataFilter {
		operator := aef.Operators[field]
		actualValue, exists := data[field]

		if !exists {
			return false
		}

		if !aef.compareValues(actualValue, expectedValue, operator) {
			return false
		}
	}

	return true
}

// compareValues compares two values using the specified operator
func (aef *AdvancedEventFilter) compareValues(actual, expected interface{}, operator string) bool {
	switch operator {
	case "eq":
		return actual == expected
	case "ne":
		return actual != expected
	case "gt":
		return aef.numericCompare(actual, expected, ">")
	case "gte":
		return aef.numericCompare(actual, expected, ">=")
	case "lt":
		return aef.numericCompare(actual, expected, "<")
	case "lte":
		return aef.numericCompare(actual, expected, "<=")
	case "contains":
		if actualStr, ok := actual.(string); ok {
			if expectedStr, ok := expected.(string); ok {
				return len(actualStr) > 0 && len(expectedStr) > 0
			}
		}
		return false
	case "in":
		if expectedList, ok := expected.([]interface{}); ok {
			for _, item := range expectedList {
				if actual == item {
					return true
				}
			}
		}
		return false
	default:
		return actual == expected
	}
}

// numericCompare compares numeric values
func (aef *AdvancedEventFilter) numericCompare(actual, expected interface{}, operator string) bool {
	actualNum, ok1 := toFloat64(actual)
	expectedNum, ok2 := toFloat64(expected)

	if !ok1 || !ok2 {
		return false
	}

	switch operator {
	case ">":
		return actualNum > expectedNum
	case ">=":
		return actualNum >= expectedNum
	case "<":
		return actualNum < expectedNum
	case "<=":
		return actualNum <= expectedNum
	default:
		return false
	}
}

// toFloat64 converts a value to float64
func toFloat64(v interface{}) (float64, bool) {
	switch val := v.(type) {
	case float64:
		return val, true
	case float32:
		return float64(val), true
	case int:
		return float64(val), true
	case int64:
		return float64(val), true
	case string:
		var f float64
		_, err := fmt.Sscanf(val, "%f", &f)
		return f, err == nil
	default:
		return 0, false
	}
}

// EventFilterStatistics provides statistics about filtered events
type EventFilterStatistics struct {
	TotalMatched      int
	MatchedByType     map[string]int
	AmountStats       AmountStatistics
	TimeDistribution  map[string]int // Hour -> count
	ContractStats     map[string]int // Contract -> count
}

// AmountStatistics provides statistics about amounts
type AmountStatistics struct {
	Total   float64
	Average float64
	Min     float64
	Max     float64
	Count   int
}

// CalculateStatistics calculates statistics for filtered events
func CalculateStatistics(events []ContractEvent) *EventFilterStatistics {
	stats := &EventFilterStatistics{
		TotalMatched:     len(events),
		MatchedByType:    make(map[string]int),
		TimeDistribution: make(map[string]int),
		ContractStats:    make(map[string]int),
	}

	stats.AmountStats.Min = float64(^uint64(0) >> 1) // Max float64

	for _, event := range events {
		// Count by type
		stats.MatchedByType[event.EventType]++

		// Count by contract
		stats.ContractStats[event.ContractID]++

		// Time distribution
		hour := time.Unix(event.Timestamp, 0).Format("2006-01-02 15:00")
		stats.TimeDistribution[hour]++

		// Amount statistics
		var data map[string]interface{}
		if err := json.Unmarshal(event.Data, &data); err != nil {
			continue
		}

		if amount, ok := data["amount"].(float64); ok {
			stats.AmountStats.Total += amount
			stats.AmountStats.Count++

			if amount < stats.AmountStats.Min {
				stats.AmountStats.Min = amount
			}
			if amount > stats.AmountStats.Max {
				stats.AmountStats.Max = amount
			}
		}
	}

	if stats.AmountStats.Count > 0 {
		stats.AmountStats.Average = stats.AmountStats.Total / float64(stats.AmountStats.Count)
	}

	return stats
}

// EventFilterExporter exports filtered events in various formats
type EventFilterExporter struct{}

// ExportJSON exports events as JSON
func (efe *EventFilterExporter) ExportJSON(events []ContractEvent) ([]byte, error) {
	return json.MarshalIndent(events, "", "  ")
}

// ExportCSV exports events as CSV
func (efe *EventFilterExporter) ExportCSV(events []ContractEvent) (string, error) {
	if len(events) == 0 {
		return "", fmt.Errorf("no events to export")
	}

	csv := "ID,ContractID,EventType,Version,Timestamp,Data\n"

	for _, event := range events {
		dataStr := string(event.Data)
		// Escape quotes in data
		dataStr = fmt.Sprintf("\"%s\"", dataStr)

		csv += fmt.Sprintf("%s,%s,%s,%d,%d,%s\n",
			event.ID,
			event.ContractID,
			event.EventType,
			event.Version,
			event.Timestamp,
			dataStr,
		)
	}

	return csv, nil
}

// ExportSummary exports a summary of events
func (efe *EventFilterExporter) ExportSummary(events []ContractEvent) (map[string]interface{}, error) {
	stats := CalculateStatistics(events)

	return map[string]interface{}{
		"total_events":      stats.TotalMatched,
		"events_by_type":    stats.MatchedByType,
		"amount_statistics": stats.AmountStats,
		"unique_contracts":  len(stats.ContractStats),
		"time_range": map[string]interface{}{
			"earliest": events[len(events)-1].Timestamp,
			"latest":   events[0].Timestamp,
		},
	}, nil
}
