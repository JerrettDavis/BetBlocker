CREATE OR REPLACE FUNCTION fn_audit_trigger()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_values, new_values, actor_id)
    VALUES (
        TG_TABLE_NAME,
        COALESCE(NEW.id, OLD.id),
        TG_OP,
        CASE WHEN TG_OP IN ('UPDATE', 'DELETE') THEN to_jsonb(OLD) END,
        CASE WHEN TG_OP IN ('INSERT', 'UPDATE') THEN to_jsonb(NEW) END,
        current_setting('app.current_account_id', true)::bigint
    );
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Attach to security-critical tables
CREATE TRIGGER trg_audit_accounts AFTER INSERT OR UPDATE OR DELETE ON accounts FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_enrollments AFTER INSERT OR UPDATE OR DELETE ON enrollments FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_devices AFTER INSERT OR UPDATE OR DELETE ON devices FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_unenroll_requests AFTER INSERT OR UPDATE OR DELETE ON enrollment_unenroll_requests FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_partner_relationships AFTER INSERT OR UPDATE OR DELETE ON partner_relationships FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_blocklist_entries AFTER INSERT OR UPDATE OR DELETE ON blocklist_entries FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_device_certificates AFTER INSERT OR UPDATE OR DELETE ON device_certificates FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_subscriptions AFTER INSERT OR UPDATE OR DELETE ON subscriptions FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
