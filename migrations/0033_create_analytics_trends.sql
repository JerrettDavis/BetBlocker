-- Analytics trends table for pre-computed trend metrics
CREATE TABLE analytics_trends (
    id              BIGSERIAL PRIMARY KEY,
    device_id       BIGINT NOT NULL,
    metric_name     VARCHAR(255) NOT NULL,
    metric_value    JSONB NOT NULL DEFAULT '{}',
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    period_start    TIMESTAMPTZ NOT NULL,
    period_end      TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_analytics_trends_device_metric
    ON analytics_trends (device_id, metric_name, computed_at DESC);
