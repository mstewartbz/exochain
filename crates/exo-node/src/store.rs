//! SQLite-backed `DagStore` implementation.
//!
//! Each node persists the DAG and committed state in a local SQLite database.
//! This implementation mirrors `MemoryStore` from `exo-dag` but uses durable
//! storage so state survives restarts.

use std::path::Path;

use exo_core::types::Hash256;
use exo_dag::{
    dag::DagNode,
    error::{DagError, Result as DagResult},
    store::DagStore,
};

/// Map a SQLite / CBOR error into `DagError::StoreError`.
fn store_err(e: impl std::fmt::Display) -> DagError {
    DagError::StoreError(e.to_string())
}
use rusqlite::{Connection, params};

/// SQLite-backed DAG store.
pub struct SqliteDagStore {
    conn: Connection,
}

impl SqliteDagStore {
    /// Open (or create) the SQLite database in the given data directory.
    pub fn open(data_dir: &Path) -> anyhow::Result<Self> {
        let db_path = data_dir.join("dag.db");
        let conn = Connection::open(&db_path)?;

        // WAL mode for concurrent reads.
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA synchronous=NORMAL;")?;

        // Create tables if they don't exist.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dag_nodes (
                hash         BLOB PRIMARY KEY NOT NULL,
                cbor_payload BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS dag_parents (
                child_hash   BLOB NOT NULL,
                parent_hash  BLOB NOT NULL,
                PRIMARY KEY (child_hash, parent_hash)
            );

            CREATE TABLE IF NOT EXISTS committed (
                hash   BLOB PRIMARY KEY NOT NULL,
                height INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_parents_parent ON dag_parents(parent_hash);
            CREATE INDEX IF NOT EXISTS idx_committed_height ON committed(height);",
        )?;

        Ok(Self { conn })
    }

    /// Convenience accessor for the current committed height.
    #[must_use]
    pub fn committed_height_value(&self) -> u64 {
        self.committed_height().unwrap_or(0)
    }

    /// Serialize a `DagNode` to CBOR bytes.
    fn encode_node(node: &DagNode) -> DagResult<Vec<u8>> {
        let mut buf = Vec::new();
        ciborium::into_writer(node, &mut buf)
            .map_err(|e| store_err(format!("CBOR encode: {e}")))?;
        Ok(buf)
    }

    /// Deserialize a `DagNode` from CBOR bytes.
    fn decode_node(bytes: &[u8]) -> DagResult<DagNode> {
        ciborium::from_reader(bytes)
            .map_err(|e| store_err(format!("CBOR decode: {e}")))
    }

    /// Query committed nodes in a height range (inclusive), ordered by height.
    ///
    /// Returns `(hash, height)` pairs. Used by state sync to serve snapshot chunks.
    pub fn committed_nodes_in_range(
        &self,
        from_height: u64,
        to_height: u64,
    ) -> DagResult<Vec<(Hash256, u64)>> {
        #[allow(clippy::as_conversions)]
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT hash, height FROM committed
                 WHERE height >= ?1 AND height <= ?2
                 ORDER BY height ASC",
            )
            .map_err(store_err)?;

