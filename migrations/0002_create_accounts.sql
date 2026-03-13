-- Accounts table

CREATE TABLE accounts (
    id                      BIGSERIAL PRIMARY KEY,
    public_id               UUID NOT NULL DEFAULT gen_random_uuid(),
    email                   VARCHAR(255) NOT NULL,
    password_hash           VARCHAR(255) NOT NULL,
    role                    account_role NOT NULL DEFAULT 'user',
    email_verified          BOOLEAN NOT NULL DEFAULT FALSE,
    display_name            VARCHAR(100) NOT NULL DEFAULT '',
    mfa_enabled             BOOLEAN NOT NULL DEFAULT FALSE,
    timezone                VARCHAR(50) NOT NULL DEFAULT 'UTC',
    locale                  VARCHAR(20) NOT NULL DEFAULT 'en-US',
    organization_id         BIGINT,
    locked_until            TIMESTAMPTZ,
    failed_login_attempts   INTEGER NOT NULL DEFAULT 0,
    email_verification_token VARCHAR(255),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE accounts ADD CONSTRAINT uq_accounts_public_id UNIQUE (public_id);
ALTER TABLE accounts ADD CONSTRAINT uq_accounts_email UNIQUE (email);

CREATE INDEX idx_accounts_role ON accounts (role);
CREATE INDEX idx_accounts_created_at ON accounts (created_at DESC);
