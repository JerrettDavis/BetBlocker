-- Continuous aggregate: hourly block statistics
CREATE MATERIALIZED VIEW hourly_block_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', created_at) AS bucket,
    device_id,
    event_type::text AS event_type,
    COUNT(*) AS event_count
FROM events
GROUP BY bucket, device_id, event_type
WITH NO DATA;

-- Refresh policy: materialize data older than 3 hours, refresh every 1 hour
SELECT add_continuous_aggregate_policy('hourly_block_stats',
    start_offset    => INTERVAL '3 hours',
    end_offset      => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour',
    if_not_exists   => true
);
