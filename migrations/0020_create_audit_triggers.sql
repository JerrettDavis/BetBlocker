-- Audit trigger function and triggers

CREATE OR REPLACE FUNCTION fn_audit_trigger()
RETURNS TRIGGER AS $$
DECLARE
    v_actor   BIGINT;
    v_ip      INET;
    v_old     JSONB;
    v_new     JSONB;
    v_row_id  BIGINT;
BEGIN
    -- Read application context (set via SET LOCAL in the transaction)
    BEGIN
        v_actor := current_setting('app.current_account_id')::BIGINT;
    EXCEPTION WHEN OTHERS THEN
        v_actor := NULL;
    END;

    BEGIN
        v_ip := current_setting('app.current_ip')::INET;
    EXCEPTION WHEN OTHERS THEN
        v_ip := NULL;
    END;

    IF TG_OP = 'DELETE' THEN
        v_old := to_jsonb(OLD);
        v_new := NULL;
        v_row_id := OLD.id;
    ELSIF TG_OP = 'UPDATE' THEN
        v_old := to_jsonb(OLD);
        v_new := to_jsonb(NEW);
        v_row_id := NEW.id;
    ELSIF TG_OP = 'INSERT' THEN
        v_old := NULL;
        v_new := to_jsonb(NEW);
        v_row_id := NEW.id;
    END IF;

    INSERT INTO audit_log (actor_account_id, table_name, row_id, action, old_values, new_values, client_ip)
    VALUES (v_actor, TG_TABLE_NAME, v_row_id, TG_OP::audit_action, v_old, v_new, v_ip);

    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Attach audit triggers to security-critical tables
CREATE TRIGGER trg_audit_accounts
    AFTER INSERT OR UPDATE OR DELETE ON accounts
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_enrollments
    AFTER INSERT OR UPDATE OR DELETE ON enrollments
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_enrollment_unenroll_requests
    AFTER INSERT OR UPDATE OR DELETE ON enrollment_unenroll_requests
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_devices
    AFTER INSERT OR UPDATE OR DELETE ON devices
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_device_certificates
    AFTER INSERT OR UPDATE OR DELETE ON device_certificates
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_partner_relationships
    AFTER INSERT OR UPDATE OR DELETE ON partner_relationships
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_organization_members
    AFTER INSERT OR UPDATE OR DELETE ON organization_members
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_blocklist_entries
    AFTER INSERT OR UPDATE OR DELETE ON blocklist_entries
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();

CREATE TRIGGER trg_audit_subscriptions
    AFTER INSERT OR UPDATE OR DELETE ON subscriptions
    FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
