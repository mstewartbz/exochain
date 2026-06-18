-- PRD17B context-packet persistence/proof-binding schema.

CREATE TABLE IF NOT EXISTS dagdb_context_packet_records (
  packet_id TEXT PRIMARY KEY,
  route_id TEXT NOT NULL,
  query_hash TEXT NOT NULL,
  tenant_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  memory_namespace TEXT NOT NULL,
  selected_memory_ids JSONB NOT NULL,
  selected_edge_ids JSONB NOT NULL,
  token_budget INTEGER NOT NULL,
  token_estimate INTEGER NOT NULL,
  context_quality TEXT NOT NULL,
  citation_coverage_bp INTEGER NOT NULL,
  validation_coverage_bp INTEGER NOT NULL,
  freshness_status TEXT NOT NULL,
  validation_status TEXT NOT NULL,
  source_proof_refs JSONB NOT NULL,
  fallback_reason TEXT,
  idempotency_key TEXT NOT NULL,
  persistence_status TEXT NOT NULL,
  production_default_route_approval_status TEXT NOT NULL,
  packet_quality_review_status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE (tenant_id, project_id, memory_namespace, route_id, query_hash, token_budget),
  UNIQUE (tenant_id, project_id, memory_namespace, idempotency_key),
  CONSTRAINT dagdb_context_packet_scope_non_empty CHECK (
    length(trim(packet_id)) > 0
    AND length(trim(route_id)) > 0
    AND length(trim(query_hash)) > 0
    AND length(trim(tenant_id)) > 0
    AND length(trim(project_id)) > 0
    AND length(trim(memory_namespace)) > 0
  ),
  CONSTRAINT dagdb_context_packet_budget_check CHECK (
    token_budget > 0
    AND token_estimate > 0
    AND token_estimate <= token_budget
  ),
  CONSTRAINT dagdb_context_packet_bp_check CHECK (
    citation_coverage_bp BETWEEN 0 AND 10000
    AND validation_coverage_bp BETWEEN 0 AND 10000
  ),
  CONSTRAINT dagdb_context_packet_quality_check CHECK (
    context_quality IN (
      'usable_context',
      'empty_context',
      'over_budget',
      'stale_context',
      'forbidden_route',
      'gateway_unavailable',
      'raw_fallback'
    )
  ),
  CONSTRAINT dagdb_context_packet_freshness_check CHECK (
    freshness_status IN (
      'current',
      'stale_memory',
      'stale_catalog',
      'stale_validation',
      'route_invalidated',
      'unknown'
    )
  ),
  CONSTRAINT dagdb_context_packet_validation_check CHECK (
    validation_status IN ('passed', 'failed', 'stale', 'missing', 'operator_deferred')
  ),
  CONSTRAINT dagdb_context_packet_persistence_check CHECK (
    persistence_status IN ('persisted', 'proof_bound', 'preview_only', 'dry_run_only', 'target_artifact_only')
  ),
  CONSTRAINT dagdb_context_packet_no_raw_refs CHECK (
    selected_memory_ids::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'body' || '%')
    AND selected_memory_ids::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'markdown' || '%')
    AND selected_memory_ids::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'private' || '_' || 'payload' || '%')
    AND selected_memory_ids::TEXT NOT ILIKE ('%' || 'source' || '_' || 'excerpt' || '%')
    AND selected_memory_ids::TEXT NOT ILIKE '%/Users/%'
    AND selected_memory_ids::TEXT NOT ILIKE '%DATABASE_URL%'
    AND source_proof_refs::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'body' || '%')
    AND source_proof_refs::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'markdown' || '%')
    AND source_proof_refs::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'private' || '_' || 'payload' || '%')
    AND source_proof_refs::TEXT NOT ILIKE ('%' || 'source' || '_' || 'excerpt' || '%')
    AND source_proof_refs::TEXT NOT ILIKE '%/Users/%'
    AND source_proof_refs::TEXT NOT ILIKE '%DATABASE_URL%'
    AND COALESCE(fallback_reason, '') NOT ILIKE ('%' || 'raw' || '_' || 'body' || '%')
    AND COALESCE(fallback_reason, '') NOT ILIKE ('%' || 'raw' || '_' || 'markdown' || '%')
    AND COALESCE(fallback_reason, '') NOT ILIKE ('%' || 'source' || '_' || 'excerpt' || '%')
  )
);

CREATE INDEX IF NOT EXISTS dagdb_context_packet_records_route_idx
  ON dagdb_context_packet_records (tenant_id, project_id, memory_namespace, route_id);
