CREATE TABLE blocklist_entries (
    id                  BIGSERIAL PRIMARY KEY,
    public_id           UUID NOT NULL DEFAULT gen_random_uuid(),
    domain              VARCHAR(500) NOT NULL,
    pattern             VARCHAR(500),
    category            gambling_category NOT NULL DEFAULT 'other',
    source              blocklist_source NOT NULL DEFAULT 'curated',
    confidence          FLOAT NOT NULL DEFAULT 1.0
                        CONSTRAINT chk_confidence CHECK (confidence BETWEEN 0.0 AND 1.0),
    status              blocklist_entry_status NOT NULL DEFAULT 'active',
    added_by            BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    reviewed_by         BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    evidence_url        VARCHAR(1000),
    tags                TEXT[] NOT NULL DEFAULT '{}',
    blocklist_version_added   BIGINT,
    blocklist_version_removed BIGINT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE blocklist_entries ADD CONSTRAINT uq_blocklist_public_id UNIQUE (public_id);

-- Domain is unique -- no duplicates in the blocklist
ALTER TABLE blocklist_entries ADD CONSTRAINT uq_blocklist_domain UNIQUE (domain);

-- Trigram index for fuzzy domain search in admin panel
-- Requires pg_trgm extension
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX idx_blocklist_domain_trgm
    ON blocklist_entries USING gin (domain gin_trgm_ops);

-- Category filter
CREATE INDEX idx_blocklist_category ON blocklist_entries (category)
    WHERE status = 'active';

-- Review queue: pending entries sorted by recency
CREATE INDEX idx_blocklist_review_queue
    ON blocklist_entries (created_at DESC)
    WHERE status = 'pending_review';

-- Source breakdown for analytics
CREATE INDEX idx_blocklist_source ON blocklist_entries (source);
