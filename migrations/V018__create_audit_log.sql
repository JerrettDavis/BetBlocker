-- Audit log: captures all mutations to security-critical tables.
-- This table is append-only -- no UPDATE or DELETE is permitted
-- at the application level.

CREATE TABLE audit_log (
    id                  BIGSERIAL PRIMARY KEY,
    table_name          VARCHAR(100) NOT NULL,
    record_id           BIGINT NOT NULL,
    action              VARCHAR(10) NOT NULL,
    old_values          JSONB,
    new_values          JSONB,
    actor_id            BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    client_ip           INET,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Query audit log for a specific table/row (compliance review)
CREATE INDEX idx_audit_log_table_row
    ON audit_log (table_name, record_id, created_at DESC);

-- Query audit log by actor (who did what)
CREATE INDEX idx_audit_log_actor
    ON audit_log (actor_id, created_at DESC)
    WHERE actor_id IS NOT NULL;

-- Time range scans for export
CREATE INDEX idx_audit_log_created
    ON audit_log (created_at DESC);
