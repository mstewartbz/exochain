//! Checkpoint finality aggregation for the EXOCHAIN DAG.
//!
//! Aggregates MMR event root, SMT state root, finalized height, frontier
//! tip set, and multi-validator signatures into a single checkpoint payload.
//! Provides the canonical signing preimage (Spec 9.4) with domain separation
//! tag `EXOCHAIN-CHECKPOINT-v1`.

use exo_core::{Did, Hash256, Signature};
use serde::{Deserialize, Serialize};

use crate::error::{DagError, Result};

/// A checkpoint payload aggregating finality proof components.
///
/// Validators collectively attest to a finalized DAG state by signing
/// the canonical preimage computed by [`checkpoint_signing_preimage`].
#[derive(Clone, Serialize, Deserialize)]
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

impl std::fmt::Debug for CheckpointPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckpointPayload")
            .field("event_root", &self.event_root)
            .field("state_root", &self.state_root)
            .field("height", &self.height)
            .field("finalized_events", &self.finalized_events)
            .field("frontier", &self.frontier)
            .field("validator_sig_count", &self.validator_sigs.len())
            .finish()
    }
}

/// A single validator's attestation to a checkpoint.
#[derive(Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    /// DID of the validating entity.
    pub validator_did: Did,
    /// Key version used for signing.
    pub key_version: u64,
    /// Cryptographic signature over the checkpoint preimage.
    pub signature: Signature,
}

impl std::fmt::Debug for ValidatorSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorSignature")
            .field("validator_did", &self.validator_did)
            .field("key_version", &self.key_version)
            .field("signature", &"<redacted>")
            .finish()
    }
}

/// Domain separation tag for checkpoint signing (EXOCHAIN Specification v2.2 §9.4).
pub const CHECKPOINT_DOMAIN_SEP: &[u8] = b"EXOCHAIN-CHECKPOINT-v1";

