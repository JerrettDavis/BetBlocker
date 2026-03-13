-- Subscriptions (hosted deployment only)
-- This table is created in all environments but only populated
-- when BILLING_ENABLED=true.

CREATE TABLE subscriptions (
    id                      BIGSERIAL PRIMARY KEY,
    account_id              BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    stripe_customer_id      VARCHAR(255) NOT NULL,
    stripe_subscription_id  VARCHAR(255) NOT NULL,
    plan                    subscription_plan NOT NULL DEFAULT 'standard',
    status                  subscription_status NOT NULL DEFAULT 'active',
    current_period_start    TIMESTAMPTZ NOT NULL,
    current_period_end      TIMESTAMPTZ NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE subscriptions
    ADD CONSTRAINT uq_subscriptions_stripe_customer UNIQUE (stripe_customer_id);
ALTER TABLE subscriptions
    ADD CONSTRAINT uq_subscriptions_stripe_sub UNIQUE (stripe_subscription_id);

-- One active subscription per account
CREATE UNIQUE INDEX uq_subscriptions_account_active
    ON subscriptions (account_id)
    WHERE status IN ('active', 'trialing', 'past_due');

-- Webhook lookup by Stripe IDs
CREATE INDEX idx_subscriptions_stripe_customer
    ON subscriptions (stripe_customer_id);

-- Renewal processing
CREATE INDEX idx_subscriptions_period_end
    ON subscriptions (current_period_end)
    WHERE status = 'active';
