//! Event system for EXOCHAIN.
//!
//! Every significant action produces a signed, timestamped event that can
//! be verified independently.  Events carry a CBOR-encoded payload and are
//! attributed to a DID via an Ed25519 signature.

use serde::{Deserialize, Serialize};

use crate::crypto;
use crate::types::{CorrelationId, Did, PublicKey, Signature, Timestamp};

// ---------------------------------------------------------------------------
// EventType
// ---------------------------------------------------------------------------

/// Classification of system events.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// A BCTS transaction changed state.
    TransactionStateChanged,
    /// An identity was resolved.
    IdentityResolved,
    /// Consent was granted.
    ConsentGranted,
    /// Consent was revoked.
    ConsentRevoked,
    /// An invariant was checked.
    InvariantChecked,
    /// An invariant was violated.
    InvariantViolated,
    /// A governance decision was made.
    GovernanceDecision,
    /// An escalation was triggered.
    EscalationTriggered,
    /// A sybil detection alert was raised.
    SybilAlert,
    /// A cryptographic key was rotated.
    KeyRotated,
    /// A new entity was registered.
    EntityRegistered,
    /// An audit log entry.
    AuditEntry,
    /// Custom / extension event.
    Custom(String),
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// A signed, timestamped event in the EXOCHAIN system.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for this event.
    pub id: CorrelationId,
    /// HLC timestamp of when the event was created.
    pub timestamp: Timestamp,
    /// Classification of the event.
    pub event_type: EventType,
    /// CBOR-encoded payload (opaque bytes).
    pub payload: Vec<u8>,
    /// DID of the entity that produced the event.
    pub source_did: Did,
    /// Ed25519 signature over the canonical event content.
    pub signature: Signature,
}

impl Event {
    /// Construct the canonical bytes that are signed.
    ///
    /// The signed content is: `id || timestamp || event_type || payload || source_did`
    /// serialized as CBOR.
    #[must_use]
    pub fn signable_bytes(&self) -> Vec<u8> {
        #[derive(Serialize)]
        struct Signable<'a> {
            id: &'a CorrelationId,
            timestamp: &'a Timestamp,
            event_type: &'a EventType,
            payload: &'a [u8],
            source_did: &'a Did,
        }
        let s = Signable {
            id: &self.id,
            timestamp: &self.timestamp,
            event_type: &self.event_type,
            payload: &self.payload,
            source_did: &self.source_did,
        };
        let mut buf = Vec::new();
        // CBOR encoding should not fail for these types in practice.
        ciborium::into_writer(&s, &mut buf).expect("CBOR encoding of signable bytes");
        buf
    }
}

/// Verify that an event's signature is valid for the given public key.
#[must_use]
pub fn verify_event(event: &Event, public_key: &PublicKey) -> bool {
    let bytes = event.signable_bytes();
    crypto::verify(&bytes, &event.signature, public_key)
}

