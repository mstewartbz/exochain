//! Checkpoint finality aggregation for the EXOCHAIN DAG.
//!
//! Aggregates MMR event root, SMT state root, finalized height, frontier
//! tip set, and multi-validator signatures into a single checkpoint payload.
//! Provides the canonical signing preimage (Spec 9.4) with domain separation
//! tag `EXOCHAIN-CHECKPOINT-v1`.

use exo_core::{Did, Hash256, Signature};
use serde::{Deserialize, Serialize};

/// A checkpoint payload aggregating finality proof components.
///
/// Validators collectively attest to a finalized DAG state by signing
/// the canonical preimage computed by [`checkpoint_signing_preimage`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointPayload {
    /// MMR root over finalized event IDs.
    pub event_root: Hash256,

    /// SMT root over derived state.
    pub state_root: Hash256,

    /// Height (sequence number) of this checkpoint.
    pub height: u64,

    /// Count of finalized events covered by this checkpoint.
    pub finalized_events: u64,

    /// Frontier hashes (DAG tips at checkpoint time).
    pub frontier: Vec<Hash256>,

    /// Validator signatures attesting to this checkpoint.
    pub validator_sigs: Vec<ValidatorSignature>,
}

/// A single validator's attestation to a checkpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorSignature {
    /// DID of the validating entity.
    pub validator_did: Did,
    /// Key version used for signing.
    pub key_version: u64,
    /// Cryptographic signature over the checkpoint preimage.
    pub signature: Signature,
}

/// Domain separation tag for checkpoint signing (EXOCHAIN Specification v2.2 §9.4).
pub const CHECKPOINT_DOMAIN_SEP: &[u8] = b"EXOCHAIN-CHECKPOINT-v1";

/// Compute the normative checkpoint signing preimage (EXOCHAIN Specification v2.2 §9.4).
///
/// The preimage layout is:
/// `[domain_sep | event_root | state_root | height_le | finalized_events_le | frontier...]`
///
/// Validators sign this preimage to attest to the checkpoint.
#[must_use]
pub fn checkpoint_signing_preimage(cp: &CheckpointPayload) -> Vec<u8> {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(CHECKPOINT_DOMAIN_SEP);
    preimage.extend_from_slice(cp.event_root.as_bytes());
    preimage.extend_from_slice(cp.state_root.as_bytes());
    preimage.extend_from_slice(&cp.height.to_le_bytes());
    preimage.extend_from_slice(&cp.finalized_events.to_le_bytes());
    for frontier_hash in &cp.frontier {
        preimage.extend_from_slice(frontier_hash.as_bytes());
    }
    preimage
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did() -> Did {
        Did::new("did:exo:validator1").expect("valid")
    }

    #[test]
    fn preimage_deterministic() {
        let cp = CheckpointPayload {
            event_root: Hash256::digest(b"events"),
            state_root: Hash256::digest(b"state"),
            height: 42,
            finalized_events: 100,
            frontier: vec![Hash256::digest(b"tip1"), Hash256::digest(b"tip2")],
            validator_sigs: vec![],
        };

        let p1 = checkpoint_signing_preimage(&cp);
        let p2 = checkpoint_signing_preimage(&cp);
        assert_eq!(p1, p2, "preimage must be deterministic");
    }

    #[test]
    fn preimage_starts_with_domain_sep() {
        let cp = CheckpointPayload {
            event_root: Hash256::ZERO,
            state_root: Hash256::ZERO,
            height: 0,
            finalized_events: 0,
            frontier: vec![],
            validator_sigs: vec![],
        };

        let preimage = checkpoint_signing_preimage(&cp);
        assert!(preimage.starts_with(CHECKPOINT_DOMAIN_SEP));
    }

    #[test]
    fn preimage_includes_frontier() {
        let tip = Hash256::digest(b"frontier-tip");
        let cp_with = CheckpointPayload {
            event_root: Hash256::ZERO,
            state_root: Hash256::ZERO,
            height: 1,
            finalized_events: 1,
            frontier: vec![tip],
            validator_sigs: vec![],
        };
        let cp_without = CheckpointPayload {
            event_root: Hash256::ZERO,
            state_root: Hash256::ZERO,
            height: 1,
            finalized_events: 1,
            frontier: vec![],
            validator_sigs: vec![],
        };

        assert_ne!(
            checkpoint_signing_preimage(&cp_with),
            checkpoint_signing_preimage(&cp_without),
            "frontier must affect preimage"
        );
    }

    #[test]
    fn validator_signature_construction() {
        let sig = ValidatorSignature {
            validator_did: test_did(),
            key_version: 1,
            signature: Signature::Empty,
        };
        assert_eq!(sig.key_version, 1);
    }
}