/// Compute the normative checkpoint signing preimage (EXOCHAIN Specification v2.2 §9.4).
///
/// The preimage layout is:
/// `[domain_sep | event_root | state_root | height_le | finalized_events_le | frontier_len_le | frontier...]`
///
/// Validators sign this preimage to attest to the checkpoint.
///
/// # Errors
///
/// Returns [`DagError::Serialization`] if a checkpoint field cannot be
/// represented exactly in the canonical signing frame.
pub fn checkpoint_signing_preimage(cp: &CheckpointPayload) -> Result<Vec<u8>> {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(CHECKPOINT_DOMAIN_SEP);
    preimage.extend_from_slice(cp.event_root.as_bytes());
    preimage.extend_from_slice(cp.state_root.as_bytes());
    preimage.extend_from_slice(&cp.height.to_le_bytes());
    preimage.extend_from_slice(&cp.finalized_events.to_le_bytes());
    let frontier_len = u64::try_from(cp.frontier.len()).map_err(|_| {
        DagError::Serialization(format!(
            "checkpoint frontier length {} cannot be represented in canonical signing frame",
            cp.frontier.len()
        ))
    })?;
    preimage.extend_from_slice(&frontier_len.to_le_bytes());
    for frontier_hash in &cp.frontier {
        preimage.extend_from_slice(frontier_hash.as_bytes());
    }
    Ok(preimage)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    fn arb_hash256() -> impl Strategy<Value = Hash256> {
        any::<[u8; 32]>().prop_map(Hash256::from_bytes)
    }

    fn arb_checkpoint() -> impl Strategy<Value = CheckpointPayload> {
        (
            arb_hash256(),
            arb_hash256(),
            any::<u64>(),
            any::<u64>(),
            prop::collection::vec(arb_hash256(), 0..=8usize),
        )
            .prop_map(
                |(event_root, state_root, height, finalized_events, frontier)| CheckpointPayload {
                    event_root,
                    state_root,
                    height,
                    finalized_events,
                    frontier,
                    validator_sigs: vec![],
                },
            )
    }

    fn preimage(cp: &CheckpointPayload) -> Vec<u8> {
        checkpoint_signing_preimage(cp).expect("test checkpoint preimage must encode")
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]

        /// The preimage function must be pure: same input → same bytes.
        #[test]
        fn preimage_is_deterministic(cp in arb_checkpoint()) {
            let p1 = preimage(&cp);
            let p2 = preimage(&cp);
            prop_assert_eq!(p1, p2);
        }

        /// Domain separation tag must always lead the preimage.
        #[test]
        fn preimage_starts_with_domain_sep(cp in arb_checkpoint()) {
            let preimage = preimage(&cp);
            prop_assert!(preimage.starts_with(CHECKPOINT_DOMAIN_SEP));
        }

        /// Preimage length must equal the sum of all fixed and variable fields.
        #[test]
        fn preimage_length_accounts_for_all_fields(cp in arb_checkpoint()) {
            let expected = CHECKPOINT_DOMAIN_SEP.len() // 22
                + 32  // event_root
                + 32  // state_root
                + 8   // height (le64)
                + 8   // finalized_events (le64)
                + 8   // frontier length (le64)
                + 32 * cp.frontier.len();
            let preimage = preimage(&cp);
            prop_assert_eq!(preimage.len(), expected);
        }

        /// Any change to `height` must change the preimage.
        #[test]
        fn different_heights_produce_different_preimages(
            mut cp in arb_checkpoint(),
            alt_height in any::<u64>(),
        ) {
            prop_assume!(cp.height != alt_height);
            let p1 = preimage(&cp);
            cp.height = alt_height;
            let p2 = preimage(&cp);
            prop_assert_ne!(p1, p2);
        }

        /// Any change to `event_root` must change the preimage.
        #[test]
        fn different_event_roots_produce_different_preimages(
            mut cp in arb_checkpoint(),
            alt_root in arb_hash256(),
        ) {
            prop_assume!(cp.event_root != alt_root);
            let p1 = preimage(&cp);
            cp.event_root = alt_root;
            let p2 = preimage(&cp);
            prop_assert_ne!(p1, p2);
        }

        /// Any change to `state_root` must change the preimage.
        #[test]
        fn different_state_roots_produce_different_preimages(
            mut cp in arb_checkpoint(),
            alt_root in arb_hash256(),
        ) {
            prop_assume!(cp.state_root != alt_root);
            let p1 = preimage(&cp);
            cp.state_root = alt_root;
            let p2 = preimage(&cp);
            prop_assert_ne!(p1, p2);
        }

        /// Any change to `finalized_events` must change the preimage.
        #[test]
        fn different_finalized_events_produce_different_preimages(
            mut cp in arb_checkpoint(),
            alt_count in any::<u64>(),
        ) {
            prop_assume!(cp.finalized_events != alt_count);
            let p1 = preimage(&cp);
            cp.finalized_events = alt_count;
            let p2 = preimage(&cp);
            prop_assert_ne!(p1, p2);
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_did() -> Did {
        Did::new("did:exo:validator1").expect("valid")
    }

    fn preimage(cp: &CheckpointPayload) -> Vec<u8> {
        checkpoint_signing_preimage(cp).expect("test checkpoint preimage must encode")
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

        let p1 = preimage(&cp);
        let p2 = preimage(&cp);
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

        let preimage = preimage(&cp);
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
            preimage(&cp_with),
            preimage(&cp_without),
            "frontier must affect preimage"
        );
    }

    #[test]
    fn preimage_encodes_frontier_length_before_frontier_hashes() {
        let tip1 = Hash256::digest(b"frontier-tip-1");
        let tip2 = Hash256::digest(b"frontier-tip-2");
        let cp = CheckpointPayload {
            event_root: Hash256::digest(b"events"),
            state_root: Hash256::digest(b"state"),
            height: 7,
            finalized_events: 11,
            frontier: vec![tip1, tip2],
            validator_sigs: vec![],
        };

        let preimage = preimage(&cp);
        let offset = CHECKPOINT_DOMAIN_SEP.len() + 32 + 32 + 8 + 8;

        assert_eq!(&preimage[offset..offset + 8], &2u64.to_le_bytes());
        assert_eq!(&preimage[offset + 8..offset + 40], tip1.as_bytes());
        assert_eq!(&preimage[offset + 40..offset + 72], tip2.as_bytes());
    }

    #[test]
    fn checkpoint_preimage_does_not_saturate_frontier_length() {
        let production = include_str!("checkpoint.rs");
        let preimage_section = production
            .split("pub fn checkpoint_signing_preimage")
            .nth(1)
            .expect("checkpoint_signing_preimage function must exist")
            .split("// ===========================================================================")
            .next()
            .expect("test separator must follow checkpoint_signing_preimage");

        assert!(
            !preimage_section.contains("unwrap_or(u64::MAX)"),
            "checkpoint signing preimage must fail closed instead of saturating frontier length"
        );
    }

    #[test]
    fn checkpoint_debug_redacts_validator_signature_material() {
        let checkpoint = CheckpointPayload {
            event_root: Hash256::digest(b"events"),
            state_root: Hash256::digest(b"state"),
            height: 7,
            finalized_events: 11,
            frontier: vec![Hash256::digest(b"frontier-tip")],
            validator_sigs: vec![ValidatorSignature {
                validator_did: test_did(),
                key_version: 1,
                signature: Signature::from_bytes([0xAB; 64]),
            }],
        };

        let checkpoint_debug = format!("{checkpoint:?}");
        let validator_debug = format!("{:?}", checkpoint.validator_sigs[0]);

        assert!(
            checkpoint_debug.contains("validator_sig_count: 1"),
            "Checkpoint Debug output should expose signature count, not signature bodies"
        );
        assert!(
            validator_debug.contains("signature: \"<redacted>\""),
            "ValidatorSignature Debug output must explicitly redact signature material"
        );
        assert!(
            !checkpoint_debug.contains("Signature::Ed25519")
                && !validator_debug.contains("Signature::Ed25519"),
            "Debug output must not delegate to Signature Debug for checkpoint signatures"
        );
        assert!(
            !checkpoint_debug.contains("abab") && !validator_debug.contains("abab"),
            "Debug output must not expose validator signature byte prefixes"
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