/// Helper: create a signed event.
pub fn create_signed_event(
    id: CorrelationId,
    timestamp: Timestamp,
    event_type: EventType,
    payload: Vec<u8>,
    source_did: Did,
    secret_key: &crate::types::SecretKey,
) -> Event {
    // Build a temporary event with a dummy signature to compute signable bytes
    let mut event = Event {
        id,
        timestamp,
        event_type,
        payload,
        source_did,
        signature: Signature::from_bytes([0u8; 64]),
    };
    let bytes = event.signable_bytes();
    event.signature = crypto::sign(&bytes, secret_key);
    event
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::types::{CorrelationId, Did, Timestamp};

    fn make_event(kp: &KeyPair) -> Event {
        let did = Did::new("did:exo:test-source").expect("valid");
        create_signed_event(
            CorrelationId::new(),
            Timestamp::new(1000, 0),
            EventType::AuditEntry,
            b"test payload".to_vec(),
            did,
            kp.secret_key(),
        )
    }

    #[test]
    fn create_and_verify_event() {
        let kp = KeyPair::generate();
        let event = make_event(&kp);
        assert!(verify_event(&event, kp.public_key()));
    }

    #[test]
    fn verify_fails_wrong_key() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let event = make_event(&kp1);
        assert!(!verify_event(&event, kp2.public_key()));
    }

    #[test]
    fn verify_fails_tampered_payload() {
        let kp = KeyPair::generate();
        let mut event = make_event(&kp);
        event.payload = b"tampered".to_vec();
        assert!(!verify_event(&event, kp.public_key()));
    }

    #[test]
    fn verify_fails_tampered_timestamp() {
        let kp = KeyPair::generate();
        let mut event = make_event(&kp);
        event.timestamp = Timestamp::new(9999, 99);
        assert!(!verify_event(&event, kp.public_key()));
    }

    #[test]
    fn verify_fails_tampered_event_type() {
        let kp = KeyPair::generate();
        let mut event = make_event(&kp);
        event.event_type = EventType::SybilAlert;
        assert!(!verify_event(&event, kp.public_key()));
    }

    #[test]
    fn event_type_serde_roundtrip() {
        let types = vec![
            EventType::TransactionStateChanged,
            EventType::IdentityResolved,
            EventType::ConsentGranted,
            EventType::ConsentRevoked,
            EventType::InvariantChecked,
            EventType::InvariantViolated,
            EventType::GovernanceDecision,
            EventType::EscalationTriggered,
            EventType::SybilAlert,
            EventType::KeyRotated,
            EventType::EntityRegistered,
            EventType::AuditEntry,
            EventType::Custom("my-event".into()),
        ];
        for t in &types {
            let json = serde_json::to_string(t).expect("ser");
            let t2: EventType = serde_json::from_str(&json).expect("de");
            assert_eq!(t, &t2);
        }
    }

    #[test]
    fn event_serde_roundtrip() {
        let kp = KeyPair::generate();
        let event = make_event(&kp);
        let json = serde_json::to_string(&event).expect("ser");
        let event2: Event = serde_json::from_str(&json).expect("de");
        assert_eq!(event, event2);
        // Signature should still verify after deserialization
        assert!(verify_event(&event2, kp.public_key()));
    }

    #[test]
    fn signable_bytes_deterministic() {
        let kp = KeyPair::generate();
        let event = make_event(&kp);
        let b1 = event.signable_bytes();
        let b2 = event.signable_bytes();
        assert_eq!(b1, b2);
    }

    #[test]
    fn event_type_ord() {
        let a = EventType::AuditEntry;
        let b = EventType::SybilAlert;
        // Just verify Ord doesn't panic
        let _ = a.cmp(&b);
    }

    #[test]
    fn event_type_hash() {
        use std::hash::{Hash, Hasher};
        let t = EventType::KeyRotated;
        let mut h = std::hash::DefaultHasher::new();
        t.hash(&mut h);
        let _ = h.finish();
    }

    #[test]
    fn event_with_empty_payload() {
        let kp = KeyPair::generate();
        let did = Did::new("did:exo:empty-payload").expect("valid");
        let event = create_signed_event(
            CorrelationId::new(),
            Timestamp::new(500, 1),
            EventType::Custom("empty".into()),
            Vec::new(),
            did,
            kp.secret_key(),
        );
        assert!(verify_event(&event, kp.public_key()));
    }

    #[test]
    fn event_with_large_payload() {
        let kp = KeyPair::generate();
        let did = Did::new("did:exo:large-payload").expect("valid");
        let payload = vec![0xab_u8; 10_000];
        let event = create_signed_event(
            CorrelationId::new(),
            Timestamp::new(500, 1),
            EventType::AuditEntry,
            payload,
            did,
            kp.secret_key(),
        );
        assert!(verify_event(&event, kp.public_key()));
    }

    #[test]
    fn event_debug_format() {
        let kp = KeyPair::generate();
        let event = make_event(&kp);
        let dbg = format!("{event:?}");
        assert!(dbg.contains("Event"));
    }
}
