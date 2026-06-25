-- Expand dagdb_receipts_event_type_check with 11 new operational event types.
-- This is an additive ALTER that supersedes the constraint last set in
-- 20260511000001_create_dagdb_export_persistence_schema.sql and extended by
-- 20260620000001_add_dagdb_operational_receipt_event_types.sql.
ALTER TABLE dagdb_receipts DROP CONSTRAINT IF EXISTS dagdb_receipts_event_type_check;
ALTER TABLE dagdb_receipts
    ADD CONSTRAINT dagdb_receipts_event_type_check
    CHECK (event_type IN ('intake_created','duplicate_rejected','validation_created','validation_passed','validation_failed','memory_approved','memory_routable','memory_revoked','memory_superseded','route_created','route_activated','route_stale','route_invalidated','context_packet_created','writeback_created','trust_check_created','council_decision_recorded','dag_finality_committed','dag_finality_failed','dag_finality_compensated','export_created','export_verified','export_failed','export_challenge_created','export_challenge_verified','dagdb_approval_request_submitted','dagdb_approval_granted','dagdb_approval_denied','dagdb_record_accepted','dagdb_import_completed','dagdb_export_completed','dagdb_replay_detected','dagdb_idempotency_conflict','dagdb_rls_tenant_violation','dagdb_signature_failure','dagdb_council_operator_decision'));

-- Enable tenant RLS on the 13 new DAG DB tables introduced after the original
-- 20260619000001_enable_dagdb_tenant_rls.sql migration was applied.  A NULL
-- guard is used so that this block is safe to run even if a table has not yet
-- been created (e.g. on a fresh schema that runs all migrations in order).
DO $$
DECLARE
    tenant_table TEXT;
BEGIN
    FOREACH tenant_table IN ARRAY ARRAY[
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
        'dagdb_node_economy_meta',
        'dagdb_zerodentity_records',
        'dagdb_gateway_state_records'
    ] LOOP
        IF to_regclass(tenant_table) IS NULL THEN
            CONTINUE;
        END IF;
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', tenant_table);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', tenant_table);
        EXECUTE format('DROP POLICY IF EXISTS dagdb_tenant_isolation ON %I', tenant_table);
        EXECUTE format(
            'CREATE POLICY dagdb_tenant_isolation ON %I USING (tenant_id = dagdb_current_tenant_id()) WITH CHECK (tenant_id = dagdb_current_tenant_id())',
            tenant_table
        );
    END LOOP;
END $$;
