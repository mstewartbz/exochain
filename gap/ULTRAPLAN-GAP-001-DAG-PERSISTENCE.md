# ULTRAPLAN — GAP-001: DAG Persistence Layer

**Status:** Implementing  
**Author:** Aeon (Chief-of-Staff AI)  
**Date:** 2026-04-15  
**Scope:** `exo-dag` crate — PostgreSQL-backed `DagStore` + async trait migration

---

## 1. Schema Design

Two tables in PostgreSQL, mirroring the `DagNode` struct and commit tracking:

### `dag_nodes` — Primary node storage

```sql
CREATE TABLE IF NOT EXISTS dag_nodes (
    hash            BYTEA PRIMARY KEY,      -- 32 bytes (Hash256)
    parents         BYTEA[] NOT NULL DEFAULT '{}',  -- array of 32-byte hashes
    payload_hash    BYTEA NOT NULL,         -- 32 bytes (Hash256)
    creator_did     TEXT NOT NULL,           -- DID string
    ts_physical_ms  BIGINT NOT NULL,        -- Timestamp.physical_ms
    ts_logical      BIGINT NOT NULL,        -- Timestamp.logical
    signature       BYTEA NOT NULL,         -- 64 bytes (Signature)
    inserted_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### `dag_committed` — Commit height tracking

```sql
CREATE TABLE IF NOT EXISTS dag_committed (
    hash   BYTEA PRIMARY KEY REFERENCES dag_nodes(hash),
    height BIGINT NOT NULL
);
```

### Indexes

- `idx_dag_nodes_creator ON dag_nodes(creator_did)` — filter by creator DID
- `idx_dag_committed_height ON dag_committed(height)` — range queries on committed heights

### Tip Query Strategy

Tips are nodes whose hash does not appear in any other node's parents array:

```sql
SELECT hash FROM dag_nodes dn
WHERE NOT EXISTS (
    SELECT 1 FROM dag_nodes other
    WHERE dn.hash = ANY(other.parents)
)
ORDER BY hash
```

This leverages PostgreSQL's native array operations. For large DAGs (>100k nodes), a GIN index on `parents` could be added, but the `NOT EXISTS` subquery with `ANY()` is efficient for typical workloads. The `ORDER BY hash` ensures deterministic output matching `MemoryStore`.

### Committed Height

```sql
SELECT COALESCE(MAX(height), 0) FROM dag_committed
```

Simple aggregate with the `idx_dag_committed_height` index.

---

## 2. PostgresStore Implementation

### Struct

```rust
pub struct PostgresStore {
    pool: PgPool,
}
```

Wraps a `sqlx::PgPool` for connection pooling. All methods are async, matching the new async `DagStore` trait.

### Async Trait Decision: Option A (Make DagStore Async)

The `DagStore` trait is currently sync. We choose **Option A — make the trait fully async** because:

1. **MemoryStore trivially implements async** — just return immediately, no blocking
2. **Everything downstream is async** — gateway runs on tokio, node runs on tokio
3. **SqliteDagStore in exo-node** — already wrapped in `Arc<Mutex<>>`, can implement async trait by doing sync work inside the lock (no actual I/O wait, rusqlite is CPU-bound)
4. **Impedance mismatch** — keeping it sync forces every async consumer to bridge, whereas making it async only adds `async` keywords to sync implementations

We use `async-trait` crate since Rust's native async trait support doesn't yet cover all our use cases (dyn dispatch, Send bounds).

### Method Implementations

- **`get`**: `SELECT` by hash, decode columns back into `DagNode`
- **`put`**: `INSERT INTO dag_nodes` with all fields; uses `ON CONFLICT DO NOTHING` for idempotency
- **`contains`**: `SELECT EXISTS(SELECT 1 FROM dag_nodes WHERE hash = $1)`
- **`tips`**: The `NOT EXISTS` / `ANY(parents)` query above
- **`committed_height`**: `COALESCE(MAX(height), 0)` aggregate
- **`mark_committed`**: Check node exists, then `INSERT INTO dag_committed`; returns error if hash not found

### Constructor & Migration

```rust
impl PostgresStore {
    pub async fn new(pool: PgPool) -> Result<Self> { Ok(Self { pool }) }
    pub async fn migrate(pool: &PgPool) -> Result<()> { /* run embedded SQL */ }
}
```

Migration is embedded in the `migrate()` method using raw SQL execution, not sqlx's migration framework, because the DAG tables live in the same database as the gateway but are owned by a different crate.

---

## 3. Migration Strategy

The SQL schema is embedded directly in `PostgresStore::migrate()` using `sqlx::query!` raw execution. This avoids coupling to the gateway's migration directory while ensuring tables exist before use. The `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` pattern is idempotent.

For production deployments, the gateway's migration directory (`crates/exo-gateway/migrations/`) could also include a versioned migration file. But for this initial implementation, the embedded approach keeps `exo-dag` self-contained.

---

## 4. Serialization — Column Mapping

| DagNode field   | Postgres column   | Postgres type | Encoding |
|----------------|-------------------|---------------|----------|
| `hash`         | `hash`            | `BYTEA`       | Raw 32 bytes via `as_bytes()` / `from_bytes()` |
| `parents`      | `parents`         | `BYTEA[]`     | Array of 32-byte slices |
| `payload_hash` | `payload_hash`    | `BYTEA`       | Raw 32 bytes |
| `creator_did`  | `creator_did`     | `TEXT`         | `Did::as_str()` / `Did::new()` |
| `timestamp`    | `ts_physical_ms`  | `BIGINT`      | Direct u64→i64 cast (physical_ms fits in i64) |
| `timestamp`    | `ts_logical`      | `BIGINT`      | Direct u64→i64 cast |
| `signature`    | `signature`       | `BYTEA`       | Raw 64 bytes |

**Why not CBOR/JSONB?** Columnar storage enables SQL-level queries (filter by creator, range on timestamp) without deserializing. The `MemoryStore` and `SqliteDagStore` (exo-node) use CBOR for the whole node, but Postgres benefits from columnar decomposition.

**Parents as `BYTEA[]`**: PostgreSQL natively supports `ANY(array_column)` for membership checks, making tip queries efficient. Each element is a raw 32-byte hash.

**Timestamp split**: Two `BIGINT` columns avoid any floating-point or complex type. `u64` values are stored as `i64` (safe since physical_ms and logical counters won't exceed i64::MAX in practice).

---

## 5. Testing Strategy

### PostgresStore Integration Tests (11 tests)

All gated behind `DATABASE_URL` environment variable. If not set, tests are skipped with a message. Each test creates a unique schema or uses transactions that roll back.

1. **`test_pg_put_and_get`** — Store a node, retrieve by hash, verify all fields match
2. **`test_pg_contains`** — Check existence before and after put
3. **`test_pg_tips_single`** — One node = one tip
4. **`test_pg_tips_with_children`** — Parent consumed by child, only child is tip
5. **`test_pg_tips_multiple`** — Fork creates two tips
6. **`test_pg_committed_height`** — Mark committed, verify height increases
7. **`test_pg_committed_nonexistent_fails`** — Mark unknown hash returns error
8. **`test_pg_roundtrip_deterministic`** — Put then get returns identical DagNode (field-by-field comparison)
9. **`test_pg_parents_ordering`** — Parents stored and retrieved in sorted order
10. **`test_pg_large_payload_hash`** — Boundary values (all-zero, all-0xFF) for 32-byte fields
11. **`test_memory_and_pg_parity`** — Run identical operation sequence on MemoryStore and PostgresStore, verify tips and committed heights match (oracle test)

### Existing MemoryStore Tests

All existing tests updated to `#[tokio::test]` with `.await` on DagStore method calls. No behavioral changes.

