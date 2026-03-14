-- Add invited_by column to organization_members

ALTER TABLE organization_members
    ADD COLUMN invited_by BIGINT REFERENCES accounts(id);
