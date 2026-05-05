-- Match the tenant-scoped audit decision lookup used by /api/v1/audit/:decision_id.
CREATE INDEX IF NOT EXISTS idx_audit_entries_decision_tenant_sequence
    ON audit_entries(decision_id, tenant_id, sequence);
