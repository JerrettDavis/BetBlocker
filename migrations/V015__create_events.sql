-- Events table (partitioned by month)
-- This is the highest-volume table. At scale, expect millions of rows
-- per day (heartbeats alone: 1 per device per 5 minutes).

CREATE TABLE events (
    id              BIGSERIAL,
    public_id       UUID NOT NULL DEFAULT gen_random_uuid(),
    device_id       BIGINT NOT NULL,
    enrollment_id   BIGINT,
    event_type      event_type NOT NULL,
    category        event_category NOT NULL,
    severity        event_severity NOT NULL DEFAULT 'info',
    metadata        JSONB NOT NULL DEFAULT '{}',
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Composite primary key required for partitioned tables
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

-- Create initial partitions for 2026
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

-- Indexes on partitioned table (automatically created on each partition)

-- Dashboard: events for a specific device, recent first
CREATE INDEX idx_events_device_occurred
    ON events (device_id, occurred_at DESC);

-- Dashboard: events for a specific enrollment, recent first
CREATE INDEX idx_events_enrollment_occurred
    ON events (enrollment_id, occurred_at DESC)
    WHERE enrollment_id IS NOT NULL;

-- Filtering by event type within a time range
CREATE INDEX idx_events_type_occurred
    ON events (event_type, occurred_at DESC);

-- Tamper and bypass alerts (high priority events for real-time alerting)
CREATE INDEX idx_events_alerts
    ON events (device_id, occurred_at DESC)
    WHERE event_type IN ('bypass_attempt', 'tamper_detected', 'vpn_detected');
