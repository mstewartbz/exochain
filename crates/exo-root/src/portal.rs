//! Server-side root genesis portal relay policy.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::{Did, Hash256, SecretKey, Signature, crypto, hash::hash_structured};
use frost_ristretto255 as frost;
use serde::{Deserialize, Serialize};

use crate::{
    GenesisCeremonyConfig, PairwiseEncryptedPayload, Result, RootError,
    dkg::{
        RootParticipantDkgOutput, RootPublicKeyPackage, deserialize_frost, frost_identifier,
        validate_public_key_package,
    },
};

const MAX_PORTAL_PAYLOAD_BYTES: usize = 64 * 1024;
pub const FINAL_KEY_CONFIRMATION_DOMAIN: &str = "EXOCHAIN_ROOT_FINAL_KEY_CONFIRMATION_V1";
pub const FINAL_KEY_CONFIRMATION_SCHEMA_VERSION: u16 = 1;

/// Ceremony phase associated with a portal envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CeremonyPhase {
    /// DKG round one broadcast.
    Round1,
    /// Roster-wide round one set attestation.
    Round1SetAttestation,
    /// DKG round two pairwise exchange.
    Round2,
    /// Final DKG confirmation.
    Finalize,
    /// Root artifact signing.
    RootSigning,
}

/// Bounded payload type carried by a portal envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CeremonyPayloadKind {
    /// DKG round one public package.
    Round1Package,
    /// Signed statement binding the full round one set.
    Round1SetAttestation,
    /// Recipient-bound encrypted DKG round two package.
    Round2EncryptedPackage,
    /// Rejected DKG round two raw package.
    Round2PlaintextPackage,
    /// Final key confirmation package.
    FinalKeyConfirmation,
    /// Root signing nonce commitment.
    RootSigningCommitment,
    /// Root signing share.
    RootSignatureShare,
}

/// Signed, bounded, untrusted relay envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CeremonyEnvelope {
    /// Ceremony identifier.
    pub ceremony_id: String,
    /// Ceremony phase.
    pub phase: CeremonyPhase,
    /// Payload type.
    pub payload_kind: CeremonyPayloadKind,
    /// Rostered sender DID.
    pub sender_did: Did,
    /// Optional rostered recipient DID.
    pub recipient_did: Option<Did>,
    /// Monotonic sender sequence.
    pub sequence: u64,
    /// Bounded opaque payload.
    pub payload_bytes: Vec<u8>,
    /// Canonical payload hash.
    pub payload_hash: Hash256,
    /// Ed25519 signature by the sender.
    pub signature: Signature,
}

/// Inputs that are signed into a portal relay envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CeremonyEnvelopeDraft {
    /// Ceremony identifier.
    pub ceremony_id: String,
    /// Ceremony phase.
    pub phase: CeremonyPhase,
    /// Payload type.
    pub payload_kind: CeremonyPayloadKind,
    /// Rostered sender DID.
    pub sender_did: Did,
    /// Optional rostered recipient DID.
    pub recipient_did: Option<Did>,
    /// Monotonic sender sequence.
    pub sequence: u64,
    /// Bounded opaque payload.
    pub payload_bytes: Vec<u8>,
}

/// Ratified final DKG key confirmation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalKeyConfirmation {
    /// Domain separator; must equal [`FINAL_KEY_CONFIRMATION_DOMAIN`].
    pub domain: String,
    /// Schema version; must equal [`FINAL_KEY_CONFIRMATION_SCHEMA_VERSION`].
    pub schema_version: u16,
    /// Ceremony identifier.
    pub ceremony_id: String,
    /// Confirming certifier DID.
    pub certifier_did: Did,
    /// Confirming certifier FROST identifier.
    pub frost_identifier: u16,
    /// Canonical hash of the ceremony config.
    pub config_hash: Hash256,
    /// Canonical hash of the completed DKG relay transcript.
    pub dkg_transcript_hash: Hash256,
    /// Public key package independently derived by the certifier.
    pub public_key_package: RootPublicKeyPackage,
    /// Canonical hash of the full public key package.
    pub root_public_key_package_hash: Hash256,
    /// Hash of `public_key_package.root_public_key`.
    pub root_public_key_hash: Hash256,
    /// Hash of this certifier's verifying share in the public key package.
    pub certifier_verifying_share_hash: Hash256,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PortalEnvelopeKey {
    sender_did: Did,
    phase: CeremonyPhase,
    payload_kind: CeremonyPayloadKind,
    sequence: u64,
    recipient_did: Option<Did>,
}

/// In-memory portal store used by the server relay and tests.
#[derive(Debug, Clone)]
pub struct PortalStore {
    config: GenesisCeremonyConfig,
    envelopes: BTreeMap<PortalEnvelopeKey, CeremonyEnvelope>,
    seen_sequences: BTreeSet<(Did, u64)>,
    final_key_confirmations: BTreeMap<Did, FinalKeyConfirmation>,
    /// Signers who have already submitted a root signature share this session.
    /// Enforces one share per signer (single-use of the signer's nonces).
    signature_share_senders: BTreeSet<Did>,
}

