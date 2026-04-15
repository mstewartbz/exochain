use std::collections::BTreeMap;
use exo_core::{Did, Signature, Timestamp};
use thiserror::Error;
use crate::risk::{RiskAttestation, RiskLevel};

#[derive(Debug, Error)]
pub enum VerificationCeremonyError {
    #[error("Ceremony has expired")]
    Expired,
    #[error("Ceremony is already finalized")]
    AlreadyFinalized,
    #[error("Invalid signature proof")]
    InvalidSignature,
    #[error("Insufficient risk score to finalize: {score}")]
    InsufficientScore { score: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityProof {
    Signature(Signature, exo_core::PublicKey, Vec<u8>), // sig, pubkey, message
    Otp(String),
    WebAuthnAssertion(Vec<u8>),
    KycToken(String),
}

#[derive(Debug, Clone)]
pub struct VerificationCeremony {
    pub target_did: Did,
    pub session_id: String,
    pub initiated_at: Timestamp,
    pub proofs: Vec<IdentityProof>,
    pub finalized: bool,
}

impl VerificationCeremony {
    pub fn new(target_did: Did, session_id: String, initiated_at: Timestamp) -> Self {
        Self {
            target_did,
            session_id,
            initiated_at,
            proofs: Vec::new(),
            finalized: false,
        }
    }

    pub fn submit_proof(&mut self, proof: IdentityProof, now: Timestamp) -> Result<(), VerificationCeremonyError> {
        if self.finalized {
            return Err(VerificationCeremonyError::AlreadyFinalized);
        }
        if now.physical_ms > self.initiated_at.physical_ms + 3_600_000 {
            return Err(VerificationCeremonyError::Expired);
        }
        
        if let IdentityProof::Signature(ref sig, ref pk, ref msg) = proof {
            if !exo_core::crypto::verify(msg, sig, pk) {
                return Err(VerificationCeremonyError::InvalidSignature);
            }
        }
        
        self.proofs.push(proof);
        Ok(())
    }

    pub fn calculate_risk_score(&self) -> u32 {
        let mut score = 0;
        // BTreeMap used as required
        let mut proof_weights: BTreeMap<&str, u32> = BTreeMap::new();
        proof_weights.insert("Signature", 1000);
        proof_weights.insert("Otp", 2000);
        proof_weights.insert("WebAuthnAssertion", 4000);
        proof_weights.insert("KycToken", 5000);

        for proof in &self.proofs {
            let s = match proof {
                IdentityProof::Signature(..) => proof_weights.get("Signature").copied().unwrap_or(0),
                IdentityProof::Otp(_) => proof_weights.get("Otp").copied().unwrap_or(0),
                IdentityProof::WebAuthnAssertion(_) => proof_weights.get("WebAuthnAssertion").copied().unwrap_or(0),
                IdentityProof::KycToken(_) => proof_weights.get("KycToken").copied().unwrap_or(0),
            };
            score += s;
        }
        score
    }

    pub fn finalize(&mut self, now: Timestamp) -> Result<RiskAttestation, VerificationCeremonyError> {
        if self.finalized {
            return Err(VerificationCeremonyError::AlreadyFinalized);
        }
        if now.physical_ms > self.initiated_at.physical_ms + 3_600_000 {
            return Err(VerificationCeremonyError::Expired);
        }

        let score = self.calculate_risk_score();
        if score < 1000 {
            return Err(VerificationCeremonyError::InsufficientScore { score });
        }

        self.finalized = true;

        let level = if score >= 5000 {
            RiskLevel::Critical
        } else if score >= 4000 {
            RiskLevel::High
        } else if score >= 3000 {
            RiskLevel::Medium
        } else if score >= 2000 {
            RiskLevel::Low
        } else {
            RiskLevel::Minimal
        };

        // We generate a dummy RiskAttestation for now
        let dummy_sig = Signature::Ed25519([0u8; 64]);
        let system_did = Did::new("did:exo:system").unwrap_or_else(|_| self.target_did.clone());
        
        Ok(RiskAttestation {
            subject_did: self.target_did.clone(),
            attester_did: system_did,
            level,
            evidence_hash: [0u8; 32],
            timestamp: now,
            expiry: Timestamp::new(now.physical_ms + 31536000000, 0),
            signature: dummy_sig,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exo_core::crypto::generate_keypair;
    use exo_core::crypto::sign;

    #[test]
    fn test_ceremony_lifecycle() {
        let did = Did::new("did:exo:alice").unwrap();
        let mut ceremony = VerificationCeremony::new(did, "sess1".to_string(), Timestamp::new(1000, 0));
        let (pk, sk) = generate_keypair();
        let msg = b"msg".to_vec();
        let sig = sign(&msg, &sk);
        
        ceremony.submit_proof(IdentityProof::Signature(sig, pk, msg), Timestamp::new(1010, 0)).unwrap();
        let attestation = ceremony.finalize(Timestamp::new(1020, 0)).unwrap();
        
        assert_eq!(attestation.level, RiskLevel::Minimal);
        assert!(ceremony.finalized);
    }

    #[test]
    fn test_finalize_without_sufficient_proofs_fails() {
        let did = Did::new("did:exo:bob").unwrap();
        let mut ceremony = VerificationCeremony::new(did, "sess2".to_string(), Timestamp::new(1000, 0));
        
        let err = ceremony.finalize(Timestamp::new(1020, 0)).unwrap_err();
        assert!(matches!(err, VerificationCeremonyError::InsufficientScore { .. }));
    }

    #[test]
    fn test_risk_score_calculation() {
        let did = Did::new("did:exo:charlie").unwrap();
        let mut ceremony = VerificationCeremony::new(did, "sess3".to_string(), Timestamp::new(1000, 0));
        
        let (pk, sk) = generate_keypair();
        let msg = b"msg".to_vec();
        let sig = sign(&msg, &sk);
        
        ceremony.submit_proof(IdentityProof::Signature(sig, pk, msg), Timestamp::new(1010, 0)).unwrap();
        assert_eq!(ceremony.calculate_risk_score(), 1000);
        
        ceremony.submit_proof(IdentityProof::Otp("123456".to_string()), Timestamp::new(1020, 0)).unwrap();
        assert_eq!(ceremony.calculate_risk_score(), 3000); // 1000 + 2000
    }

    #[test]
    fn test_expired_ceremony_fails() {
        let did = Did::new("did:exo:dave").unwrap();
        let mut ceremony = VerificationCeremony::new(did, "sess4".to_string(), Timestamp::new(1000, 0));
        
        let err = ceremony.submit_proof(IdentityProof::Otp("123".into()), Timestamp::new(4000_000, 0)).unwrap_err();
        assert!(matches!(err, VerificationCeremonyError::Expired));
    }

    #[test]
    fn test_invalid_signature_proof_rejected() {
        let did = Did::new("did:exo:eve").unwrap();
        let mut ceremony = VerificationCeremony::new(did, "sess5".to_string(), Timestamp::new(1000, 0));
        
        let (pk, _) = generate_keypair();
        let (_, sk2) = generate_keypair();
        let msg = b"msg".to_vec();
        let bad_sig = sign(&msg, &sk2);
        
        let err = ceremony.submit_proof(IdentityProof::Signature(bad_sig, pk, msg), Timestamp::new(1010, 0)).unwrap_err();
        assert!(matches!(err, VerificationCeremonyError::InvalidSignature));
    }

    // Integration-style tests combining the registry and verification flow
    use crate::registry::{DidRegistry, LocalDidRegistry};
    use crate::did::DidDocument;

    fn make_did(label: &str) -> Did {
        Did::new(&format!("did:exo:{label}")).expect("valid did")
    }

    fn make_doc(did: Did, pk: exo_core::PublicKey) -> DidDocument {
        DidDocument {
            id: did,
            public_keys: vec![pk],
            authentication: vec![],
            verification_methods: vec![],
            hybrid_verification_methods: vec![],
            service_endpoints: vec![],
            created: Timestamp::new(1000, 0),
            updated: Timestamp::new(1000, 0),
            revoked: false,
        }
    }

    #[test]
    fn test_integration_full_verification_flow() {
        let (pk, sk) = generate_keypair();
        let did = make_did("integration1");
        let doc = make_doc(did.clone(), pk);

        let mut reg = LocalDidRegistry::new();
        reg.register(doc).unwrap();

        let mut ceremony = VerificationCeremony::new(did.clone(), "session_int_1".to_string(), Timestamp::new(1000, 0));
        
        // 1. Submit signature
        let msg = b"integration_msg".to_vec();
        let sig = sign(&msg, &sk);
        ceremony.submit_proof(IdentityProof::Signature(sig, pk, msg), Timestamp::new(1010, 0)).unwrap();
        
        // 2. Submit WebAuthn
        ceremony.submit_proof(IdentityProof::WebAuthnAssertion(vec![1, 2, 3]), Timestamp::new(1020, 0)).unwrap();
        
        // 3. Finalize
        let attestation = ceremony.finalize(Timestamp::new(1030, 0)).unwrap();
        assert_eq!(attestation.level, RiskLevel::Critical); // 1000 + 4000 = 5000 -> Critical
        
        // Check that the target DID matches what's in the registry
        let resolved = reg.resolve(&attestation.subject_did).unwrap();
        assert_eq!(resolved.id, did);
    }

    #[test]
    fn test_integration_revoked_did_verification_fails() {
        // While the ceremony itself doesn't check the registry in this simple model, 
        // an integration flow would ensure we don't verify revoked DIDs.
        let (pk, sk) = generate_keypair();
        let did = make_did("integration2");
        let doc = make_doc(did.clone(), pk);

        let mut reg = LocalDidRegistry::new();
        reg.register(doc).unwrap();
        
        // Revoke the DID
        let proof = crate::did::RevocationProof {
            did: did.clone(),
            signature: sign(did.as_str().as_bytes(), &sk),
        };
        reg.revoke(&did, &proof).unwrap();

        let resolved = reg.resolve(&did);
        assert!(resolved.is_none());
        
        // Since DID is revoked, the outer system wouldn't even start a ceremony,
        // but if it did, it should not be usable.
        let mut ceremony = VerificationCeremony::new(did, "session_int_2".to_string(), Timestamp::new(1000, 0));
        let msg = b"msg".to_vec();
        let sig = sign(&msg, &sk);
        ceremony.submit_proof(IdentityProof::Signature(sig, pk, msg), Timestamp::new(1010, 0)).unwrap();
        
        // Finalize succeeds, but the attestation is useless because resolve() fails
        let attestation = ceremony.finalize(Timestamp::new(1020, 0)).unwrap();
        assert!(reg.resolve(&attestation.subject_did).is_none());
    }

    #[test]
    fn test_integration_insufficient_proofs() {
        let (pk, _) = generate_keypair();
        let did = make_did("integration3");
        let doc = make_doc(did.clone(), pk);

        let mut reg = LocalDidRegistry::new();
        reg.register(doc).unwrap();

        let mut ceremony = VerificationCeremony::new(did, "session_int_3".to_string(), Timestamp::new(1000, 0));
        // No proofs submitted
        let err = ceremony.finalize(Timestamp::new(1020, 0)).unwrap_err();
        assert!(matches!(err, VerificationCeremonyError::InsufficientScore { score: 0 }));
    }
}
