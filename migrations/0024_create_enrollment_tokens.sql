-- Enrollment tokens table

CREATE TABLE enrollment_tokens (
    id                  BIGSERIAL PRIMARY KEY,
    public_id           UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    organization_id     BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    created_by          BIGINT NOT NULL REFERENCES accounts(id),
    label               VARCHAR(200),
    protection_config   JSONB NOT NULL,
    reporting_config    JSONB NOT NULL,
    unenrollment_policy JSONB NOT NULL,
    max_uses            INT,
    uses_count          INT NOT NULL DEFAULT 0,
    expires_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_enrollment_tokens_org ON enrollment_tokens (organization_id);
CREATE INDEX idx_enrollment_tokens_public_id ON enrollment_tokens (public_id);
