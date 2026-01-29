-- Drop functions
DROP FUNCTION IF EXISTS get_events_by_type_and_time(VARCHAR, BIGINT, BIGINT, INTEGER);
DROP FUNCTION IF EXISTS get_event_statistics(BIGINT, BIGINT);
DROP FUNCTION IF EXISTS cleanup_old_events(INTEGER);
DROP FUNCTION IF EXISTS refresh_daily_event_stats();
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop triggers
DROP TRIGGER IF EXISTS update_event_replay_log_updated_at ON event_replay_log;
DROP TRIGGER IF EXISTS update_contract_events_updated_at ON contract_events;

-- Drop materialized views
DROP MATERIALIZED VIEW IF EXISTS daily_event_stats;

-- Drop tables
DROP TABLE IF EXISTS event_replay_log;
DROP TABLE IF EXISTS event_metrics;
DROP TABLE IF EXISTS event_alerts;
DROP TABLE IF EXISTS contract_events;
