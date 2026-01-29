package events

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

// EventIndexer provides efficient querying and indexing of contract events
type EventIndexer struct {
	pool *pgxpool.Pool
}

// NewEventIndexer creates a new event indexer
func NewEventIndexer(pool *pgxpool.Pool) *EventIndexer {
	return &EventIndexer{pool: pool}
}

// ContractEvent represents a stored contract event with metadata
type ContractEvent struct {
	ID            string          `json:"id"`
	ContractID    string          `json:"contract_id"`
	EventType     string          `json:"event_type"`
	Version       int             `json:"version"`
	CorrelationID string          `json:"correlation_id"`
	Timestamp     int64           `json:"timestamp"`
	Data          json.RawMessage `json:"data"`
	Indexed       bool            `json:"indexed"`
	IndexedAt     *time.Time      `json:"indexed_at,omitempty"`
}

// EventQuery represents a query for events
type EventQuery struct {
	EventTypes    []string
	ContractID    string
	StartTime     int64
	EndTime       int64
	Limit         int
	Offset        int
	OrderBy       string // "timestamp" or "id"
	Order         string // "ASC" or "DESC"
	CorrelationID string
}

// QueryEvents queries events based on criteria
func (ei *EventIndexer) QueryEvents(ctx context.Context, query EventQuery) ([]ContractEvent, error) {
	if ei.pool == nil {
		return nil, fmt.Errorf("event indexer not initialized")
	}

	// Build query
	sql := `
		SELECT id, contract_id, event_type, version, correlation_id, 
		       timestamp, data, indexed, indexed_at
		FROM contract_events
		WHERE 1=1
	`
	args := []interface{}{}
	argCount := 1

	// Add filters
	if len(query.EventTypes) > 0 {
		sql += fmt.Sprintf(" AND event_type = ANY($%d)", argCount)
		args = append(args, query.EventTypes)
		argCount++
	}

	if query.ContractID != "" {
		sql += fmt.Sprintf(" AND contract_id = $%d", argCount)
		args = append(args, query.ContractID)
		argCount++
	}

	if query.StartTime > 0 {
		sql += fmt.Sprintf(" AND timestamp >= $%d", argCount)
		args = append(args, query.StartTime)
		argCount++
	}

	if query.EndTime > 0 {
		sql += fmt.Sprintf(" AND timestamp <= $%d", argCount)
		args = append(args, query.EndTime)
		argCount++
	}

	if query.CorrelationID != "" {
		sql += fmt.Sprintf(" AND correlation_id = $%d", argCount)
		args = append(args, query.CorrelationID)
		argCount++
	}

	// Add ordering
	orderBy := "timestamp"
	if query.OrderBy != "" {
		orderBy = query.OrderBy
	}
	order := "DESC"
	if query.Order != "" {
		order = query.Order
	}
	sql += fmt.Sprintf(" ORDER BY %s %s", orderBy, order)

	// Add limit and offset
	if query.Limit <= 0 {
		query.Limit = 1000
	}
	if query.Limit > 10000 {
		query.Limit = 10000
	}
	sql += fmt.Sprintf(" LIMIT $%d OFFSET $%d", argCount, argCount+1)
	args = append(args, query.Limit, query.Offset)

	// Execute query
	rows, err := ei.pool.Query(ctx, sql, args...)
	if err != nil {
		slog.Error("failed to query events", "error", err)
		return nil, err
	}
	defer rows.Close()

	var events []ContractEvent
	for rows.Next() {
		var event ContractEvent
		if err := rows.Scan(
			&event.ID, &event.ContractID, &event.EventType, &event.Version,
			&event.CorrelationID, &event.Timestamp, &event.Data,
			&event.Indexed, &event.IndexedAt,
		); err != nil {
			slog.Error("failed to scan event", "error", err)
			return nil, err
		}
		events = append(events, event)
	}

	return events, rows.Err()
}

// AggregateEvents aggregates events by a field
type AggregateQuery struct {
	EventTypes []string
	ContractID string
	StartTime  int64
	EndTime    int64
	GroupBy    string // "event_type", "contract_id", etc.
	Aggregate  string // "COUNT", "SUM", etc.
	Field      string // Field to aggregate (e.g., "amount")
}

// AggregateResult represents aggregation result
type AggregateResult struct {
	GroupKey string      `json:"group_key"`
	Value    interface{} `json:"value"`
}

// Aggregate aggregates events
func (ei *EventIndexer) Aggregate(ctx context.Context, query AggregateQuery) ([]AggregateResult, error) {
	if ei.pool == nil {
		return nil, fmt.Errorf("event indexer not initialized")
	}

	// Build query
	sql := fmt.Sprintf(`
		SELECT %s as group_key, %s(%s) as value
		FROM contract_events
		WHERE 1=1
	`, query.GroupBy, query.Aggregate, query.Field)

	args := []interface{}{}
	argCount := 1

	// Add filters
	if len(query.EventTypes) > 0 {
		sql += fmt.Sprintf(" AND event_type = ANY($%d)", argCount)
		args = append(args, query.EventTypes)
		argCount++
	}

	if query.ContractID != "" {
		sql += fmt.Sprintf(" AND contract_id = $%d", argCount)
		args = append(args, query.ContractID)
		argCount++
	}

	if query.StartTime > 0 {
		sql += fmt.Sprintf(" AND timestamp >= $%d", argCount)
		args = append(args, query.StartTime)
		argCount++
	}

	if query.EndTime > 0 {
		sql += fmt.Sprintf(" AND timestamp <= $%d", argCount)
		args = append(args, query.EndTime)
		argCount++
	}

	sql += fmt.Sprintf(" GROUP BY %s ORDER BY value DESC", query.GroupBy)

	// Execute query
	rows, err := ei.pool.Query(ctx, sql, args...)
	if err != nil {
		slog.Error("failed to aggregate events", "error", err)
		return nil, err
	}
	defer rows.Close()

	var results []AggregateResult
	for rows.Next() {
		var result AggregateResult
		if err := rows.Scan(&result.GroupKey, &result.Value); err != nil {
			slog.Error("failed to scan aggregate result", "error", err)
			return nil, err
		}
		results = append(results, result)
	}

	return results, rows.Err()
}

