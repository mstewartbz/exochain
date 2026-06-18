CREATE TABLE IF NOT EXISTS dagdb_graph_nodes (
    graph_node_id BYTEA PRIMARY KEY CHECK (octet_length(graph_node_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    graph_style TEXT NOT NULL CHECK (graph_style IN ('provenance_receipt_dag','canonical_memory_graph','semantic_catalog_graph','similarity_overlay_graph','dependency_dag','routing_view_graph','contradiction_supersession_graph','context_packet_graph')),
    node_kind TEXT NOT NULL CHECK (node_kind IN ('raw','chunk','summary','concept','canonical','duplicate_reference','related','replacement','contradiction','supersession','alternate_summary','decision','route','validation_report','savings_report')),
    canonical_memory_id BYTEA NULL REFERENCES dagdb_memory_objects(memory_id),
    catalog_path TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    UNIQUE (tenant_id, namespace, graph_style, memory_id, node_kind)
);

CREATE TABLE IF NOT EXISTS dagdb_graph_edges (
    graph_edge_id BYTEA PRIMARY KEY CHECK (octet_length(graph_edge_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    graph_style TEXT NOT NULL CHECK (graph_style IN ('provenance_receipt_dag','canonical_memory_graph','semantic_catalog_graph','similarity_overlay_graph','dependency_dag','routing_view_graph','contradiction_supersession_graph','context_packet_graph')),
    from_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    to_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    edge_kind TEXT NOT NULL CHECK (edge_kind IN ('derived_from','summarizes','supports','contradicts','supersedes','replaces','duplicate_of','near_duplicate_of','related_to','alternative_summary_of','depends_on','part_of','owned_by','access_granted_by','verified_by','used_by_route','included_in_context_packet','revoked_by')),
    receipt_hash BYTEA NULL REFERENCES dagdb_receipts(receipt_hash),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    UNIQUE (tenant_id, namespace, graph_style, from_memory_id, to_memory_id, edge_kind)
);

CREATE TABLE IF NOT EXISTS dagdb_graph_similarity_results (
    similarity_result_id BYTEA PRIMARY KEY CHECK (octet_length(similarity_result_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    candidate_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    matched_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    similarity_type TEXT NOT NULL CHECK (similarity_type IN ('exact_hash','near_duplicate','concept_overlap','weak_related')),
    similarity_bp INTEGER NOT NULL CHECK (similarity_bp >= 0 AND similarity_bp <= 10000),
    matched_fields JSONB NOT NULL DEFAULT '[]'::jsonb,
    reason TEXT NOT NULL CHECK (length(reason) > 0),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    UNIQUE (tenant_id, namespace, candidate_memory_id, matched_memory_id, similarity_type)
);

CREATE TABLE IF NOT EXISTS dagdb_graph_canonicalization_decisions (
    decision_id BYTEA PRIMARY KEY CHECK (octet_length(decision_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    input_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    canonical_memory_id BYTEA NULL REFERENCES dagdb_memory_objects(memory_id),
    matched_memory_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    decision_kind TEXT NOT NULL CHECK (decision_kind IN ('new_canonical','exact_duplicate','near_duplicate','related','replacement','contradiction','supersession','alternate_summary','rejected_needs_review')),
    decision_reason TEXT NOT NULL CHECK (length(decision_reason) > 0),
    confidence_bp INTEGER NOT NULL CHECK (confidence_bp >= 0 AND confidence_bp <= 10000),
    risk_class TEXT NOT NULL CHECK (risk_class IN ('R0','R1','R2','R3','R4','R5')),
    validator_status TEXT NOT NULL CHECK (validator_status IN ('not_required','pending','passed','failed','contradictory','expired','needs_council')),
    required_edges_to_create JSONB NOT NULL DEFAULT '[]'::jsonb,
    receipt_hash BYTEA NULL REFERENCES dagdb_receipts(receipt_hash),
    receipt_intent TEXT NULL,
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0)
);

CREATE TABLE IF NOT EXISTS dagdb_graph_views (
    view_id BYTEA PRIMARY KEY CHECK (octet_length(view_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    graph_style TEXT NOT NULL CHECK (graph_style IN ('provenance_receipt_dag','canonical_memory_graph','semantic_catalog_graph','similarity_overlay_graph','dependency_dag','routing_view_graph','contradiction_supersession_graph','context_packet_graph')),
    source_root_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    included_node_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    included_edge_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    view_type TEXT NOT NULL CHECK (view_type IN ('full_provenance','routing_view','canonical_view','dependency_view','contradiction_view','context_packet_view')),
    topological_order JSONB NOT NULL DEFAULT '[]'::jsonb,
    transitive_reduction_edges JSONB NOT NULL DEFAULT '[]'::jsonb,
    omitted_edges JSONB NOT NULL DEFAULT '[]'::jsonb,
    reason_edges_omitted JSONB NOT NULL DEFAULT '[]'::jsonb,
    source_records_hash BYTEA NOT NULL CHECK (octet_length(source_records_hash) = 32),
    stale BOOLEAN NOT NULL DEFAULT false,
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    refreshed_at_physical_ms BIGINT NOT NULL CHECK (refreshed_at_physical_ms >= 0),
    refreshed_at_logical INTEGER NOT NULL CHECK (refreshed_at_logical >= 0)
);

CREATE TABLE IF NOT EXISTS dagdb_graph_placement_traces (
    placement_trace_id BYTEA PRIMARY KEY CHECK (octet_length(placement_trace_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    input_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    trace_steps JSONB NOT NULL,
    completed BOOLEAN NOT NULL DEFAULT false,
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0)
);

CREATE TABLE IF NOT EXISTS dagdb_graph_route_invalidations (
    invalidation_id BYTEA PRIMARY KEY CHECK (octet_length(invalidation_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    route_id BYTEA NOT NULL REFERENCES dagdb_route_receipts(route_id),
    affected_memory_ids JSONB NOT NULL,
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('revoked','superseded','contradicted','replaced','permission_changed','risk_changed')),
    triggering_receipt_id BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash),
    prior_route_status TEXT NOT NULL CHECK (prior_route_status IN ('pending','active','stale','invalidated','blocked')),
    new_route_status TEXT NOT NULL CHECK (new_route_status IN ('stale','invalidated','needs_review','superseded')),
    invalidation_reason TEXT NOT NULL CHECK (length(invalidation_reason) > 0),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    validator_id TEXT NULL CHECK (validator_id IS NULL OR (length(validator_id) > 0 AND validator_id LIKE 'did:%')),
    validation_report_id BYTEA NULL REFERENCES dagdb_validation_reports(validation_report_id),
    receipt_hash BYTEA NULL REFERENCES dagdb_receipts(receipt_hash),
    receipt_intent TEXT NOT NULL DEFAULT 'route_invalidated'
);

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_nodes_memory ON dagdb_graph_nodes USING btree (tenant_id, namespace, memory_id, graph_style);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_nodes_canonical ON dagdb_graph_nodes USING btree (tenant_id, namespace, canonical_memory_id) WHERE canonical_memory_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_edges_from_kind ON dagdb_graph_edges USING btree (tenant_id, namespace, from_memory_id, edge_kind);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_edges_to_kind ON dagdb_graph_edges USING btree (tenant_id, namespace, to_memory_id, edge_kind);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_edges_style ON dagdb_graph_edges USING btree (tenant_id, namespace, graph_style);

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_similarity_candidate ON dagdb_graph_similarity_results USING btree (tenant_id, namespace, candidate_memory_id, similarity_bp DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_similarity_matched ON dagdb_graph_similarity_results USING btree (tenant_id, namespace, matched_memory_id, similarity_bp DESC);

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_canon_input ON dagdb_graph_canonicalization_decisions USING btree (tenant_id, namespace, input_memory_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_canon_canonical ON dagdb_graph_canonicalization_decisions USING btree (tenant_id, namespace, canonical_memory_id) WHERE canonical_memory_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_canon_kind ON dagdb_graph_canonicalization_decisions USING btree (tenant_id, namespace, decision_kind, created_at_physical_ms DESC, created_at_logical DESC);

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_views_root_style ON dagdb_graph_views USING btree (tenant_id, namespace, source_root_id, graph_style, stale);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_views_stale ON dagdb_graph_views USING btree (tenant_id, namespace, stale, refreshed_at_physical_ms, refreshed_at_logical) WHERE stale = true;

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_placement_input ON dagdb_graph_placement_traces USING btree (tenant_id, namespace, input_memory_id, created_at_physical_ms DESC, created_at_logical DESC);

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_route_invalidations_route ON dagdb_graph_route_invalidations USING btree (tenant_id, namespace, route_id, created_at_physical_ms DESC, created_at_logical DESC);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_route_invalidations_trigger ON dagdb_graph_route_invalidations USING btree (tenant_id, namespace, trigger_type, created_at_physical_ms DESC, created_at_logical DESC);
