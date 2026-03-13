-- Organization members table

CREATE TABLE organization_members (
    id              BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    account_id      BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    role            org_member_role NOT NULL DEFAULT 'member',
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uq_org_member
    ON organization_members (organization_id, account_id);

CREATE INDEX idx_org_members_account ON organization_members (account_id);

CREATE INDEX idx_org_members_org_role ON organization_members (organization_id, role);
