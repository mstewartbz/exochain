CREATE TABLE IF NOT EXISTS dagdb_root_bundle_receipts (
    bundle_id BYTEA PRIMARY KEY NOT NULL CHECK (octet_length(bundle_id) = 32),
    root_bundle_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(root_bundle_hash) = 32),
    ceremony_id TEXT NOT NULL CHECK (length(trim(ceremony_id)) > 0),
    issuer_did TEXT NOT NULL CHECK (length(trim(issuer_did)) > 0),
    issuer_public_key_hash BYTEA NOT NULL CHECK (octet_length(issuer_public_key_hash) = 32),
    signing_set_hash BYTEA NOT NULL CHECK (octet_length(signing_set_hash) = 32),
    quorum_threshold INTEGER NOT NULL CHECK (quorum_threshold > 0),
    verifier_version TEXT NOT NULL CHECK (length(trim(verifier_version)) > 0),
    verification_receipt_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(verification_receipt_hash) = 32),
    verification_receipt_body JSONB NOT NULL,
    verified_at_physical_ms BIGINT NOT NULL CHECK (verified_at_physical_ms >= 0),
    verified_at_logical INTEGER NOT NULL CHECK (verified_at_logical >= 0),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    immutable BOOLEAN NOT NULL DEFAULT true CHECK (immutable = true)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_root_bundle_receipts_ceremony
    ON dagdb_root_bundle_receipts USING btree (ceremony_id, verified_at_physical_ms DESC, verified_at_logical DESC);

CREATE OR REPLACE FUNCTION prevent_dagdb_root_bundle_receipt_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'dagdb_root_bundle_receipts are immutable'
        USING ERRCODE = '25006';
END $$;

DROP TRIGGER IF EXISTS root_bundle_receipts_are_immutable_update
    ON dagdb_root_bundle_receipts;
CREATE TRIGGER root_bundle_receipts_are_immutable_update
    BEFORE UPDATE ON dagdb_root_bundle_receipts
    FOR EACH ROW
    EXECUTE FUNCTION prevent_dagdb_root_bundle_receipt_mutation();

DROP TRIGGER IF EXISTS root_bundle_receipts_are_immutable_delete
    ON dagdb_root_bundle_receipts;
CREATE TRIGGER root_bundle_receipts_are_immutable_delete
    BEFORE DELETE ON dagdb_root_bundle_receipts
    FOR EACH ROW
    EXECUTE FUNCTION prevent_dagdb_root_bundle_receipt_mutation();
