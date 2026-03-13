-- Device certificates table

CREATE TABLE device_certificates (
    id                      BIGSERIAL PRIMARY KEY,
    device_id               BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    certificate_fingerprint VARCHAR(128) NOT NULL,
    issued_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at              TIMESTAMPTZ NOT NULL,
    revoked_at              TIMESTAMPTZ
);

ALTER TABLE device_certificates
    ADD CONSTRAINT uq_device_cert_fingerprint UNIQUE (certificate_fingerprint);

CREATE INDEX idx_device_certs_device_active
    ON device_certificates (device_id)
    WHERE revoked_at IS NULL;

CREATE INDEX idx_device_certs_expires
    ON device_certificates (expires_at)
    WHERE revoked_at IS NULL;
