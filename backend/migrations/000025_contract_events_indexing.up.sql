-- Create contract_events table for comprehensive event indexing
CREATE TABLE IF NOT EXISTS contract_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    correlation_id VARCHAR(255),
    timestamp BIGINT NOT NULL,
    data JSONB NOT NULL,
    indexed BOOLEAN NOT NULL DEFAULT false,
    indexed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
-- Time-series index for event type and timestamp
CREATE INDEX idx_contract_events_type_timestamp 
ON contract_events(event_type, timestamp DESC);

-- Entity-based index for contract_id
CREATE INDEX idx_contract_events_contract_id 
ON contract_events(contract_id, timestamp DESC);

-- Correlation ID index for tracing related events
CREATE INDEX idx_contract_events_correlation_id 
ON contract_events(correlation_id) 
WHERE correlation_id IS NOT NULL;

-- Index for unindexed events (for background processing)
CREATE INDEX idx_contract_events_unindexed 
ON contract_events(timestamp ASC) 
WHERE indexed = false;

-- Composite index for common queries
CREATE INDEX idx_contract_events_type_contract_timestamp 
ON contract_events(event_type, contract_id, timestamp DESC);

-- JSONB index for efficient data queries
CREATE INDEX idx_contract_events_data 
ON contract_events USING GIN (data);

-- Create materialized view for daily event statistics
CREATE MATERIALIZED VIEW IF NOT EXISTS daily_event_stats AS
SELECT 
    DATE(to_timestamp(timestamp)) as date,
    event_type,
    COUNT(*) as event_count,
    COUNT(DISTINCT contract_id) as unique_contracts,
    COUNT(DISTINCT correlation_id) as unique_correlations
FROM contract_events
GROUP BY DATE(to_timestamp(timestamp)), event_type;

-- Create index on materialized view
CREATE INDEX idx_daily_event_stats_date_type 
ON daily_event_stats(date DESC, event_type);

-- Create event_alerts table for monitoring alerts
CREATE TABLE IF NOT EXISTS event_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_id VARCHAR(255) NOT NULL UNIQUE,
    severity VARCHAR(50) NOT NULL,
    message TEXT NOT NULL,
    event_type VARCHAR(100),
    event_id UUID REFERENCES contract_events(id) ON DELETE CASCADE,
    data JSONB,
    acknowledged BOOLEAN NOT NULL DEFAULT false,
    acknowledged_at TIMESTAMP,
    acknowledged_by VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create indexes for alerts
CREATE INDEX idx_event_alerts_severity_created 
ON event_alerts(severity, created_at DESC);

CREATE INDEX idx_event_alerts_event_type 
ON event_alerts(event_type, created_at DESC);

CREATE INDEX idx_event_alerts_acknowledged 
ON event_alerts(acknowledged, created_at DESC);

-- Create event_metrics table for performance tracking
CREATE TABLE IF NOT EXISTS event_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type VARCHAR(100) NOT NULL,
    contract_id VARCHAR(255),
    operation_name VARCHAR(255),
    duration_ms BIGINT,
    success BOOLEAN,
    error_message TEXT,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create indexes for metrics
CREATE INDEX idx_event_metrics_type_timestamp 
ON event_metrics(event_type, timestamp DESC);

CREATE INDEX idx_event_metrics_operation_timestamp 
ON event_metrics(operation_name, timestamp DESC);

CREATE INDEX idx_event_metrics_success 
ON event_metrics(success, timestamp DESC);

-- Create event_replay_log table for event replay capability
CREATE TABLE IF NOT EXISTS event_replay_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES contract_events(id) ON DELETE CASCADE,
    replay_count INTEGER NOT NULL DEFAULT 0,
    last_replayed_at TIMESTAMP,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create index for replay log
CREATE INDEX idx_event_replay_log_status 
ON event_replay_log(status, created_at DESC);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for contract_events
CREATE TRIGGER update_contract_events_updated_at
BEFORE UPDATE ON contract_events
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