#[derive(Serialize)]
struct PayloadHashEnvelope<'a> {
    domain: &'static str,
    payload_kind: CeremonyPayloadKind,
    payload_bytes: &'a [u8],
}

#[derive(Serialize)]
struct EnvelopeSigningPayload<'a> {
    domain: &'static str,
    ceremony_id: &'a str,
    phase: CeremonyPhase,
    payload_kind: CeremonyPayloadKind,
    sender_did: &'a Did,
    recipient_did: &'a Option<Did>,
    sequence: u64,
    payload_hash: Hash256,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct TranscriptEnvelopeRecord {
    phase: CeremonyPhase,
    payload_kind: CeremonyPayloadKind,
    sender_did: Did,
    recipient_did: Option<Did>,
    sequence: u64,
    envelope_id: Hash256,
    envelope_hash: Hash256,
}

#[derive(Serialize)]
struct DkgTranscriptPayload<'a> {
    domain: &'static str,
    config_hash: Hash256,
    envelopes: &'a [TranscriptEnvelopeRecord],
}

#[derive(Serialize)]
struct FinalTranscriptPayload<'a> {
    domain: &'static str,
    config_hash: Hash256,
    dkg_transcript_hash: Hash256,
    final_key_confirmations: &'a [TranscriptEnvelopeRecord],
}

fn payload_hash(kind: CeremonyPayloadKind, payload_bytes: &[u8]) -> Result<Hash256> {
    hash_structured(&PayloadHashEnvelope {
        domain: "EXOCHAIN_ROOT_PORTAL_PAYLOAD_V1",
        payload_kind: kind,
        payload_bytes,
    })
    .map_err(canonical_encoding_error)
}

/// Canonical hash of a root genesis ceremony config.
pub fn ceremony_config_hash(config: &GenesisCeremonyConfig) -> Result<Hash256> {
    hash_structured(config).map_err(canonical_encoding_error)
}

/// Encode a ratified final key confirmation as portal payload bytes.
pub fn encode_final_key_confirmation_payload(
    confirmation: &FinalKeyConfirmation,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    ciborium::into_writer(confirmation, &mut bytes).map_err(canonical_encoding_error)?;
    Ok(bytes)
}

fn decode_final_key_confirmation_payload(bytes: &[u8]) -> Result<FinalKeyConfirmation> {
    ciborium::from_reader(bytes).map_err(|error| RootError::PortalRejected {
        reason: format!("final key confirmation payload failed schema validation: {error}"),
    })
}

/// Build the ratified final key confirmation payload for one finalized
/// certifier. This emits only public confirmation material; the secret FROST key
/// package is parsed locally to bind the certifier identifier but is never copied
/// into the payload.
pub fn build_final_key_confirmation(
    config: &GenesisCeremonyConfig,
    dkg_output: &RootParticipantDkgOutput,
    dkg_transcript_hash: Hash256,
) -> Result<FinalKeyConfirmation> {
    config.validate()?;
    validate_public_key_package(config, &dkg_output.public_key_package)?;
    let frost_identifier_value = dkg_output.key_package.frost_identifier;
    let certifier = config
        .certifier_by_identifier(frost_identifier_value)
        .ok_or_else(|| RootError::InvalidConfig {
            reason: format!(
                "final key confirmation certifier {frost_identifier_value} is not rostered"
            ),
        })?;
    let parsed_key_package: frost::keys::KeyPackage =
        deserialize_frost(dkg_output.key_package.key_package.as_slice())?;
    if *parsed_key_package.identifier() != frost_identifier(frost_identifier_value)? {
        return Err(RootError::Frost {
            detail: "final key confirmation key package identifier mismatch".to_owned(),
        });
    }
    let verifying_share = dkg_output
        .public_key_package
        .verifying_shares
        .get(&frost_identifier_value)
        .ok_or_else(|| RootError::BundleRejected {
            reason: format!(
                "public key package missing verifying share for certifier {frost_identifier_value}"
            ),
        })?;
    Ok(FinalKeyConfirmation {
        domain: FINAL_KEY_CONFIRMATION_DOMAIN.to_owned(),
        schema_version: FINAL_KEY_CONFIRMATION_SCHEMA_VERSION,
        ceremony_id: config.ceremony_id.clone(),
        certifier_did: certifier.did.clone(),
        frost_identifier: frost_identifier_value,
        config_hash: ceremony_config_hash(config)?,
        dkg_transcript_hash,
        public_key_package: dkg_output.public_key_package.clone(),
        root_public_key_package_hash: hash_structured(&dkg_output.public_key_package)
            .map_err(canonical_encoding_error)?,
        root_public_key_hash: Hash256::digest(
            dkg_output.public_key_package.root_public_key.as_slice(),
        ),
        certifier_verifying_share_hash: Hash256::digest(verifying_share.as_slice()),
    })
}

