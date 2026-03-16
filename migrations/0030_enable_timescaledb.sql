-- Enable TimescaleDB extension.
-- The events table already uses native PARTITION BY RANGE (migration 0015),
-- so we only enable the extension for its aggregate/compression functions
-- without converting to a hypertable.
CREATE EXTENSION IF NOT EXISTS timescaledb;
