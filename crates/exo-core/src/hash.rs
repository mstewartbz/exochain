//! Canonical hashing utilities.
//!
//! All hashing in EXOCHAIN uses **blake3** and all structured data is
//! first serialized to **CBOR** (deterministic canonical encoding) before
//! hashing.  This guarantees that identical logical values always produce
//! the same hash regardless of serialization order or platform.

use serde::Serialize;

use crate::error::{ExoError, Result};
use crate::types::Hash256;

/// Compute the blake3 hash of raw bytes.
#[must_use]
pub fn canonical_hash(data: &[u8]) -> Hash256 {
    Hash256::digest(data)
}

/// Serialize `value` to CBOR, then compute the blake3 hash.
///
/// # Errors
///
/// Returns `ExoError::SerializationError` if CBOR encoding fails.
pub fn hash_structured<T: Serialize>(value: &T) -> Result<Hash256> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf)?;
    Ok(canonical_hash(&buf))
}

/// Compute a deterministic Merkle root from a slice of leaf hashes.
///
/// - Empty input returns `Hash256::ZERO`.
/// - Single leaf returns the leaf itself.
/// - Otherwise, leaves are paired left-to-right; an odd leaf is promoted
///   (duplicated) to fill the pair.  This process repeats until one root
///   remains.
#[must_use]
pub fn merkle_root(leaves: &[Hash256]) -> Hash256 {
    if leaves.is_empty() {
        return Hash256::ZERO;
    }
    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut current: Vec<Hash256> = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::with_capacity((current.len() + 1) / 2);
        let mut i = 0;
        while i < current.len() {
            let left = &current[i];
            let right = if i + 1 < current.len() {
                &current[i + 1]
            } else {
                // Odd leaf — duplicate
                left
            };
            next.push(hash_pair(left, right));
            i += 2;
        }
        current = next;
    }
    current[0]
}

/// Generate a Merkle proof for the leaf at `index`.
///
/// Returns the sibling hashes needed to reconstruct the root, along with
/// the directions (false = left sibling, true = right sibling path element).
///
/// # Errors
///
/// Returns `ExoError::InvalidMerkleProof` if `index` is out of bounds.
pub fn merkle_proof(leaves: &[Hash256], index: usize) -> Result<Vec<Hash256>> {
    if index >= leaves.len() || leaves.is_empty() {
        return Err(ExoError::InvalidMerkleProof);
    }
    if leaves.len() == 1 {
        return Ok(Vec::new());
    }

    let mut proof = Vec::new();
    let mut current: Vec<Hash256> = leaves.to_vec();
    let mut idx = index;

    while current.len() > 1 {
        // If odd number, duplicate the last element
        if current.len() % 2 != 0 {
            let last = current[current.len() - 1];
            current.push(last);
        }
        let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
        proof.push(current[sibling_idx]);

        // Build next level
        let mut next = Vec::with_capacity(current.len() / 2);
        let mut i = 0;
        while i < current.len() {
            next.push(hash_pair(&current[i], &current[i + 1]));
            i += 2;
        }
        current = next;
        idx /= 2;
    }

    Ok(proof)
}

/// Verify a Merkle proof.
///
/// Given the expected `root`, a `leaf` hash, the `proof` (sibling hashes),
/// and the `index` of the leaf in the original tree, returns `true` if the
/// proof is valid.
#[must_use]
pub fn verify_merkle_proof(
    root: &Hash256,
    leaf: &Hash256,
    proof: &[Hash256],
    index: usize,
) -> bool {
    let mut current = *leaf;
    let mut idx = index;

    for sibling in proof {
        if idx % 2 == 0 {
            current = hash_pair(&current, sibling);
        } else {
            current = hash_pair(sibling, &current);
        }
        idx /= 2;
    }

    current == *root
}