fn signing_payload(envelope: &CeremonyEnvelope) -> Result<Vec<u8>> {
    let payload = EnvelopeSigningPayload {
        domain: "EXOCHAIN_ROOT_PORTAL_ENVELOPE_V1",
        ceremony_id: &envelope.ceremony_id,
        phase: envelope.phase,
        payload_kind: envelope.payload_kind,
        sender_did: &envelope.sender_did,
        recipient_did: &envelope.recipient_did,
        sequence: envelope.sequence,
        payload_hash: envelope.payload_hash,
    };
    let mut bytes = Vec::new();
    ciborium::into_writer(&payload, &mut bytes).map_err(canonical_encoding_error)?;
    Ok(bytes)
}

fn canonical_encoding_error(error: impl core::fmt::Display) -> RootError {
    RootError::CanonicalEncoding {
        detail: error.to_string(),
    }
}

impl CeremonyEnvelope {
    /// Create and sign a portal relay envelope.
    pub fn sign(draft: CeremonyEnvelopeDraft, signing_secret: &SecretKey) -> Result<Self> {
        let mut envelope = Self {
            ceremony_id: draft.ceremony_id,
            phase: draft.phase,
            payload_kind: draft.payload_kind,
            sender_did: draft.sender_did,
            recipient_did: draft.recipient_did,
            sequence: draft.sequence,
            payload_hash: payload_hash(draft.payload_kind, draft.payload_bytes.as_slice())?,
            payload_bytes: draft.payload_bytes,
            signature: Signature::Empty,
        };
        let payload = signing_payload(&envelope)?;
        envelope.signature = crypto::sign(payload.as_slice(), signing_secret);
        Ok(envelope)
    }
}

impl PortalStore {
    /// Construct an empty portal store for a ceremony.
    #[must_use]
    pub fn new(config: GenesisCeremonyConfig) -> Self {
        Self {
            config,
            envelopes: BTreeMap::new(),
            seen_sequences: BTreeSet::new(),
            final_key_confirmations: BTreeMap::new(),
            signature_share_senders: BTreeSet::new(),
        }
    }

    /// Number of accepted relay envelopes.
    #[must_use]
    pub fn envelope_count(&self) -> usize {
        self.envelopes.len()
    }

    /// Return accepted envelopes matching all of the supplied filters; a `None`
    /// filter matches any value. Envelopes are relay data — already signed and
    /// (for round two) encrypted — so returning them to rostered participants is
    /// the read half of the relay, used to collect round-one packages, pull
    /// recipient-bound round-two packages, and gather signing commitments/shares.
    #[must_use]
    pub fn query(
        &self,
        phase: Option<CeremonyPhase>,
        payload_kind: Option<CeremonyPayloadKind>,
        recipient_did: Option<&Did>,
    ) -> Vec<CeremonyEnvelope> {
        self.envelopes
            .values()
            .filter(|envelope| phase.is_none_or(|value| envelope.phase == value))
            .filter(|envelope| payload_kind.is_none_or(|value| envelope.payload_kind == value))
            .filter(|envelope| {
                recipient_did.is_none_or(|value| envelope.recipient_did.as_ref() == Some(value))
            })
            .cloned()
            .collect()
    }

    /// Submit a signed envelope to the relay.
    pub fn submit(&mut self, envelope: CeremonyEnvelope) -> Result<Hash256> {
        self.validate_envelope(&envelope)?;
        let final_key_confirmation =
            if envelope.payload_kind == CeremonyPayloadKind::FinalKeyConfirmation {
                Some(self.validate_final_key_confirmation(&envelope)?)
            } else {
                None
            };
        let sequence_key = (envelope.sender_did.clone(), envelope.sequence);
        if self.seen_sequences.contains(&sequence_key) {
            return Err(RootError::PortalRejected {
                reason: "sender sequence replay".to_owned(),
            });
        }
        // One root signature share per signer per session: a second share from the
        // same signer is rejected, enforcing single-use of that signer's nonces.
        if envelope.payload_kind == CeremonyPayloadKind::RootSignatureShare
            && self.signature_share_senders.contains(&envelope.sender_did)
        {
            return Err(RootError::PortalRejected {
                reason: "signer has already submitted a signature share this session".to_owned(),
            });
        }
        let key = PortalEnvelopeKey {
            sender_did: envelope.sender_did.clone(),
            phase: envelope.phase,
            payload_kind: envelope.payload_kind,
            sequence: envelope.sequence,
            recipient_did: envelope.recipient_did.clone(),
        };
        let envelope_id = hash_structured(&key_parts(&key)).map_err(canonical_encoding_error)?;
        self.seen_sequences.insert(sequence_key);
        if envelope.payload_kind == CeremonyPayloadKind::RootSignatureShare {
            self.signature_share_senders
                .insert(envelope.sender_did.clone());
        }
        if let Some(confirmation) = final_key_confirmation {
            self.final_key_confirmations
                .insert(envelope.sender_did.clone(), confirmation);
        }
        self.envelopes.insert(key, envelope);
        Ok(envelope_id)
    }

