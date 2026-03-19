//! Sparse Merkle Tree -- 256-bit key space with blake3 hashing.
//!
//! Supports inclusion and non-inclusion (absence) proofs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use exo_core::types::Hash256;

use crate::error::{DagError, Result};

// ---------------------------------------------------------------------------
// Constants & helpers
// ---------------------------------------------------------------------------

const TREE_DEPTH: usize = 256;

fn default_hash(level: usize) -> Hash256 {
    let mut h = Hash256::digest(b"smt:empty:leaf");
    for _ in 0..level {
        let mut hasher = blake3::Hasher::new();
        hasher.update(h.as_bytes());
        hasher.update(h.as_bytes());
        h = Hash256::from_bytes(*hasher.finalize().as_bytes());
    }
    h
}

fn hash_pair(left: &Hash256, right: &Hash256) -> Hash256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    Hash256::from_bytes(*hasher.finalize().as_bytes())
}

fn hash_leaf(value: &[u8]) -> Hash256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"smt:leaf:");
    hasher.update(value);
    Hash256::from_bytes(*hasher.finalize().as_bytes())
}

fn get_bit(key: &Hash256, pos: usize) -> bool {
    let byte_idx = pos / 8;
    let bit_idx = 7 - (pos % 8);
    (key.0[byte_idx] >> bit_idx) & 1 == 1
}

fn bits_to_key(bits: &[bool]) -> Hash256 {
    let mut bytes = [0u8; 32];
    for (i, &bit) in bits.iter().enumerate() {
        if i >= 256 {
            break;
        }
        if bit {
            let byte_idx = i / 8;
            let bit_idx = 7 - (i % 8);
            bytes[byte_idx] |= 1 << bit_idx;
        }
    }
    Hash256::from_bytes(bytes)
}

// ---------------------------------------------------------------------------
// MerkleProof
// ---------------------------------------------------------------------------

/// A Merkle proof for inclusion or non-inclusion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Sibling hashes along the path from leaf to root.
    pub siblings: Vec<Hash256>,
    /// The key being proved.
    pub key: Hash256,
    /// The value at the key (None for non-inclusion proof).
    pub value: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// SparseMerkleTree
// ---------------------------------------------------------------------------

/// A Sparse Merkle Tree with 256-bit key space.
#[derive(Debug, Clone, Default)]
pub struct SparseMerkleTree {
    leaves: BTreeMap<Hash256, Vec<u8>>,
}

