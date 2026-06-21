-- PRD17B default-route activation/readiness schema.
-- Route invalidation mutation remains PRD17C-owned.

CREATE TABLE IF NOT EXISTS dagdb_default_routes (
  tenant_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  memory_namespace TEXT NOT NULL,
  route_id TEXT NOT NULL,
  status TEXT NOT NULL,
  route_source TEXT NOT NULL,
  policy_ref TEXT NOT NULL,
  freshness_ref TEXT NOT NULL,
  policy_allowed BOOLEAN NOT NULL,
  freshness_status TEXT NOT NULL,
  invalidated BOOLEAN NOT NULL DEFAULT FALSE,
  production_default_route_approval_status TEXT NOT NULL,
  packet_quality_review_status TEXT NOT NULL,
  selected_memory_refs JSONB NOT NULL,
  selected_memory_ref_count INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (tenant_id, project_id, memory_namespace, route_id),
  CONSTRAINT dagdb_default_routes_scope_non_empty CHECK (
    length(trim(tenant_id)) > 0
    AND length(trim(project_id)) > 0
    AND length(trim(memory_namespace)) > 0
    AND length(trim(route_id)) > 0
  ),
  CONSTRAINT dagdb_default_routes_selected_count_non_negative CHECK (
    selected_memory_ref_count >= 0
  ),
  CONSTRAINT dagdb_default_routes_status_check CHECK (
    status IN (
      'active',
      'forbidden',
      'stale',
      'invalidated',
      'preview_only',
      'dry_run_only',
      'non_default'
    )
  ),
  CONSTRAINT dagdb_default_routes_source_check CHECK (
    route_source IN ('persisted', 'preview', 'dry_run', 'target_artifact')
  ),
  CONSTRAINT dagdb_default_routes_freshness_check CHECK (
    freshness_status IN (
      'current',
      'stale_memory',
      'stale_catalog',
      'stale_validation',
      'route_invalidated',
      'unknown'
    )
  ),
  CONSTRAINT dagdb_default_routes_no_raw_refs CHECK (
    selected_memory_refs::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'body' || '%')
    AND selected_memory_refs::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'markdown' || '%')
    AND selected_memory_refs::TEXT NOT ILIKE ('%' || 'raw' || '_' || 'private' || '_' || 'payload' || '%')
    AND selected_memory_refs::TEXT NOT ILIKE ('%' || 'source' || '_' || 'excerpt' || '%')
    AND selected_memory_refs::TEXT NOT ILIKE '%/Users/%'
    AND selected_memory_refs::TEXT NOT ILIKE '%DATABASE_URL%'
    AND selected_memory_refs::TEXT NOT ILIKE '%postgres://%'
    AND selected_memory_refs::TEXT NOT ILIKE '%postgresql://%'
  )
);

CREATE INDEX IF NOT EXISTS dagdb_default_routes_active_idx
  ON dagdb_default_routes (tenant_id, project_id, memory_namespace, status, freshness_status)
  WHERE status = 'active';
