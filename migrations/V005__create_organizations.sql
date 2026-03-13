CREATE TABLE organizations (
    id          BIGSERIAL PRIMARY KEY,
    public_id   UUID NOT NULL DEFAULT gen_random_uuid(),
    name        VARCHAR(200) NOT NULL,
    org_type    organization_type NOT NULL,
    owner_id    BIGINT NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE organizations ADD CONSTRAINT uq_organizations_public_id UNIQUE (public_id);

CREATE INDEX idx_organizations_type ON organizations (org_type);

-- Add the FK from accounts to organizations now that the table exists
ALTER TABLE accounts ADD CONSTRAINT fk_accounts_organization
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE SET NULL;
