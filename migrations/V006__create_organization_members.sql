CREATE TABLE organization_members (
    id              BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    account_id      BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    role            org_member_role NOT NULL DEFAULT 'member',
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One membership per account per org
CREATE UNIQUE INDEX uq_org_member
    ON organization_members (organization_id, account_id);

-- Find orgs for an account
CREATE INDEX idx_org_members_account ON organization_members (account_id);

-- Find members of an org (for dashboards)
CREATE INDEX idx_org_members_org_role ON organization_members (organization_id, role);
