CREATE TABLE blocklist_versions (
    id                  BIGSERIAL PRIMARY KEY,
    version_number      BIGINT NOT NULL,
    entry_count         BIGINT NOT NULL DEFAULT 0,
    signature           BYTEA NOT NULL,
    delta_binary        BYTEA,
    delta_metadata      JSONB NOT NULL DEFAULT '{}',
    published_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE blocklist_versions
    ADD CONSTRAINT uq_blocklist_version_number UNIQUE (version_number);

-- Fast lookup of latest version
CREATE INDEX idx_blocklist_versions_published
    ON blocklist_versions (published_at DESC);

-- Add FK from blocklist_entries to blocklist_versions
ALTER TABLE blocklist_entries ADD CONSTRAINT fk_blocklist_entries_version_added
    FOREIGN KEY (blocklist_version_added) REFERENCES blocklist_versions(id) ON DELETE SET NULL;
ALTER TABLE blocklist_entries ADD CONSTRAINT fk_blocklist_entries_version_removed
    FOREIGN KEY (blocklist_version_removed) REFERENCES blocklist_versions(id) ON DELETE SET NULL;
