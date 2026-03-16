-- Hourly block statistics (regular materialized view).
-- The events table uses native PARTITION BY RANGE, not a TimescaleDB hypertable,
-- so we use a standard materialized view instead of a continuous aggregate.
CREATE MATERIALIZED VIEW IF NOT EXISTS hourly_block_stats AS
SELECT
    date_trunc('hour', created_at) AS bucket,
    device_id,
    event_type::text AS event_type,
    COUNT(*) AS event_count
FROM events
GROUP BY bucket, device_id, event_type
WITH NO DATA;

CREATE INDEX IF NOT EXISTS idx_hourly_block_stats_bucket
    ON hourly_block_stats (bucket);
CREATE INDEX IF NOT EXISTS idx_hourly_block_stats_device
    ON hourly_block_stats (device_id, bucket);
