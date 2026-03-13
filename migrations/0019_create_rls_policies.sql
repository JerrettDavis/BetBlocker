-- Row-Level Security policies

-- Enable RLS on tenant-scoped tables
ALTER TABLE accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollments ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollment_unenroll_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE partner_relationships ENABLE ROW LEVEL SECURITY;
ALTER TABLE refresh_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE device_certificates ENABLE ROW LEVEL SECURITY;
ALTER TABLE subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE reporting_snapshots ENABLE ROW LEVEL SECURITY;

-- Helper function: current account ID from session
CREATE OR REPLACE FUNCTION current_account_id() RETURNS BIGINT AS $$
BEGIN
    RETURN current_setting('app.current_account_id', true)::BIGINT;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION current_account_role() RETURNS TEXT AS $$
BEGIN
    RETURN current_setting('app.current_role', true);
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- ACCOUNTS: users see only their own row; admins see all
CREATE POLICY accounts_self ON accounts
    FOR ALL
    USING (
        id = current_account_id()
        OR current_account_role() = 'admin'
    );

-- DEVICES: owners see their own devices; partners/authorities see devices
-- they have active enrollments on; admins see all
CREATE POLICY devices_owner ON devices
    FOR ALL
    USING (
        account_id = current_account_id()
        OR EXISTS (
            SELECT 1 FROM enrollments e
            WHERE e.device_id = devices.id
              AND e.enrolled_by = current_account_id()
              AND e.status = 'active'
        )
        OR current_account_role() = 'admin'
    );

-- ENROLLMENTS: device owner or enrollment authority can see the enrollment
CREATE POLICY enrollments_access ON enrollments
    FOR ALL
    USING (
        enrolled_by = current_account_id()
        OR EXISTS (
            SELECT 1 FROM devices d
            WHERE d.id = enrollments.device_id
              AND d.account_id = current_account_id()
        )
        OR current_account_role() = 'admin'
    );

-- ENROLLMENT UNENROLL REQUESTS: requester, required approver, or admin
CREATE POLICY unenroll_requests_access ON enrollment_unenroll_requests
    FOR ALL
    USING (
        requested_by_account_id = current_account_id()
        OR required_approver_account_id = current_account_id()
        OR current_account_role() = 'admin'
    );

-- PARTNER RELATIONSHIPS: either party
CREATE POLICY partner_relationships_access ON partner_relationships
    FOR ALL
    USING (
        account_id = current_account_id()
        OR partner_account_id = current_account_id()
        OR current_account_role() = 'admin'
    );

-- REFRESH TOKENS: own tokens only
CREATE POLICY refresh_tokens_self ON refresh_tokens
    FOR ALL
    USING (
        account_id = current_account_id()
        OR current_account_role() = 'admin'
    );

-- DEVICE CERTIFICATES: device owner or enrollment authority
CREATE POLICY device_certs_access ON device_certificates
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM devices d
            WHERE d.id = device_certificates.device_id
              AND (
                  d.account_id = current_account_id()
                  OR EXISTS (
                      SELECT 1 FROM enrollments e
                      WHERE e.device_id = d.id
                        AND e.enrolled_by = current_account_id()
                        AND e.status = 'active'
                  )
              )
        )
        OR current_account_role() = 'admin'
    );

-- SUBSCRIPTIONS: own subscription only
CREATE POLICY subscriptions_self ON subscriptions
    FOR ALL
    USING (
        account_id = current_account_id()
        OR current_account_role() = 'admin'
    );

-- REPORTING SNAPSHOTS: accessible via enrollment visibility
CREATE POLICY reporting_snapshots_access ON reporting_snapshots
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM enrollments e
            WHERE e.id = reporting_snapshots.enrollment_id
              AND (
                  e.enrolled_by = current_account_id()
                  OR EXISTS (
                      SELECT 1 FROM devices d
                      WHERE d.id = e.device_id
                        AND d.account_id = current_account_id()
                  )
              )
        )
        OR current_account_role() = 'admin'
    );