impl SparseMerkleTree {
    /// Create a new empty sparse Merkle tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the root hash.
    #[must_use]
    pub fn root(&self) -> Hash256 {
        if self.leaves.is_empty() {
            return default_hash(TREE_DEPTH);
        }
        self.compute_node(TREE_DEPTH, &[])
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether the tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    fn compute_node(&self, level: usize, path_bits: &[bool]) -> Hash256 {
        if level == 0 {
            if path_bits.len() == TREE_DEPTH {
                let key = bits_to_key(path_bits);
                if let Some(value) = self.leaves.get(&key) {
                    return hash_leaf(value);
                }
            }
            return default_hash(0);
        }

        if !self.has_leaf_with_prefix(path_bits) {
            return default_hash(level);
        }

        let mut left_path = path_bits.to_vec();
        left_path.push(false);
        let left = self.compute_node(level - 1, &left_path);

        let mut right_path = path_bits.to_vec();
        right_path.push(true);
        let right = self.compute_node(level - 1, &right_path);

        hash_pair(&left, &right)
    }

    fn has_leaf_with_prefix(&self, prefix: &[bool]) -> bool {
        for key in self.leaves.keys() {
            let mut matches = true;
            for (i, &bit) in prefix.iter().enumerate() {
                if get_bit(key, i) != bit {
                    matches = false;
                    break;
                }
            }
            if matches {
                return true;
            }
        }
        false
    }

    fn sibling_hash(&self, key: &Hash256, level: usize) -> Hash256 {
        let depth = TREE_DEPTH - level;
        let mut path_bits: Vec<bool> = (0..depth).map(|i| get_bit(key, i)).collect();
        if let Some(last) = path_bits.last_mut() {
            *last = !*last;
        }
        self.compute_node(level, &path_bits)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Insert a key-value pair. Returns the new root hash.
pub fn insert(tree: &mut SparseMerkleTree, key: &Hash256, value: &[u8]) -> Result<Hash256> {
    if value.is_empty() {
        return Err(DagError::SmtError("empty value".to_string()));
    }
    tree.leaves.insert(*key, value.to_vec());
    Ok(tree.root())
}

/// Get the value at a key.
#[must_use]
pub fn get(tree: &SparseMerkleTree, key: &Hash256) -> Option<Vec<u8>> {
    tree.leaves.get(key).cloned()
}

/// Generate a Merkle proof for a key.
pub fn prove(tree: &SparseMerkleTree, key: &Hash256) -> MerkleProof {
    let mut siblings = Vec::with_capacity(TREE_DEPTH);
    for level in 1..=TREE_DEPTH {
        let sibling = tree.sibling_hash(key, level - 1);
        siblings.push(sibling);
    }

    MerkleProof {
        siblings,
        key: *key,
        value: tree.leaves.get(key).cloned(),
    }
}

/// Verify a Merkle proof against a root.
pub fn verify_proof(
    root: &Hash256,
    key: &Hash256,
    value: Option<&[u8]>,
    proof: &MerkleProof,
) -> bool {
    if proof.siblings.len() != TREE_DEPTH {
        return false;
    }

    let mut current = match value {
        Some(v) => hash_leaf(v),
        None => default_hash(0),
    };

    for (i, sibling) in proof.siblings.iter().enumerate() {
        let depth = TREE_DEPTH - 1 - i;
        if get_bit(key, depth) {
            current = hash_pair(sibling, &current);
        } else {
            current = hash_pair(&current, sibling);
        }
    }

    current == *root
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_root() {
        let tree = SparseMerkleTree::new();
        let root = tree.root();
        assert_eq!(root, default_hash(TREE_DEPTH));
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn insert_and_get() {
        let mut tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"key1");
        let value = b"value1";

        let root = insert(&mut tree, &key, value).unwrap();
        assert_ne!(root, default_hash(TREE_DEPTH));
        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 1);

        let retrieved = get(&tree, &key);
        assert_eq!(retrieved, Some(value.to_vec()));
    }

    #[test]
    fn get_nonexistent() {
        let tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"nonexistent");
        assert!(get(&tree, &key).is_none());
    }

    #[test]
    fn insert_empty_value_rejected() {
        let mut tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"key");
        let err = insert(&mut tree, &key, b"").unwrap_err();
        assert!(matches!(err, DagError::SmtError(_)));
    }

    #[test]
    fn inclusion_proof() {
        let mut tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"key1");
        let value = b"value1";
        let root = insert(&mut tree, &key, value).unwrap();

