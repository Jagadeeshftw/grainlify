# Event Indexing and Monitoring Implementation

## Overview

This implementation provides comprehensive event indexing and monitoring capabilities for Grainlify contracts. It enables efficient off-chain tracking, real-time monitoring, and analytics for all on-chain operations.

## Components

### 1. Database Layer (`migrations/000025_contract_events_indexing.up.sql`)

**Tables:**
- `contract_events`: Stores all contract events with comprehensive indexing
- `event_alerts`: Stores monitoring alerts
- `event_metrics`: Stores performance metrics
- `event_replay_log`: Tracks event replay attempts

**Indexes:**
- Time-series: `(event_type, timestamp DESC)`
- Entity-based: `(contract_id, timestamp DESC)`
- Composite: `(event_type, contract_id, timestamp DESC)`
- JSONB: GIN index on event data
- Correlation: `(correlation_id)` for tracing

**Materialized Views:**
- `daily_event_stats`: Pre-computed daily statistics

**Functions:**
- `cleanup_old_events()`: Enforce retention policies
- `refresh_daily_event_stats()`: Update statistics
- `get_event_statistics()`: Query event stats
- `get_events_by_type_and_time()`: Efficient time-range queries

### 2. Event Indexing (`internal/events/indexing.go`)

**EventIndexer:**
- Query events with flexible filtering
- Aggregate events by field
- Store and retrieve events
- Track indexing status
- Get event statistics

**Key Methods:**
```go
// Query events
events, err := indexer.QueryEvents(ctx, EventQuery{
    EventTypes: []string{"FundsLocked"},
    StartTime:  time.Now().Add(-24 * time.Hour).Unix(),
    EndTime:    time.Now().Unix(),
    Limit:      1000,
})

// Aggregate events
results, err := indexer.Aggregate(ctx, AggregateQuery{
    EventTypes: []string{"FundsLocked"},
    GroupBy:    "event_type",
    Aggregate:  "COUNT",
})

// Get statistics
stats, err := indexer.GetStats(ctx)
```

### 3. Event Monitoring (`internal/events/monitoring.go`)

**EventMonitor:**
- Real-time event listening
- Anomaly detection
- Alert generation and handling
- Event filtering and aggregation

**Key Features:**
```go
// Register event listener
monitor.On("FundsLocked", func(event ContractEvent) error {
    // Handle event
    return nil
})

// Register alert handler
monitor.OnAlert(func(alert Alert) error {
    // Handle alert
    return nil
})

// Emit event
monitor.Emit(ctx, event)
```

**AnomalyDetector:**
- Detects unusual transaction amounts
- Detects operation failures
- Detects performance anomalies
- Configurable thresholds

### 4. Event Filtering (`internal/events/filtering.go`)

**FilterBuilder:**
- Fluent API for building filters
- Chainable filter operations

```go
filter := NewFilterBuilder().
    WithEventTypes("FundsLocked", "FundsReleased").
    WithMinAmount(100000).
    WithTimeRange(startTime, endTime).
    Build()
```

**AdvancedEventFilter:**
- Complex filtering with operators
- Data field filtering
- Comparison operators: eq, ne, gt, gte, lt, lte, contains, in

```go
filter := NewAdvancedEventFilter(baseFilter).
    WithDataFilter("amount", 1000000, "gt").
    WithDataFilter("status", []interface{}{"pending", "completed"}, "in")
```

**EventFilterStatistics:**
- Calculate statistics on filtered events
- Amount statistics (total, average, min, max)
- Distribution by type, contract, time

**EventFilterExporter:**
- Export events as JSON
- Export events as CSV
- Export summary statistics

## Usage Examples

### Example 1: Query Recent Events

```go
package main

import (
    "context"
    "time"
    "github.com/jagadeesh/grainlify/backend/internal/events"
)

func main() {
    ctx := context.Background()
    indexer := events.NewEventIndexer(pool)
    
    // Query recent FundsLocked events
    query := events.EventQuery{
        EventTypes: []string{"FundsLocked"},
        StartTime:  time.Now().Add(-24 * time.Hour).Unix(),
        EndTime:    time.Now().Unix(),
        OrderBy:    "timestamp",
        Order:      "DESC",
        Limit:      100,
    }
    
    events, err := indexer.QueryEvents(ctx, query)
    if err != nil {
        panic(err)
    }
    
    for _, event := range events {
        println(event.EventType, event.Timestamp)
    }
}
```

### Example 2: Monitor Events with Anomaly Detection

