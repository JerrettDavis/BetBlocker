CREATE TABLE partner_relationships (
    id                  BIGSERIAL PRIMARY KEY,
    account_id          BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    partner_account_id  BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    status              partner_relationship_status NOT NULL DEFAULT 'pending',
    role                partner_role NOT NULL,
    invited_by          BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    invite_token_hash   VARCHAR(255),
    invited_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at         TIMESTAMPTZ,
    revoked_at          TIMESTAMPTZ,

    -- A user cannot partner with themselves
    CONSTRAINT chk_partner_not_self CHECK (account_id <> partner_account_id)
);

-- Prevent duplicate partnership in either direction
CREATE UNIQUE INDEX uq_partner_pair
    ON partner_relationships (LEAST(account_id, partner_account_id),
                              GREATEST(account_id, partner_account_id));

-- Find partnerships for a given account
CREATE INDEX idx_partner_relationships_account ON partner_relationships (account_id);
CREATE INDEX idx_partner_relationships_partner ON partner_relationships (partner_account_id);

-- Find pending invitations
CREATE INDEX idx_partner_relationships_pending
    ON partner_relationships (partner_account_id, invited_at DESC)
    WHERE status = 'pending';