        let proof = prove(&tree, &key);
        assert!(verify_proof(&root, &key, Some(value), &proof));
    }

    #[test]
    fn non_inclusion_proof() {
        let mut tree = SparseMerkleTree::new();
        let key1 = Hash256::digest(b"key1");
        let root = insert(&mut tree, &key1, b"value1").unwrap();

        let absent_key = Hash256::digest(b"absent");
        let proof = prove(&tree, &absent_key);
        assert!(verify_proof(&root, &absent_key, None, &proof));
    }

    #[test]
    fn proof_fails_wrong_value() {
        let mut tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"key1");
        let root = insert(&mut tree, &key, b"value1").unwrap();

        let proof = prove(&tree, &key);
        assert!(!verify_proof(&root, &key, Some(b"wrong"), &proof));
    }

    #[test]
    fn proof_fails_wrong_root() {
        let mut tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"key1");
        let value = b"value1";
        let _root = insert(&mut tree, &key, value).unwrap();

        let proof = prove(&tree, &key);
        assert!(!verify_proof(&Hash256::ZERO, &key, Some(value), &proof));
    }

    #[test]
    fn multiple_inserts() {
        let mut tree = SparseMerkleTree::new();
        let k1 = Hash256::digest(b"k1");
        let k2 = Hash256::digest(b"k2");
        let k3 = Hash256::digest(b"k3");

        let _r1 = insert(&mut tree, &k1, b"v1").unwrap();
        let _r2 = insert(&mut tree, &k2, b"v2").unwrap();
        let r3 = insert(&mut tree, &k3, b"v3").unwrap();

        assert_eq!(tree.len(), 3);

        let p1 = prove(&tree, &k1);
        let p2 = prove(&tree, &k2);
        let p3 = prove(&tree, &k3);

        assert!(verify_proof(&r3, &k1, Some(b"v1"), &p1));
        assert!(verify_proof(&r3, &k2, Some(b"v2"), &p2));
        assert!(verify_proof(&r3, &k3, Some(b"v3"), &p3));
    }

    #[test]
    fn overwrite_value() {
        let mut tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"key");

        let r1 = insert(&mut tree, &key, b"v1").unwrap();
        let r2 = insert(&mut tree, &key, b"v2").unwrap();

        assert_ne!(r1, r2);
        assert_eq!(get(&tree, &key), Some(b"v2".to_vec()));
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn deterministic_root() {
        let mut tree1 = SparseMerkleTree::new();
        let mut tree2 = SparseMerkleTree::new();

        let k1 = Hash256::digest(b"k1");
        let k2 = Hash256::digest(b"k2");

        insert(&mut tree1, &k1, b"v1").unwrap();
        insert(&mut tree1, &k2, b"v2").unwrap();

        insert(&mut tree2, &k2, b"v2").unwrap();
        insert(&mut tree2, &k1, b"v1").unwrap();

        assert_eq!(tree1.root(), tree2.root());
    }

    #[test]
    fn default_tree() {
        let tree = SparseMerkleTree::default();
        assert!(tree.is_empty());
    }

    #[test]
    fn get_bit_works() {
        let key = Hash256::from_bytes([0xFF; 32]);
        for i in 0..256 {
            assert!(get_bit(&key, i));
        }

        let key_zero = Hash256::ZERO;
        for i in 0..256 {
            assert!(!get_bit(&key_zero, i));
        }
    }

    #[test]
    fn bits_to_key_roundtrip() {
        let key = Hash256::digest(b"test_key");
        let bits: Vec<bool> = (0..256).map(|i| get_bit(&key, i)).collect();
        let reconstructed = bits_to_key(&bits);
        assert_eq!(key, reconstructed);
    }

    #[test]
    fn proof_siblings_length() {
        let mut tree = SparseMerkleTree::new();
        let key = Hash256::digest(b"key");
        insert(&mut tree, &key, b"value").unwrap();

        let proof = prove(&tree, &key);
        assert_eq!(proof.siblings.len(), TREE_DEPTH);
    }

    #[test]
    fn verify_proof_wrong_siblings_length() {
        let root = Hash256::ZERO;
        let key = Hash256::digest(b"key");
        let proof = MerkleProof {
            siblings: vec![Hash256::ZERO],
            key,
            value: Some(b"val".to_vec()),
        };
        assert!(!verify_proof(&root, &key, Some(b"val"), &proof));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_hash256() -> impl Strategy<Value = Hash256> {
        prop::array::uniform32(any::<u8>()).prop_map(|b| Hash256::from_bytes(b))
    }

    fn arb_value() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 1..64)
    }

    proptest! {
        #[test]
        fn inclusion_proof_always_verifies(
            key in arb_hash256(),
            value in arb_value()
        ) {
            let mut tree = SparseMerkleTree::new();
            let root = insert(&mut tree, &key, &value).unwrap();
            let proof = prove(&tree, &key);
            prop_assert!(verify_proof(&root, &key, Some(&value), &proof));
        }

        #[test]
        fn non_inclusion_proof_verifies_for_absent_keys(
            present_key in arb_hash256(),
            absent_key in arb_hash256(),
            value in arb_value()
        ) {
            prop_assume!(present_key != absent_key);
            let mut tree = SparseMerkleTree::new();
            let root = insert(&mut tree, &present_key, &value).unwrap();
            let proof = prove(&tree, &absent_key);
            prop_assert!(verify_proof(&root, &absent_key, None, &proof));
        }

        #[test]
        fn wrong_value_never_verifies(
            key in arb_hash256(),
            value in arb_value(),
            wrong_value in arb_value()
        ) {
            prop_assume!(value != wrong_value);
            let mut tree = SparseMerkleTree::new();
            let root = insert(&mut tree, &key, &value).unwrap();
            let proof = prove(&tree, &key);
            prop_assert!(!verify_proof(&root, &key, Some(&wrong_value), &proof));
        }

        #[test]
        fn root_deterministic_regardless_of_insert_order(
            entries in prop::collection::vec((arb_hash256(), arb_value()), 1..4)
        ) {
            let mut unique: std::collections::BTreeMap<Hash256, Vec<u8>> = std::collections::BTreeMap::new();
            for (k, v) in &entries {
                unique.insert(*k, v.clone());
            }
            let items: Vec<(Hash256, Vec<u8>)> = unique.into_iter().collect();

            if items.len() < 2 {
                return Ok(());
            }

            let mut tree1 = SparseMerkleTree::new();
            for (k, v) in &items {
                insert(&mut tree1, k, v).unwrap();
            }

            let mut tree2 = SparseMerkleTree::new();
            for (k, v) in items.iter().rev() {
                insert(&mut tree2, k, v).unwrap();
            }

            prop_assert_eq!(tree1.root(), tree2.root());
        }
    }
}
