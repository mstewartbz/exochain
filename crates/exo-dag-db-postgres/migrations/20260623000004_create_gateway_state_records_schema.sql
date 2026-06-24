CREATE TABLE IF NOT EXISTS dagdb_gateway_state_records (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    state_family TEXT NOT NULL CHECK (state_family IN (
        'did_document',
        'session',
        'user',
        'agent',
        'decision',
        'delegation',
        'audit_entry',
        'constitution',
        'identity_score',
        'enrollment',
        'livesafe_identity',
        'scan_receipt',
        'consent_anchor',
        'trustee_shard',
        'agent_role',
        'consent_record',
        'authority_chain',
        'layout_template',
        'feedback_issue',
        'conflict_declaration',
        'avc_registry_state',
        'hlc_counter'
    )),
    record_key TEXT NOT NULL CHECK (length(record_key) > 0),
    secondary_key TEXT NULL CHECK (secondary_key IS NULL OR length(secondary_key) > 0),
    cbor_payload BYTEA NOT NULL CHECK (octet_length(cbor_payload) > 0),
    payload_hash BYTEA NOT NULL CHECK (octet_length(payload_hash) = 32),
    json_projection JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','revoked','deleted','tombstoned')),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    tombstoned_at_physical_ms BIGINT NULL CHECK (tombstoned_at_physical_ms IS NULL OR tombstoned_at_physical_ms >= 0),
    tombstoned_at_logical INTEGER NULL CHECK (tombstoned_at_logical IS NULL OR tombstoned_at_logical >= 0),
    PRIMARY KEY (tenant_id, namespace, state_family, record_key),
    CHECK (
        (tombstoned_at_physical_ms IS NULL AND tombstoned_at_logical IS NULL)
        OR (tombstoned_at_physical_ms IS NOT NULL AND tombstoned_at_logical IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_dagdb_gateway_state_secondary_key
    ON dagdb_gateway_state_records (tenant_id, namespace, state_family, secondary_key)
    WHERE secondary_key IS NOT NULL AND status = 'active';

CREATE INDEX IF NOT EXISTS idx_dagdb_gateway_state_family_status
    ON dagdb_gateway_state_records
        USING btree (tenant_id, namespace, state_family, status, updated_at_physical_ms DESC, updated_at_logical DESC);

CREATE INDEX IF NOT EXISTS idx_dagdb_gateway_state_payload_hash
    ON dagdb_gateway_state_records
        USING btree (tenant_id, namespace, payload_hash);

ALTER TABLE dagdb_gateway_state_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE dagdb_gateway_state_records FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS dagdb_tenant_isolation ON dagdb_gateway_state_records;
CREATE POLICY dagdb_tenant_isolation ON dagdb_gateway_state_records
    USING (tenant_id = dagdb_current_tenant_id())
    WITH CHECK (tenant_id = dagdb_current_tenant_id());
