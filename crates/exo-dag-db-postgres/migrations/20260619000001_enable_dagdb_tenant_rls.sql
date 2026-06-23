CREATE OR REPLACE FUNCTION dagdb_current_tenant_id()
RETURNS TEXT
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    bound_tenant_id TEXT;
BEGIN
    bound_tenant_id := current_setting('exo.tenant_id');
    IF bound_tenant_id = '' THEN
        RAISE EXCEPTION 'exo.tenant_id is not set'
            USING ERRCODE = '42501';
    END IF;
    RETURN bound_tenant_id;
END $$;

DO $$
DECLARE
    tenant_table TEXT;
BEGIN
    FOREACH tenant_table IN ARRAY ARRAY[
        'dagdb_receipts',
        'dagdb_subject_receipt_heads',
        'dagdb_memory_objects',
        'dagdb_memory_edges',
        'dagdb_catalog_entries',
        'dagdb_route_receipts',
        'dagdb_context_packets',
        'dagdb_validation_reports',
        'dagdb_agent_safety_scores',
        'dagdb_inbound_agent_credentials',
        'dagdb_council_decisions',
        'dagdb_idempotency_keys',
        'dagdb_dag_outbox',
        'dagdb_graph_nodes',
        'dagdb_graph_edges',
        'dagdb_graph_similarity_results',
        'dagdb_graph_canonicalization_decisions',
        'dagdb_graph_views',
        'dagdb_graph_placement_traces',
        'dagdb_graph_route_invalidations',
        'dagdb_exports',
        'dagdb_export_challenges',
        'dagdb_graph_edge_tombstones',
        'dagdb_graph_layers',
        'dagdb_graph_layer_memberships',
        'dagdb_graph_layer_edges',
        'dagdb_default_routes',
        'dagdb_context_packet_records',
        'dagdb_lifecycle_actions',
        'dagdb_route_invalidation_events',
        'dagdb_continuation_records',
        'dagdb_node_dag_nodes',
        'dagdb_node_dag_parents',
        'dagdb_node_committed',
        'dagdb_node_consensus_meta',
        'dagdb_node_consensus_votes',
        'dagdb_node_commit_certificates',
        'dagdb_node_validators',
        'dagdb_node_trust_receipts',
        'dagdb_node_economy_objects',
        'dagdb_node_economy_anchors',
        'dagdb_node_economy_meta'
    ] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', tenant_table);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', tenant_table);
        EXECUTE format('DROP POLICY IF EXISTS dagdb_tenant_isolation ON %I', tenant_table);
        EXECUTE format(
            'CREATE POLICY dagdb_tenant_isolation ON %I USING (tenant_id = dagdb_current_tenant_id()) WITH CHECK (tenant_id = dagdb_current_tenant_id())',
            tenant_table
        );
    END LOOP;
END $$;
