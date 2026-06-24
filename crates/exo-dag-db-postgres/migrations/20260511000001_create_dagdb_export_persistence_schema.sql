ALTER TABLE dagdb_receipts DROP CONSTRAINT IF EXISTS dagdb_receipts_subject_kind_check;
ALTER TABLE dagdb_receipts
    ADD CONSTRAINT dagdb_receipts_subject_kind_check
    CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision','export'));

ALTER TABLE dagdb_subject_receipt_heads DROP CONSTRAINT IF EXISTS dagdb_subject_receipt_heads_subject_kind_check;
ALTER TABLE dagdb_subject_receipt_heads
    ADD CONSTRAINT dagdb_subject_receipt_heads_subject_kind_check
    CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision','export'));

ALTER TABLE dagdb_receipts DROP CONSTRAINT IF EXISTS dagdb_receipts_event_type_check;
ALTER TABLE dagdb_receipts
    ADD CONSTRAINT dagdb_receipts_event_type_check
    CHECK (event_type IN ('intake_created','duplicate_rejected','validation_created','validation_passed','validation_failed','memory_approved','memory_routable','memory_revoked','memory_superseded','route_created','route_activated','route_stale','route_invalidated','context_packet_created','writeback_created','trust_check_created','council_decision_recorded','dag_finality_committed','dag_finality_failed','dag_finality_compensated','export_created','export_verified','export_failed','export_challenge_created','export_challenge_verified','dagdb_approval_request_submitted','dagdb_approval_granted','dagdb_approval_denied','dagdb_record_accepted','dagdb_import_completed','dagdb_export_completed','dagdb_replay_detected','dagdb_idempotency_conflict','dagdb_rls_tenant_violation','dagdb_signature_failure','dagdb_council_operator_decision'));

CREATE TABLE IF NOT EXISTS dagdb_exports (
    export_id BYTEA PRIMARY KEY CHECK (octet_length(export_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    schema_version TEXT NOT NULL CHECK (schema_version = 'dagdb_kg_portable_export_v1'),
    export_scope_hash BYTEA NOT NULL CHECK (octet_length(export_scope_hash) = 32),
    source_commit_or_repo_ref TEXT NULL CHECK (
        source_commit_or_repo_ref IS NULL
        OR (
            length(source_commit_or_repo_ref) > 0
            AND source_commit_or_repo_ref NOT LIKE '/%'
            AND source_commit_or_repo_ref NOT LIKE 'file:%'
            AND source_commit_or_repo_ref NOT ILIKE '%postgres://%'
        )
    ),
    included_memory_ids_hash BYTEA NOT NULL CHECK (octet_length(included_memory_ids_hash) = 32),
    included_receipt_heads_hash BYTEA NOT NULL CHECK (octet_length(included_receipt_heads_hash) = 32),
    section_hashes JSONB NOT NULL CHECK (jsonb_typeof(section_hashes) = 'object'),
    section_counts JSONB NOT NULL CHECK (jsonb_typeof(section_counts) = 'object'),
    citation_index_hash BYTEA NOT NULL CHECK (octet_length(citation_index_hash) = 32),
    provenance_index_hash BYTEA NOT NULL CHECK (octet_length(provenance_index_hash) = 32),
    redaction_summary_hash BYTEA NOT NULL CHECK (octet_length(redaction_summary_hash) = 32),
    omission_summary_hash BYTEA NOT NULL CHECK (octet_length(omission_summary_hash) = 32),
    verification_hash BYTEA NOT NULL CHECK (octet_length(verification_hash) = 32),
    whole_export_hash BYTEA NOT NULL CHECK (octet_length(whole_export_hash) = 32),
    export_status TEXT NOT NULL DEFAULT 'pending' CHECK (export_status IN ('pending','verified','failed','revoked')),
    authority_ref_hash BYTEA NULL CHECK (authority_ref_hash IS NULL OR octet_length(authority_ref_hash) = 32),
    consent_ref_hash BYTEA NULL CHECK (consent_ref_hash IS NULL OR octet_length(consent_ref_hash) = 32),
    approval_ref_hash BYTEA NULL CHECK (approval_ref_hash IS NULL OR octet_length(approval_ref_hash) = 32),
    requester_did TEXT NOT NULL CHECK (length(requester_did) > 0 AND requester_did LIKE 'did:%'),
    latest_receipt_hash BYTEA NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    UNIQUE (tenant_id, namespace, export_scope_hash, whole_export_hash),
    CHECK (
        updated_at_physical_ms > created_at_physical_ms
        OR (
            updated_at_physical_ms = created_at_physical_ms
            AND updated_at_logical >= created_at_logical
        )
    )
);

CREATE TABLE IF NOT EXISTS dagdb_export_challenges (
    challenge_id BYTEA PRIMARY KEY CHECK (octet_length(challenge_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    export_id BYTEA NOT NULL REFERENCES dagdb_exports(export_id),
    challenge_kind TEXT NOT NULL CHECK (challenge_kind IN ('whole_export_hash','section_hash','citation_index_hash','provenance_index_hash','redaction_summary_hash','omission_summary_hash')),
    challenge_hash BYTEA NOT NULL CHECK (octet_length(challenge_hash) = 32),
    proof_hash BYTEA NOT NULL CHECK (octet_length(proof_hash) = 32),
    proof_algorithm TEXT NOT NULL CHECK (proof_algorithm = 'hash_commitment_v1'),
    verifier_did TEXT NULL CHECK (verifier_did IS NULL OR (length(verifier_did) > 0 AND verifier_did LIKE 'did:%')),
    verification_status TEXT NOT NULL DEFAULT 'pending' CHECK (verification_status IN ('pending','verified','failed')),
    verification_notes_hash BYTEA NULL CHECK (verification_notes_hash IS NULL OR octet_length(verification_notes_hash) = 32),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    UNIQUE (tenant_id, namespace, export_id, challenge_kind, challenge_hash)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_exports_scope_status ON dagdb_exports USING btree (tenant_id ASC, namespace ASC, export_status ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_exports_scope_hash ON dagdb_exports USING btree (tenant_id ASC, namespace ASC, export_scope_hash ASC, whole_export_hash ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_exports_receipt ON dagdb_exports USING btree (latest_receipt_hash ASC) WHERE latest_receipt_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_dagdb_export_challenges_export ON dagdb_export_challenges USING btree (tenant_id ASC, namespace ASC, export_id ASC, challenge_kind ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_export_challenges_status ON dagdb_export_challenges USING btree (tenant_id ASC, namespace ASC, verification_status ASC, created_at_physical_ms DESC, created_at_logical DESC);