### Test Infrastructure

Tests use a live Postgres instance via `DATABASE_URL`. Each test function creates the tables in a transaction or uses unique table prefixes. For CI, the existing `docker-compose.yml` provides Postgres 16.

---

## 6. Stub Removal Plan

### `crates/exo-node/src/reactor.rs:374`
```rust
// Before:
// DAG sync and state snapshot handled by Phase 4
_ => {}

// After: Comment updated to reference GAP-001 completion
// DAG persistence layer shipped (GAP-001). State sync TBD.
_ => {}
```

### `crates/exo-node/src/passport.rs:348`
```rust
// Before: "planned for the Phase 4 state-sync milestone"
// After: "DAG persistence shipped (GAP-001). Delegation persistence TBD."
```

### `crates/exo-node/src/passport.rs:359`
```rust
// Before: "planned for Phase 4"
// After: "DAG persistence shipped (GAP-001). Consent persistence TBD."
```

These stubs reference higher-level state-sync features that depend on but go beyond the DAG persistence layer. We update the comments to reflect GAP-001 completion while noting the remaining work.

---

## Implementation Order

1. Add `async-trait` to workspace `Cargo.toml`
2. Update `exo-dag/Cargo.toml` — add features, optional deps
3. Rewrite `DagStore` trait to async — update `MemoryStore`
4. Update `append.rs` — make `validated_append` and `verify_stored_integrity` async
5. Create `pg_store.rs` — `PostgresStore` implementation
6. Update `lib.rs` — feature-gated module
7. Update all exo-dag tests for async
8. Update `exo-node/src/store.rs` — `SqliteDagStore` implements async `DagStore`
9. Update exo-node callers (reactor, passport, telegram, holons)
10. Update stub comments
11. Build, test, clippy, commit
