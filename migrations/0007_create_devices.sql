-- Devices table

CREATE TABLE devices (
    id              BIGSERIAL PRIMARY KEY,
    public_id       UUID NOT NULL DEFAULT gen_random_uuid(),
    account_id      BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name            VARCHAR(100),
    platform        device_platform NOT NULL,
    os_version      VARCHAR(50),
    agent_version   VARCHAR(50),
    hostname        VARCHAR(255),
    hardware_id     VARCHAR(255),
    blocklist_version BIGINT,
    last_heartbeat_at TIMESTAMPTZ,
    enrollment_id   BIGINT,
    status          device_status NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE devices ADD CONSTRAINT uq_devices_public_id UNIQUE (public_id);

CREATE INDEX idx_devices_account_id ON devices (account_id);

CREATE INDEX idx_devices_heartbeat_active
    ON devices (last_heartbeat_at ASC NULLS FIRST)
    WHERE status = 'active';

CREATE INDEX idx_devices_hardware_id ON devices (hardware_id)
    WHERE hardware_id IS NOT NULL;

CREATE INDEX idx_devices_platform ON devices (platform);