    /// Canonical hash over the complete accepted DKG relay transcript.
    pub fn dkg_transcript_hash(&self) -> Result<Hash256> {
        let records = self.dkg_transcript_records()?;
        self.ensure_dkg_transcript_complete(records.as_slice())?;
        let payload = DkgTranscriptPayload {
            domain: "EXOCHAIN_ROOT_DKG_TRANSCRIPT_V1",
            config_hash: ceremony_config_hash(&self.config)?,
            envelopes: records.as_slice(),
        };
        hash_structured(&payload).map_err(canonical_encoding_error)
    }

    /// Canonical final ceremony transcript hash, including all accepted final
    /// key confirmation envelopes. This is the transcript hash root artifacts
    /// must bind before root signing begins.
    pub fn final_transcript_hash(&self) -> Result<Hash256> {
        self.ensure_final_key_confirmations_complete()?;
        let records = self.final_key_confirmation_records()?;
        let payload = FinalTranscriptPayload {
            domain: "EXOCHAIN_ROOT_FINAL_TRANSCRIPT_V1",
            config_hash: ceremony_config_hash(&self.config)?,
            dkg_transcript_hash: self.dkg_transcript_hash()?,
            final_key_confirmations: records.as_slice(),
        };
        hash_structured(&payload).map_err(canonical_encoding_error)
    }

    fn validate_envelope(&self, envelope: &CeremonyEnvelope) -> Result<()> {
        self.config.validate()?;
        if envelope.ceremony_id != self.config.ceremony_id {
            return Err(RootError::PortalRejected {
                reason: "ceremony_id mismatch".to_owned(),
            });
        }
        if envelope.payload_bytes.len() > MAX_PORTAL_PAYLOAD_BYTES {
            return Err(RootError::PortalRejected {
                reason: "payload exceeds portal limit".to_owned(),
            });
        }
        if envelope.payload_hash
            != payload_hash(envelope.payload_kind, envelope.payload_bytes.as_slice())?
        {
            return Err(RootError::PortalRejected {
                reason: "payload hash mismatch".to_owned(),
            });
        }
        self.validate_phase_policy(envelope)?;

        let sender = self
            .config
            .certifier_by_did(&envelope.sender_did)
            .ok_or_else(|| RootError::PortalRejected {
                reason: "sender is not rostered".to_owned(),
            })?;
        if let Some(recipient) = &envelope.recipient_did {
            if self.config.certifier_by_did(recipient).is_none() {
                return Err(RootError::PortalRejected {
                    reason: "recipient is not rostered".to_owned(),
                });
            }
            if recipient == &envelope.sender_did {
                return Err(RootError::PortalRejected {
                    reason: "sender cannot target itself".to_owned(),
                });
            }
        }

        let payload = signing_payload(envelope)?;
        if !crypto::verify(
            payload.as_slice(),
            &envelope.signature,
            &sender.signing_public_key,
        ) {
            return Err(RootError::SignatureRejected {
                reason: "certifier envelope signature rejected".to_owned(),
            });
        }
        Ok(())
    }

