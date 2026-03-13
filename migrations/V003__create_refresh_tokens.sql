CREATE TABLE refresh_tokens (
    id              BIGSERIAL PRIMARY KEY,
    account_id      BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash      BYTEA NOT NULL,
    device_info     VARCHAR(500),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ
);

-- Unique on token_hash to prevent duplicates and enable fast lookup
ALTER TABLE refresh_tokens ADD CONSTRAINT uq_refresh_tokens_hash UNIQUE (token_hash);

-- Find active tokens for an account (for revocation)
CREATE INDEX idx_refresh_tokens_account_id ON refresh_tokens (account_id);

-- Cleanup job: find expired tokens
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;
