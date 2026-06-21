-- PRD-D2 (dimension3-prd-02) S1: aggregate root summary column.
--
-- Strictly additive and nullable so unmigrated rows stay valid and the
-- column can be re-applied idempotently by the raw `psql -f` migration glob in
-- tools/start_dagdb_local.sh. The aggregate summary is a deterministic
-- system-side digest of a layer's members distilled by the layer policy
-- (layer_creation_policy aggregate distiller); NULL means "no aggregate yet"
-- and the rollup path falls back to the prior behavior for that layer.
--
-- The stored value is a JSONB object carrying the SafeMetadata-shaped distilled
-- summary so the rollup read surfaces the same shape as member summaries.
ALTER TABLE dagdb_graph_layers
    ADD COLUMN IF NOT EXISTS aggregate_summary JSONB NULL
        CHECK (aggregate_summary IS NULL OR jsonb_typeof(aggregate_summary) = 'object');
