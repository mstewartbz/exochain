CREATE TABLE IF NOT EXISTS dagdb_node_dag_nodes (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    hash BYTEA NOT NULL CHECK (octet_length(hash) = 32),
    cbor_payload BYTEA NOT NULL CHECK (octet_length(cbor_payload) > 0),
    PRIMARY KEY (tenant_id, namespace, hash)
);

CREATE TABLE IF NOT EXISTS dagdb_node_dag_parents (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    child_hash BYTEA NOT NULL CHECK (octet_length(child_hash) = 32),
    parent_hash BYTEA NOT NULL CHECK (octet_length(parent_hash) = 32),
    PRIMARY KEY (tenant_id, namespace, child_hash, parent_hash),
    FOREIGN KEY (tenant_id, namespace, child_hash)
        REFERENCES dagdb_node_dag_nodes (tenant_id, namespace, hash)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_dagdb_node_parents_parent
    ON dagdb_node_dag_parents USING btree (tenant_id, namespace, parent_hash);

CREATE TABLE IF NOT EXISTS dagdb_node_committed (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    hash BYTEA NOT NULL CHECK (octet_length(hash) = 32),
    height BIGINT NOT NULL CHECK (height > 0),
    PRIMARY KEY (tenant_id, namespace, hash),
    UNIQUE (tenant_id, namespace, height),
    FOREIGN KEY (tenant_id, namespace, hash)
        REFERENCES dagdb_node_dag_nodes (tenant_id, namespace, hash)
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_dagdb_node_committed_height
    ON dagdb_node_committed USING btree (tenant_id, namespace, height);

CREATE TABLE IF NOT EXISTS dagdb_node_consensus_meta (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    key TEXT NOT NULL CHECK (length(key) > 0),
    value TEXT NOT NULL,
    PRIMARY KEY (tenant_id, namespace, key)
);

CREATE TABLE IF NOT EXISTS dagdb_node_consensus_votes (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    round BIGINT NOT NULL CHECK (round >= 0),
    node_hash BYTEA NOT NULL CHECK (octet_length(node_hash) = 32),
    voter_did TEXT NOT NULL CHECK (length(voter_did) > 0 AND voter_did LIKE 'did:%'),
    signature BYTEA NOT NULL CHECK (octet_length(signature) = 64),
    PRIMARY KEY (tenant_id, namespace, round, node_hash, voter_did)
);

CREATE TABLE IF NOT EXISTS dagdb_node_commit_certificates (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    node_hash BYTEA NOT NULL CHECK (octet_length(node_hash) = 32),
    round BIGINT NOT NULL CHECK (round >= 0),
    cbor_data BYTEA NOT NULL CHECK (octet_length(cbor_data) > 0),
    PRIMARY KEY (tenant_id, namespace, node_hash),
    FOREIGN KEY (tenant_id, namespace, node_hash)
        REFERENCES dagdb_node_dag_nodes (tenant_id, namespace, hash)
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_dagdb_node_commit_certificates_round
    ON dagdb_node_commit_certificates USING btree (tenant_id, namespace, round);

CREATE TABLE IF NOT EXISTS dagdb_node_validators (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    did TEXT NOT NULL CHECK (length(did) > 0 AND did LIKE 'did:%'),
    PRIMARY KEY (tenant_id, namespace, did)
);

CREATE TABLE IF NOT EXISTS dagdb_node_trust_receipts (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    receipt_hash BYTEA NOT NULL CHECK (octet_length(receipt_hash) = 32),
    actor_did TEXT NOT NULL CHECK (length(actor_did) > 0 AND actor_did LIKE 'did:%'),
    action_type TEXT NOT NULL CHECK (length(action_type) > 0),
    outcome TEXT NOT NULL CHECK (length(outcome) > 0),
    timestamp_ms BIGINT NOT NULL CHECK (timestamp_ms >= 0),
    cbor_data BYTEA NOT NULL CHECK (octet_length(cbor_data) > 0),
    PRIMARY KEY (tenant_id, namespace, receipt_hash)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_node_trust_receipts_actor
    ON dagdb_node_trust_receipts USING btree (tenant_id, namespace, actor_did);
CREATE INDEX IF NOT EXISTS idx_dagdb_node_trust_receipts_ts
    ON dagdb_node_trust_receipts USING btree (tenant_id, namespace, timestamp_ms DESC, receipt_hash ASC);

CREATE TABLE IF NOT EXISTS dagdb_node_economy_objects (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    object_kind TEXT NOT NULL CHECK (length(object_kind) > 0),
    object_id BYTEA NOT NULL CHECK (octet_length(object_id) = 32),
    content_hash BYTEA NOT NULL CHECK (octet_length(content_hash) = 32),
    created_physical_ms BIGINT NOT NULL CHECK (created_physical_ms >= 0),
    created_logical BIGINT NOT NULL CHECK (created_logical >= 0),
    cbor_data BYTEA NOT NULL CHECK (octet_length(cbor_data) > 0),
    PRIMARY KEY (tenant_id, namespace, object_kind, object_id)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_node_economy_objects_hash
    ON dagdb_node_economy_objects USING btree (tenant_id, namespace, content_hash);

CREATE TABLE IF NOT EXISTS dagdb_node_economy_anchors (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    anchor_hash BYTEA NOT NULL CHECK (octet_length(anchor_hash) = 32),
    previous_anchor_hash BYTEA NOT NULL CHECK (octet_length(previous_anchor_hash) = 32),
    object_kind TEXT NOT NULL CHECK (length(object_kind) > 0),
    object_id BYTEA NOT NULL CHECK (octet_length(object_id) = 32),
    object_hash BYTEA NOT NULL CHECK (octet_length(object_hash) = 32),
    created_physical_ms BIGINT NOT NULL CHECK (created_physical_ms >= 0),
    created_logical BIGINT NOT NULL CHECK (created_logical >= 0),
    cbor_data BYTEA NOT NULL CHECK (octet_length(cbor_data) > 0),
    PRIMARY KEY (tenant_id, namespace, anchor_hash)
);

CREATE INDEX IF NOT EXISTS idx_dagdb_node_economy_anchors_object
    ON dagdb_node_economy_anchors USING btree (tenant_id, namespace, object_kind, object_id);

CREATE TABLE IF NOT EXISTS dagdb_node_economy_meta (
    tenant_id TEXT NOT NULL CHECK (length(tenant_id) > 0),
    namespace TEXT NOT NULL CHECK (length(namespace) > 0),
    key TEXT NOT NULL CHECK (length(key) > 0),
    value BYTEA NOT NULL CHECK (octet_length(value) > 0),
    PRIMARY KEY (tenant_id, namespace, key)
);
