CREATE TABLE IF NOT EXISTS dag_nodes (
    hash            BYTEA PRIMARY KEY,
    parents         BYTEA[] NOT NULL DEFAULT '{}',
    payload_hash    BYTEA NOT NULL,
    creator_did     TEXT NOT NULL,
    ts_physical_ms  BIGINT NOT NULL,
    ts_logical      BIGINT NOT NULL,
    signature       BYTEA NOT NULL,
    inserted_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS dag_committed (
    hash   BYTEA PRIMARY KEY REFERENCES dag_nodes(hash),
    height BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_dag_nodes_creator ON dag_nodes(creator_did);
CREATE INDEX IF NOT EXISTS idx_dag_committed_height ON dag_committed(height);
