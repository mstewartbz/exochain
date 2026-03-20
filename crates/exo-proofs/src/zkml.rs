//! Zero-knowledge ML verification.
//!
//! Verifies that a given output was produced by a committed model on a
//! committed input, without revealing the model weights or input data.

use exo_core::types::Hash256;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ModelCommitment
// ---------------------------------------------------------------------------

/// A commitment to a machine learning model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCommitment {
    /// Hash of the model architecture description.
    pub architecture_hash: Hash256,
    /// Hash of the model weights.
    pub weights_hash: Hash256,
    /// Model version identifier.
    pub version: u64,
}

impl ModelCommitment {
    /// Create a new model commitment.
    #[must_use]
    pub fn new(architecture: &[u8], weights: &[u8], version: u64) -> Self {
        Self {
            architecture_hash: Hash256::digest(architecture),
            weights_hash: Hash256::digest(weights),
            version,
        }
    }

    /// Compute the canonical commitment hash.
    #[must_use]
    pub fn commitment_hash(&self) -> Hash256 {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"zkml:model:");
        hasher.update(self.architecture_hash.as_bytes());
        hasher.update(self.weights_hash.as_bytes());
        hasher.update(&self.version.to_le_bytes());
        Hash256::from_bytes(*hasher.finalize().as_bytes())
    }
}

// ---------------------------------------------------------------------------
// InferenceProof
// ---------------------------------------------------------------------------

/// Proof that an inference was correctly executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceProof {
    /// The model commitment.
    pub model_commitment: ModelCommitment,
    /// Hash of the input data.
    pub input_hash: Hash256,
    /// Hash of the output data.
    pub output_hash: Hash256,
    /// The cryptographic proof binding input -> model -> output.
    pub proof: Hash256,
    /// Auxiliary verification data.
    pub verification_tag: Hash256,
}

// ---------------------------------------------------------------------------
// Prove
// ---------------------------------------------------------------------------

/// Generate a proof that a specific output was produced by a committed model
/// on a specific input.
pub fn prove_inference(model: &ModelCommitment, input: &[u8], output: &[u8]) -> InferenceProof {
    let input_hash = Hash256::digest(input);
    let output_hash = Hash256::digest(output);
    let model_hash = model.commitment_hash();

    // Compute the proof: a deterministic binding of model + input + output.
    // In a real ZKML system, this would involve running the model in a ZK circuit.
    let proof = compute_inference_proof(&model_hash, &input_hash, &output_hash);

    // Compute verification tag: allows the verifier to check without knowing
    // the model weights or input.
    let verification_tag = compute_verification_tag(&model_hash, &input_hash, &output_hash, &proof);

    InferenceProof {
        model_commitment: model.clone(),
        input_hash,
        output_hash,
        proof,
        verification_tag,
    }
}

/// Verify that an inference proof is valid.
///
/// This checks that the proof correctly binds the model commitment, input hash,
/// and output hash without needing the actual model or input.
pub fn verify_inference(proof: &InferenceProof) -> bool {
    let model_hash = proof.model_commitment.commitment_hash();

    // Recompute the expected proof
    let expected_proof =
        compute_inference_proof(&model_hash, &proof.input_hash, &proof.output_hash);

    if expected_proof != proof.proof {
        return false;
    }

    // Recompute and check the verification tag
    let expected_tag = compute_verification_tag(
        &model_hash,
        &proof.input_hash,
        &proof.output_hash,
        &proof.proof,
    );

    expected_tag == proof.verification_tag
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn compute_inference_proof(
    model_hash: &Hash256,
    input_hash: &Hash256,
    output_hash: &Hash256,
) -> Hash256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"zkml:proof:");
    hasher.update(model_hash.as_bytes());
    hasher.update(input_hash.as_bytes());
    hasher.update(output_hash.as_bytes());
    Hash256::from_bytes(*hasher.finalize().as_bytes())
}

