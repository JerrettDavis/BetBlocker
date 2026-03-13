-- Blocklist version entries (mapping table)

CREATE TABLE blocklist_version_entries (
    blocklist_version_id BIGINT NOT NULL REFERENCES blocklist_versions(id) ON DELETE CASCADE,
    blocklist_entry_id   BIGINT NOT NULL REFERENCES blocklist_entries(id) ON DELETE CASCADE,
    PRIMARY KEY (blocklist_version_id, blocklist_entry_id)
);

CREATE INDEX idx_bve_entry ON blocklist_version_entries (blocklist_entry_id);
