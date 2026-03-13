-- Blocklist entries table

CREATE TABLE blocklist_entries (
    id                          BIGSERIAL PRIMARY KEY,
    public_id                   UUID NOT NULL DEFAULT gen_random_uuid(),
    domain                      VARCHAR(500) NOT NULL,
    pattern                     VARCHAR(500),
    category                    VARCHAR(100),
    source                      blocklist_source NOT NULL DEFAULT 'curated',
    confidence                  DOUBLE PRECISION NOT NULL DEFAULT 100.0,
    added_by                    BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    reviewed_by                 BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    evidence_url                VARCHAR(1000),
    tags                        TEXT[] NOT NULL DEFAULT '{}',
    blocklist_version_added     BIGINT,
    blocklist_version_removed   BIGINT,
    status                      blocklist_entry_status NOT NULL DEFAULT 'active',
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE blocklist_entries ADD CONSTRAINT uq_blocklist_public_id UNIQUE (public_id);
ALTER TABLE blocklist_entries ADD CONSTRAINT uq_blocklist_domain UNIQUE (domain);

CREATE INDEX idx_blocklist_domain_trgm
    ON blocklist_entries USING gin (domain gin_trgm_ops);

CREATE INDEX idx_blocklist_category ON blocklist_entries (category)
    WHERE status = 'active';

CREATE INDEX idx_blocklist_review_queue
    ON blocklist_entries (created_at DESC)
    WHERE status = 'review';

CREATE INDEX idx_blocklist_source ON blocklist_entries (source);
