-- Enrollments table

CREATE TABLE enrollments (
    id                      BIGSERIAL PRIMARY KEY,
    public_id               UUID NOT NULL DEFAULT gen_random_uuid(),
    device_id               BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    account_id              BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    enrolled_by             BIGINT NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    tier                    enrollment_tier NOT NULL,
    protection_config       JSONB NOT NULL DEFAULT '{}',
    reporting_config        JSONB NOT NULL DEFAULT '{}',
    unenrollment_policy     JSONB NOT NULL DEFAULT '{}',
    status                  enrollment_status NOT NULL DEFAULT 'active',
    expires_at              TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE enrollments ADD CONSTRAINT uq_enrollments_public_id UNIQUE (public_id);

CREATE UNIQUE INDEX uq_enrollments_device_active
    ON enrollments (device_id)
    WHERE status = 'active';

CREATE INDEX idx_enrollments_enrolled_by
    ON enrollments (enrolled_by);

CREATE INDEX idx_enrollments_tier_status
    ON enrollments (tier, status);

CREATE INDEX idx_enrollments_device_status
    ON enrollments (device_id, status);
