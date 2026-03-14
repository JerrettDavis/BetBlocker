-- Discovery candidates table for the intelligence pipeline

CREATE TYPE discovery_candidate_status AS ENUM (
    'pending',
    'approved',
    'rejected',
    'deferred'
);

CREATE TYPE crawler_source AS ENUM (
    'affiliate',
    'license_registry',
    'whois_pattern',
    'dns_zone',
    'search_engine',
    'federated'
);

CREATE TABLE discovery_candidates (
    id                  BIGSERIAL PRIMARY KEY,
    domain              VARCHAR(500) NOT NULL,
    source              crawler_source NOT NULL,
    source_metadata     JSONB NOT NULL DEFAULT '{}',
    confidence_score    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    classification      JSONB NOT NULL DEFAULT '{}',
    status              discovery_candidate_status NOT NULL DEFAULT 'pending',
    reviewed_by         BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    reviewed_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE discovery_candidates
    ADD CONSTRAINT uq_discovery_domain_source UNIQUE (domain, source);

CREATE INDEX idx_discovery_candidates_status
    ON discovery_candidates (status);

CREATE INDEX idx_discovery_candidates_domain
    ON discovery_candidates (domain);
