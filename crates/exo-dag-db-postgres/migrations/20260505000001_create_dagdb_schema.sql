CREATE TABLE IF NOT EXISTS dagdb_receipts (
    receipt_hash BYTEA PRIMARY KEY CHECK (octet_length(receipt_hash) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision','export')),
    subject_id BYTEA NOT NULL CHECK (octet_length(subject_id) = 32),
    prev_receipt_hash BYTEA NOT NULL CHECK (octet_length(prev_receipt_hash) = 32),
    seq BIGINT NOT NULL CHECK (seq > 0),
    event_type TEXT NOT NULL CHECK (event_type IN ('intake_created','duplicate_rejected','validation_created','validation_passed','validation_failed','memory_approved','memory_routable','memory_revoked','memory_superseded','route_created','route_activated','route_stale','route_invalidated','context_packet_created','writeback_created','trust_check_created','council_decision_recorded','dag_finality_committed','dag_finality_failed','dag_finality_compensated','dagdb_approval_request_submitted','dagdb_approval_granted','dagdb_approval_denied','dagdb_record_accepted','dagdb_import_completed','dagdb_export_completed','dagdb_replay_detected','dagdb_idempotency_conflict','dagdb_rls_tenant_violation','dagdb_signature_failure','dagdb_council_operator_decision')),
    actor_did TEXT NOT NULL CHECK (length(actor_did) > 0 AND actor_did LIKE 'did:%'),
    event_hlc_physical_ms BIGINT NOT NULL CHECK (event_hlc_physical_ms >= 0),
    event_hlc_logical INTEGER NOT NULL CHECK (event_hlc_logical >= 0),
    event_hash BYTEA NOT NULL CHECK (octet_length(event_hash) = 32),
    receipt_body JSONB NOT NULL,
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    UNIQUE (tenant_id, namespace, subject_kind, subject_id, seq),
    UNIQUE (tenant_id, namespace, subject_kind, subject_id, prev_receipt_hash)
);

CREATE TABLE IF NOT EXISTS dagdb_subject_receipt_heads (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision','export')),
    subject_id BYTEA NOT NULL CHECK (octet_length(subject_id) = 32),
    latest_receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    latest_seq BIGINT NOT NULL CHECK (latest_seq > 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    PRIMARY KEY (tenant_id, namespace, subject_kind, subject_id)
);

CREATE TABLE IF NOT EXISTS dagdb_memory_objects (
    memory_id BYTEA PRIMARY KEY CHECK (octet_length(memory_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    node_type TEXT NOT NULL CHECK (node_type IN ('source','excerpt','embedding','summary','answer','validation_report','catalog','route','context_packet')),
    source_type TEXT NOT NULL CHECK (source_type IN ('public_web','private_customer','ip_sensitive','generated','open_source','unknown_provenance','benchmark_fixture')),
    consent_purpose TEXT NOT NULL CHECK (consent_purpose IN ('retrieval','validation','writeback','benchmark','trust_check')),
    payload_hash BYTEA NOT NULL CHECK (octet_length(payload_hash) = 32),
    source_hash BYTEA NOT NULL CHECK (octet_length(source_hash) = 32),
    payload_uri_hash BYTEA NULL CHECK (payload_uri_hash IS NULL OR octet_length(payload_uri_hash) = 32),
    owner_did TEXT NOT NULL CHECK (length(owner_did) > 0 AND owner_did LIKE 'did:%'),
    controller_did TEXT NOT NULL CHECK (length(controller_did) > 0 AND controller_did LIKE 'did:%'),
    submitted_by_did TEXT NOT NULL CHECK (length(submitted_by_did) > 0 AND submitted_by_did LIKE 'did:%'),
    access_policy_hash BYTEA NULL CHECK (access_policy_hash IS NULL OR octet_length(access_policy_hash) = 32),
    declared_rights_hash BYTEA NULL CHECK (declared_rights_hash IS NULL OR octet_length(declared_rights_hash) = 32),
    title JSONB NOT NULL,
    summary JSONB NOT NULL,
    keywords JSONB NOT NULL DEFAULT '[]'::jsonb,
    risk_class TEXT NOT NULL CHECK (risk_class IN ('R0','R1','R2','R3','R4','R5')),
    risk_bp INTEGER NOT NULL CHECK (risk_bp >= 0 AND risk_bp <= 10000),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','approved','routable','blocked','revoked','superseded','rejected')),
    validation_status TEXT NOT NULL DEFAULT 'pending' CHECK (validation_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    council_status TEXT NOT NULL DEFAULT 'not_required' CHECK (council_status IN ('not_required','required','pending','approved','denied','expired','escalated')),
    dag_finality_status TEXT NOT NULL DEFAULT 'pending' CHECK (dag_finality_status IN ('pending','committed','failed','compensated')),
    latest_receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    revoked_at_physical_ms BIGINT NULL CHECK (revoked_at_physical_ms IS NULL OR revoked_at_physical_ms >= 0),
    revoked_at_logical INTEGER NULL CHECK (revoked_at_logical IS NULL OR revoked_at_logical >= 0),
    superseded_by_memory_id BYTEA NULL REFERENCES dagdb_memory_objects(memory_id),
    CHECK ((revoked_at_physical_ms IS NULL AND revoked_at_logical IS NULL) OR (revoked_at_physical_ms IS NOT NULL AND revoked_at_logical IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS dagdb_memory_edges (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    from_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    to_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    edge_type TEXT NOT NULL CHECK (edge_type IN ('parent','derived_from','cites','contradicts','supersedes','validates')),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    PRIMARY KEY (tenant_id, namespace, from_memory_id, to_memory_id, edge_type)
);

CREATE TABLE IF NOT EXISTS dagdb_catalog_entries (
    catalog_id BYTEA PRIMARY KEY CHECK (octet_length(catalog_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    memory_id BYTEA NULL REFERENCES dagdb_memory_objects(memory_id),
    parent_catalog_id BYTEA NULL REFERENCES dagdb_catalog_entries(catalog_id),
    catalog_level INTEGER NOT NULL CHECK (catalog_level >= 0),
    title JSONB NOT NULL,
    summary JSONB NOT NULL,
    keywords JSONB NOT NULL DEFAULT '[]'::jsonb,
    payload_hash BYTEA NOT NULL CHECK (octet_length(payload_hash) = 32),
    source_hash BYTEA NOT NULL CHECK (octet_length(source_hash) = 32),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','approved','routable','blocked','revoked','superseded','rejected')),
    validation_status TEXT NOT NULL DEFAULT 'pending' CHECK (validation_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    council_status TEXT NOT NULL DEFAULT 'not_required' CHECK (council_status IN ('not_required','required','pending','approved','denied','expired','escalated')),
    dag_finality_status TEXT NOT NULL DEFAULT 'pending' CHECK (dag_finality_status IN ('pending','committed','failed','compensated')),
    latest_receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0)
);

COMMENT ON COLUMN dagdb_catalog_entries.payload_hash IS 'For root/domain catalog rows without raw payloads, this is the canonical hash of catalog material.';
COMMENT ON COLUMN dagdb_catalog_entries.source_hash IS 'For root/domain catalog rows without raw payloads, this is the canonical hash of catalog material.';

CREATE TABLE IF NOT EXISTS dagdb_route_receipts (
    route_id BYTEA PRIMARY KEY CHECK (octet_length(route_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    requesting_agent_did TEXT NOT NULL CHECK (length(requesting_agent_did) > 0 AND requesting_agent_did LIKE 'did:%'),
    task_signature_hash BYTEA NOT NULL CHECK (octet_length(task_signature_hash) = 32),
    approved_scope_hash BYTEA NOT NULL CHECK (octet_length(approved_scope_hash) = 32),
    credential_id BYTEA NULL CHECK (credential_id IS NULL OR octet_length(credential_id) = 32),
    candidate_memory_ids JSONB NOT NULL,
    selected_memory_ids JSONB NOT NULL,
    rejected_memory_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    route_score_bp INTEGER NOT NULL CHECK (route_score_bp >= 0 AND route_score_bp <= 10000),
    token_budget INTEGER NOT NULL CHECK (token_budget > 0),
    token_estimate INTEGER NOT NULL CHECK (token_estimate >= 0),
    overuse_penalty_bp INTEGER NOT NULL DEFAULT 0 CHECK (overuse_penalty_bp >= 0 AND overuse_penalty_bp <= 10000),
    risk_bp INTEGER NOT NULL CHECK (risk_bp >= 0 AND risk_bp <= 10000),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','active','stale','invalidated','blocked')),
    validation_status TEXT NOT NULL DEFAULT 'pending' CHECK (validation_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    council_status TEXT NOT NULL DEFAULT 'not_required' CHECK (council_status IN ('not_required','required','pending','approved','denied','expired','escalated')),
    dag_finality_status TEXT NOT NULL DEFAULT 'pending' CHECK (dag_finality_status IN ('pending','committed','failed','compensated')),
    validation_report_id BYTEA NULL CHECK (validation_report_id IS NULL OR octet_length(validation_report_id) = 32),
    council_decision_id BYTEA NULL CHECK (council_decision_id IS NULL OR octet_length(council_decision_id) = 32),
    stale_at_physical_ms BIGINT NOT NULL CHECK (stale_at_physical_ms >= 0),
    stale_at_logical INTEGER NOT NULL CHECK (stale_at_logical >= 0),
    latest_receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0)
);

CREATE TABLE IF NOT EXISTS dagdb_context_packets (
    context_packet_id BYTEA PRIMARY KEY CHECK (octet_length(context_packet_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    request_id TEXT NOT NULL CHECK (length(request_id) > 0),
    route_id BYTEA NOT NULL REFERENCES dagdb_route_receipts(route_id),
    task_hash BYTEA NOT NULL CHECK (octet_length(task_hash) = 32),
    requesting_agent_did TEXT NOT NULL CHECK (length(requesting_agent_did) > 0 AND requesting_agent_did LIKE 'did:%'),
    memory_refs JSONB NOT NULL,
    packet_hash BYTEA NOT NULL CHECK (octet_length(packet_hash) = 32),
    token_budget INTEGER NOT NULL CHECK (token_budget > 0),
    token_estimate INTEGER NOT NULL CHECK (token_estimate >= 0),
    validation_report_id BYTEA NULL CHECK (validation_report_id IS NULL OR octet_length(validation_report_id) = 32),
    council_decision_id BYTEA NULL CHECK (council_decision_id IS NULL OR octet_length(council_decision_id) = 32),
    validation_status TEXT NOT NULL DEFAULT 'pending' CHECK (validation_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    council_status TEXT NOT NULL DEFAULT 'not_required' CHECK (council_status IN ('not_required','required','pending','approved','denied','expired','escalated')),
    dag_finality_status TEXT NOT NULL DEFAULT 'pending' CHECK (dag_finality_status IN ('pending','committed','failed','compensated')),
    latest_receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0)
);

CREATE TABLE IF NOT EXISTS dagdb_validation_reports (
    validation_report_id BYTEA PRIMARY KEY CHECK (octet_length(validation_report_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision')),
    subject_id BYTEA NOT NULL CHECK (octet_length(subject_id) = 32),
    validator_did TEXT NOT NULL CHECK (length(validator_did) > 0 AND validator_did LIKE 'did:%'),
    input_hash BYTEA NOT NULL CHECK (octet_length(input_hash) = 32),
    policy_hash BYTEA NOT NULL CHECK (octet_length(policy_hash) = 32),
    validation_status TEXT NOT NULL CHECK (validation_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    risk_class TEXT NOT NULL CHECK (risk_class IN ('R0','R1','R2','R3','R4','R5')),
    risk_bp INTEGER NOT NULL CHECK (risk_bp >= 0 AND risk_bp <= 10000),
    decision TEXT NOT NULL CHECK (decision IN ('allow','block','needs_council','invalidate','revoke','supersede')),
    notes JSONB NOT NULL,
    contradictory_report_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    council_decision_id BYTEA NULL CHECK (council_decision_id IS NULL OR octet_length(council_decision_id) = 32),
    latest_receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0)
);

CREATE TABLE IF NOT EXISTS dagdb_agent_safety_scores (
    safety_score_id BYTEA PRIMARY KEY CHECK (octet_length(safety_score_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    agent_did TEXT NOT NULL CHECK (length(agent_did) > 0 AND agent_did LIKE 'did:%'),
    operator_did TEXT NOT NULL CHECK (length(operator_did) > 0 AND operator_did LIKE 'did:%'),
    window_start_physical_ms BIGINT NOT NULL CHECK (window_start_physical_ms >= 0),
    window_start_logical INTEGER NOT NULL CHECK (window_start_logical >= 0),
    window_end_physical_ms BIGINT NOT NULL CHECK (window_end_physical_ms >= 0),
    window_end_logical INTEGER NOT NULL CHECK (window_end_logical >= 0),
    evidence_hash BYTEA NOT NULL CHECK (octet_length(evidence_hash) = 32),
    identity_bp INTEGER NOT NULL CHECK (identity_bp >= 0 AND identity_bp <= 10000),
    authority_bp INTEGER NOT NULL CHECK (authority_bp >= 0 AND authority_bp <= 10000),
    consent_bp INTEGER NOT NULL CHECK (consent_bp >= 0 AND consent_bp <= 10000),
    provenance_bp INTEGER NOT NULL CHECK (provenance_bp >= 0 AND provenance_bp <= 10000),
    validation_bp INTEGER NOT NULL CHECK (validation_bp >= 0 AND validation_bp <= 10000),
    recency_bp INTEGER NOT NULL CHECK (recency_bp >= 0 AND recency_bp <= 10000),
    revocation_bp INTEGER NOT NULL CHECK (revocation_bp >= 0 AND revocation_bp <= 10000),
    route_quality_bp INTEGER NOT NULL CHECK (route_quality_bp >= 0 AND route_quality_bp <= 10000),
    incident_penalty_bp INTEGER NOT NULL CHECK (incident_penalty_bp >= 0 AND incident_penalty_bp <= 10000),
    total_score_bp INTEGER NOT NULL CHECK (total_score_bp >= 0 AND total_score_bp <= 10000),
    validation_status TEXT NOT NULL CHECK (validation_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    council_status TEXT NOT NULL CHECK (council_status IN ('not_required','required','pending','approved','denied','expired','escalated')),
    latest_receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    CHECK ((window_end_physical_ms > window_start_physical_ms) OR (window_end_physical_ms = window_start_physical_ms AND window_end_logical >= window_start_logical))
);

CREATE TABLE IF NOT EXISTS dagdb_inbound_agent_credentials (
    credential_id BYTEA PRIMARY KEY CHECK (octet_length(credential_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    agent_did TEXT NOT NULL CHECK (length(agent_did) > 0 AND agent_did LIKE 'did:%'),
    operator_did TEXT NOT NULL CHECK (length(operator_did) > 0 AND operator_did LIKE 'did:%'),
    model_name TEXT NOT NULL CHECK (length(model_name) > 0),
    model_version TEXT NOT NULL CHECK (length(model_version) > 0),
    provider_or_builder TEXT NOT NULL CHECK (length(provider_or_builder) > 0),
    requested_action TEXT NOT NULL CHECK (length(requested_action) > 0),
    requested_scope_hash BYTEA NOT NULL CHECK (octet_length(requested_scope_hash) = 32),
    purpose TEXT NOT NULL CHECK (length(purpose) > 0),
    autonomy_level TEXT NOT NULL CHECK (length(autonomy_level) > 0),
    nonce TEXT NOT NULL CHECK (length(nonce) > 0),
    expires_at_physical_ms BIGINT NOT NULL CHECK (expires_at_physical_ms >= 0),
    expires_at_logical INTEGER NOT NULL CHECK (expires_at_logical >= 0),
    signature_hash BYTEA NOT NULL CHECK (octet_length(signature_hash) = 32),
    credential_status TEXT NOT NULL DEFAULT 'pending' CHECK (credential_status IN ('pending','active','expired','revoked','blocked')),
    checkpoint_hash BYTEA NULL CHECK (checkpoint_hash IS NULL OR octet_length(checkpoint_hash) = 32),
    attestation_hash BYTEA NULL CHECK (attestation_hash IS NULL OR octet_length(attestation_hash) = 32),
    prior_trust_receipt_hash BYTEA NULL CHECK (prior_trust_receipt_hash IS NULL OR octet_length(prior_trust_receipt_hash) = 32),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0)
);

CREATE TABLE IF NOT EXISTS dagdb_council_decisions (
    decision_id BYTEA PRIMARY KEY CHECK (octet_length(decision_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision')),
    subject_id BYTEA NOT NULL CHECK (octet_length(subject_id) = 32),
    requested_action TEXT NOT NULL CHECK (length(requested_action) > 0),
    approved_scope_hash BYTEA NOT NULL CHECK (octet_length(approved_scope_hash) = 32),
    risk_class TEXT NOT NULL CHECK (risk_class IN ('R0','R1','R2','R3','R4','R5')),
    approver_did TEXT NOT NULL CHECK (length(approver_did) > 0 AND approver_did LIKE 'did:%'),
    decision_source TEXT NOT NULL CHECK (decision_source IN ('human','council','policy')),
    decision_status TEXT NOT NULL CHECK (decision_status IN ('approved','denied','expired','escalated','revoked')),
    reason_code TEXT NOT NULL CHECK (length(reason_code) > 0),
    validation_report_id BYTEA NULL REFERENCES dagdb_validation_reports(validation_report_id),
    route_id BYTEA NULL REFERENCES dagdb_route_receipts(route_id),
    context_packet_id BYTEA NULL REFERENCES dagdb_context_packets(context_packet_id),
    notes JSONB NULL,
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    expires_at_physical_ms BIGINT NOT NULL CHECK (expires_at_physical_ms >= 0),
    expires_at_logical INTEGER NOT NULL CHECK (expires_at_logical >= 0),
    receipt_hash BYTEA NOT NULL UNIQUE REFERENCES dagdb_receipts(receipt_hash),
    CHECK ((expires_at_physical_ms > created_at_physical_ms) OR (expires_at_physical_ms = created_at_physical_ms AND expires_at_logical > created_at_logical))
);

CREATE TABLE IF NOT EXISTS dagdb_idempotency_keys (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    route_name TEXT NOT NULL CHECK (length(route_name) > 0),
    idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) > 0),
    request_hash BYTEA NOT NULL CHECK (octet_length(request_hash) = 32),
    response_hash BYTEA NOT NULL CHECK (octet_length(response_hash) = 32),
    response_body JSONB NOT NULL,
    status_code INTEGER NOT NULL CHECK (status_code >= 200 AND status_code <= 599),
    cached_failure BOOLEAN NOT NULL DEFAULT false,
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    expires_at_physical_ms BIGINT NOT NULL CHECK (expires_at_physical_ms >= 0),
    expires_at_logical INTEGER NOT NULL CHECK (expires_at_logical >= 0),
    PRIMARY KEY (tenant_id, namespace, route_name, idempotency_key),
    UNIQUE (tenant_id, namespace, route_name, idempotency_key, request_hash),
    CHECK ((cached_failure = false AND status_code >= 200 AND status_code < 300) OR (cached_failure = true AND status_code = 409))
);

CREATE TABLE IF NOT EXISTS dagdb_dag_outbox (
    outbox_id BYTEA PRIMARY KEY CHECK (octet_length(outbox_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision','export')),
    subject_id BYTEA NOT NULL CHECK (octet_length(subject_id) = 32),
    dag_write_id TEXT NOT NULL CHECK (length(dag_write_id) > 0),
    dag_payload_hash BYTEA NOT NULL CHECK (octet_length(dag_payload_hash) = 32),
    dag_finality_status TEXT NOT NULL DEFAULT 'pending' CHECK (dag_finality_status IN ('pending','committed','failed','compensated')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0 AND attempt_count <= 6),
    max_attempts INTEGER NOT NULL DEFAULT 6 CHECK (max_attempts = 6),
    next_attempt_at_physical_ms BIGINT NULL CHECK (next_attempt_at_physical_ms IS NULL OR next_attempt_at_physical_ms >= 0),
    next_attempt_at_logical INTEGER NULL CHECK (next_attempt_at_logical IS NULL OR next_attempt_at_logical >= 0),
    last_error_code TEXT NULL,
    dag_receipt_hash BYTEA NULL CHECK (dag_receipt_hash IS NULL OR octet_length(dag_receipt_hash) = 32),
    compensation_receipt_hash BYTEA NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    UNIQUE (tenant_id, namespace, subject_kind, subject_id, dag_write_id),
    CHECK ((next_attempt_at_physical_ms IS NULL AND next_attempt_at_logical IS NULL) OR (next_attempt_at_physical_ms IS NOT NULL AND next_attempt_at_logical IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS dagdb_benchmark_runs (
    benchmark_run_id BYTEA PRIMARY KEY CHECK (octet_length(benchmark_run_id) = 32),
    fixture_id TEXT NOT NULL CHECK (length(fixture_id) > 0),
    runner_name TEXT NOT NULL CHECK (runner_name IN ('no_memory','long_context_dump','flat_rag','dag_db_routing','governed_dag_db_routing')),
    deterministic_seed BIGINT NOT NULL CHECK (deterministic_seed >= 0),
    config_hash BYTEA NOT NULL CHECK (octet_length(config_hash) = 32),
    risk_mix_hash BYTEA NOT NULL CHECK (octet_length(risk_mix_hash) = 32),
    corpus_item_count INTEGER NOT NULL CHECK (corpus_item_count > 0),
    quality_score_bp INTEGER NOT NULL CHECK (quality_score_bp >= 0 AND quality_score_bp <= 10000),
    citation_accuracy_bp INTEGER NOT NULL CHECK (citation_accuracy_bp >= 0 AND citation_accuracy_bp <= 10000),
    unsupported_claim_rate_bp INTEGER NOT NULL CHECK (unsupported_claim_rate_bp >= 0 AND unsupported_claim_rate_bp <= 10000),
    prompt_tokens INTEGER NOT NULL CHECK (prompt_tokens >= 0),
    completion_tokens INTEGER NOT NULL CHECK (completion_tokens >= 0),
    overhead_tokens INTEGER NOT NULL CHECK (overhead_tokens >= 0),
    gross_savings_micro_exo BIGINT NOT NULL CHECK (gross_savings_micro_exo >= 0),
    overhead_micro_exo BIGINT NOT NULL CHECK (overhead_micro_exo >= 0),
    net_savings_micro_exo BIGINT NOT NULL CHECK (net_savings_micro_exo >= 0),
    validation_status TEXT NOT NULL CHECK (validation_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0)
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_dagdb_route_validation_report'
          AND conrelid = 'dagdb_route_receipts'::regclass
    ) THEN
        ALTER TABLE dagdb_route_receipts
            ADD CONSTRAINT fk_dagdb_route_validation_report
            FOREIGN KEY (validation_report_id) REFERENCES dagdb_validation_reports(validation_report_id)
            DEFERRABLE INITIALLY IMMEDIATE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_dagdb_route_council_decision'
          AND conrelid = 'dagdb_route_receipts'::regclass
    ) THEN
        ALTER TABLE dagdb_route_receipts
            ADD CONSTRAINT fk_dagdb_route_council_decision
            FOREIGN KEY (council_decision_id) REFERENCES dagdb_council_decisions(decision_id)
            DEFERRABLE INITIALLY IMMEDIATE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_dagdb_context_validation_report'
          AND conrelid = 'dagdb_context_packets'::regclass
    ) THEN
        ALTER TABLE dagdb_context_packets
            ADD CONSTRAINT fk_dagdb_context_validation_report
            FOREIGN KEY (validation_report_id) REFERENCES dagdb_validation_reports(validation_report_id)
            DEFERRABLE INITIALLY IMMEDIATE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_dagdb_context_council_decision'
          AND conrelid = 'dagdb_context_packets'::regclass
    ) THEN
        ALTER TABLE dagdb_context_packets
            ADD CONSTRAINT fk_dagdb_context_council_decision
            FOREIGN KEY (council_decision_id) REFERENCES dagdb_council_decisions(decision_id)
            DEFERRABLE INITIALLY IMMEDIATE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_dagdb_validation_council_decision'
          AND conrelid = 'dagdb_validation_reports'::regclass
    ) THEN
        ALTER TABLE dagdb_validation_reports
            ADD CONSTRAINT fk_dagdb_validation_council_decision
            FOREIGN KEY (council_decision_id) REFERENCES dagdb_council_decisions(decision_id)
            DEFERRABLE INITIALLY IMMEDIATE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_dagdb_route_credential'
          AND conrelid = 'dagdb_route_receipts'::regclass
    ) THEN
        ALTER TABLE dagdb_route_receipts
            ADD CONSTRAINT fk_dagdb_route_credential
            FOREIGN KEY (credential_id) REFERENCES dagdb_inbound_agent_credentials(credential_id)
            DEFERRABLE INITIALLY IMMEDIATE;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_dagdb_receipts_subject ON dagdb_receipts USING btree (tenant_id ASC, namespace ASC, subject_kind ASC, subject_id ASC, seq DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_receipts_event_type ON dagdb_receipts USING btree (tenant_id ASC, namespace ASC, event_type ASC, event_hlc_physical_ms DESC, event_hlc_logical DESC);
CREATE UNIQUE INDEX IF NOT EXISTS uq_dagdb_memory_active_duplicate ON dagdb_memory_objects USING btree (tenant_id ASC, namespace ASC, payload_hash ASC, source_hash ASC, node_type ASC) WHERE status IN ('pending','approved','routable') AND revoked_at_physical_ms IS NULL AND superseded_by_memory_id IS NULL;
CREATE INDEX IF NOT EXISTS idx_dagdb_memory_status ON dagdb_memory_objects USING btree (tenant_id ASC, namespace ASC, status ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_memory_risk ON dagdb_memory_objects USING btree (tenant_id ASC, namespace ASC, risk_class ASC, risk_bp DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_memory_receipt ON dagdb_memory_objects USING btree (latest_receipt_hash ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_memory_dag_finality ON dagdb_memory_objects USING btree (tenant_id ASC, namespace ASC, dag_finality_status ASC, status ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_edges_to_type ON dagdb_memory_edges USING btree (tenant_id ASC, namespace ASC, to_memory_id ASC, edge_type ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_catalog_level ON dagdb_catalog_entries USING btree (tenant_id ASC, namespace ASC, catalog_level ASC, catalog_id ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_catalog_status ON dagdb_catalog_entries USING btree (tenant_id ASC, namespace ASC, status ASC, validation_status ASC, council_status ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_routes_status ON dagdb_route_receipts USING btree (tenant_id ASC, namespace ASC, status ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_routes_task ON dagdb_route_receipts USING btree (tenant_id ASC, namespace ASC, task_signature_hash ASC, route_score_bp DESC, route_id ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_routes_stale ON dagdb_route_receipts USING btree (tenant_id ASC, namespace ASC, stale_at_physical_ms ASC, stale_at_logical ASC) WHERE status = 'active';
CREATE INDEX IF NOT EXISTS idx_dagdb_routes_finality ON dagdb_route_receipts USING btree (tenant_id ASC, namespace ASC, dag_finality_status ASC, status ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_packets_validation ON dagdb_context_packets USING btree (tenant_id ASC, namespace ASC, validation_status ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_packets_request ON dagdb_context_packets USING btree (tenant_id ASC, namespace ASC, request_id ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_packets_finality ON dagdb_context_packets USING btree (tenant_id ASC, namespace ASC, dag_finality_status ASC, validation_status ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_validation_subject ON dagdb_validation_reports USING btree (tenant_id ASC, namespace ASC, subject_kind ASC, subject_id ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_validation_status ON dagdb_validation_reports USING btree (tenant_id ASC, namespace ASC, validation_status ASC, risk_bp DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_safety_agent_window ON dagdb_agent_safety_scores USING btree (tenant_id ASC, namespace ASC, agent_did ASC, window_end_physical_ms DESC, window_end_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_credentials_agent ON dagdb_inbound_agent_credentials USING btree (tenant_id ASC, namespace ASC, agent_did ASC, credential_status ASC, expires_at_physical_ms ASC, expires_at_logical ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_council_subject ON dagdb_council_decisions USING btree (tenant_id ASC, namespace ASC, subject_kind ASC, subject_id ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_council_status ON dagdb_council_decisions USING btree (tenant_id ASC, namespace ASC, decision_status ASC, risk_class ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_council_expiry ON dagdb_council_decisions USING btree (tenant_id ASC, namespace ASC, expires_at_physical_ms ASC, expires_at_logical ASC) WHERE decision_status IN ('approved','escalated');
CREATE INDEX IF NOT EXISTS idx_dagdb_idempotency_expires ON dagdb_idempotency_keys USING btree (expires_at_physical_ms ASC, expires_at_logical ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_outbox_status_next ON dagdb_dag_outbox USING btree (dag_finality_status ASC, next_attempt_at_physical_ms ASC, next_attempt_at_logical ASC, attempt_count ASC) WHERE dag_finality_status IN ('pending','failed');
CREATE INDEX IF NOT EXISTS idx_dagdb_outbox_subject ON dagdb_dag_outbox USING btree (tenant_id ASC, namespace ASC, subject_kind ASC, subject_id ASC);
CREATE INDEX IF NOT EXISTS idx_dagdb_benchmark_fixture ON dagdb_benchmark_runs USING btree (fixture_id ASC, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_benchmark_runner ON dagdb_benchmark_runs USING btree (runner_name ASC, fixture_id ASC, deterministic_seed ASC);
