CREATE TABLE enrollments (
    id                      BIGSERIAL PRIMARY KEY,
    public_id               UUID NOT NULL DEFAULT gen_random_uuid(),
    device_id               BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    account_id              BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    enrolled_by             BIGINT NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    tier                    enrollment_tier NOT NULL,
    status                  enrollment_status NOT NULL DEFAULT 'active',
    protection_config       JSONB NOT NULL DEFAULT '{}',
    reporting_config        JSONB NOT NULL DEFAULT '{}',
    unenrollment_policy     JSONB NOT NULL DEFAULT '{}',
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at              TIMESTAMPTZ
);

ALTER TABLE enrollments ADD CONSTRAINT uq_enrollments_public_id UNIQUE (public_id);

-- A device should have at most one active enrollment
CREATE UNIQUE INDEX uq_enrollments_device_active
    ON enrollments (device_id)
    WHERE status = 'active';

-- Find enrollments managed by a given authority
CREATE INDEX idx_enrollments_enrolled_by
    ON enrollments (enrolled_by);

-- Dashboard queries: active enrollments by tier
CREATE INDEX idx_enrollments_tier_status
    ON enrollments (tier, status);

-- Device lookup for active enrollment
CREATE INDEX idx_enrollments_device_status
    ON enrollments (device_id, status);

-- Add the FK from devices to enrollments now that the table exists
ALTER TABLE devices ADD CONSTRAINT fk_devices_enrollment
    FOREIGN KEY (enrollment_id) REFERENCES enrollments(id) ON DELETE SET NULL;
