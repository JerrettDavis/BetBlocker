-- Audit log table

CREATE TABLE audit_log (
    id                  BIGSERIAL PRIMARY KEY,
    actor_account_id    BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    table_name          VARCHAR(100) NOT NULL,
    row_id              BIGINT NOT NULL,
    action              audit_action NOT NULL,
    old_values          JSONB,
    new_values          JSONB,
    client_ip           INET,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_table_row
    ON audit_log (table_name, row_id, created_at DESC);

CREATE INDEX idx_audit_log_actor
    ON audit_log (actor_account_id, created_at DESC)
    WHERE actor_account_id IS NOT NULL;

CREATE INDEX idx_audit_log_created
    ON audit_log (created_at DESC);