    fn validate_phase_policy(&self, envelope: &CeremonyEnvelope) -> Result<()> {
        // Every accepted payload kind is schema-validated by decoding it to its
        // concrete type before storage — the portal never stores opaque bytes for
        // a security-sensitive kind. Kinds without a ratified, decodable schema
        // (round-one set attestation) are disabled.
        let bytes = envelope.payload_bytes.as_slice();
        match (envelope.phase, envelope.payload_kind) {
            (CeremonyPhase::Round1, CeremonyPayloadKind::Round1Package) => {
                reject_recipient(envelope)?;
                self.reject_dkg_mutation_after_final_confirmation()?;
                self.reject_duplicate_broadcast_sender(
                    envelope,
                    CeremonyPhase::Round1,
                    CeremonyPayloadKind::Round1Package,
                    "round-one package already submitted by sender",
                )?;
                reject_unless_decodable::<frost::keys::dkg::round1::Package>(
                    bytes,
                    "round-one package",
                )
            }
            (CeremonyPhase::RootSigning, CeremonyPayloadKind::RootSigningCommitment) => {
                reject_recipient(envelope)?;
                self.ensure_final_key_confirmations_complete()?;
                self.reject_duplicate_broadcast_sender(
                    envelope,
                    CeremonyPhase::RootSigning,
                    CeremonyPayloadKind::RootSigningCommitment,
                    "root signing commitment already submitted by sender",
                )?;
                reject_unless_decodable::<frost::round1::SigningCommitments>(
                    bytes,
                    "root signing commitment",
                )
            }
            (CeremonyPhase::RootSigning, CeremonyPayloadKind::RootSignatureShare) => {
                reject_recipient(envelope)?;
                self.ensure_final_key_confirmations_complete()?;
                reject_unless_decodable::<frost::round2::SignatureShare>(
                    bytes,
                    "root signature share",
                )
            }
            (CeremonyPhase::Round2, CeremonyPayloadKind::Round2EncryptedPackage) => {
                if envelope.recipient_did.is_none() {
                    return Err(RootError::PortalRejected {
                        reason: "round-two encrypted package requires recipient".to_owned(),
                    });
                }
                self.reject_dkg_mutation_after_final_confirmation()?;
                self.reject_duplicate_pairwise_sender_recipient(
                    envelope,
                    CeremonyPhase::Round2,
                    CeremonyPayloadKind::Round2EncryptedPackage,
                    "round-two encrypted package already submitted for sender and recipient",
                )?;
                validate_encrypted_round2_payload(bytes)
            }
            (CeremonyPhase::Round1SetAttestation, CeremonyPayloadKind::Round1SetAttestation) => {
                Err(RootError::PortalRejected {
                    reason: "round-one set attestation is disabled pending a ratified, \
                             portal-validated payload schema"
                        .to_owned(),
                })
            }
            (CeremonyPhase::Finalize, CeremonyPayloadKind::FinalKeyConfirmation) => {
                reject_recipient(envelope)?;
                self.validate_final_key_confirmation(envelope).map(|_| ())
            }
            (_, CeremonyPayloadKind::Round2PlaintextPackage) => Err(RootError::PortalRejected {
                reason: "round-two raw package is rejected".to_owned(),
            }),
            _ => Err(RootError::PortalRejected {
                reason: "payload kind is not valid for phase".to_owned(),
            }),
        }
    }

    fn reject_dkg_mutation_after_final_confirmation(&self) -> Result<()> {
        if !self.final_key_confirmations.is_empty() {
            return Err(RootError::PortalRejected {
                reason: "dkg transcript is frozen after final key confirmation".to_owned(),
            });
        }
        Ok(())
    }

    fn reject_duplicate_broadcast_sender(
        &self,
        envelope: &CeremonyEnvelope,
        phase: CeremonyPhase,
        payload_kind: CeremonyPayloadKind,
        reason: &str,
    ) -> Result<()> {
        if self.envelopes.values().any(|accepted| {
            accepted.sender_did == envelope.sender_did
                && accepted.phase == phase
                && accepted.payload_kind == payload_kind
                && accepted.recipient_did.is_none()
                && accepted.sequence != envelope.sequence
        }) {
            return Err(RootError::PortalRejected {
                reason: reason.to_owned(),
            });
        }
        Ok(())
    }

    fn reject_duplicate_pairwise_sender_recipient(
        &self,
        envelope: &CeremonyEnvelope,
        phase: CeremonyPhase,
        payload_kind: CeremonyPayloadKind,
        reason: &str,
    ) -> Result<()> {
        if self.envelopes.values().any(|accepted| {
            accepted.sender_did == envelope.sender_did
                && accepted.recipient_did == envelope.recipient_did
                && accepted.phase == phase
                && accepted.payload_kind == payload_kind
                && accepted.sequence != envelope.sequence
        }) {
            return Err(RootError::PortalRejected {
                reason: reason.to_owned(),
            });
        }
        Ok(())
    }