-- Create trigger for event_replay_log
CREATE TRIGGER update_event_replay_log_updated_at
BEFORE UPDATE ON event_replay_log
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

-- Create function to refresh daily statistics
CREATE OR REPLACE FUNCTION refresh_daily_event_stats()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY daily_event_stats;
END;
$$ LANGUAGE plpgsql;

-- Create function to clean up old events (retention policy)
CREATE OR REPLACE FUNCTION cleanup_old_events(retention_days INTEGER DEFAULT 2555)
RETURNS TABLE(deleted_count BIGINT) AS $$
DECLARE
    cutoff_timestamp BIGINT;
    deleted BIGINT;
BEGIN
    cutoff_timestamp := EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day' * retention_days)::BIGINT;
    
    DELETE FROM event_alerts 
    WHERE event_id IN (
        SELECT id FROM contract_events 
        WHERE timestamp < cutoff_timestamp
    );
    
    DELETE FROM event_replay_log 
    WHERE event_id IN (
        SELECT id FROM contract_events 
        WHERE timestamp < cutoff_timestamp
    );
    
    DELETE FROM contract_events 
    WHERE timestamp < cutoff_timestamp;
    
    GET DIAGNOSTICS deleted = ROW_COUNT;
    
    RETURN QUERY SELECT deleted;
END;
$$ LANGUAGE plpgsql;

-- Create function to get event statistics
CREATE OR REPLACE FUNCTION get_event_statistics(
    start_time BIGINT DEFAULT 0,
    end_time BIGINT DEFAULT 0
)
RETURNS TABLE(
    total_events BIGINT,
    unique_contracts BIGINT,
    unique_event_types BIGINT,
    oldest_timestamp BIGINT,
    newest_timestamp BIGINT
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        COUNT(*)::BIGINT,
        COUNT(DISTINCT contract_id)::BIGINT,
        COUNT(DISTINCT event_type)::BIGINT,
        MIN(timestamp)::BIGINT,
        MAX(timestamp)::BIGINT
    FROM contract_events
    WHERE (start_time = 0 OR timestamp >= start_time)
    AND (end_time = 0 OR timestamp <= end_time);
END;
$$ LANGUAGE plpgsql;

-- Create function to get events by type and time range
CREATE OR REPLACE FUNCTION get_events_by_type_and_time(
    p_event_type VARCHAR,
    p_start_time BIGINT,
    p_end_time BIGINT,
    p_limit INTEGER DEFAULT 1000
)
RETURNS TABLE(
    id UUID,
    contract_id VARCHAR,
    event_type VARCHAR,
    version INTEGER,
    correlation_id VARCHAR,
    timestamp BIGINT,
    data JSONB
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        ce.id,
        ce.contract_id,
        ce.event_type,
        ce.version,
        ce.correlation_id,
        ce.timestamp,
        ce.data
    FROM contract_events ce
    WHERE ce.event_type = p_event_type
    AND ce.timestamp >= p_start_time
    AND ce.timestamp <= p_end_time
    ORDER BY ce.timestamp DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Add comments for documentation
COMMENT ON TABLE contract_events IS 'Stores all contract events for comprehensive indexing and monitoring';
COMMENT ON TABLE event_alerts IS 'Stores monitoring alerts generated from contract events';
COMMENT ON TABLE event_metrics IS 'Stores performance metrics for contract operations';
COMMENT ON TABLE event_replay_log IS 'Tracks event replay attempts for recovery and debugging';

COMMENT ON COLUMN contract_events.correlation_id IS 'Unique identifier for tracing related events across contracts';
COMMENT ON COLUMN contract_events.indexed IS 'Flag indicating if event has been processed by indexers';
COMMENT ON COLUMN event_alerts.acknowledged IS 'Flag indicating if alert has been reviewed';
COMMENT ON COLUMN event_metrics.duration_ms IS 'Operation duration in milliseconds';