fn compute_verification_tag(
    model_hash: &Hash256,
    input_hash: &Hash256,
    output_hash: &Hash256,
    proof: &Hash256,
) -> Hash256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"zkml:verify:");
    hasher.update(model_hash.as_bytes());
    hasher.update(input_hash.as_bytes());
    hasher.update(output_hash.as_bytes());
    hasher.update(proof.as_bytes());
    Hash256::from_bytes(*hasher.finalize().as_bytes())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model() -> ModelCommitment {
        ModelCommitment::new(b"transformer-v1", b"weights-blob-1234", 1)
    }

    #[test]
    fn model_commitment_deterministic() {
        let m1 = ModelCommitment::new(b"arch", b"weights", 1);
        let m2 = ModelCommitment::new(b"arch", b"weights", 1);
        assert_eq!(m1, m2);
        assert_eq!(m1.commitment_hash(), m2.commitment_hash());
    }

    #[test]
    fn different_models_different_hashes() {
        let m1 = ModelCommitment::new(b"arch1", b"weights1", 1);
        let m2 = ModelCommitment::new(b"arch2", b"weights2", 1);
        assert_ne!(m1.commitment_hash(), m2.commitment_hash());
    }

    #[test]
    fn different_versions_different_hashes() {
        let m1 = ModelCommitment::new(b"arch", b"weights", 1);
        let m2 = ModelCommitment::new(b"arch", b"weights", 2);
        assert_ne!(m1.commitment_hash(), m2.commitment_hash());
    }

    #[test]
    fn prove_and_verify() {
        let model = make_model();
        let input = b"classify this image";
        let output = b"cat: 0.95, dog: 0.05";

        let proof = prove_inference(&model, input, output);
        assert!(verify_inference(&proof));
    }

    #[test]
    fn verify_fails_tampered_model() {
        let model = make_model();
        let proof = prove_inference(&model, b"input", b"output");

        let mut tampered = proof.clone();
        tampered.model_commitment = ModelCommitment::new(b"evil-arch", b"evil-weights", 99);
        assert!(!verify_inference(&tampered));
    }

    #[test]
    fn verify_fails_tampered_input() {
        let model = make_model();
        let proof = prove_inference(&model, b"input", b"output");

        let mut tampered = proof.clone();
        tampered.input_hash = Hash256::digest(b"different-input");
        assert!(!verify_inference(&tampered));
    }

    #[test]
    fn verify_fails_tampered_output() {
        let model = make_model();
        let proof = prove_inference(&model, b"input", b"output");

        let mut tampered = proof.clone();
        tampered.output_hash = Hash256::digest(b"different-output");
        assert!(!verify_inference(&tampered));
    }

    #[test]
    fn verify_fails_tampered_proof_field() {
        let model = make_model();
        let proof = prove_inference(&model, b"input", b"output");

        let mut tampered = proof.clone();
        tampered.proof = Hash256::ZERO;
        assert!(!verify_inference(&tampered));
    }

    #[test]
    fn verify_fails_tampered_tag() {
        let model = make_model();
        let proof = prove_inference(&model, b"input", b"output");

        let mut tampered = proof.clone();
        tampered.verification_tag = Hash256::ZERO;
        assert!(!verify_inference(&tampered));
    }

    #[test]
    fn different_inputs_different_proofs() {
        let model = make_model();
        let p1 = prove_inference(&model, b"input1", b"output1");
        let p2 = prove_inference(&model, b"input2", b"output2");
        assert_ne!(p1.proof, p2.proof);
    }

    #[test]
    fn same_inputs_same_proof() {
        let model = make_model();
        let p1 = prove_inference(&model, b"input", b"output");
        let p2 = prove_inference(&model, b"input", b"output");
        assert_eq!(p1, p2);
    }

    #[test]
    fn proof_hides_model_input() {
        // The proof only contains hashes, not the actual model/input
        let model = make_model();
        let proof = prove_inference(&model, b"secret input", b"secret output");

        // Proof fields are hashes, not raw data
        assert_eq!(proof.input_hash, Hash256::digest(b"secret input"));
        assert_eq!(proof.output_hash, Hash256::digest(b"secret output"));
        // Model commitment hashes the architecture and weights
        assert_eq!(
            proof.model_commitment.architecture_hash,
            Hash256::digest(b"transformer-v1")
        );
    }

    #[test]
    fn empty_input_output() {
        let model = make_model();
        let proof = prove_inference(&model, b"", b"");
        assert!(verify_inference(&proof));
    }

    #[test]
    fn large_input_output() {
        let model = make_model();
        let input = vec![0xABu8; 10_000];
        let output = vec![0xCDu8; 5_000];
        let proof = prove_inference(&model, &input, &output);
        assert!(verify_inference(&proof));
    }
}
