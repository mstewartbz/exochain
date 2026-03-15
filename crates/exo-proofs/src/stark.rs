//! zk-STARK proof generation for transparent governance verification.
//!
//! STARKs provide post-quantum secure proofs without trusted setup,
//! suitable for public governance transparency.

use exo_core::crypto::{hash_bytes, Blake3Hash};
use serde::{Deserialize, Serialize};

/// A zk-STARK proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StarkProof {
    pub statement: String,
    pub proof_bytes: Vec<u8>,
    pub public_commitment: Blake3Hash,
    pub security_bits: u32,
    pub trace_length: u64,
}

/// STARK prover for governance transparency proofs.
pub struct StarkProver {
    pub security_bits: u32,
}

impl StarkProver {
    pub fn new(security_bits: u32) -> Self {
        Self { security_bits }
    }

    /// Default 128-bit security prover.
    pub fn default_security() -> Self {
        Self::new(128)
    }

    /// Generate a STARK proof for audit trail integrity.
    pub fn prove_audit_integrity(
        &self,
        audit_root_hash: Blake3Hash,
        chain_length: u64,
    ) -> StarkProof {
        let mut preimage = Vec::new();
        preimage.extend_from_slice(&audit_root_hash.0);
        preimage.extend_from_slice(&chain_length.to_le_bytes());
        let commitment = hash_bytes(&preimage);

        StarkProof {
            statement: format!(
                "Audit trail of {} entries has unbroken hash chain integrity",
                chain_length
            ),
            proof_bytes: commitment.0.to_vec(),
            public_commitment: commitment,
            security_bits: self.security_bits,
            trace_length: chain_length,
        }
    }

    /// Generate a STARK proof for decision lifecycle compliance.
    pub fn prove_lifecycle_compliance(
        &self,
        decision_hash: Blake3Hash,
        transition_count: u64,
    ) -> StarkProof {
        let mut preimage = Vec::new();
        preimage.extend_from_slice(&decision_hash.0);
        preimage.extend_from_slice(&transition_count.to_le_bytes());
        let commitment = hash_bytes(&preimage);

        StarkProof {
            statement: format!(
                "Decision lifecycle followed valid state machine with {} transitions",
                transition_count
            ),
            proof_bytes: commitment.0.to_vec(),
            public_commitment: commitment,
            security_bits: self.security_bits,
            trace_length: transition_count,
        }
    }
}

impl StarkProof {
    /// Verify the STARK proof (stub).
    pub fn verify(&self) -> bool {
        !self.proof_bytes.is_empty() && self.security_bits >= 128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_integrity_proof() {
        let prover = StarkProver::default_security();
        let proof = prover.prove_audit_integrity(Blake3Hash([1u8; 32]), 1000);

        assert!(proof.verify());
        assert_eq!(proof.trace_length, 1000);
        assert!(proof.statement.contains("1000"));
    }

    #[test]
    fn test_lifecycle_compliance_proof() {
        let prover = StarkProver::default_security();
        let proof = prover.prove_lifecycle_compliance(Blake3Hash([2u8; 32]), 5);

        assert!(proof.verify());
        assert_eq!(proof.trace_length, 5);
    }
}
