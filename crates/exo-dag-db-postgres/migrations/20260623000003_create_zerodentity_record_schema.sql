CREATE TABLE IF NOT EXISTS dagdb_zerodentity_records (
    tenant_id TEXT NOT NULL,
    namespace TEXT NOT NULL,
    state_family TEXT NOT NULL CHECK (state_family IN (
        'claim',
        'score',
        'previous_score',
        'score_history',
        'device_fingerprint',
        'behavioral_sample',
        'otp_challenge',
        'otp_lockout',
        'attestation',
        'identity_session',
        'session_nonce',
        'dag_node',
        'trust_receipt'
    )),
    subject_did TEXT NOT NULL,
    record_key TEXT NOT NULL,
    secondary_key TEXT NOT NULL DEFAULT '',
    cbor_payload BYTEA NOT NULL,
    payload_hash BYTEA NOT NULL CHECK (octet_length(payload_hash) = 32),
    PRIMARY KEY (tenant_id, namespace, state_family, record_key, secondary_key)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_zerodentity_records_subject
    ON dagdb_zerodentity_records USING btree (tenant_id, namespace, subject_did, state_family);

CREATE INDEX IF NOT EXISTS idx_dagdb_zerodentity_records_family
    ON dagdb_zerodentity_records USING btree (tenant_id, namespace, state_family, record_key, secondary_key);

ALTER TABLE dagdb_zerodentity_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE dagdb_zerodentity_records FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS dagdb_tenant_isolation ON dagdb_zerodentity_records;
CREATE POLICY dagdb_tenant_isolation ON dagdb_zerodentity_records
    USING (tenant_id = dagdb_current_tenant_id())
    WITH CHECK (tenant_id = dagdb_current_tenant_id());