/// Hash two nodes together: `H(left || right)`.
#[must_use]
fn hash_pair(left: &Hash256, right: &Hash256) -> Hash256 {
    let mut combined = [0u8; 64];
    combined[..32].copy_from_slice(left.as_bytes());
    combined[32..].copy_from_slice(right.as_bytes());
    canonical_hash(&combined)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_hash_deterministic() {
        let h1 = canonical_hash(b"hello world");
        let h2 = canonical_hash(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn canonical_hash_different_inputs() {
        let h1 = canonical_hash(b"aaa");
        let h2 = canonical_hash(b"bbb");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_structured_deterministic() {
        #[derive(Serialize)]
        struct Foo {
            a: u32,
            b: String,
        }
        let v = Foo {
            a: 42,
            b: "hello".into(),
        };
        let h1 = hash_structured(&v).expect("ok");
        let h2 = hash_structured(&v).expect("ok");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_structured_different_values() {
        let h1 = hash_structured(&42u32).expect("ok");
        let h2 = hash_structured(&43u32).expect("ok");
        assert_ne!(h1, h2);
    }

    #[test]
    fn merkle_root_empty() {
        assert_eq!(merkle_root(&[]), Hash256::ZERO);
    }

    #[test]
    fn merkle_root_single() {
        let leaf = Hash256::digest(b"only");
        assert_eq!(merkle_root(&[leaf]), leaf);
    }

    #[test]
    fn merkle_root_two_leaves() {
        let a = Hash256::digest(b"a");
        let b = Hash256::digest(b"b");
        let root = merkle_root(&[a, b]);
        // Should be hash_pair(a, b)
        let expected = hash_pair(&a, &b);
        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_three_leaves_odd() {
        let a = Hash256::digest(b"a");
        let b = Hash256::digest(b"b");
        let c = Hash256::digest(b"c");
        let root = merkle_root(&[a, b, c]);
        // Level 1: hash_pair(a,b), hash_pair(c,c)
        // Level 0: hash_pair(hash_pair(a,b), hash_pair(c,c))
        let ab = hash_pair(&a, &b);
        let cc = hash_pair(&c, &c);
        let expected = hash_pair(&ab, &cc);
        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_four_leaves() {
        let leaves: Vec<Hash256> = (0..4u8).map(|i| Hash256::digest(&[i])).collect();
        let root = merkle_root(&leaves);
        let ab = hash_pair(&leaves[0], &leaves[1]);
        let cd = hash_pair(&leaves[2], &leaves[3]);
        let expected = hash_pair(&ab, &cd);
        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_deterministic() {
        let leaves: Vec<Hash256> = (0..7u8).map(|i| Hash256::digest(&[i])).collect();
        let r1 = merkle_root(&leaves);
        let r2 = merkle_root(&leaves);
        assert_eq!(r1, r2);
    }

    #[test]
    fn merkle_proof_empty() {
        let result = merkle_proof(&[], 0);
        assert!(result.is_err());
    }

    #[test]
    fn merkle_proof_out_of_bounds() {
        let leaf = Hash256::digest(b"x");
        let result = merkle_proof(&[leaf], 1);
        assert!(result.is_err());
    }

    #[test]
    fn merkle_proof_single_leaf() {
        let leaf = Hash256::digest(b"only");
        let proof = merkle_proof(&[leaf], 0).expect("ok");
        assert!(proof.is_empty());
        let root = merkle_root(&[leaf]);
        assert!(verify_merkle_proof(&root, &leaf, &proof, 0));
    }

    #[test]
    fn merkle_proof_two_leaves() {
        let leaves = vec![Hash256::digest(b"a"), Hash256::digest(b"b")];
        let root = merkle_root(&leaves);

        for i in 0..2 {
            let proof = merkle_proof(&leaves, i).expect("ok");
            assert!(verify_merkle_proof(&root, &leaves[i], &proof, i));
        }
    }

    #[test]
    fn merkle_proof_four_leaves() {
        let leaves: Vec<Hash256> = (0..4u8).map(|i| Hash256::digest(&[i])).collect();
        let root = merkle_root(&leaves);

        for i in 0..4 {
            let proof = merkle_proof(&leaves, i).expect("ok");
            assert!(
                verify_merkle_proof(&root, &leaves[i], &proof, i),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn merkle_proof_odd_leaves() {
        let leaves: Vec<Hash256> = (0..5u8).map(|i| Hash256::digest(&[i])).collect();
        let root = merkle_root(&leaves);

        for i in 0..5 {
            let proof = merkle_proof(&leaves, i).expect("ok");
            assert!(
                verify_merkle_proof(&root, &leaves[i], &proof, i),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn merkle_proof_seven_leaves() {
        let leaves: Vec<Hash256> = (0..7u8).map(|i| Hash256::digest(&[i])).collect();
        let root = merkle_root(&leaves);

        for i in 0..7 {
            let proof = merkle_proof(&leaves, i).expect("ok");
            assert!(
                verify_merkle_proof(&root, &leaves[i], &proof, i),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn verify_merkle_proof_rejects_wrong_leaf() {
        let leaves: Vec<Hash256> = (0..4u8).map(|i| Hash256::digest(&[i])).collect();
        let root = merkle_root(&leaves);
        let proof = merkle_proof(&leaves, 0).expect("ok");
        let wrong_leaf = Hash256::digest(b"wrong");
        assert!(!verify_merkle_proof(&root, &wrong_leaf, &proof, 0));
    }

    #[test]
    fn verify_merkle_proof_rejects_wrong_index() {
        let leaves: Vec<Hash256> = (0..4u8).map(|i| Hash256::digest(&[i])).collect();
        let root = merkle_root(&leaves);
        let proof = merkle_proof(&leaves, 0).expect("ok");
        // Use correct leaf but wrong index
        assert!(!verify_merkle_proof(&root, &leaves[0], &proof, 1));
    }

    #[test]
    fn verify_merkle_proof_rejects_wrong_root() {
        let leaves: Vec<Hash256> = (0..4u8).map(|i| Hash256::digest(&[i])).collect();
        let proof = merkle_proof(&leaves, 0).expect("ok");
        let wrong_root = Hash256::digest(b"wrong root");
        assert!(!verify_merkle_proof(&wrong_root, &leaves[0], &proof, 0));
    }

    #[test]
    fn hash_pair_deterministic() {
        let a = Hash256::digest(b"left");
        let b = Hash256::digest(b"right");
        let h1 = hash_pair(&a, &b);
        let h2 = hash_pair(&a, &b);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_pair_not_commutative() {
        let a = Hash256::digest(b"left");
        let b = Hash256::digest(b"right");
        assert_ne!(hash_pair(&a, &b), hash_pair(&b, &a));
    }
}
