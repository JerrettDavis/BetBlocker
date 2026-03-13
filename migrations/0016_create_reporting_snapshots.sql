-- Reporting snapshots table

CREATE TABLE reporting_snapshots (
    id              BIGSERIAL PRIMARY KEY,
    enrollment_id   BIGINT NOT NULL REFERENCES enrollments(id) ON DELETE CASCADE,
    snapshot_date   DATE NOT NULL,
    summary         JSONB NOT NULL DEFAULT '{}',
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uq_reporting_snapshot
    ON reporting_snapshots (enrollment_id, snapshot_date);

CREATE INDEX idx_reporting_snapshots_date
    ON reporting_snapshots (snapshot_date DESC);