```go
package main

import (
    "context"
    "github.com/jagadeesh/grainlify/backend/internal/events"
)

func main() {
    ctx := context.Background()
    monitor := events.NewEventMonitor()
    
    // Register listener for large transactions
    monitor.On("FundsLocked", func(event events.ContractEvent) error {
        println("FundsLocked event:", event.ID)
        return nil
    })
    
    // Register alert handler
    monitor.OnAlert(func(alert events.Alert) error {
        println("Alert:", alert.Severity, alert.Message)
        return nil
    })
    
    // Emit event (anomaly detection runs automatically)
    event := events.ContractEvent{
        ID:        "event-1",
        EventType: "FundsLocked",
        Timestamp: time.Now().Unix(),
        Data:      []byte(`{"amount": 5000000}`),
    }
    
    monitor.Emit(ctx, event)
}
```

### Example 3: Filter and Export Events

```go
package main

import (
    "github.com/jagadeesh/grainlify/backend/internal/events"
)

func main() {
    // Build filter
    filter := events.NewFilterBuilder().
        WithEventTypes("FundsLocked", "FundsReleased").
        WithMinAmount(100000).
        Build()
    
    // Apply filter
    var filtered []events.ContractEvent
    for _, event := range allEvents {
        if filter.Matches(event) {
            filtered = append(filtered, event)
        }
    }
    
    // Export as JSON
    exporter := &events.EventFilterExporter{}
    jsonData, err := exporter.ExportJSON(filtered)
    if err != nil {
        panic(err)
    }
    
    println(string(jsonData))
}
```

### Example 4: Aggregate Events

```go
package main

import (
    "context"
    "github.com/jagadeesh/grainlify/backend/internal/events"
)

func main() {
    ctx := context.Background()
    indexer := events.NewEventIndexer(pool)
    
    // Aggregate by event type
    results, err := indexer.Aggregate(ctx, events.AggregateQuery{
        EventTypes: []string{"FundsLocked", "FundsReleased"},
        GroupBy:    "event_type",
        Aggregate:  "COUNT",
        Field:      "id",
    })
    if err != nil {
        panic(err)
    }
    
    for _, result := range results {
        println(result.GroupKey, result.Value)
    }
}
```

## API Integration

### Event Query Endpoint

```go
// GET /api/v1/events
// Query parameters:
// - event_types: comma-separated list of event types
// - contract_id: filter by contract
// - start_time: Unix timestamp
// - end_time: Unix timestamp
// - limit: max results (default 1000, max 10000)
// - offset: pagination offset

func QueryEventsHandler(w http.ResponseWriter, r *http.Request) {
    query := events.EventQuery{
        EventTypes: parseEventTypes(r.URL.Query().Get("event_types")),
        ContractID: r.URL.Query().Get("contract_id"),
        StartTime:  parseTime(r.URL.Query().Get("start_time")),
        EndTime:    parseTime(r.URL.Query().Get("end_time")),
        Limit:      parseLimit(r.URL.Query().Get("limit")),
        Offset:     parseOffset(r.URL.Query().Get("offset")),
    }
    
    events, err := indexer.QueryEvents(r.Context(), query)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    
    json.NewEncoder(w).Encode(events)
}
```

### Event Statistics Endpoint

```go
// GET /api/v1/events/stats

func GetStatsHandler(w http.ResponseWriter, r *http.Request) {
    stats, err := indexer.GetStats(r.Context())
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    
    json.NewEncoder(w).Encode(stats)
}
```

### Alert Management Endpoint

```go
// GET /api/v1/alerts
// POST /api/v1/alerts/:id/acknowledge

func GetAlertsHandler(w http.ResponseWriter, r *http.Request) {
    // Query alerts from database
    // Return JSON response
}

func AcknowledgeAlertHandler(w http.ResponseWriter, r *http.Request) {
    alertID := chi.URLParam(r, "id")
    // Mark alert as acknowledged
    // Return success response
}
```

## Monitoring Dashboard

### Key Metrics

1. **Event Volume**
   - Events per second
   - Events per day
   - Events by type

2. **Performance**
   - Query latency (p50, p95, p99)
   - Index size
   - Database size

3. **Alerts**
   - Active alerts
   - Alert rate
   - Alert resolution time

4. **Data Quality**
   - Unindexed events
   - Failed indexing
   - Data anomalies

### Dashboard Queries

```sql
-- Events per hour
SELECT 
    DATE_TRUNC('hour', to_timestamp(timestamp)) as hour,
    COUNT(*) as count
FROM contract_events
WHERE timestamp > EXTRACT(EPOCH FROM NOW() - INTERVAL '24 hours')
GROUP BY DATE_TRUNC('hour', to_timestamp(timestamp))
ORDER BY hour DESC;

-- Top event types
SELECT 
    event_type,
    COUNT(*) as count
FROM contract_events
WHERE timestamp > EXTRACT(EPOCH FROM NOW() - INTERVAL '7 days')
GROUP BY event_type
ORDER BY count DESC;

-- Active alerts
SELECT 
    severity,
    COUNT(*) as count
FROM event_alerts
WHERE acknowledged = false
GROUP BY severity;
```

