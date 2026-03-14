-- Organization devices join table

CREATE TABLE organization_devices (
    id              BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    device_id       BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    assigned_by     BIGINT REFERENCES accounts(id),
    assigned_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, device_id)
);

CREATE INDEX idx_org_devices_device ON organization_devices (device_id);
