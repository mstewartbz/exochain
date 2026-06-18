-- PRD-D3 (D3-S1): two-tier summaries — depth on demand.
--
-- Adds the nullable deep-detail-summary tier alongside the existing short
-- `summary` tier on `dagdb_memory_objects`. The short tier (`summary`) is
-- unchanged and stays the breadth-pass payload; this column carries the fuller
-- distilled fact set served only by the membership-triggered drilldown pass.
--
-- Strictly additive and idempotent: the column is NULLABLE so every existing
-- row remains valid until an operator-gated re-import backfills it (D3-S4).
-- A NULL deep tier falls back to the short tier in drilldown, so adding this
-- column changes no existing behavior. Like the short tier, the value is a
-- distilled SafeMetadata JSON object (decision/text/redaction_codes/...), never
-- a raw body; the same forbidden-material discipline screens both tiers.
ALTER TABLE dagdb_memory_objects
    ADD COLUMN IF NOT EXISTS deep_detail_summary JSONB NULL;
