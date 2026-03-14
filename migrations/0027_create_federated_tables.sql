-- Federated report and aggregate tables for the discovery pipeline

CREATE TYPE federated_aggregate_status AS ENUM (
    'collecting',
    'threshold_met',
    'reviewing',
    'promoted',
    'rejected'
);

-- New federated reports table (v2) for the discovery pipeline.
-- The original federated_reports table (migration 0014) is device-centric;
-- this table captures anonymous reporter-token-based reports with batch tracking.
CREATE TABLE federated_reports_v2 (
    id                  BIGSERIAL PRIMARY KEY,
    domain              VARCHAR(500) NOT NULL,
    reporter_token      VARCHAR(500) NOT NULL,
    heuristic_score     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    category_guess      VARCHAR(200),
    reported_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    batch_id            UUID NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_federated_reports_v2_domain
    ON federated_reports_v2 (domain);

CREATE TABLE federated_aggregates (
    id                      BIGSERIAL PRIMARY KEY,
    domain                  VARCHAR(500) NOT NULL,
    unique_reporters        INTEGER NOT NULL DEFAULT 0,
    avg_heuristic_score     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    first_reported_at       TIMESTAMPTZ NOT NULL,
    last_reported_at        TIMESTAMPTZ NOT NULL,
    status                  federated_aggregate_status NOT NULL DEFAULT 'collecting',
    discovery_candidate_id  BIGINT REFERENCES discovery_candidates(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE federated_aggregates
    ADD CONSTRAINT uq_federated_aggregates_domain UNIQUE (domain);
