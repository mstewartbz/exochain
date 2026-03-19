use ed25519_dalek::Signature;
use exo_core::{Blake3Hash, Did};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointPayload {
    /// MMR root over finalized event_ids.
    pub event_root: Blake3Hash,

    /// SMT root over derived state.
    pub state_root: Blake3Hash,

    /// Height (sequence number).
    pub height: u64,

    /// Count of finalized events.
    pub finalized_events: u64,

    /// Frontier hashes.
    pub frontier: Vec<Blake3Hash>,

    /// Signatures.
    pub validator_sigs: Vec<ValidatorSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorSignature {
    pub validator_did: Did,
    pub key_version: u64,
    pub signature: Signature,
}

pub const CHECKPOINT_DOMAIN_SEP: &[u8] = b"EXOCHAIN-CHECKPOINT-v1";

/// Compute normative checkpoint signing preimage (Spec 9.4).
pub fn checkpoint_signing_preimage(cp: &CheckpointPayload) -> Vec<u8> {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(CHECKPOINT_DOMAIN_SEP);
    preimage.extend_from_slice(&cp.event_root.0);
    preimage.extend_from_slice(&cp.state_root.0);
    preimage.extend_from_slice(&cp.height.to_le_bytes());
    preimage.extend_from_slice(&cp.finalized_events.to_le_bytes());
    for frontier_hash in &cp.frontier {
        preimage.extend_from_slice(&frontier_hash.0);
    }
    preimage
}
