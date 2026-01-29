# Event Indexing Strategy Guide

## Overview

This guide provides comprehensive strategies for efficiently indexing and querying contract events in Grainlify. The indexing infrastructure enables real-time monitoring, analytics, and audit trails for all on-chain operations.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Event Flow Architecture                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Soroban Contracts                                          │
│  ├─ Bounty Escrow                                           │
│  ├─ Program Escrow                                          │
│  └─ Grainlify Core                                          │
│         ↓                                                    │
│  Event Emission (env.events().publish())                    │
│         ↓                                                    │
│  Soroban RPC Event Retrieval                                │
│         ↓                                                    │
│  Backend Event Ingestion                                    │
│         ↓                                                    │
│  PostgreSQL contract_events Table                           │
│         ↓                                                    │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Indexing Layer                                       │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ • Time-Series Indexes                               │  │
│  │ • Entity-Based Indexes                              │  │
│  │ • Composite Indexes                                 │  │
│  │ • JSONB Indexes                                     │  │
│  │ • Materialized Views                                │  │
│  └──────────────────────────────────────────────────────┘  │
│         ↓                                                    │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Query Layer                                          │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ • EventIndexer (Go)                                 │  │
│  │ • EventMonitor (Go)                                 │  │
│  │ • EventFilter (Go)                                  │  │
│  │ • EventAggregator (Go)                              │  │
│  └──────────────────────────────────────────────────────┘  │
│         ↓                                                    │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ API Layer                                            │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ • Event Query Endpoints                             │  │
│  │ • Event Statistics Endpoints                        │  │
│  │ • Alert Management Endpoints                        │  │
│  │ • Monitoring Dashboard                              │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Database Schema

### contract_events Table

Primary table for storing all contract events:

```sql
CREATE TABLE contract_events (
    id UUID PRIMARY KEY,
    contract_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    version INTEGER NOT NULL,
    correlation_id VARCHAR(255),
    timestamp BIGINT NOT NULL,
    data JSONB NOT NULL,
    indexed BOOLEAN NOT NULL,
    indexed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

**Key Fields:**
- `id`: Unique event identifier (UUID)
- `contract_id`: Address of the contract that emitted the event
- `event_type`: Type of event (e.g., "FundsLocked", "FundsReleased")
- `version`: Schema version for backward compatibility
- `correlation_id`: Trace ID for correlating related events
- `timestamp`: Unix timestamp (seconds) of event emission
- `data`: JSONB containing event-specific data
- `indexed`: Flag for background indexing status
- `indexed_at`: Timestamp when event was indexed

### Supporting Tables

**event_alerts**: Stores monitoring alerts
```sql
CREATE TABLE event_alerts (
    id UUID PRIMARY KEY,
    alert_id VARCHAR(255) UNIQUE,
    severity VARCHAR(50),
    message TEXT,
    event_type VARCHAR(100),
    event_id UUID REFERENCES contract_events(id),
    data JSONB,
    acknowledged BOOLEAN,
    acknowledged_at TIMESTAMP,
    acknowledged_by VARCHAR(255),
    created_at TIMESTAMP
);
```

**event_metrics**: Stores performance metrics
```sql
CREATE TABLE event_metrics (
    id UUID PRIMARY KEY,
    event_type VARCHAR(100),
    contract_id VARCHAR(255),
    operation_name VARCHAR(255),
    duration_ms BIGINT,
    success BOOLEAN,
    error_message TEXT,
    timestamp BIGINT,
    created_at TIMESTAMP
);
```

**event_replay_log**: Tracks event replay attempts
```sql
CREATE TABLE event_replay_log (
    id UUID PRIMARY KEY,
    event_id UUID REFERENCES contract_events(id),
    replay_count INTEGER,
    last_replayed_at TIMESTAMP,
    status VARCHAR(50),
    error_message TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);
