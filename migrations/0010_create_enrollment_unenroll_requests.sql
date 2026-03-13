-- Enrollment unenroll requests table

CREATE TABLE enrollment_unenroll_requests (
    id                          BIGSERIAL PRIMARY KEY,
    enrollment_id               BIGINT NOT NULL REFERENCES enrollments(id) ON DELETE CASCADE,
    requested_by_account_id     BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    requested_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    required_approver_account_id BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    approved_by_account_id      BIGINT REFERENCES accounts(id) ON DELETE SET NULL,
    approved_at                 TIMESTAMPTZ,
    delay_until                 TIMESTAMPTZ,
    status                      unenroll_request_status NOT NULL DEFAULT 'pending'
);

CREATE UNIQUE INDEX uq_unenroll_request_pending
    ON enrollment_unenroll_requests (enrollment_id)
    WHERE status = 'pending';

CREATE INDEX idx_unenroll_requests_approver
    ON enrollment_unenroll_requests (required_approver_account_id)
    WHERE status = 'pending';

CREATE INDEX idx_unenroll_requests_delay
    ON enrollment_unenroll_requests (delay_until)
    WHERE status = 'pending' AND delay_until IS NOT NULL;
