-- Daily rollup over hourly block statistics (regular materialized view).
-- Uses standard materialized view since the source is not a hypertable.
CREATE MATERIALIZED VIEW IF NOT EXISTS daily_block_stats AS
SELECT
    date_trunc('day', bucket) AS day,
    device_id,
    event_type,
    SUM(event_count) AS event_count
FROM hourly_block_stats
GROUP BY day, device_id, event_type
WITH NO DATA;

CREATE INDEX IF NOT EXISTS idx_daily_block_stats_day
    ON daily_block_stats (day);
CREATE INDEX IF NOT EXISTS idx_daily_block_stats_device
    ON daily_block_stats (device_id, day);