```

## Indexing Strategies

### 1. Time-Series Indexing

**Purpose:** Efficiently query events over time ranges

**Index Definition:**
```sql
CREATE INDEX idx_contract_events_type_timestamp 
ON contract_events(event_type, timestamp DESC);
```

**Use Cases:**
- Get all events of a type in a time range
- Real-time event streaming
- Historical event retrieval

**Query Example:**
```go
// Get all FundsLocked events in the last 24 hours
query := EventQuery{
    EventTypes: []string{"FundsLocked"},
    StartTime:  time.Now().Add(-24 * time.Hour).Unix(),
    EndTime:    time.Now().Unix(),
    OrderBy:    "timestamp",
    Order:      "DESC",
    Limit:      1000,
}
events, err := indexer.QueryEvents(ctx, query)
```

**Performance:**
- Index size: ~100 bytes per event
- Query time: O(log n) for range queries
- Typical query: <100ms for 1M events

### 2. Entity-Based Indexing

**Purpose:** Efficiently query events by entity (bounty, program, recipient)

**Index Definition:**
```sql
CREATE INDEX idx_contract_events_contract_id 
ON contract_events(contract_id, timestamp DESC);
```

**Use Cases:**
- Get all events for a specific contract
- Contract-specific monitoring
- Contract lifecycle tracking

**Query Example:**
```go
// Get all events for a specific contract
query := EventQuery{
    ContractID: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
    OrderBy:    "timestamp",
    Order:      "DESC",
    Limit:      1000,
}
events, err := indexer.QueryEvents(ctx, query)
```

**Performance:**
- Index size: ~100 bytes per event
- Query time: O(log n)
- Typical query: <50ms for 1M events

### 3. Composite Indexing

**Purpose:** Efficient multi-field queries

**Index Definition:**
```sql
CREATE INDEX idx_contract_events_type_contract_timestamp 
ON contract_events(event_type, contract_id, timestamp DESC);
```

**Use Cases:**
- Get specific event types for a contract
- Filtered event retrieval
- Complex queries

**Query Example:**
```go
// Get all FundsReleased events for a contract
query := EventQuery{
    EventTypes: []string{"FundsReleased"},
    ContractID: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
    OrderBy:    "timestamp",
    Order:      "DESC",
}
events, err := indexer.QueryEvents(ctx, query)
```

**Performance:**
- Index size: ~150 bytes per event
- Query time: O(log n)
- Typical query: <30ms for 1M events

### 4. JSONB Indexing

**Purpose:** Efficient queries on event data

**Index Definition:**
```sql
CREATE INDEX idx_contract_events_data 
ON contract_events USING GIN (data);
```

**Use Cases:**
- Query events by specific data fields
- Amount-based filtering
- Complex data queries

**Query Example:**
```sql
-- Find all events with amount > 1000
SELECT * FROM contract_events 
WHERE data->>'amount'::numeric > 1000
ORDER BY timestamp DESC;
```

**Performance:**
- Index size: ~500 bytes per event (larger due to JSONB)
- Query time: O(log n)
- Typical query: <100ms for 1M events

### 5. Materialized Views

**Purpose:** Pre-computed aggregations for fast reporting

**View Definition:**
```sql
CREATE MATERIALIZED VIEW daily_event_stats AS
SELECT 
    DATE(to_timestamp(timestamp)) as date,
    event_type,
    COUNT(*) as event_count,
    COUNT(DISTINCT contract_id) as unique_contracts
FROM contract_events
GROUP BY DATE(to_timestamp(timestamp)), event_type;
```

**Use Cases:**
- Daily statistics
- Trend analysis
- Dashboard reporting

**Refresh Strategy:**
```sql
-- Refresh daily at 2 AM UTC
REFRESH MATERIALIZED VIEW CONCURRENTLY daily_event_stats;
```

**Query Example:**
```go
// Get daily statistics
rows, err := pool.Query(ctx, `
    SELECT date, event_type, event_count, unique_contracts
    FROM daily_event_stats
    WHERE date >= CURRENT_DATE - INTERVAL '30 days'
    ORDER BY date DESC
`)
```

**Performance:**
- Query time: <10ms (pre-computed)
- Refresh time: ~1-5 seconds (depends on data volume)

## Query Patterns

### Pattern 1: Recent Events

```go
// Get recent events of a specific type
query := EventQuery{
    EventTypes: []string{"FundsLocked"},
    StartTime:  time.Now().Add(-1 * time.Hour).Unix(),
    OrderBy:    "timestamp",
    Order:      "DESC",
    Limit:      100,
}
events, err := indexer.QueryEvents(ctx, query)
```

### Pattern 2: Entity History

```go
// Get complete history for an entity
query := EventQuery{
    ContractID: contractID,
    OrderBy:    "timestamp",
    Order:      "ASC",
    Limit:      10000,
}
events, err := indexer.QueryEvents(ctx, query)
```

### Pattern 3: Aggregation

```go
// Aggregate events by type
aggQuery := AggregateQuery{
    EventTypes: []string{"FundsLocked", "FundsReleased"},
    GroupBy:    "event_type",
    Aggregate:  "COUNT",
    Field:      "id",
}
results, err := indexer.Aggregate(ctx, aggQuery)
```

### Pattern 4: Time-Series Analysis

```go
// Get events for time-series analysis
query := EventQuery{
    EventTypes: []string{"PerformanceMetric"},
    StartTime:  time.Now().Add(-7 * 24 * time.Hour).Unix(),
    EndTime:    time.Now().Unix(),
    OrderBy:    "timestamp",
    Order:      "ASC",
    Limit:      100000,
}
events, err := indexer.QueryEvents(ctx, query)
```

## Monitoring Hooks

### Hook 1: Large Transaction Alert

```go
monitor.On("FundsLocked", func(event ContractEvent) error {
    var data map[string]interface{}
    json.Unmarshal(event.Data, &data)
    
    amount := data["amount"].(float64)
    if amount > 1000000 { // 1M stroops
        alert := Alert{
            Severity: "INFO",
            Message:  fmt.Sprintf("Large transaction: %.0f stroops", amount),
            EventID:  event.ID,
        }
        // Handle alert
    }
    return nil
})
```

### Hook 2: Operation Failure Alert

```go
monitor.On("OperationMetric", func(event ContractEvent) error {
    var data map[string]interface{}
    json.Unmarshal(event.Data, &data)
    
    if !data["success"].(bool) {
        alert := Alert{
            Severity: "WARNING",
            Message:  fmt.Sprintf("Operation failed: %s", data["operation"]),
            EventID:  event.ID,
        }
        // Handle alert
    }
    return nil
})
```

### Hook 3: Performance SLA Violation

```go
monitor.On("PerformanceMetric", func(event ContractEvent) error {
    var data map[string]interface{}
    json.Unmarshal(event.Data, &data)
    
    duration := data["duration_ms"].(float64)
    operation := data["operation"].(string)
    
    slaThresholds := map[string]float64{
        "lock_funds":    1000,
        "release_funds": 1000,
        "batch_payout":  5000,
    }
    
    if duration > slaThresholds[operation] {
        alert := Alert{
            Severity: "WARNING",
            Message:  fmt.Sprintf("SLA violation: %s", operation),
            EventID:  event.ID,
        }
        // Handle alert
    }
    return nil
})
```

## Event Filtering Examples

### Filter by Amount Range

```go
filter := EventFilter{
    EventTypes: []string{"FundsLocked", "FundsReleased"},
    MinAmount:  100000,
    MaxAmount:  1000000,
    StartTime:  time.Now().Add(-7 * 24 * time.Hour).Unix(),
    EndTime:    time.Now().Unix(),
}

