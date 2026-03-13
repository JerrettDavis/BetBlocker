-- Events table (partitioned by month)

CREATE TABLE events (
    id              BIGSERIAL,
    device_id       BIGINT NOT NULL,
    enrollment_id   BIGINT,
    event_type      event_type NOT NULL,
    metadata        JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Create monthly partitions for 2026
CREATE TABLE events_y2026m01 PARTITION OF events
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE events_y2026m02 PARTITION OF events
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE events_y2026m03 PARTITION OF events
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE events_y2026m04 PARTITION OF events
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE events_y2026m05 PARTITION OF events
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE events_y2026m06 PARTITION OF events
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE events_y2026m07 PARTITION OF events
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE events_y2026m08 PARTITION OF events
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE events_y2026m09 PARTITION OF events
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE events_y2026m10 PARTITION OF events
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE events_y2026m11 PARTITION OF events
    FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE events_y2026m12 PARTITION OF events
    FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');

-- Indexes on partitioned table
CREATE INDEX idx_events_device_created
    ON events (device_id, created_at DESC);

CREATE INDEX idx_events_enrollment_created
    ON events (enrollment_id, created_at DESC)
    WHERE enrollment_id IS NOT NULL;

CREATE INDEX idx_events_type_created
    ON events (event_type, created_at DESC);

CREATE INDEX idx_events_alerts
    ON events (device_id, created_at DESC)
    WHERE event_type IN ('bypass_attempt', 'tamper', 'vpn_detected', 'extension_removed');
