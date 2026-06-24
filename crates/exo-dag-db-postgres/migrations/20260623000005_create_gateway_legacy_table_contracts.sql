CREATE TABLE IF NOT EXISTS users (
    did TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    roles JSONB NOT NULL DEFAULT '[]',
    tenant_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    pace_status TEXT NOT NULL DEFAULT 'Unenrolled',
    password_hash TEXT NOT NULL,
    salt TEXT NOT NULL,
    mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);
CREATE INDEX IF NOT EXISTS idx_users_tenant_created_at ON users(tenant_id, created_at);

CREATE TABLE IF NOT EXISTS agents (
    did TEXT PRIMARY KEY,
    agent_name TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    owner_did TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    capabilities JSONB NOT NULL DEFAULT '[]',
    trust_tier TEXT NOT NULL DEFAULT 'Untrusted',
    trust_score INTEGER NOT NULL DEFAULT 0,
    delegation_id TEXT,
    pace_status TEXT NOT NULL DEFAULT 'Unenrolled',
    created_at BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    max_decision_class TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agents_tenant_created_at ON agents(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_agents_created_at ON agents(created_at);

CREATE TABLE IF NOT EXISTS decisions (
    id_hash TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    status TEXT NOT NULL,
    title TEXT NOT NULL,
    decision_class TEXT NOT NULL,
    author TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    constitution_version TEXT NOT NULL,
    payload JSONB NOT NULL,
    PRIMARY KEY (tenant_id, id_hash)
);

CREATE INDEX IF NOT EXISTS idx_decisions_status ON decisions(status);
CREATE INDEX IF NOT EXISTS idx_decisions_tenant_created_at_ms ON decisions(tenant_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_decisions_created_at_ms ON decisions(created_at_ms);

CREATE TABLE IF NOT EXISTS delegations (
    id_hash TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    delegator TEXT NOT NULL,
    delegatee TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    revoked_at BIGINT,
    constitution_version TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_delegations_delegator ON delegations(delegator);
CREATE INDEX IF NOT EXISTS idx_delegations_delegatee ON delegations(delegatee);
CREATE INDEX IF NOT EXISTS idx_delegations_created_at_ms ON delegations(created_at_ms);
CREATE INDEX IF NOT EXISTS idx_delegations_active_delegatee
    ON delegations(delegatee)
    WHERE revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_delegations_active_delegator
    ON delegations(delegator)
    WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS audit_entries (
    sequence BIGINT PRIMARY KEY,
    prev_hash TEXT NOT NULL,
    event_hash TEXT NOT NULL,
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    decision_id TEXT NOT NULL DEFAULT '',
    timestamp_physical_ms BIGINT NOT NULL,
    timestamp_logical INTEGER NOT NULL DEFAULT 0,
    entry_hash TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_entries_actor_event_type
    ON audit_entries(actor, event_type);
CREATE INDEX IF NOT EXISTS idx_audit_entries_decision_id
    ON audit_entries(decision_id);
CREATE INDEX IF NOT EXISTS idx_audit_entries_decision_tenant_sequence
    ON audit_entries(decision_id, tenant_id, sequence);

CREATE TABLE IF NOT EXISTS constitutions (
    tenant_id TEXT NOT NULL,
    version TEXT NOT NULL,
    payload JSONB NOT NULL,
    PRIMARY KEY (tenant_id, version)
);

CREATE TABLE IF NOT EXISTS identity_scores (
    did TEXT PRIMARY KEY,
    score INTEGER NOT NULL,
    tier TEXT NOT NULL,
    factors JSONB NOT NULL,
    last_updated BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS enrollment_log (
    id BIGSERIAL PRIMARY KEY,
    did TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    step TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    verified_by TEXT NOT NULL,
    audit_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS hlc_state (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    counter BIGINT NOT NULL DEFAULT 1000
);

INSERT INTO hlc_state (counter) VALUES (1000) ON CONFLICT DO NOTHING;

CREATE TABLE IF NOT EXISTS livesafe_identities (
    did TEXT PRIMARY KEY,
    odentity_composite_basis_points INTEGER NOT NULL DEFAULT 0,
    pace_status TEXT NOT NULL DEFAULT 'Incomplete',
    card_status TEXT NOT NULL DEFAULT 'NotIssued',
    created_at_ms BIGINT NOT NULL,
    exochain_anchor TEXT
);

CREATE TABLE IF NOT EXISTS scan_receipts (
    scan_id TEXT PRIMARY KEY,
    subscriber_did TEXT NOT NULL,
    responder_did TEXT NOT NULL,
    location TEXT,
    scanned_at_ms BIGINT NOT NULL,
    consent_expires_at_ms BIGINT NOT NULL,
    audit_receipt_hash TEXT NOT NULL,
    anchor_receipt TEXT
);

CREATE INDEX IF NOT EXISTS idx_scans_subscriber ON scan_receipts(subscriber_did);

CREATE TABLE IF NOT EXISTS consent_anchors (
    consent_id TEXT PRIMARY KEY,
    subscriber_did TEXT NOT NULL,
    provider_did TEXT NOT NULL,
    scope JSONB NOT NULL DEFAULT '[]',
    granted_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT,
    revoked_at_ms BIGINT,
    audit_receipt_hash TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_consent_subscriber ON consent_anchors(subscriber_did);

CREATE TABLE IF NOT EXISTS trustee_shard_status (
    id BIGSERIAL PRIMARY KEY,
    subscriber_did TEXT NOT NULL,
    trustee_did TEXT NOT NULL,
    role TEXT NOT NULL,
    shard_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    accepted_at_ms BIGINT
);

CREATE INDEX IF NOT EXISTS idx_shard_subscriber ON trustee_shard_status(subscriber_did);

CREATE TABLE IF NOT EXISTS sessions (
    token TEXT PRIMARY KEY,
    actor_did TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_sessions_actor_did ON sessions(actor_did);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

CREATE TABLE IF NOT EXISTS agent_roles (
    agent_did TEXT NOT NULL,
    role TEXT NOT NULL,
    branch TEXT NOT NULL DEFAULT 'executive',
    granted_by TEXT NOT NULL,
    valid_from BIGINT NOT NULL,
    expires_at BIGINT,
    PRIMARY KEY (agent_did, role)
);

CREATE INDEX IF NOT EXISTS idx_agent_roles_agent_did ON agent_roles(agent_did);

CREATE TABLE IF NOT EXISTS consent_records (
    subject_did TEXT NOT NULL,
    actor_did TEXT NOT NULL,
    scope TEXT NOT NULL,
    bailment_type TEXT NOT NULL DEFAULT 'standard',
    status TEXT NOT NULL DEFAULT 'active',
    created_at BIGINT NOT NULL,
    expires_at BIGINT,
    PRIMARY KEY (subject_did, actor_did, scope)
);

CREATE INDEX IF NOT EXISTS idx_consent_records_actor ON consent_records(actor_did, status);
CREATE INDEX IF NOT EXISTS idx_consent_records_subject ON consent_records(subject_did);

CREATE TABLE IF NOT EXISTS authority_chains (
    actor_did TEXT NOT NULL,
    chain_json JSONB NOT NULL,
    valid_from BIGINT NOT NULL,
    expires_at BIGINT,
    PRIMARY KEY (actor_did, valid_from)
);

CREATE INDEX IF NOT EXISTS idx_authority_chains_actor ON authority_chains(actor_did);

CREATE TABLE IF NOT EXISTS layout_templates (
    id TEXT PRIMARY KEY,
    user_did TEXT,
    name TEXT NOT NULL,
    layout_json JSONB NOT NULL,
    hidden_panels JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_built_in BOOLEAN NOT NULL DEFAULT FALSE,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_layout_templates_user ON layout_templates(user_did);

CREATE TABLE IF NOT EXISTS feedback_issues (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    severity TEXT NOT NULL DEFAULT 'medium',
    category TEXT NOT NULL DEFAULT 'bug',
    status TEXT NOT NULL DEFAULT 'open',
    source_widget_id TEXT NOT NULL,
    source_module_type TEXT NOT NULL DEFAULT '',
    reporter_did TEXT,
    assigned_agent_team TEXT,
    widget_state JSONB,
    browser_info JSONB,
    resolution_notes TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_feedback_issues_status ON feedback_issues(status);
CREATE INDEX IF NOT EXISTS idx_feedback_issues_widget ON feedback_issues(source_widget_id);
CREATE INDEX IF NOT EXISTS idx_feedback_issues_severity ON feedback_issues(severity);

CREATE TABLE IF NOT EXISTS conflict_declarations (
    id_hash TEXT PRIMARY KEY,
    declarant_did TEXT NOT NULL,
    nature TEXT NOT NULL,
    related_dids JSONB NOT NULL,
    timestamp_physical_ms BIGINT NOT NULL CHECK (timestamp_physical_ms > 0),
    timestamp_logical INTEGER NOT NULL DEFAULT 0 CHECK (timestamp_logical >= 0),
    payload JSONB NOT NULL,
    CHECK (length(trim(nature)) > 0),
    CHECK (jsonb_typeof(related_dids) = 'array'),
    CHECK (jsonb_array_length(related_dids) > 0)
);

CREATE INDEX IF NOT EXISTS idx_conflict_declarations_declarant
    ON conflict_declarations(declarant_did);

CREATE TABLE IF NOT EXISTS did_documents (
    did TEXT PRIMARY KEY,
    document JSONB NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    erased_at_ms BIGINT
);

CREATE INDEX IF NOT EXISTS idx_did_documents_active_did
    ON did_documents(did)
    WHERE revoked = false;
CREATE INDEX IF NOT EXISTS idx_did_documents_erased_at_ms
    ON did_documents(erased_at_ms)
    WHERE erased_at_ms IS NOT NULL;

CREATE TABLE IF NOT EXISTS avc_registry_state (
    registry_key TEXT PRIMARY KEY,
    state_cbor BYTEA NOT NULL,
    CONSTRAINT avc_registry_state_singleton CHECK (registry_key = 'default')
);
