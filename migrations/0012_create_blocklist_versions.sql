-- Blocklist versions table

CREATE TABLE blocklist_versions (
    id                  BIGSERIAL PRIMARY KEY,
    version_number      BIGINT NOT NULL,
    published_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    entry_count         BIGINT NOT NULL DEFAULT 0,
    signature           BYTEA NOT NULL DEFAULT '',
    delta_binary        BYTEA,
    delta_metadata      JSONB NOT NULL DEFAULT '{}'
);

ALTER TABLE blocklist_versions
    ADD CONSTRAINT uq_blocklist_version_number UNIQUE (version_number);

CREATE INDEX idx_blocklist_versions_published
    ON blocklist_versions (published_at DESC);
