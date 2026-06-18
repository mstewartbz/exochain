CREATE TABLE IF NOT EXISTS dagdb_graph_edge_tombstones (
    tombstone_id BYTEA PRIMARY KEY CHECK (octet_length(tombstone_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    prior_edge_id BYTEA NOT NULL REFERENCES dagdb_graph_edges(graph_edge_id),
    tombstone_reason TEXT NOT NULL CHECK (length(tombstone_reason) > 0),
    recommended_action TEXT NOT NULL CHECK (recommended_action IN ('tombstone','weaken','review','split_hub','merge_duplicate')),
    receipt_hash BYTEA NOT NULL REFERENCES dagdb_receipts(receipt_hash) CHECK (octet_length(receipt_hash) = 32),
    idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) > 0),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    tombstone_body JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (tenant_id, namespace, prior_edge_id, tombstone_reason),
    UNIQUE (tenant_id, namespace, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_edge_tombstones_edge ON dagdb_graph_edge_tombstones USING btree (tenant_id, namespace, prior_edge_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_edge_tombstones_created ON dagdb_graph_edge_tombstones USING btree (tenant_id, namespace, created_at_physical_ms DESC, created_at_logical DESC);
