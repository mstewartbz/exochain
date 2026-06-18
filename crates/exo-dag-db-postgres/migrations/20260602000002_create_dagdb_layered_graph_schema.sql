CREATE TABLE IF NOT EXISTS dagdb_graph_layers (
    layer_id BYTEA PRIMARY KEY CHECK (octet_length(layer_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    root_memory_id BYTEA NOT NULL REFERENCES dagdb_memory_objects(memory_id),
    parent_layer_id BYTEA NULL,
    parent_graph_node_id BYTEA NULL REFERENCES dagdb_graph_nodes(graph_node_id),
    layer_depth INTEGER NOT NULL CHECK (layer_depth >= 0),
    layer_kind TEXT NOT NULL CHECK (layer_kind IN ('root','repository','knowledge_graph','source_subgraph','task_subgraph','rollup','route')),
    graph_style TEXT NOT NULL CHECK (graph_style IN ('provenance_receipt_dag','canonical_memory_graph','semantic_catalog_graph','similarity_overlay_graph','dependency_dag','routing_view_graph','contradiction_supersession_graph','context_packet_graph')),
    layer_path TEXT NOT NULL CHECK (length(layer_path) > 0 AND layer_path NOT LIKE '/%' AND layer_path NOT LIKE '%/' AND layer_path NOT LIKE '%//%'),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(metadata) = 'object'),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    UNIQUE (tenant_id, namespace, layer_id),
    UNIQUE (tenant_id, namespace, layer_path),
    FOREIGN KEY (tenant_id, namespace, parent_layer_id) REFERENCES dagdb_graph_layers(tenant_id, namespace, layer_id),
    CHECK (
        (layer_depth = 0 AND parent_layer_id IS NULL AND parent_graph_node_id IS NULL)
        OR
        (layer_depth > 0 AND parent_layer_id IS NOT NULL AND parent_graph_node_id IS NOT NULL)
    ),
    CHECK (
        (updated_at_physical_ms > created_at_physical_ms)
        OR
        (updated_at_physical_ms = created_at_physical_ms AND updated_at_logical >= created_at_logical)
    )
);

CREATE TABLE IF NOT EXISTS dagdb_graph_layer_memberships (
    layer_membership_id BYTEA PRIMARY KEY CHECK (octet_length(layer_membership_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    layer_id BYTEA NOT NULL,
    graph_node_id BYTEA NOT NULL REFERENCES dagdb_graph_nodes(graph_node_id),
    graph_style TEXT NOT NULL CHECK (graph_style IN ('provenance_receipt_dag','canonical_memory_graph','semantic_catalog_graph','similarity_overlay_graph','dependency_dag','routing_view_graph','contradiction_supersession_graph','context_packet_graph')),
    membership_role TEXT NOT NULL CHECK (membership_role IN ('root','container','member','summary','route_anchor')),
    local_node_rank INTEGER NOT NULL CHECK (local_node_rank >= 0),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(metadata) = 'object'),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    UNIQUE (tenant_id, namespace, layer_id, graph_node_id),
    FOREIGN KEY (tenant_id, namespace, layer_id) REFERENCES dagdb_graph_layers(tenant_id, namespace, layer_id),
    CHECK (
        (updated_at_physical_ms > created_at_physical_ms)
        OR
        (updated_at_physical_ms = created_at_physical_ms AND updated_at_logical >= created_at_logical)
    )
);

CREATE TABLE IF NOT EXISTS dagdb_graph_layer_edges (
    layer_edge_id BYTEA PRIMARY KEY CHECK (octet_length(layer_edge_id) = 32),
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    graph_style TEXT NOT NULL CHECK (graph_style IN ('provenance_receipt_dag','canonical_memory_graph','semantic_catalog_graph','similarity_overlay_graph','dependency_dag','routing_view_graph','contradiction_supersession_graph','context_packet_graph')),
    from_layer_id BYTEA NOT NULL,
    to_layer_id BYTEA NOT NULL,
    edge_kind TEXT NOT NULL CHECK (edge_kind IN ('contains_subgraph','drills_down_to','rolls_up_to','cross_layer_ref','summarizes_layer')),
    receipt_hash BYTEA NULL REFERENCES dagdb_receipts(receipt_hash) CHECK (receipt_hash IS NULL OR octet_length(receipt_hash) = 32),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(metadata) = 'object'),
    created_at_physical_ms BIGINT NOT NULL CHECK (created_at_physical_ms >= 0),
    created_at_logical INTEGER NOT NULL CHECK (created_at_logical >= 0),
    updated_at_physical_ms BIGINT NOT NULL CHECK (updated_at_physical_ms >= 0),
    updated_at_logical INTEGER NOT NULL CHECK (updated_at_logical >= 0),
    UNIQUE (tenant_id, namespace, graph_style, from_layer_id, to_layer_id, edge_kind),
    FOREIGN KEY (tenant_id, namespace, from_layer_id) REFERENCES dagdb_graph_layers(tenant_id, namespace, layer_id),
    FOREIGN KEY (tenant_id, namespace, to_layer_id) REFERENCES dagdb_graph_layers(tenant_id, namespace, layer_id),
    CHECK (from_layer_id <> to_layer_id),
    CHECK (
        (updated_at_physical_ms > created_at_physical_ms)
        OR
        (updated_at_physical_ms = created_at_physical_ms AND updated_at_logical >= created_at_logical)
    )
);

CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layers_path ON dagdb_graph_layers USING btree (tenant_id, namespace, layer_path);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layers_parent_layer ON dagdb_graph_layers USING btree (tenant_id, namespace, parent_layer_id) WHERE parent_layer_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layers_parent_graph_node ON dagdb_graph_layers USING btree (tenant_id, namespace, parent_graph_node_id) WHERE parent_graph_node_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layer_memberships_layer_node ON dagdb_graph_layer_memberships USING btree (tenant_id, namespace, layer_id, graph_node_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layer_memberships_graph_node ON dagdb_graph_layer_memberships USING btree (tenant_id, namespace, graph_node_id, layer_id);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layer_edges_from ON dagdb_graph_layer_edges USING btree (tenant_id, namespace, from_layer_id, edge_kind);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layer_edges_to ON dagdb_graph_layer_edges USING btree (tenant_id, namespace, to_layer_id, edge_kind);
CREATE INDEX IF NOT EXISTS idx_dagdb_graph_layer_edges_kind ON dagdb_graph_layer_edges USING btree (tenant_id, namespace, edge_kind, created_at_physical_ms DESC, created_at_logical DESC);
