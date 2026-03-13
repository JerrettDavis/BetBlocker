-- Federated reports table

CREATE TABLE federated_reports (
    id                      BIGSERIAL PRIMARY KEY,
    device_id               BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    domain                  VARCHAR(500) NOT NULL,
    heuristic_match_type    VARCHAR(100),
    confidence              DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    reported_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    review_status           federated_report_status NOT NULL DEFAULT 'pending',
    reviewed_by_account_id  BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    resolved_to_entry_id    BIGINT REFERENCES blocklist_entries(id) ON DELETE SET NULL
);

CREATE INDEX idx_federated_reports_pending
    ON federated_reports (reported_at DESC)
    WHERE review_status = 'pending';

CREATE INDEX idx_federated_reports_domain
    ON federated_reports (domain);

CREATE INDEX idx_federated_reports_device
    ON federated_reports (device_id);
