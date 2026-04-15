use exo_core::types::Hash256;

/// Cryptographically commits to a position before revealing it.
/// Uses BLAKE3 under the hood.
pub fn commit(position_text: &str) -> Hash256 {
    Hash256::digest(position_text.as_bytes())
}

/// Verifies that a revealed position matches its prior commitment.
pub fn verify_commitment(position_text: &str, commitment: &Hash256) -> bool {
    commit(position_text) == *commitment
}
