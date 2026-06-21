-- PRD-D4 (Telemetry Facet Separation), slice D4-S1: structural home for the
-- telemetry facet.
--
-- Usage-event telemetry rows were previously inserted into the knowledge table
-- as node_type='excerpt', distinguishable only by a read-side title-prefix
-- heuristic. This additive migration extends the dagdb_memory_objects node_type
-- CHECK with a dedicated 'usage_event' value so usage-event telemetry has its
-- own structural home, write-side. Context-packet telemetry already has the
-- distinct 'context_packet' node_type; together they form the telemetry facet
-- that packet selection excludes by structure (no string match).
--
-- The unified store is retained: telemetry rows stay in the same table and
-- remain queryable; this migration only adds a node_type value, it drops no
-- data and rewrites no rows. The system-side migration of existing legacy
-- 'excerpt'-shaped usage-event rows into 'usage_event' is the operator-gated
-- D4-S3 tool, not this schema migration.
--
-- Idempotent: drops the auto-named CHECK and re-adds the superset, exactly as
-- the export-persistence migration extends the receipts subject_kind/event_type
-- CHECKs. The 'usage_event' value participates in the existing
-- uq_dagdb_memory_active_duplicate unique index (which is keyed on node_type),
-- so telemetry rows of distinct node_type never collide with knowledge rows.

ALTER TABLE dagdb_memory_objects DROP CONSTRAINT IF EXISTS dagdb_memory_objects_node_type_check;
ALTER TABLE dagdb_memory_objects
    ADD CONSTRAINT dagdb_memory_objects_node_type_check
    CHECK (node_type IN ('source','excerpt','embedding','summary','answer','validation_report','catalog','route','context_packet','usage_event'));