for _, event := range events {
    if filter.Matches(event) {
        // Process event
    }
}
```

### Filter by Correlation ID

```go
filter := EventFilter{
    CorrelationID: "trace-123",
}

for _, event := range events {
    if filter.Matches(event) {
        // Process related event
    }
}
```

### Filter by Time Window

```go
filter := EventFilter{
    EventTypes: []string{"OperationMetric"},
    StartTime:  time.Now().Add(-24 * time.Hour).Unix(),
    EndTime:    time.Now().Unix(),
}

for _, event := range events {
    if filter.Matches(event) {
        // Process event
    }
}
```

## Performance Optimization

### 1. Index Maintenance

```sql
-- Analyze table for query planner
ANALYZE contract_events;

-- Reindex if fragmented
REINDEX INDEX idx_contract_events_type_timestamp;

-- Check index size
SELECT 
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_indexes
WHERE tablename = 'contract_events'
ORDER BY pg_relation_size(indexrelid) DESC;
```

### 2. Query Optimization

```go
// Use pagination for large result sets
query := EventQuery{
    EventTypes: []string{"FundsLocked"},
    Limit:      1000,
    Offset:     0,
}

// Process in batches
for offset := 0; offset < totalEvents; offset += 1000 {
    query.Offset = offset
    events, err := indexer.QueryEvents(ctx, query)
    // Process batch
}
```

### 3. Materialized View Refresh

```sql
-- Refresh during low-traffic periods
REFRESH MATERIALIZED VIEW CONCURRENTLY daily_event_stats;

-- Schedule with cron
-- 0 2 * * * psql -d grainlify -c "REFRESH MATERIALIZED VIEW CONCURRENTLY daily_event_stats;"
```

## Event Retention Policy

| Event Type | Retention | Reason |
|-----------|-----------|--------|
| FundsLocked | 7 years | Financial/regulatory |
| FundsReleased | 7 years | Financial/regulatory |
| FundsRefunded | 7 years | Financial/regulatory |
| OperationMetric | 90 days | Operational |
| PerformanceMetric | 30 days | Performance |

**Cleanup Function:**
```go
// Run daily cleanup
func cleanupOldEvents(ctx context.Context, pool *pgxpool.Pool) error {
    _, err := pool.Exec(ctx, "SELECT cleanup_old_events(2555)") // 7 years
    return err
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

## Implementation Checklist

- [x] Create contract_events table
- [x] Create supporting tables (alerts, metrics, replay_log)
- [x] Create indexes (time-series, entity, composite, JSONB)
- [x] Create materialized views
- [x] Implement EventIndexer (Go)
- [x] Implement EventMonitor (Go)
- [x] Implement EventFilter (Go)
- [x] Implement EventAggregator (Go)
- [ ] Implement Soroban RPC event retrieval
- [ ] Create event query API endpoints
- [ ] Create event statistics API endpoints
- [ ] Create alert management API endpoints
- [ ] Create monitoring dashboard
- [ ] Add event replay capability
- [ ] Add correlation ID generation
- [ ] Create event documentation API

## References

- [PostgreSQL Indexing](https://www.postgresql.org/docs/current/indexes.html)
- [JSONB Performance](https://www.postgresql.org/docs/current/datatype-json.html)
- [Materialized Views](https://www.postgresql.org/docs/current/rules-materializedviews.html)
- [Time-Series Best Practices](https://www.timescale.com/blog/what-is-a-time-series-database/)