// StoreEvent stores a contract event
func (ei *EventIndexer) StoreEvent(ctx context.Context, event ContractEvent) error {
	if ei.pool == nil {
		return fmt.Errorf("event indexer not initialized")
	}

	sql := `
		INSERT INTO contract_events 
		(id, contract_id, event_type, version, correlation_id, timestamp, data, indexed)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		ON CONFLICT (id) DO NOTHING
	`

	_, err := ei.pool.Exec(ctx, sql,
		event.ID, event.ContractID, event.EventType, event.Version,
		event.CorrelationID, event.Timestamp, event.Data, event.Indexed,
	)

	if err != nil {
		slog.Error("failed to store event", "error", err)
		return err
	}

	return nil
}

// MarkEventIndexed marks an event as indexed
func (ei *EventIndexer) MarkEventIndexed(ctx context.Context, eventID string) error {
	if ei.pool == nil {
		return fmt.Errorf("event indexer not initialized")
	}

	sql := `
		UPDATE contract_events 
		SET indexed = true, indexed_at = NOW()
		WHERE id = $1
	`

	_, err := ei.pool.Exec(ctx, sql, eventID)
	if err != nil {
		slog.Error("failed to mark event indexed", "error", err)
		return err
	}

	return nil
}

// GetUnindexedEvents retrieves events that haven't been indexed yet
func (ei *EventIndexer) GetUnindexedEvents(ctx context.Context, limit int) ([]ContractEvent, error) {
	if ei.pool == nil {
		return nil, fmt.Errorf("event indexer not initialized")
	}

	if limit <= 0 {
		limit = 100
	}
	if limit > 1000 {
		limit = 1000
	}

	sql := `
		SELECT id, contract_id, event_type, version, correlation_id, 
		       timestamp, data, indexed, indexed_at
		FROM contract_events
		WHERE indexed = false
		ORDER BY timestamp ASC
		LIMIT $1
	`

	rows, err := ei.pool.Query(ctx, sql, limit)
	if err != nil {
		slog.Error("failed to get unindexed events", "error", err)
		return nil, err
	}
	defer rows.Close()

	var events []ContractEvent
	for rows.Next() {
		var event ContractEvent
		if err := rows.Scan(
			&event.ID, &event.ContractID, &event.EventType, &event.Version,
			&event.CorrelationID, &event.Timestamp, &event.Data,
			&event.Indexed, &event.IndexedAt,
		); err != nil {
			slog.Error("failed to scan event", "error", err)
			return nil, err
		}
		events = append(events, event)
	}

	return events, rows.Err()
}

// GetEventStats returns statistics about stored events
type EventStats struct {
	TotalEvents      int64
	EventsByType     map[string]int64
	OldestEventTime  int64
	NewestEventTime  int64
	UnindexedCount   int64
	AveragePerDay    float64
}

// GetStats returns event statistics
func (ei *EventIndexer) GetStats(ctx context.Context) (*EventStats, error) {
	if ei.pool == nil {
		return nil, fmt.Errorf("event indexer not initialized")
	}

	stats := &EventStats{
		EventsByType: make(map[string]int64),
	}

	// Get total count
	err := ei.pool.QueryRow(ctx, "SELECT COUNT(*) FROM contract_events").Scan(&stats.TotalEvents)
	if err != nil {
		slog.Error("failed to get total event count", "error", err)
		return nil, err
	}

	// Get events by type
	rows, err := ei.pool.Query(ctx, `
		SELECT event_type, COUNT(*) 
		FROM contract_events 
		GROUP BY event_type
	`)
	if err != nil {
		slog.Error("failed to get events by type", "error", err)
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		var eventType string
		var count int64
		if err := rows.Scan(&eventType, &count); err != nil {
			slog.Error("failed to scan event type count", "error", err)
			return nil, err
		}
		stats.EventsByType[eventType] = count
	}

	// Get time range
	err = ei.pool.QueryRow(ctx, `
		SELECT MIN(timestamp), MAX(timestamp) 
		FROM contract_events
	`).Scan(&stats.OldestEventTime, &stats.NewestEventTime)
	if err != nil {
		slog.Error("failed to get event time range", "error", err)
		return nil, err
	}

	// Get unindexed count
	err = ei.pool.QueryRow(ctx, `
		SELECT COUNT(*) FROM contract_events WHERE indexed = false
	`).Scan(&stats.UnindexedCount)
	if err != nil {
		slog.Error("failed to get unindexed count", "error", err)
		return nil, err
	}

	// Calculate average per day
	if stats.OldestEventTime > 0 && stats.NewestEventTime > 0 {
		daysDiff := (stats.NewestEventTime - stats.OldestEventTime) / 86400
		if daysDiff > 0 {
			stats.AveragePerDay = float64(stats.TotalEvents) / float64(daysDiff)
		}
	}

	return stats, nil
}
