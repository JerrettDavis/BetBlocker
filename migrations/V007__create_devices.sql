CREATE TABLE devices (
    id              BIGSERIAL PRIMARY KEY,
    public_id       UUID NOT NULL DEFAULT gen_random_uuid(),
    account_id      BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name            VARCHAR(255),
    platform        platform_type NOT NULL,
    os_version      VARCHAR(50),
    agent_version   VARCHAR(50),
    hostname        VARCHAR(255),
    hardware_id     VARCHAR(255),
    status          device_status NOT NULL DEFAULT 'pending',
    blocklist_version BIGINT,
    last_heartbeat_at TIMESTAMPTZ,
    enrollment_id   BIGINT, -- FK added after enrollments table
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE devices ADD CONSTRAINT uq_devices_public_id UNIQUE (public_id);

-- Primary lookup: devices owned by an account
CREATE INDEX idx_devices_account_id ON devices (account_id);

-- Stale heartbeat detection (background worker query)
CREATE INDEX idx_devices_heartbeat_active
    ON devices (last_heartbeat_at ASC NULLS FIRST)
    WHERE status = 'active';

-- Hardware deduplication check
CREATE INDEX idx_devices_hardware_id ON devices (hardware_id)
    WHERE hardware_id IS NOT NULL;

-- Platform analytics
CREATE INDEX idx_devices_platform ON devices (platform);
