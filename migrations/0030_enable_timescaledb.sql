-- Enable TimescaleDB extension and convert events to a hypertable
CREATE EXTENSION IF NOT EXISTS timescaledb;

SELECT create_hypertable('events', 'created_at', migrate_data => true, if_not_exists => true);
