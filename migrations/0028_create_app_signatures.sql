-- App signatures table for application blocking

CREATE TYPE app_signature_status AS ENUM (
    'active',
    'inactive',
    'pending_review'
);

CREATE TYPE app_signature_platform AS ENUM (
    'windows',
    'macos',
    'linux',
    'android',
    'ios',
    'all'
);

CREATE TABLE app_signatures (
    id                          BIGSERIAL PRIMARY KEY,
    public_id                   UUID NOT NULL DEFAULT gen_random_uuid(),
    name                        VARCHAR(500) NOT NULL,
    package_names               TEXT[] NOT NULL DEFAULT '{}',
    executable_names            TEXT[] NOT NULL DEFAULT '{}',
    cert_hashes                 TEXT[] NOT NULL DEFAULT '{}',
    display_name_patterns       TEXT[] NOT NULL DEFAULT '{}',
    platforms                   TEXT[] NOT NULL DEFAULT '{}',
    category                    VARCHAR(100) NOT NULL,
    status                      app_signature_status NOT NULL DEFAULT 'pending_review',
    confidence                  DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    source                      blocklist_source NOT NULL DEFAULT 'curated',
    evidence_url                VARCHAR(1000),
    tags                        TEXT[] NOT NULL DEFAULT '{}',
    blocklist_version_added     BIGINT,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE app_signatures ADD CONSTRAINT uq_app_signatures_public_id UNIQUE (public_id);

CREATE INDEX idx_app_signatures_package_names
    ON app_signatures USING gin (package_names);

CREATE INDEX idx_app_signatures_executable_names
    ON app_signatures USING gin (executable_names);

CREATE INDEX idx_app_signatures_status_platforms
    ON app_signatures (status, platforms);
