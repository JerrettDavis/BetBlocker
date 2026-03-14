-- Extend organizations table with owner_id and default config columns

-- Add 'owner' variant to org_member_role enum
ALTER TYPE org_member_role ADD VALUE IF NOT EXISTS 'owner' BEFORE 'admin';

ALTER TABLE organizations
    ADD COLUMN owner_id BIGINT NOT NULL REFERENCES accounts(id),
    ADD COLUMN default_protection_config JSONB,
    ADD COLUMN default_reporting_config JSONB,
    ADD COLUMN default_unenrollment_policy JSONB;

CREATE INDEX idx_organizations_owner ON organizations (owner_id);