        #[allow(clippy::as_conversions)]
        let rows = stmt
            .query_map(params![from_height as i64, to_height as i64], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                let height: i64 = row.get(1)?;
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok((Hash256::from_bytes(arr), height as u64))
            })
            .map_err(store_err)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(store_err)?);
        }
        Ok(result)
    }

    /// Get all committed nodes with their full DagNode data, ordered by height.
    ///
    /// Used by state sync to serve snapshot chunks with actual node payloads.
    pub fn committed_dag_nodes_in_range(
        &self,
        from_height: u64,
        to_height: u64,
    ) -> DagResult<Vec<DagNode>> {
        let committed = self.committed_nodes_in_range(from_height, to_height)?;
        let mut nodes = Vec::with_capacity(committed.len());
        for (hash, _height) in committed {
            if let Some(node) = self.get(&hash)? {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }
}

impl DagStore for SqliteDagStore {
    fn get(&self, hash: &Hash256) -> DagResult<Option<DagNode>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT cbor_payload FROM dag_nodes WHERE hash = ?1")
            .map_err(|e| store_err(e))?;

        let result: Option<Vec<u8>> = stmt
            .query_row(params![hash.0.as_slice()], |row| row.get(0))
            .ok();

        match result {
            Some(bytes) => Ok(Some(Self::decode_node(&bytes)?)),
            None => Ok(None),
        }
    }

    fn put(&mut self, node: DagNode) -> DagResult<()> {
        let cbor = Self::encode_node(&node)?;

        self.conn
            .execute(
                "INSERT OR IGNORE INTO dag_nodes (hash, cbor_payload) VALUES (?1, ?2)",
                params![node.hash.0.as_slice(), cbor],
            )
            .map_err(|e| store_err(e))?;

        // Record parent relationships.
        for parent in &node.parents {
            self.conn
                .execute(
                    "INSERT OR IGNORE INTO dag_parents (child_hash, parent_hash) VALUES (?1, ?2)",
                    params![node.hash.0.as_slice(), parent.0.as_slice()],
                )
                .map_err(|e| store_err(e))?;
        }

        Ok(())
    }

    fn contains(&self, hash: &Hash256) -> DagResult<bool> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT 1 FROM dag_nodes WHERE hash = ?1")
            .map_err(|e| store_err(e))?;

        let exists = stmt
            .query_row(params![hash.0.as_slice()], |_| Ok(()))
            .is_ok();

        Ok(exists)
    }

    fn tips(&self) -> DagResult<Vec<Hash256>> {
        // Tips are nodes that are not parents of any other node.
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT hash FROM dag_nodes
                 WHERE hash NOT IN (SELECT parent_hash FROM dag_parents)
                 ORDER BY hash",
            )
            .map_err(|e| store_err(e))?;

        let rows = stmt
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(Hash256::from_bytes(arr))
            })
            .map_err(|e| store_err(e))?;

        let mut tips = Vec::new();
        for row in rows {
            tips.push(row.map_err(|e| store_err(e))?);
        }
        Ok(tips)
    }

    fn committed_height(&self) -> DagResult<u64> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT COALESCE(MAX(height), 0) FROM committed")
            .map_err(|e| store_err(e))?;

        let height: i64 = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| store_err(e))?;

        #[allow(clippy::as_conversions)]
        Ok(height as u64)
    }

    fn mark_committed(&mut self, hash: &Hash256, height: u64) -> DagResult<()> {
        // Verify the node exists.
        if !self.contains(hash)? {
            return Err(DagError::NodeNotFound(*hash));
        }

        #[allow(clippy::as_conversions)]
        self.conn
            .execute(
                "INSERT OR REPLACE INTO committed (hash, height) VALUES (?1, ?2)",
                params![hash.0.as_slice(), height as i64],
            )
            .map_err(|e| store_err(e))?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use exo_core::types::{Did, Signature};
    use exo_dag::dag::{Dag, HybridClock, append};

    use super::*;

    type SignFn = Box<dyn Fn(&[u8]) -> Signature>;

    fn make_sign_fn() -> SignFn {
        Box::new(|data: &[u8]| {
            let h = blake3::hash(data);
            let mut sig = [0u8; 64];
            sig[..32].copy_from_slice(h.as_bytes());
            Signature::from_bytes(sig)
        })
    }

    fn make_test_node() -> DagNode {
        let mut dag = Dag::new();
        let mut clock = HybridClock::new();
        let creator = Did::new("did:exo:test").expect("valid");
        let sign_fn = make_sign_fn();
        append(&mut dag, &[], b"genesis", &creator, &*sign_fn, &mut clock).unwrap()
    }

    fn temp_store() -> SqliteDagStore {
        let dir = tempfile::tempdir().unwrap();
        SqliteDagStore::open(dir.path()).unwrap()
    }

    #[test]
    fn new_store_is_empty() {
        let store = temp_store();
        assert_eq!(store.committed_height().unwrap(), 0);
        assert!(store.tips().unwrap().is_empty());
    }

    #[test]
    fn put_and_get() {
        let mut store = temp_store();
        let node = make_test_node();

        store.put(node.clone()).unwrap();
        let retrieved = store.get(&node.hash).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().hash, node.hash);
    }

    #[test]
    fn get_nonexistent() {
        let store = temp_store();
        let result = store.get(&Hash256::ZERO).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn contains() {
        let mut store = temp_store();
        let node = make_test_node();

        assert!(!store.contains(&node.hash).unwrap());
        store.put(node.clone()).unwrap();
        assert!(store.contains(&node.hash).unwrap());
    }

    #[test]
    fn tips_single_node() {
        let mut store = temp_store();
        let node = make_test_node();
        store.put(node.clone()).unwrap();
        let t = store.tips().unwrap();
        assert_eq!(t, vec![node.hash]);
    }

    #[test]
    fn tips_with_children() {
        let mut dag = Dag::new();
        let mut clock = HybridClock::new();
        let creator = Did::new("did:exo:test").expect("valid");
        let sign_fn = make_sign_fn();

        let genesis = append(&mut dag, &[], b"genesis", &creator, &*sign_fn, &mut clock).unwrap();
        let child = append(
            &mut dag,
            &[genesis.hash],
            b"child",
            &creator,
            &*sign_fn,
            &mut clock,
        )
        .unwrap();

        let mut store = temp_store();
        store.put(genesis).unwrap();
        store.put(child.clone()).unwrap();

        let t = store.tips().unwrap();
        assert_eq!(t, vec![child.hash]);
    }

    #[test]
    fn committed_height_tracking() {
        let mut store = temp_store();
        let node = make_test_node();
        store.put(node.clone()).unwrap();

        assert_eq!(store.committed_height().unwrap(), 0);

        store.mark_committed(&node.hash, 1).unwrap();
        assert_eq!(store.committed_height().unwrap(), 1);
    }

    #[test]
    fn mark_committed_nonexistent_fails() {
        let mut store = temp_store();
        let err = store.mark_committed(&Hash256::ZERO, 1).unwrap_err();
        assert!(matches!(err, DagError::NodeNotFound(_)));
    }

    #[test]
    fn multiple_tips() {
        let mut dag = Dag::new();
        let mut clock = HybridClock::new();
        let creator = Did::new("did:exo:test").expect("valid");
        let sign_fn = make_sign_fn();

        let genesis = append(&mut dag, &[], b"genesis", &creator, &*sign_fn, &mut clock).unwrap();
        let c1 = append(
            &mut dag,
            &[genesis.hash],
            b"c1",
            &creator,
            &*sign_fn,
            &mut clock,
        )
        .unwrap();
        let c2 = append(
            &mut dag,
            &[genesis.hash],
            b"c2",
            &creator,
            &*sign_fn,
            &mut clock,
        )
        .unwrap();

        let mut store = temp_store();
        store.put(genesis).unwrap();
        store.put(c1.clone()).unwrap();
        store.put(c2.clone()).unwrap();

        let t = store.tips().unwrap();
        assert_eq!(t.len(), 2);
        assert!(t.contains(&c1.hash));
        assert!(t.contains(&c2.hash));
    }
}