    fn validate_final_key_confirmation(
        &self,
        envelope: &CeremonyEnvelope,
    ) -> Result<FinalKeyConfirmation> {
        if self
            .final_key_confirmations
            .contains_key(&envelope.sender_did)
        {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation already submitted by sender".to_owned(),
            });
        }
        let confirmation =
            decode_final_key_confirmation_payload(envelope.payload_bytes.as_slice())?;
        self.validate_final_key_confirmation_semantics(envelope, &confirmation)?;
        for accepted in self.final_key_confirmations.values() {
            if accepted.config_hash != confirmation.config_hash {
                return Err(RootError::PortalRejected {
                    reason: "final key confirmation config hash disagrees with accepted set"
                        .to_owned(),
                });
            }
            if accepted.dkg_transcript_hash != confirmation.dkg_transcript_hash {
                return Err(RootError::PortalRejected {
                    reason:
                        "final key confirmation DKG transcript hash disagrees with accepted set"
                            .to_owned(),
                });
            }
            if accepted.public_key_package != confirmation.public_key_package {
                return Err(RootError::PortalRejected {
                    reason: "final key confirmation public key package disagrees with accepted set"
                        .to_owned(),
                });
            }
            if accepted.root_public_key_package_hash != confirmation.root_public_key_package_hash {
                return Err(RootError::PortalRejected {
                    reason:
                        "final key confirmation public key package hash disagrees with accepted set"
                            .to_owned(),
                });
            }
            if accepted.root_public_key_hash != confirmation.root_public_key_hash {
                return Err(RootError::PortalRejected {
                    reason: "final key confirmation root key hash disagrees with accepted set"
                        .to_owned(),
                });
            }
        }
        Ok(confirmation)
    }

    fn validate_final_key_confirmation_semantics(
        &self,
        envelope: &CeremonyEnvelope,
        confirmation: &FinalKeyConfirmation,
    ) -> Result<()> {
        if confirmation.domain != FINAL_KEY_CONFIRMATION_DOMAIN {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation domain mismatch".to_owned(),
            });
        }
        if confirmation.schema_version != FINAL_KEY_CONFIRMATION_SCHEMA_VERSION {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation schema version mismatch".to_owned(),
            });
        }
        if confirmation.ceremony_id != self.config.ceremony_id {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation ceremony_id mismatch".to_owned(),
            });
        }
        if confirmation.certifier_did != envelope.sender_did {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation certifier_did must match envelope sender"
                    .to_owned(),
            });
        }
        let certifier = self
            .config
            .certifier_by_did(&confirmation.certifier_did)
            .ok_or_else(|| RootError::PortalRejected {
                reason: "final key confirmation certifier is not rostered".to_owned(),
            })?;
        if certifier.frost_identifier != confirmation.frost_identifier {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation DID and FROST identifier mismatch".to_owned(),
            });
        }
        let expected_config_hash = ceremony_config_hash(&self.config)?;
        if confirmation.config_hash != expected_config_hash {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation config hash mismatch".to_owned(),
            });
        }
        let expected_dkg_transcript_hash = self.dkg_transcript_hash()?;
        if confirmation.dkg_transcript_hash != expected_dkg_transcript_hash {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation DKG transcript hash mismatch".to_owned(),
            });
        }
        validate_public_key_package(&self.config, &confirmation.public_key_package)?;
        let expected_package_hash =
            hash_structured(&confirmation.public_key_package).map_err(canonical_encoding_error)?;
        if confirmation.root_public_key_package_hash != expected_package_hash {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation public key package hash mismatch".to_owned(),
            });
        }
        let expected_root_public_key_hash =
            Hash256::digest(confirmation.public_key_package.root_public_key.as_slice());
        if confirmation.root_public_key_hash != expected_root_public_key_hash {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation root public key hash mismatch".to_owned(),
            });
        }
        let verifying_share = confirmation
            .public_key_package
            .verifying_shares
            .get(&confirmation.frost_identifier)
            .ok_or_else(|| RootError::PortalRejected {
                reason: "final key confirmation verifying share is missing".to_owned(),
            })?;
        if confirmation.certifier_verifying_share_hash
            != Hash256::digest(verifying_share.as_slice())
        {
            return Err(RootError::PortalRejected {
                reason: "final key confirmation verifying share hash mismatch".to_owned(),
            });
        }
        Ok(())
    }

    fn dkg_transcript_records(&self) -> Result<Vec<TranscriptEnvelopeRecord>> {
        let mut records = Vec::new();
        for (key, envelope) in &self.envelopes {
            if matches!(
                (envelope.phase, envelope.payload_kind),
                (CeremonyPhase::Round1, CeremonyPayloadKind::Round1Package)
                    | (
                        CeremonyPhase::Round2,
                        CeremonyPayloadKind::Round2EncryptedPackage
                    )
            ) {
                records.push(transcript_record(key, envelope)?);
            }
        }
        records.sort();
        Ok(records)
    }

    fn final_key_confirmation_records(&self) -> Result<Vec<TranscriptEnvelopeRecord>> {
        let mut records = Vec::new();
        for (key, envelope) in &self.envelopes {
            if envelope.phase == CeremonyPhase::Finalize
                && envelope.payload_kind == CeremonyPayloadKind::FinalKeyConfirmation
            {
                records.push(transcript_record(key, envelope)?);
            }
        }
        records.sort();
        Ok(records)
    }

    fn ensure_dkg_transcript_complete(&self, records: &[TranscriptEnvelopeRecord]) -> Result<()> {
        let expected_certifiers: BTreeSet<Did> = self
            .config
            .certifiers
            .iter()
            .map(|certifier| certifier.did.clone())
            .collect();
        let mut round1_senders = BTreeSet::new();
        let mut round2_pairs = BTreeSet::new();
        for record in records {
            match (record.phase, record.payload_kind) {
                (CeremonyPhase::Round1, CeremonyPayloadKind::Round1Package) => {
                    if record.recipient_did.is_some() {
                        return Err(RootError::PortalRejected {
                            reason: "dkg transcript round-one record has a recipient".to_owned(),
                        });
                    }
                    if !round1_senders.insert(record.sender_did.clone()) {
                        return Err(RootError::PortalRejected {
                            reason: "dkg transcript contains duplicate round-one sender".to_owned(),
                        });
                    }
                }
                (CeremonyPhase::Round2, CeremonyPayloadKind::Round2EncryptedPackage) => {
                    let recipient =
                        record
                            .recipient_did
                            .clone()
                            .ok_or_else(|| RootError::PortalRejected {
                                reason: "dkg transcript round-two record missing recipient"
                                    .to_owned(),
                            })?;
                    if !round2_pairs.insert((record.sender_did.clone(), recipient)) {
                        return Err(RootError::PortalRejected {
                            reason:
                                "dkg transcript contains duplicate round-two sender-recipient pair"
                                    .to_owned(),
                        });
                    }
                }
                _ => {
                    return Err(RootError::PortalRejected {
                        reason: "dkg transcript contains non-DKG envelope".to_owned(),
                    });
                }
            }
        }
        if round1_senders != expected_certifiers {
            return Err(RootError::PortalRejected {
                reason: "dkg transcript requires one round-one package from every certifier"
                    .to_owned(),
            });
        }
        let expected_round2 = usize::from(self.config.max_signers)
            * usize::from(self.config.max_signers.saturating_sub(1));
        if round2_pairs.len() != expected_round2 {
            return Err(RootError::PortalRejected {
                reason: "dkg transcript requires every ordered round-two sender-recipient package"
                    .to_owned(),
            });
        }
        for sender in &expected_certifiers {
            for recipient in &expected_certifiers {
                if sender == recipient {
                    continue;
                }
                if !round2_pairs.contains(&(sender.clone(), recipient.clone())) {
                    return Err(RootError::PortalRejected {
                        reason: "dkg transcript missing round-two sender-recipient package"
                            .to_owned(),
                    });
                }
            }
        }
        Ok(())
    }

    fn ensure_final_key_confirmations_complete(&self) -> Result<()> {
        if self.final_key_confirmations.len() != usize::from(self.config.max_signers) {
            return Err(RootError::PortalRejected {
                reason: "root signing requires final key confirmations from all certifiers"
                    .to_owned(),
            });
        }
        for certifier in &self.config.certifiers {
            if !self.final_key_confirmations.contains_key(&certifier.did) {
                return Err(RootError::PortalRejected {
                    reason: "root signing missing a certifier final key confirmation".to_owned(),
                });
            }
        }
        Ok(())
    }
}

