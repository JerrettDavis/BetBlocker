-- Organizations table

CREATE TABLE organizations (
    id          BIGSERIAL PRIMARY KEY,
    public_id   UUID NOT NULL DEFAULT gen_random_uuid(),
    name        VARCHAR(200) NOT NULL,
    type        org_type NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE organizations ADD CONSTRAINT uq_organizations_public_id UNIQUE (public_id);

CREATE INDEX idx_organizations_type ON organizations (type);
