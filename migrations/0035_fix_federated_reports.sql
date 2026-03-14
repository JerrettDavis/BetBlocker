-- Fix federated_reports_v2: add unique constraint required by the ON CONFLICT
-- clause in federated_service.rs for idempotent report ingestion.
--
-- The original migration (0027) created the table without this constraint,
-- so INSERT ... ON CONFLICT (domain, reporter_token, batch_id) DO NOTHING
-- would fail at runtime.

ALTER TABLE federated_reports_v2
    ADD CONSTRAINT uq_federated_reports_v2_domain_token_batch
    UNIQUE (domain, reporter_token, batch_id);