/// Reject a broadcast payload that carries a recipient.
fn reject_recipient(envelope: &CeremonyEnvelope) -> Result<()> {
    if envelope.recipient_did.is_some() {
        return Err(RootError::PortalRejected {
            reason: "broadcast payload must not set recipient".to_owned(),
        });
    }
    Ok(())
}

/// Schema-validate a payload by decoding it to its concrete type `T`; a decode
/// failure is a portal rejection (bad request), never silent storage.
fn reject_unless_decodable<T: serde::de::DeserializeOwned>(bytes: &[u8], kind: &str) -> Result<()> {
    ciborium::from_reader::<T, _>(bytes)
        .map(|_decoded| ())
        .map_err(|error| RootError::PortalRejected {
            reason: format!("{kind} payload failed schema validation: {error}"),
        })
}

fn validate_encrypted_round2_payload(payload_bytes: &[u8]) -> Result<()> {
    let encrypted: PairwiseEncryptedPayload =
        ciborium::from_reader(payload_bytes).map_err(|error| RootError::PortalRejected {
            reason: format!("round-two encrypted package is malformed: {error}"),
        })?;
    if encrypted.ciphertext.is_empty() {
        return Err(RootError::PortalRejected {
            reason: "round-two encrypted package ciphertext must not be empty".to_owned(),
        });
    }
    Ok(())
}

#[derive(Serialize)]
struct PortalEnvelopeKeyParts<'a> {
    sender_did: &'a Did,
    phase: CeremonyPhase,
    payload_kind: CeremonyPayloadKind,
    sequence: u64,
    recipient_did: &'a Option<Did>,
}

fn key_parts(key: &PortalEnvelopeKey) -> PortalEnvelopeKeyParts<'_> {
    PortalEnvelopeKeyParts {
        sender_did: &key.sender_did,
        phase: key.phase,
        payload_kind: key.payload_kind,
        sequence: key.sequence,
        recipient_did: &key.recipient_did,
    }
}

