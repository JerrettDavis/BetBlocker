-- Enable RLS on tenant-scoped tables
ALTER TABLE accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollments ENABLE ROW LEVEL SECURITY;
ALTER TABLE partner_relationships ENABLE ROW LEVEL SECURITY;
ALTER TABLE subscriptions ENABLE ROW LEVEL SECURITY;

-- Application role for API connections
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'bb_api') THEN
        CREATE ROLE bb_api LOGIN;
    END IF;
END
$$;

-- RLS policies: accounts
CREATE POLICY accounts_own_row ON accounts
    FOR ALL TO bb_api
    USING (id = current_setting('app.current_account_id', true)::bigint);

-- RLS policies: devices
CREATE POLICY devices_own_row ON devices
    FOR ALL TO bb_api
    USING (account_id = current_setting('app.current_account_id', true)::bigint);

-- Additional policies for partner visibility will be added as needed