## Performance Tuning

### Index Maintenance

```sql
-- Analyze table for query planner
ANALYZE contract_events;

-- Reindex if fragmented
REINDEX INDEX idx_contract_events_type_timestamp;

-- Check index size
SELECT 
    indexname,
    pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_indexes
WHERE tablename = 'contract_events'
ORDER BY pg_relation_size(indexrelid) DESC;
```

### Query Optimization

```go
// Use pagination for large result sets
for offset := 0; offset < totalEvents; offset += 1000 {
    query.Offset = offset
    events, err := indexer.QueryEvents(ctx, query)
    // Process batch
}

// Use specific event types to reduce scan
query.EventTypes = []string{"FundsLocked"} // More specific

// Use time ranges to reduce data
query.StartTime = time.Now().Add(-7 * 24 * time.Hour).Unix()
```

### Materialized View Refresh

```sql
-- Refresh during low-traffic periods
REFRESH MATERIALIZED VIEW CONCURRENTLY daily_event_stats;

-- Schedule with cron
-- 0 2 * * * psql -d grainlify -c "REFRESH MATERIALIZED VIEW CONCURRENTLY daily_event_stats;"
```

## Event Retention

### Retention Policy

| Event Type | Retention | Reason |
|-----------|-----------|--------|
| FundsLocked | 7 years | Financial/regulatory |
| FundsReleased | 7 years | Financial/regulatory |
| FundsRefunded | 7 years | Financial/regulatory |
| OperationMetric | 90 days | Operational |
| PerformanceMetric | 30 days | Performance |

### Cleanup

```go
// Run daily cleanup
func cleanupOldEvents(ctx context.Context, pool *pgxpool.Pool) error {
    _, err := pool.Exec(ctx, "SELECT cleanup_old_events(2555)") // 7 years
    return err
}
```

## Testing

### Unit Tests

```go
func TestEventIndexer_QueryEvents(t *testing.T) {
    indexer := events.NewEventIndexer(pool)
    
    query := events.EventQuery{
        EventTypes: []string{"FundsLocked"},
        Limit:      100,
    }
    
    events, err := indexer.QueryEvents(context.Background(), query)
    if err != nil {
        t.Fatalf("QueryEvents failed: %v", err)
    }
    
    if len(events) == 0 {
        t.Fatal("Expected events, got none")
    }
}

func TestEventMonitor_Anomaly(t *testing.T) {
    monitor := events.NewEventMonitor()
    
    alertReceived := false
    monitor.OnAlert(func(alert events.Alert) error {
        alertReceived = true
        return nil
    })
    
    event := events.ContractEvent{
        EventType: "FundsLocked",
        Data:      []byte(`{"amount": 5000000}`),
    }
    
    monitor.Emit(context.Background(), event)
    
    if !alertReceived {
        t.Fatal("Expected alert, got none")
    }
}
```

## Troubleshooting

### Issue: Slow Queries

**Solution:**
1. Check index usage: `EXPLAIN ANALYZE SELECT ...`
2. Rebuild indexes: `REINDEX INDEX idx_name`
3. Analyze table: `ANALYZE contract_events`
4. Increase work_mem: `SET work_mem = '256MB'`

### Issue: High Database Size

**Solution:**
1. Check event retention: `SELECT COUNT(*) FROM contract_events`
2. Run cleanup: `SELECT cleanup_old_events(2555)`
3. Vacuum table: `VACUUM ANALYZE contract_events`
4. Check for bloat: `SELECT * FROM pgstattuple('contract_events')`

### Issue: Missing Events

**Solution:**
1. Check unindexed events: `SELECT COUNT(*) FROM contract_events WHERE indexed = false`
2. Check event ingestion: Review logs for errors
3. Verify Soroban RPC connection
4. Check database connectivity

## Future Enhancements

- [ ] Implement Soroban RPC event retrieval
- [ ] Add event replay capability
- [ ] Create monitoring dashboard UI
- [ ] Add event correlation across contracts
- [ ] Implement event-driven state machine
- [ ] Add distributed tracing support
- [ ] Create event schema registry
- [ ] Add event versioning support
- [ ] Implement event compression
- [ ] Add event encryption for sensitive data

## References

- [Event Schema Documentation](../contracts/EVENT_SCHEMA.md)
- [Event Indexing Strategy](./EVENT_INDEXING_STRATEGY.md)
- [Event Versioning](../contracts/EVENT_VERSIONING.md)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Soroban Events](https://developers.stellar.org/learn/smart-contract-internals/events)
