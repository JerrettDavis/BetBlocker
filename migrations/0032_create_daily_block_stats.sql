-- Daily rollup over hourly block statistics
CREATE MATERIALIZED VIEW daily_block_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', bucket) AS day,
    device_id,
    event_type,
    SUM(event_count) AS event_count
FROM hourly_block_stats
GROUP BY day, device_id, event_type
WITH NO DATA;

-- Refresh policy: materialize data older than 1 day, refresh every 1 day
SELECT add_continuous_aggregate_policy('daily_block_stats',
    start_offset    => INTERVAL '3 days',
    end_offset      => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day',
    if_not_exists   => true
);