fn transcript_record(
    key: &PortalEnvelopeKey,
    envelope: &CeremonyEnvelope,
) -> Result<TranscriptEnvelopeRecord> {
    Ok(TranscriptEnvelopeRecord {
        phase: envelope.phase,
        payload_kind: envelope.payload_kind,
        sender_did: envelope.sender_did.clone(),
        recipient_did: envelope.recipient_did.clone(),
        sequence: envelope.sequence,
        envelope_id: hash_structured(&key_parts(key)).map_err(canonical_encoding_error)?,
        envelope_hash: hash_structured(envelope).map_err(canonical_encoding_error)?,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use exo_core::{Timestamp, crypto::KeyPair};
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::{CertifierContact, PairwiseEncryptedPayload};

    fn round1_package_bytes(config: &GenesisCeremonyConfig, frost_identifier: u16) -> Vec<u8> {
        let mut rng = StdRng::seed_from_u64(u64::from(frost_identifier));
        crate::dkg_round1(config, frost_identifier, &mut rng)
            .expect("round one")
            .round1_package
    }

    fn certifier(index: u16) -> (CertifierContact, exo_core::SecretKey) {
        let seed = [u8::try_from(index).expect("index fits"); 32];
        let keypair = KeyPair::from_secret_bytes(seed).expect("keypair");
        let transport_public =
            x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(seed));
        (
            CertifierContact {
                did: Did::new(&format!("did:exo:portal-query-{index:02}")).expect("did"),
                frost_identifier: index,
                signing_public_key: *keypair.public_key(),
                transport_public_key: *transport_public.as_bytes(),
            },
            keypair.secret_key().clone(),
        )
    }

    fn config_with_secrets() -> (GenesisCeremonyConfig, Vec<SecretKey>) {
        let mut certifiers = Vec::new();
        let mut secrets = Vec::new();
        for index in 1..=crate::ROOT_GENESIS_SIGNERS {
            let (contact, secret) = certifier(index);
            certifiers.push(contact);
            secrets.push(secret);
        }
        (
            GenesisCeremonyConfig {
                ceremony_id: "portal-query".into(),
                network_id: "exochain-test".into(),
                repo_commit: "d8927686a34bdc28ba36d53938f665685d2c4c04".into(),
                constitution_hash: Hash256::digest(b"constitution"),
                threshold: crate::ROOT_GENESIS_THRESHOLD,
                max_signers: crate::ROOT_GENESIS_SIGNERS,
                created_at: Timestamp::new(1, 0),
                certifiers,
                signing_set: (1..=7).collect(),
            },
            secrets,
        )
    }

    fn encrypted_payload_bytes() -> Vec<u8> {
        let payload = PairwiseEncryptedPayload {
            nonce: [9u8; 24],
            ciphertext: b"ciphertext".to_vec(),
        };
        let mut bytes = Vec::new();
        ciborium::into_writer(&payload, &mut bytes).expect("encode");
        bytes
    }

    #[test]
    fn canonical_error_conversion_is_diagnostic() {
        let error = canonical_encoding_error("portal encoding failed");
        assert!(error.to_string().contains("portal encoding failed"));
    }

    #[test]
    fn query_filters_by_phase_kind_and_recipient() {
        let (config, secrets) = config_with_secrets();
        let mut store = PortalStore::new(config.clone());

        // A round-one broadcast from certifier 1.
        store
            .submit(
                CeremonyEnvelope::sign(
                    CeremonyEnvelopeDraft {
                        ceremony_id: config.ceremony_id.clone(),
                        phase: CeremonyPhase::Round1,
                        payload_kind: CeremonyPayloadKind::Round1Package,
                        sender_did: config.certifiers[0].did.clone(),
                        recipient_did: None,
                        sequence: 0,
                        payload_bytes: round1_package_bytes(&config, 1),
                    },
                    &secrets[0],
                )
                .expect("round1 envelope"),
            )
            .expect("submit round1");

        // A round-two package from certifier 1 addressed to certifier 2.
        store
            .submit(
                CeremonyEnvelope::sign(
                    CeremonyEnvelopeDraft {
                        ceremony_id: config.ceremony_id.clone(),
                        phase: CeremonyPhase::Round2,
                        payload_kind: CeremonyPayloadKind::Round2EncryptedPackage,
                        sender_did: config.certifiers[0].did.clone(),
                        recipient_did: Some(config.certifiers[1].did.clone()),
                        sequence: 1,
                        payload_bytes: encrypted_payload_bytes(),
                    },
                    &secrets[0],
                )
                .expect("round2 envelope"),
            )
            .expect("submit round2");

        // No filters → both envelopes.
        assert_eq!(store.query(None, None, None).len(), 2);
        // Phase filter (match + non-match).
        assert_eq!(
            store.query(Some(CeremonyPhase::Round1), None, None).len(),
            1
        );
        assert_eq!(
            store.query(Some(CeremonyPhase::Finalize), None, None).len(),
            0
        );
        // Payload-kind filter.
        assert_eq!(
            store
                .query(
                    None,
                    Some(CeremonyPayloadKind::Round2EncryptedPackage),
                    None
                )
                .len(),
            1
        );
        // Recipient filter (match + non-match).
        assert_eq!(
            store
                .query(
                    Some(CeremonyPhase::Round2),
                    None,
                    Some(&config.certifiers[1].did)
                )
                .len(),
            1
        );
        assert_eq!(
            store
                .query(None, None, Some(&config.certifiers[2].did))
                .len(),
            0
        );
    }
}
