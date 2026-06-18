CREATE TABLE IF NOT EXISTS dagdb_lifecycle_rollbacks (
    rollback_id TEXT PRIMARY KEY CHECK (length(rollback_id) > 0),
    action_id TEXT NOT NULL CHECK (length(action_id) > 0),
    inverse_action_type TEXT NOT NULL CHECK (inverse_action_type IN ('writeback','relink','supersede','recycle','archive','restore','route_invalidate')),
    before_refs JSONB NOT NULL,
    after_refs JSONB NOT NULL,
    validation_ref TEXT NOT NULL CHECK (length(validation_ref) > 0),
    operator_required BOOLEAN NOT NULL,
    rollback_body JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS dagdb_lifecycle_actions (
    action_id TEXT PRIMARY KEY CHECK (length(action_id) > 0),
    action_type TEXT NOT NULL CHECK (action_type IN ('writeback','relink','supersede','recycle','archive','restore','route_invalidate')),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    project_id TEXT NOT NULL CHECK (length(project_id) > 0),
    memory_namespace TEXT NOT NULL CHECK (length(memory_namespace) > 0),
    actor_id TEXT NOT NULL CHECK (length(actor_id) > 0),
    source_packet_id TEXT NOT NULL CHECK (length(source_packet_id) > 0),
    source_receipt_id TEXT NOT NULL CHECK (length(source_receipt_id) > 0),
    target_memory_ids JSONB NOT NULL,
    parent_memory_ids JSONB NOT NULL,
    validation_report_id TEXT NOT NULL CHECK (length(validation_report_id) > 0),
    policy_ref TEXT NOT NULL CHECK (length(policy_ref) > 0),
    rollback_id TEXT NOT NULL REFERENCES dagdb_lifecycle_rollbacks(rollback_id),
    route_invalidation_event_ids JSONB NOT NULL,
    evidence_refs JSONB NOT NULL,
    terminal_state TEXT NOT NULL CHECK (terminal_state IN ('accepted','honest_blocked','operator_deferred','failed_validation')),
    production_lifecycle_approval TEXT NOT NULL CHECK (production_lifecycle_approval IN ('approved','operator_deferred')),
    idempotency_key TEXT NOT NULL UNIQUE CHECK (length(idempotency_key) > 0),
    action_body JSONB NOT NULL,
    created_at TEXT NOT NULL CHECK (length(created_at) > 0)
);

CREATE TABLE IF NOT EXISTS dagdb_route_invalidation_events (
    event_id TEXT PRIMARY KEY CHECK (length(event_id) > 0),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    project_id TEXT NOT NULL CHECK (length(project_id) > 0),
    memory_namespace TEXT NOT NULL CHECK (length(memory_namespace) > 0),
    route_id TEXT NOT NULL CHECK (length(route_id) > 0),
    source_action_id TEXT NOT NULL CHECK (length(source_action_id) > 0),
    impacted_memory_ids JSONB NOT NULL,
    reason TEXT NOT NULL CHECK (length(reason) > 0),
    invalidated_packet_ids JSONB NOT NULL,
    freshness_state_before TEXT NOT NULL CHECK (freshness_state_before = 'current'),
    freshness_state_after TEXT NOT NULL CHECK (freshness_state_after = 'stale'),
    retrieval_readiness_impact TEXT NOT NULL CHECK (retrieval_readiness_impact = 'reject_until_rebuilt'),
    validation_report_id TEXT NOT NULL CHECK (length(validation_report_id) > 0),
    rollback_ref TEXT NOT NULL CHECK (length(rollback_ref) > 0),
    idempotency_key TEXT NOT NULL UNIQUE CHECK (length(idempotency_key) > 0),
    event_body JSONB NOT NULL,
    created_at TEXT NOT NULL CHECK (length(created_at) > 0)
);

CREATE TABLE IF NOT EXISTS dagdb_continuation_records (
    continuation_id TEXT PRIMARY KEY CHECK (length(continuation_id) > 0),
    task_id TEXT NOT NULL CHECK (length(task_id) > 0),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    project_id TEXT NOT NULL CHECK (length(project_id) > 0),
    memory_namespace TEXT NOT NULL CHECK (length(memory_namespace) > 0),
    summary_ref TEXT NOT NULL CHECK (length(summary_ref) > 0),
    memory_refs JSONB NOT NULL,
    blocker_refs JSONB NOT NULL,
    validation_refs JSONB NOT NULL,
    expiry_epoch_seconds BIGINT NOT NULL CHECK (expiry_epoch_seconds > 0),
    later_retrieval_status TEXT NOT NULL CHECK (later_retrieval_status IN ('pending','retrieved','expired_rejected')),
    production_lifecycle_approval TEXT NOT NULL CHECK (production_lifecycle_approval IN ('approved','operator_deferred')),
    idempotency_key TEXT NOT NULL UNIQUE CHECK (length(idempotency_key) > 0),
    record_body JSONB NOT NULL,
    created_at TEXT NOT NULL CHECK (length(created_at) > 0)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_lifecycle_actions_scope ON dagdb_lifecycle_actions USING btree (tenant_id, project_id, memory_namespace, action_type);
CREATE INDEX IF NOT EXISTS idx_dagdb_lifecycle_actions_terminal ON dagdb_lifecycle_actions USING btree (terminal_state, production_lifecycle_approval);
CREATE INDEX IF NOT EXISTS idx_dagdb_lifecycle_rollbacks_action ON dagdb_lifecycle_rollbacks USING btree (action_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_route_invalidation_events_route ON dagdb_route_invalidation_events USING btree (tenant_id, project_id, memory_namespace, route_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_route_invalidation_events_action ON dagdb_route_invalidation_events USING btree (source_action_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_continuation_records_task ON dagdb_continuation_records USING btree (tenant_id, project_id, memory_namespace, task_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_continuation_records_expiry ON dagdb_continuation_records USING btree (expiry_epoch_seconds);
