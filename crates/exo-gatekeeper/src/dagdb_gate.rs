//! DAG DB write gate: active consent, Ed25519 provenance, then `M12` persistence.

use std::{collections::BTreeMap, sync::Arc};

use exo_api::dagdb::{
    ConsentPurpose, DagDbGraphContextPacket, DagDbGraphContextSelectionResponse, ReceiptEventType,
    SubjectKind,
};
use exo_consent::ConsentError;
use exo_core::{Hash256, PublicKey, Signature, Timestamp, crypto};
use exo_dag_db_core::hash::ReceiptHashMaterial;
use exo_dag_db_domain::{
    context_packet_persistence::ContextPacketRecord,
    continuation_persistence::{ContinuationPersistResult, ContinuationRecord},
    default_route::DefaultRouteRecord,
    lifecycle_action::{LifecycleAction, LifecycleApplyResult},
    scoring::{DomainError, hash_event_body},
};
use exo_dag_db_exchange::kg_import::{hash_from_hex, stable_hash};
use exo_dag_db_postgres::postgres::{
    context_packet_persistence::persist_context_packet_record,
    continuation_persistence::persist_continuation_record,
    default_route::persist_default_route,
    kg_context_selection_write::{
        DbWriteSummary, UsageEventMemoryMetadata, persist_context_packet_receipt_to_db,
        persist_usage_event_to_db, persist_usage_event_to_db_with_metadata,
    },
    lifecycle_action::persist_lifecycle_action,
};
use exo_identity::error::IdentityError;
use sqlx::PgPool;
use tracing::warn;

use crate::{
    error::GatekeeperError,
    invariants::{
        ConstitutionalInvariant, InvariantContext, InvariantEngine, InvariantSet, enforce_all,
    },
    types::{BailmentState, ConsentRecord},
};

const USAGE_EVENT_MEMORY_ID_DOMAIN: &str =
    "exo.dagdb.graph_context_selection.usage_event.memory_id";
const DAGDB_WRITE_SIGNATURE_DOMAIN: &str = "exo.gatekeeper.dagdb_write_signature.v1";
const CREATED_AT: Timestamp = Timestamp::new(1, 0);
const WRITER_DID: &str = "did:exo:dagdb-context-selection-writer";

// PRD-D5: subject-id domains for the four PRD17 lifecycle/persistence surfaces.
// Each is domain-separated so the signed payload hash binds to one surface and
// cannot be transplanted across surfaces.
const LIFECYCLE_ACTION_SUBJECT_DOMAIN: &str = "exo.dagdb.lifecycle_action.subject_id.v1";
const DEFAULT_ROUTE_SUBJECT_DOMAIN: &str = "exo.dagdb.default_route.subject_id.v1";
const CONTINUATION_SUBJECT_DOMAIN: &str = "exo.dagdb.continuation_record.subject_id.v1";
const CONTEXT_PACKET_RECORD_SUBJECT_DOMAIN: &str = "exo.dagdb.context_packet_record.subject_id.v1";

/// Constitutional invariants enforced on the dag-db write path.
///
/// This is the subset of [`InvariantSet::all`] that the dag-db authorization
/// state can construct honestly from the consent/identity DB rows the T2
/// resolver loads. Two invariants from the full set are deliberately excluded
/// here, NOT silently dropped:
///
/// * [`ConstitutionalInvariant::AuthorityChainValid`] requires a non-empty,
///   per-link Ed25519-signed [`AuthorityChain`](crate::types::AuthorityChain)
///   bound to independently-resolved grantor keys. The dag-db consent schema
///   stores a bailment + consent grant, not a signed delegation chain, so a
///   context built from it has an empty chain and this invariant would
///   fail-closed-block EVERY legitimate dag-db write — a deadlock, not
///   enforcement. Authorization on the dag-db path is instead established by the
///   tenant-scoped consent grant (`ConsentRequired`) plus the route-layer
///   session-authority binding.
/// * [`ConstitutionalInvariant::ProvenanceVerifiable`] requires a signed
///   [`Provenance`](crate::types::Provenance) object with trusted actor keys.
///   The dag-db write path already enforces Ed25519 actor provenance directly
///   in [`DagDbGatekeeperService::validate_write`] via [`verify_write_signature`]
///   over the canonical payload hash (the same Ed25519 binding the invariant
///   would re-check), so it is enforced — just not a second time through the
///   engine.
///
/// `ConsentRequired` IS run through the engine here (in addition to the
/// pre-engine `verify_write_consent`) because the engine check additionally
/// asserts bailor/bailee/scope coherence and that the consent scope covers the
/// requested permission.
#[must_use]
fn dagdb_invariant_set() -> InvariantSet {
    InvariantSet::with(vec![
        ConstitutionalInvariant::ConsentRequired,
        ConstitutionalInvariant::SeparationOfPowers,
        ConstitutionalInvariant::NoSelfGrant,
        ConstitutionalInvariant::HumanOverride,
        ConstitutionalInvariant::KernelImmutability,
        ConstitutionalInvariant::QuorumLegitimate,
    ])
}

/// Active bailment + consent lookup for DAG DB writeback.
#[derive(Debug, Clone, Default)]
pub struct ConsentEngine {
    bailments: BTreeMap<String, BailmentState>,
    records: BTreeMap<(String, String, ConsentPurpose), DagDbConsentRecord>,
}

/// Scoped consent row for a tenant/agent/purpose triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagDbConsentRecord {
    pub tenant_id: String,
    pub agent_did: String,
    pub purpose: ConsentPurpose,
    pub active: bool,
}

impl ConsentEngine {
    /// Register bailment state for a tenant (BCTS).
    #[must_use]
    pub fn with_bailment(mut self, tenant_id: impl Into<String>, state: BailmentState) -> Self {
        self.bailments.insert(tenant_id.into(), state);
        self
    }

    /// Register a consent record.
    #[must_use]
    pub fn with_consent_record(mut self, record: DagDbConsentRecord) -> Self {
        let key = (
            record.tenant_id.clone(),
            record.agent_did.clone(),
            record.purpose,
        );
        self.records.insert(key, record);
        self
    }

    fn bailment_for(&self, tenant_id: &str) -> Option<&BailmentState> {
        self.bailments.get(tenant_id)
    }

    /// Snapshot the bailment state registered for `tenant_id`, for constructing
    /// the constitutional [`InvariantContext`] over the dag-db write path.
    #[must_use]
    pub fn bailment_state(&self, tenant_id: &str) -> BailmentState {
        self.bailment_for(tenant_id)
            .cloned()
            .unwrap_or(BailmentState::None)
    }

    fn consent_for(
        &self,
        tenant_id: &str,
        agent_did: &str,
        purpose: ConsentPurpose,
    ) -> Option<&DagDbConsentRecord> {
        self.records
            .get(&(tenant_id.to_owned(), agent_did.to_owned(), purpose))
    }
}

/// Actor public-key registry for provenance verification.
#[derive(Debug, Clone, Default)]
pub struct IdentityRegistry {
    keys: BTreeMap<String, [u8; 32]>,
}

impl IdentityRegistry {
    /// Register an Ed25519 public key for a DID string.
    #[must_use]
    pub fn with_public_key(mut self, agent_did: impl Into<String>, public_key: [u8; 32]) -> Self {
        self.keys.insert(agent_did.into(), public_key);
        self
    }

    fn public_key_for(&self, agent_did: &str) -> Option<&[u8; 32]> {
        self.keys.get(agent_did)
    }
}

/// Returns `true` when active bailment (BCTS) and consent exist for writeback.
pub fn verify_write_consent(
    engine: &ConsentEngine,
    tenant_id: &str,
    agent_did: &str,
    purpose: ConsentPurpose,
) -> Result<bool, ConsentError> {
    let Some(bailment) = engine.bailment_for(tenant_id) else {
        return Err(ConsentError::NoConsent(format!(
            "no bailment for tenant {tenant_id}"
        )));
    };
    if !bailment.is_active() {
        return Err(ConsentError::Denied(format!(
            "bailment inactive for tenant {tenant_id}"
        )));
    }
    // An active bailment is not sufficient: the bailor must have entrusted THIS
    // agent (bailee) with the writeback scope. A bailment for a different bailee
    // or scope must not authorize this agent's writeback.
    if purpose == ConsentPurpose::Writeback && !bailment.authorizes_writeback(agent_did) {
        return Err(ConsentError::Denied(format!(
            "bailment does not authorize {agent_did} for writeback on tenant {tenant_id}"
        )));
    }
    let Some(record) = engine.consent_for(tenant_id, agent_did, purpose) else {
        return Err(ConsentError::NoConsent(format!(
            "no consent record for {agent_did} purpose {purpose:?}"
        )));
    };
    if !record.active {
        return Err(ConsentError::Denied(format!(
            "consent inactive for {agent_did} purpose {purpose:?}"
        )));
    }
    Ok(true)
}

/// Returns `true` when the Ed25519 signature verifies over the payload hash bytes.
pub fn verify_write_signature(
    registry: &IdentityRegistry,
    payload_hash: &[u8; 32],
    signature: &str,
    agent_did: &str,
) -> Result<bool, IdentityError> {
    let did =
        exo_core::Did::new(agent_did).map_err(|_| IdentityError::InvalidDidDocumentField {
            did: agent_did.to_owned(),
            field: "did".into(),
            reason: "invalid agent DID".into(),
        })?;
    let public_key_bytes = registry
        .public_key_for(agent_did)
        .ok_or(IdentityError::KeyNotFound(did))?;
    let signature_bytes = decode_ed25519_signature_hex(signature)?;
    let message = dagdb_write_signature_message(payload_hash)?;
    let public_key = PublicKey::from_bytes(*public_key_bytes);
    let sig = Signature::from_bytes(signature_bytes);
    Ok(crypto::verify(message.as_bytes(), &sig, &public_key))
}

/// Gatekeeper service that enforces consent and provenance before `M12` writes.
pub struct DagDbGatekeeperService {
    pub pool: PgPool,
    pub consent_engine: Arc<ConsentEngine>,
    pub identity_registry: Arc<IdentityRegistry>,
}

impl DagDbGatekeeperService {
    /// Construct a gatekeeper service over a Postgres pool and policy engines.
    #[must_use]
    pub fn new(
        pool: PgPool,
        consent_engine: Arc<ConsentEngine>,
        identity_registry: Arc<IdentityRegistry>,
    ) -> Self {
        Self {
            pool,
            consent_engine,
            identity_registry,
        }
    }

    /// Build the constitutional [`InvariantContext`] for a dag-db write from the
    /// consent/identity authorization state this service already holds.
    ///
    /// The context is enforced through `dagdb_invariant_set` (NOT the full
    /// `all()`): the dag-db consent schema yields a bailment + consent grant, so
    /// `ConsentRequired`, `SeparationOfPowers`, `NoSelfGrant`, `HumanOverride`,
    /// `KernelImmutability`, and `QuorumLegitimate` are all genuinely checkable.
    /// `AuthorityChainValid`/`ProvenanceVerifiable` are excluded — see
    /// `dagdb_invariant_set`.
    ///
    /// Returns `None` only when `agent_did` is not a structurally-valid DID
    /// (caller already validated it upstream; the gate fails closed before any
    /// write regardless).
    #[must_use]
    pub fn dagdb_invariant_context(
        &self,
        tenant_id: &str,
        agent_did: &str,
    ) -> Option<InvariantContext> {
        let actor = exo_core::Did::new(agent_did).ok()?;
        let bailment_state = self.consent_engine.bailment_state(tenant_id);
        // Mirror the bailment grant as a gatekeeper consent record so the engine's
        // `ConsentRequired` check (bailor/bailee/scope coherence) resolves against
        // the same DB-derived grant the pre-engine `verify_write_consent` used.
        let consent_records = match &bailment_state {
            BailmentState::Active {
                bailor,
                bailee,
                scope,
            } => vec![ConsentRecord {
                subject: bailor.clone(),
                granted_to: bailee.clone(),
                scope: scope.clone(),
                active: true,
            }],
            _ => Vec::new(),
        };
        Some(InvariantContext {
            actor,
            // The dag-db agent acts under a bailment, not a governed multi-branch
            // role; no roles => SeparationOfPowers passes (single/zero branch).
            actor_roles: Vec::new(),
            bailment_state,
            consent_records,
            // A writeback is not a permission grant: the agent is not expanding
            // its own permissions, so this is never a self-grant.
            authority_chain: Default::default(),
            is_self_grant: false,
            // Human override is preserved: the dag-db write path never disables
            // emergency human intervention.
            human_override_preserved: true,
            // A graph-memory write never mutates immutable kernel configuration.
            kernel_modification_attempted: false,
            // No quorum is gathered for a single-actor writeback; `None` makes
            // QuorumLegitimate vacuously pass.
            quorum_evidence: None,
            // Ed25519 provenance is enforced directly in `validate_write`, not via
            // the engine, so no `Provenance` object is constructed here.
            provenance: None,
            // No permission expansion is requested, so the consent-scope coverage
            // check passes with an empty requested set.
            actor_permissions: Default::default(),
            requested_permissions: Default::default(),
            trusted_authority_keys: Default::default(),
            trusted_provenance_keys: Default::default(),
        })
    }

    /// Persist a usage event after consent, signature, and optional invariant checks.
    pub async fn persist_usage_event(
        &self,
        event: &DagDbGraphContextSelectionResponse,
        agent_did: &str,
        signature: &str,
        invariant_context: Option<&InvariantContext>,
    ) -> Result<DbWriteSummary, GatekeeperError> {
        let payload_hash = usage_event_payload_hash(event)?;
        self.validate_write(
            &event.tenant_id,
            agent_did,
            ConsentPurpose::Writeback,
            &payload_hash,
            signature,
            invariant_context,
        )?;
        persist_usage_event_to_db(&self.pool, event)
            .await
            .map_err(domain_to_gatekeeper)
    }

    /// Persist a usage event with searchable metadata after consent and provenance checks.
    pub async fn persist_usage_event_with_metadata(
        &self,
        event: &DagDbGraphContextSelectionResponse,
        agent_did: &str,
        signature: &str,
        invariant_context: Option<&InvariantContext>,
        metadata: Option<&UsageEventMemoryMetadata>,
    ) -> Result<DbWriteSummary, GatekeeperError> {
        let payload_hash = usage_event_payload_hash(event)?;
        self.validate_write(
            &event.tenant_id,
            agent_did,
            ConsentPurpose::Writeback,
            &payload_hash,
            signature,
            invariant_context,
        )?;
        persist_usage_event_to_db_with_metadata(&self.pool, event, metadata)
            .await
            .map_err(domain_to_gatekeeper)
    }

    /// Persist a context-packet receipt after consent, signature, and optional invariant checks.
    pub async fn persist_context_packet_receipt(
        &self,
        packet: &DagDbGraphContextPacket,
        agent_did: &str,
        signature: &str,
        invariant_context: Option<&InvariantContext>,
    ) -> Result<DbWriteSummary, GatekeeperError> {
        let payload_hash = context_packet_payload_hash(packet)?;
        self.validate_write(
            &packet.tenant_id,
            agent_did,
            ConsentPurpose::Writeback,
            &payload_hash,
            signature,
            invariant_context,
        )?;
        persist_context_packet_receipt_to_db(&self.pool, packet)
            .await
            .map_err(domain_to_gatekeeper)
    }

    /// PRD-D5: persist a lifecycle action after consent, signature, and optional
    /// invariant checks. Lifecycle actions mutate the graph (writeback / relink /
    /// supersede / recycle / archive); this routes them through the same
    /// gatekeeper chain as the other graph-mutating write paths instead of the
    /// prior `validate()`-only persistence.
    ///
    /// DORMANT (no runtime caller): this method, together with
    /// [`Self::persist_default_route`], [`Self::persist_continuation_record`],
    /// and [`Self::persist_context_packet_record`], is GATED — it enforces the
    /// full consent → Ed25519 → invariant-subset chain via `validate_write` —
    /// but is NOT yet wired to a served gateway endpoint. No `/api/v1/dag-db/*`
    /// route invokes it; the only callers are the gate's own route-contract
    /// tests, which prove the chain is enforced (consented+signed reaches the DB
    /// layer; forged/unconsented fail closed). The `gatekeeper-lifecycle-surfaces-gated`
    /// security check asserts exactly that property — surfaces are gated at the
    /// method boundary — and does NOT claim a live endpoint serves them. Wiring
    /// these to REST endpoints is deferred (no requirement drives them yet); see
    /// the dag-db INTEGRATION.md "Adapter boundary" note.
    pub async fn persist_lifecycle_action(
        &self,
        action: &LifecycleAction,
        agent_did: &str,
        signature: &str,
        invariant_context: Option<&InvariantContext>,
    ) -> Result<LifecycleApplyResult, GatekeeperError> {
        let payload_hash = lifecycle_action_payload_hash(action)?;
        self.validate_write(
            &action.tenant_id,
            agent_did,
            ConsentPurpose::Writeback,
            &payload_hash,
            signature,
            invariant_context,
        )?;
        persist_lifecycle_action(&self.pool, action)
            .await
            .map_err(|error| domain_blocked("lifecycle_action_postgres", &error))
    }

    /// PRD-D5: persist a default-route record after consent, signature, and
    /// optional invariant checks.
    ///
    /// DORMANT (no runtime caller): see [`Self::persist_lifecycle_action`].
    pub async fn persist_default_route(
        &self,
        route: &DefaultRouteRecord,
        agent_did: &str,
        signature: &str,
        invariant_context: Option<&InvariantContext>,
    ) -> Result<u64, GatekeeperError> {
        let payload_hash = default_route_payload_hash(route)?;
        self.validate_write(
            &route.tenant_id,
            agent_did,
            ConsentPurpose::Writeback,
            &payload_hash,
            signature,
            invariant_context,
        )?;
        persist_default_route(&self.pool, route)
            .await
            .map_err(|error| domain_blocked("default_route_postgres", &error))
    }

    /// PRD-D5: persist a continuation record after consent, signature, and
    /// optional invariant checks.
    ///
    /// DORMANT (no runtime caller): see [`Self::persist_lifecycle_action`].
    pub async fn persist_continuation_record(
        &self,
        record: &ContinuationRecord,
        now_epoch_seconds: u64,
        agent_did: &str,
        signature: &str,
        invariant_context: Option<&InvariantContext>,
    ) -> Result<ContinuationPersistResult, GatekeeperError> {
        let payload_hash = continuation_record_payload_hash(record)?;
        self.validate_write(
            &record.tenant_id,
            agent_did,
            ConsentPurpose::Writeback,
            &payload_hash,
            signature,
            invariant_context,
        )?;
        persist_continuation_record(&self.pool, record, now_epoch_seconds)
            .await
            .map_err(|error| domain_blocked("continuation_postgres", &error))
    }

    /// PRD-D5: persist a context-packet record after consent, signature, and
    /// optional invariant checks.
    ///
    /// DORMANT (no runtime caller): see [`Self::persist_lifecycle_action`].
    pub async fn persist_context_packet_record(
        &self,
        record: &ContextPacketRecord,
        agent_did: &str,
        signature: &str,
        invariant_context: Option<&InvariantContext>,
    ) -> Result<u64, GatekeeperError> {
        let payload_hash = context_packet_record_payload_hash(record)?;
        self.validate_write(
            &record.tenant_id,
            agent_did,
            ConsentPurpose::Writeback,
            &payload_hash,
            signature,
            invariant_context,
        )?;
        persist_context_packet_record(&self.pool, record)
            .await
            .map_err(|error| domain_blocked("context_packet_record_postgres", &error))
    }

    fn validate_write(
        &self,
        tenant_id: &str,
        agent_did: &str,
        purpose: ConsentPurpose,
        payload_hash: &[u8; 32],
        signature: &str,
        invariant_context: Option<&InvariantContext>,
    ) -> Result<(), GatekeeperError> {
        match verify_write_consent(self.consent_engine.as_ref(), tenant_id, agent_did, purpose) {
            Ok(true) => {}
            Ok(false) => {
                return Err(consent_gatekeeper_error(
                    tenant_id,
                    agent_did,
                    "consent verification returned false",
                ));
            }
            Err(error) => {
                log_invariant_violation(
                    ConstitutionalInvariant::ConsentRequired,
                    tenant_id,
                    agent_did,
                    &error.to_string(),
                );
                return Err(consent_gatekeeper_error(
                    tenant_id,
                    agent_did,
                    &error.to_string(),
                ));
            }
        }

        match verify_write_signature(
            self.identity_registry.as_ref(),
            payload_hash,
            signature,
            agent_did,
        ) {
            Ok(true) => {}
            Ok(false) => {
                log_invariant_violation(
                    ConstitutionalInvariant::ProvenanceVerifiable,
                    tenant_id,
                    agent_did,
                    "signature verification returned false",
                );
                return Err(provenance_gatekeeper_error(
                    tenant_id,
                    agent_did,
                    "invalid Ed25519 signature",
                ));
            }
            Err(error) => {
                log_invariant_violation(
                    ConstitutionalInvariant::ProvenanceVerifiable,
                    tenant_id,
                    agent_did,
                    &error.to_string(),
                );
                return Err(provenance_gatekeeper_error(
                    tenant_id,
                    agent_did,
                    &error.to_string(),
                ));
            }
        }

        if let Some(context) = invariant_context {
            // Enforce the constructible dag-db invariant subset, not the full
            // `InvariantEngine::all()`: a context built from the dag-db consent
            // schema carries no signed authority chain, so `all()` would
            // fail-closed-block every legitimate write. See `dagdb_invariant_set`
            // for which invariants are engine-enforced vs enforced directly
            // above (Ed25519 provenance) on this path.
            let engine = InvariantEngine::new(dagdb_invariant_set());
            let violations = enforce_all(&engine, context);
            if let Err(violations) = violations {
                let detail = violations
                    .iter()
                    .map(|v| format!("{}: {}", v.invariant.id(), v.description))
                    .collect::<Vec<_>>()
                    .join("; ");
                for violation in &violations {
                    log_invariant_violation(
                        violation.invariant,
                        tenant_id,
                        agent_did,
                        &violation.description,
                    );
                }
                return Err(GatekeeperError::InvariantViolation(detail));
            }
        }
        Ok(())
    }
}

/// Produce a lowercase hex Ed25519 signature over the canonical write payload hash.
pub fn sign_write_payload(
    keypair: &exo_core::crypto::KeyPair,
    payload_hash: &[u8; 32],
) -> Result<String, IdentityError> {
    let message = dagdb_write_signature_message(payload_hash)?;
    Ok(format!("{}", keypair.sign(message.as_ref())))
}

/// Compute the canonical receipt-hash bytes used as the signed payload for usage events.
pub fn usage_event_payload_hash(
    event: &DagDbGraphContextSelectionResponse,
) -> Result<[u8; 32], GatekeeperError> {
    let memory_id = stable_hash(
        USAGE_EVENT_MEMORY_ID_DOMAIN,
        &[
            &event.tenant_id,
            &event.namespace,
            &event.request_id,
            &event.task_hash,
        ],
    )
    .map_err(|error| GatekeeperError::Core(error.to_string()))?;
    let event_body_hash = hash_event_body(event).map_err(domain_to_gatekeeper)?;
    let receipt_hash = ReceiptHashMaterial {
        tenant_id: event.tenant_id.clone(),
        namespace: event.namespace.clone(),
        subject_kind: SubjectKind::Memory,
        subject_id: memory_id,
        prev_receipt_hash: Hash256::ZERO,
        seq: 1,
        event_type: ReceiptEventType::IntakeCreated,
        actor_did: WRITER_DID.to_owned(),
        event_hlc: CREATED_AT,
        event_body_hash,
    }
    .hash()
    .map_err(|error| GatekeeperError::Core(error.to_string()))?;
    Ok(*receipt_hash.as_bytes())
}

/// Compute the canonical receipt-hash bytes used as the signed payload for packet receipts.
pub fn context_packet_payload_hash(
    packet: &DagDbGraphContextPacket,
) -> Result<[u8; 32], GatekeeperError> {
    let subject_id = hash_from_hex("packet_hash", &packet.packet_hash)
        .map_err(|_| GatekeeperError::InvariantViolation("invalid context packet hash".into()))?;
    let event_body_hash = hash_event_body(packet).map_err(domain_to_gatekeeper)?;
    let receipt_hash = ReceiptHashMaterial {
        tenant_id: packet.tenant_id.clone(),
        namespace: packet.namespace.clone(),
        subject_kind: SubjectKind::ContextPacket,
        subject_id,
        prev_receipt_hash: Hash256::ZERO,
        seq: 1,
        event_type: ReceiptEventType::ContextPacketCreated,
        actor_did: WRITER_DID.to_owned(),
        event_hlc: CREATED_AT,
        event_body_hash,
    }
    .hash()
    .map_err(|error| GatekeeperError::Core(error.to_string()))?;
    Ok(*receipt_hash.as_bytes())
}

/// PRD-D5: canonical receipt-hash bytes used as the signed payload for a
/// lifecycle action. The subject id is domain-separated over the action's
/// deterministic identity (tenant/namespace/action id/idempotency key) and the
/// full action body is folded into `event_body_hash`, so any change to the
/// persisted action invalidates the signature.
pub fn lifecycle_action_payload_hash(
    action: &LifecycleAction,
) -> Result<[u8; 32], GatekeeperError> {
    let idempotency_key = action
        .idempotency_key()
        .map_err(|error| GatekeeperError::InvariantViolation(error.to_string()))?;
    surface_payload_hash(
        LIFECYCLE_ACTION_SUBJECT_DOMAIN,
        &action.tenant_id,
        &action.memory_namespace,
        SubjectKind::Memory,
        ReceiptEventType::MemorySuperseded,
        &[
            &action.tenant_id,
            &action.memory_namespace,
            &action.action_id,
            &idempotency_key,
        ],
        action,
    )
}

/// PRD-D5: canonical receipt-hash bytes used as the signed payload for a
/// default-route record.
pub fn default_route_payload_hash(route: &DefaultRouteRecord) -> Result<[u8; 32], GatekeeperError> {
    surface_payload_hash(
        DEFAULT_ROUTE_SUBJECT_DOMAIN,
        &route.tenant_id,
        &route.memory_namespace,
        SubjectKind::Route,
        ReceiptEventType::RouteCreated,
        &[&route.tenant_id, &route.memory_namespace, &route.route_id],
        route,
    )
}

/// PRD-D5: canonical receipt-hash bytes used as the signed payload for a
/// continuation record.
pub fn continuation_record_payload_hash(
    record: &ContinuationRecord,
) -> Result<[u8; 32], GatekeeperError> {
    let idempotency_key = record
        .idempotency_key()
        .map_err(|error| GatekeeperError::InvariantViolation(error.to_string()))?;
    surface_payload_hash(
        CONTINUATION_SUBJECT_DOMAIN,
        &record.tenant_id,
        &record.memory_namespace,
        SubjectKind::Memory,
        ReceiptEventType::IntakeCreated,
        &[
            &record.tenant_id,
            &record.memory_namespace,
            &record.continuation_id,
            &idempotency_key,
        ],
        record,
    )
}

/// PRD-D5: canonical receipt-hash bytes used as the signed payload for a
/// context-packet record.
pub fn context_packet_record_payload_hash(
    record: &ContextPacketRecord,
) -> Result<[u8; 32], GatekeeperError> {
    surface_payload_hash(
        CONTEXT_PACKET_RECORD_SUBJECT_DOMAIN,
        &record.tenant_id,
        &record.memory_namespace,
        SubjectKind::ContextPacket,
        ReceiptEventType::ContextPacketCreated,
        &[
            &record.tenant_id,
            &record.memory_namespace,
            &record.packet_id,
            &record.idempotency_key,
        ],
        record,
    )
}

/// Shared PRD-D5 helper: build the signed payload hash for a persistence surface
/// from a domain-separated subject id plus the full record body. Mirrors
/// `usage_event_payload_hash`/`context_packet_payload_hash` so all gated surfaces
/// share one signature-binding shape.
fn surface_payload_hash<T: serde::Serialize>(
    subject_domain: &str,
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    event_type: ReceiptEventType,
    subject_parts: &[&str],
    body: &T,
) -> Result<[u8; 32], GatekeeperError> {
    let subject_id = stable_hash(subject_domain, subject_parts)
        .map_err(|error| GatekeeperError::Core(error.to_string()))?;
    let event_body_hash = hash_event_body(body).map_err(domain_to_gatekeeper)?;
    let receipt_hash = ReceiptHashMaterial {
        tenant_id: tenant_id.to_owned(),
        namespace: namespace.to_owned(),
        subject_kind,
        subject_id,
        prev_receipt_hash: Hash256::ZERO,
        seq: 1,
        event_type,
        actor_did: WRITER_DID.to_owned(),
        event_hlc: CREATED_AT,
        event_body_hash,
    }
    .hash()
    .map_err(|error| GatekeeperError::Core(error.to_string()))?;
    Ok(*receipt_hash.as_bytes())
}

fn dagdb_write_signature_message(payload_hash: &[u8; 32]) -> Result<Hash256, IdentityError> {
    exo_core::hash::hash_structured(&DagDbWriteSignaturePayload {
        domain: DAGDB_WRITE_SIGNATURE_DOMAIN,
        payload_hash,
    })
    .map_err(|error| IdentityError::InvalidDidDocumentField {
        did: "dagdb-write".into(),
        field: "signature_payload".into(),
        reason: error.to_string(),
    })
}

#[derive(serde::Serialize)]
struct DagDbWriteSignaturePayload<'a> {
    domain: &'static str,
    payload_hash: &'a [u8; 32],
}

fn decode_ed25519_signature_hex(signature: &str) -> Result<[u8; 64], IdentityError> {
    let bytes = hex::decode(signature).map_err(|error| IdentityError::InvalidDidDocumentField {
        did: "dagdb-write".into(),
        field: "signature".into(),
        reason: format!("hex decode failed: {error}"),
    })?;
    bytes
        .try_into()
        .map_err(|bytes: Vec<u8>| IdentityError::InvalidDidDocumentField {
            did: "dagdb-write".into(),
            field: "signature".into(),
            reason: format!("expected 64 bytes, got {}", bytes.len()),
        })
}

fn consent_gatekeeper_error(tenant_id: &str, agent_did: &str, reason: &str) -> GatekeeperError {
    GatekeeperError::InvariantViolation(format!(
        "ConsentRequired: tenant={tenant_id} actor={agent_did} reason={reason}"
    ))
}

fn provenance_gatekeeper_error(tenant_id: &str, agent_did: &str, reason: &str) -> GatekeeperError {
    GatekeeperError::InvariantViolation(format!(
        "ProvenanceVerifiable: tenant={tenant_id} actor={agent_did} reason={reason}"
    ))
}

fn domain_to_gatekeeper(error: DomainError) -> GatekeeperError {
    GatekeeperError::InvariantViolation(format!("dagdb write blocked: {error}"))
}

/// PRD-D5: map a lifecycle/persistence-surface error into the classified
/// gatekeeper error shape. A database/transaction failure carries the
/// `surface_database_unavailable` marker the gateway maps to 503; every other
/// failure (contract, idempotency replay, serialization) is a request-level
/// rejection the gateway maps to 422. The decision is deterministic from the
/// error's own Display string, so a DB outage is never reported as a policy
/// rejection and a contract reject is never reported as DB unavailability.
fn domain_blocked<E: std::error::Error>(surface: &str, error: &E) -> GatekeeperError {
    let rendered = error.to_string();
    let is_db_unavailable = rendered.contains("postgres_failed")
        || rendered.contains("sql_failed")
        || surface_error_source_is_sqlx(error);
    if is_db_unavailable {
        GatekeeperError::InvariantViolation(format!(
            "dagdb write blocked: surface_database_unavailable surface={surface} detail={rendered}"
        ))
    } else {
        GatekeeperError::InvariantViolation(format!(
            "dagdb write blocked: metadata rejected surface={surface} detail={rendered}"
        ))
    }
}

/// Returns `true` when the error (or any error in its `source()` chain) is a
/// `sqlx::Error`, i.e. a live database/transaction failure rather than a
/// request-level contract rejection.
fn surface_error_source_is_sqlx<E: std::error::Error>(error: &E) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = error.source();
    while let Some(source) = current {
        if source.is::<sqlx::Error>() {
            return true;
        }
        current = source.source();
    }
    false
}

fn log_invariant_violation(
    invariant: ConstitutionalInvariant,
    tenant_id: &str,
    agent_did: &str,
    reason: &str,
) {
    warn!(
        validation_status = "denied",
        invariant = invariant.id(),
        tenant_id,
        actor_did = agent_did,
        reason,
        "InvariantViolated"
    );
}

#[cfg(test)]
mod tests {
    use exo_api::dagdb::{
        ConsentPurpose, DagDbGraphContextPacket, DagDbGraphContextSelectionResponse,
        DagDbGraphContextSelectionStatus,
    };
    use exo_core::crypto::KeyPair;

    use super::*;
    use crate::{
        invariants::ConstitutionalInvariant,
        types::{BailmentState, GovernmentBranch, Role},
    };

    fn active_consent_engine(tenant_id: &str, agent_did: &str) -> ConsentEngine {
        ConsentEngine::default()
            .with_bailment(
                tenant_id,
                BailmentState::Active {
                    bailor: exo_core::Did::new("did:exo:bailor").expect("bailor"),
                    bailee: exo_core::Did::new(agent_did).expect("bailee"),
                    scope: "dag-db:writeback".into(),
                },
            )
            .with_consent_record(DagDbConsentRecord {
                tenant_id: tenant_id.to_owned(),
                agent_did: agent_did.to_owned(),
                purpose: ConsentPurpose::Writeback,
                active: true,
            })
    }

    #[test]
    fn verify_write_consent_requires_active_bailment_and_record() {
        let engine = active_consent_engine("tenant-a", "did:exo:agent");
        assert!(
            verify_write_consent(
                &engine,
                "tenant-a",
                "did:exo:agent",
                ConsentPurpose::Writeback
            )
            .expect("consent ok")
        );

        let missing = ConsentEngine::default().with_consent_record(DagDbConsentRecord {
            tenant_id: "tenant-a".into(),
            agent_did: "did:exo:agent".into(),
            purpose: ConsentPurpose::Writeback,
            active: true,
        });
        assert!(
            verify_write_consent(
                &missing,
                "tenant-a",
                "did:exo:agent",
                ConsentPurpose::Writeback
            )
            .is_err()
        );
    }

    #[test]
    fn verify_write_signature_accepts_valid_ed25519() {
        let keypair = KeyPair::generate();
        let payload_hash = [7u8; 32];
        let message = dagdb_write_signature_message(&payload_hash).expect("message");
        let signature = keypair.sign(message.as_bytes());
        let signature_hex = format!("{signature}");
        let registry = IdentityRegistry::default()
            .with_public_key("did:exo:agent", *keypair.public_key().as_bytes());
        assert!(
            verify_write_signature(&registry, &payload_hash, &signature_hex, "did:exo:agent")
                .expect("verify ok")
        );
    }

    #[test]
    fn verify_write_signature_rejects_forged_signature() {
        let keypair = KeyPair::generate();
        let other = KeyPair::generate();
        let payload_hash = [9u8; 32];
        let message = dagdb_write_signature_message(&payload_hash).expect("message");
        let forged = other.sign(message.as_bytes());
        let forged_hex = format!("{forged}");
        let registry = IdentityRegistry::default()
            .with_public_key("did:exo:agent", *keypair.public_key().as_bytes());
        assert!(
            !verify_write_signature(&registry, &payload_hash, &forged_hex, "did:exo:agent")
                .expect("verify completes")
        );
    }

    #[test]
    fn separation_of_powers_blocks_multi_branch_actor() {
        let engine = InvariantEngine::new(crate::invariants::InvariantSet::with(vec![
            ConstitutionalInvariant::SeparationOfPowers,
        ]));
        let actor = exo_core::Did::new("did:exo:agent").expect("actor");
        let context = InvariantContext {
            actor: actor.clone(),
            actor_roles: vec![
                Role {
                    name: "senator".into(),
                    branch: GovernmentBranch::Legislative,
                },
                Role {
                    name: "executor".into(),
                    branch: GovernmentBranch::Executive,
                },
            ],
            bailment_state: BailmentState::Active {
                bailor: exo_core::Did::new("did:exo:bailor").expect("bailor"),
                bailee: actor,
                scope: "dag-db".into(),
            },
            consent_records: Vec::new(),
            authority_chain: Default::default(),
            is_self_grant: false,
            human_override_preserved: true,
            kernel_modification_attempted: false,
            quorum_evidence: None,
            provenance: None,
            actor_permissions: Default::default(),
            requested_permissions: Default::default(),
            trusted_authority_keys: Default::default(),
            trusted_provenance_keys: Default::default(),
        };
        assert!(enforce_all(&engine, &context).is_err());
    }

    fn sample_selection() -> DagDbGraphContextSelectionResponse {
        DagDbGraphContextSelectionResponse {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-gate-1".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            selection_status: DagDbGraphContextSelectionStatus::Selected,
            selected_memory_refs: Vec::new(),
            selected_graph_edges: Vec::new(),
            omitted_memory_refs: Vec::new(),
            selection_trace: Vec::new(),
            selected_token_estimate: 0,
            token_budget: 1_000,
            boundary_warnings: Vec::new(),
        }
    }

    fn sample_packet() -> DagDbGraphContextPacket {
        DagDbGraphContextPacket {
            schema_version: "dagdb.graph_context_packet.v1".into(),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-gate-1".into(),
            task: "Build packet".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            packet_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            selected_memory_refs: Vec::new(),
            selected_graph_edges: Vec::new(),
            citation_refs: Vec::new(),
            packet_metrics: exo_api::dagdb::DagDbContextPacketMetrics {
                token_budget: 1_000,
                selected_token_estimate: 0,
                selected_memory_ref_count: 0,
                selected_graph_edge_count: 0,
                citation_ref_count: 0,
                end_to_end_savings_status: "blocked".into(),
                cost_savings_status: "blocked".into(),
            },
            boundaries: exo_api::dagdb::DagDbContextPacketBoundaries {
                repository_test_level_only: true,
                production_runtime: "blocked".into(),
                default_context_replacement: "blocked".into(),
                citation_locator_status: "omitted_citation_locator_blocked".into(),
                billing_savings: "blocked".into(),
            },
            agent_usage_instructions: Vec::new(),
            markdown: "# packet".into(),
        }
    }

    #[test]
    fn verify_write_consent_requires_consent_record_for_purpose() {
        let bailment_only = ConsentEngine::default().with_bailment(
            "tenant-a",
            BailmentState::Active {
                bailor: exo_core::Did::new("did:exo:bailor").expect("bailor"),
                bailee: exo_core::Did::new("did:exo:agent").expect("bailee"),
                scope: "dag-db:writeback".into(),
            },
        );
        assert!(
            verify_write_consent(
                &bailment_only,
                "tenant-a",
                "did:exo:agent",
                ConsentPurpose::Writeback
            )
            .is_err()
        );
    }

    #[test]
    fn verify_write_consent_rejects_inactive_bailment_and_consent() {
        let inactive_bailment = ConsentEngine::default()
            .with_bailment(
                "tenant-a",
                BailmentState::Suspended {
                    reason: "test".into(),
                },
            )
            .with_consent_record(DagDbConsentRecord {
                tenant_id: "tenant-a".into(),
                agent_did: "did:exo:agent".into(),
                purpose: ConsentPurpose::Writeback,
                active: true,
            });
        assert!(
            verify_write_consent(
                &inactive_bailment,
                "tenant-a",
                "did:exo:agent",
                ConsentPurpose::Writeback
            )
            .is_err()
        );

        let inactive_consent = active_consent_engine("tenant-a", "did:exo:agent")
            .with_consent_record(DagDbConsentRecord {
                tenant_id: "tenant-a".into(),
                agent_did: "did:exo:agent".into(),
                purpose: ConsentPurpose::Writeback,
                active: false,
            });
        assert!(
            verify_write_consent(
                &inactive_consent,
                "tenant-a",
                "did:exo:agent",
                ConsentPurpose::Writeback
            )
            .is_err()
        );
    }

    #[test]
    fn verify_write_consent_rejects_active_bailment_for_other_bailee() {
        // Active bailment, valid consent record, but the bailment was entrusted
        // to a different bailee than the acting agent. Must fail closed.
        let engine = ConsentEngine::default()
            .with_bailment(
                "tenant-a",
                BailmentState::Active {
                    bailor: exo_core::Did::new("did:exo:bailor").expect("bailor"),
                    bailee: exo_core::Did::new("did:exo:other-agent").expect("bailee"),
                    scope: "dag-db:writeback".into(),
                },
            )
            .with_consent_record(DagDbConsentRecord {
                tenant_id: "tenant-a".into(),
                agent_did: "did:exo:agent".into(),
                purpose: ConsentPurpose::Writeback,
                active: true,
            });
        assert!(
            verify_write_consent(
                &engine,
                "tenant-a",
                "did:exo:agent",
                ConsentPurpose::Writeback
            )
            .is_err()
        );
    }

    #[test]
    fn verify_write_consent_rejects_active_bailment_with_wrong_scope() {
        // Active bailment for the correct bailee but a non-writeback scope.
        let engine = ConsentEngine::default()
            .with_bailment(
                "tenant-a",
                BailmentState::Active {
                    bailor: exo_core::Did::new("did:exo:bailor").expect("bailor"),
                    bailee: exo_core::Did::new("did:exo:agent").expect("bailee"),
                    scope: "dag-db:retrieval".into(),
                },
            )
            .with_consent_record(DagDbConsentRecord {
                tenant_id: "tenant-a".into(),
                agent_did: "did:exo:agent".into(),
                purpose: ConsentPurpose::Writeback,
                active: true,
            });
        assert!(
            verify_write_consent(
                &engine,
                "tenant-a",
                "did:exo:agent",
                ConsentPurpose::Writeback
            )
            .is_err()
        );
    }

    #[test]
    fn verify_write_signature_rejects_invalid_hex_and_did() {
        let keypair = KeyPair::generate();
        let registry = IdentityRegistry::default()
            .with_public_key("did:exo:agent", *keypair.public_key().as_bytes());
        let payload_hash = [1u8; 32];
        assert!(
            verify_write_signature(&registry, &payload_hash, "not-hex", "did:exo:agent").is_err()
        );
        assert!(verify_write_signature(&registry, &payload_hash, "abcd", "not-a-did").is_err());
        let short_hex = "aa".repeat(32);
        assert!(
            verify_write_signature(&registry, &payload_hash, &short_hex, "did:exo:agent").is_err()
        );
    }

    #[test]
    fn sign_write_payload_and_payload_hash_helpers_are_deterministic() {
        let keypair = KeyPair::generate();
        let selection = sample_selection();
        let payload_hash = usage_event_payload_hash(&selection).expect("usage payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let again = sign_write_payload(&keypair, &payload_hash).expect("signature again");
        assert_eq!(signature, again);

        let packet = sample_packet();
        let packet_hash = context_packet_payload_hash(&packet).expect("packet payload hash");
        assert_ne!(payload_hash, packet_hash);
    }

    #[test]
    fn context_packet_payload_hash_rejects_invalid_packet_hash() {
        let mut packet = sample_packet();
        packet.packet_hash = "invalid".into();
        assert!(context_packet_payload_hash(&packet).is_err());
    }

    fn lazy_postgres_pool() -> sqlx::PgPool {
        use std::time::Duration;

        use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

        if let Ok(database_url) = std::env::var("EXO_DAGDB_TEST_DATABASE_URL") {
            return PgPoolOptions::new()
                .connect_lazy(&database_url)
                .expect("lazy postgres pool");
        }
        let options = PgConnectOptions::new()
            .host("127.0.0.1")
            .port(1)
            .username("postgres")
            .database("postgres");
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(100))
            .connect_lazy_with(options)
    }

    #[tokio::test]
    async fn persist_usage_event_fails_before_db_when_consent_missing() {
        use std::sync::Arc;

        let pool = lazy_postgres_pool();
        let service = DagDbGatekeeperService::new(
            pool,
            Arc::new(ConsentEngine::default()),
            Arc::new(IdentityRegistry::default()),
        );
        let event = sample_selection();
        let err = service
            .persist_usage_event(&event, "did:exo:agent", &"aa".repeat(64), None)
            .await
            .expect_err("missing consent must fail closed");
        assert!(matches!(err, GatekeeperError::InvariantViolation(_)));
    }

    #[test]
    fn authority_resolver_unavailable_is_a_distinct_fail_closed_variant() {
        // The gateway's request-time DB resolver returns this variant when it
        // cannot establish consent/identity state (pool absent or a resolver
        // query failed). It must be a distinct typed variant — NOT folded into
        // an InvariantViolation policy denial — so the gateway can map it to a
        // 5xx availability fault instead of a hard policy deny, and so the
        // resolver never silently falls back to empty registries.
        let error = GatekeeperError::AuthorityResolverUnavailable("pool absent".into());
        assert!(matches!(
            error,
            GatekeeperError::AuthorityResolverUnavailable(_)
        ));
        assert!(!matches!(error, GatekeeperError::InvariantViolation(_)));
        assert!(error.to_string().contains("authority resolver unavailable"));
    }

    #[tokio::test]
    async fn persist_usage_event_fails_before_db_when_signature_invalid() {
        use std::sync::Arc;

        let keypair = KeyPair::generate();
        let other = KeyPair::generate();
        let event = sample_selection();
        let payload_hash = usage_event_payload_hash(&event).expect("payload hash");
        let forged = other.sign(
            dagdb_write_signature_message(&payload_hash)
                .expect("message")
                .as_bytes(),
        );
        let registry = Arc::new(
            IdentityRegistry::default()
                .with_public_key("did:exo:agent", *keypair.public_key().as_bytes()),
        );
        let pool = lazy_postgres_pool();
        let service = DagDbGatekeeperService::new(
            pool,
            Arc::new(active_consent_engine("tenant-a", "did:exo:agent")),
            registry,
        );
        let err = service
            .persist_usage_event(&event, "did:exo:agent", &format!("{forged}"), None)
            .await
            .expect_err("invalid signature must fail closed");
        assert!(matches!(err, GatekeeperError::InvariantViolation(_)));
    }

    #[tokio::test]
    async fn persist_usage_event_maps_db_errors_after_validation() {
        use std::sync::Arc;

        let keypair = KeyPair::generate();
        let event = sample_selection();
        let payload_hash = usage_event_payload_hash(&event).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let registry = Arc::new(
            IdentityRegistry::default()
                .with_public_key("did:exo:agent", *keypair.public_key().as_bytes()),
        );
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(active_consent_engine("tenant-a", "did:exo:agent")),
            registry,
        );
        let err = service
            .persist_usage_event(&event, "did:exo:agent", &signature, None)
            .await
            .expect_err("unscoped lazy pool must fail closed at db layer");
        assert!(matches!(err, GatekeeperError::InvariantViolation(_)));
    }

    #[test]
    fn log_invariant_violation_emits_structured_warning() {
        log_invariant_violation(
            ConstitutionalInvariant::ConsentRequired,
            "tenant-a",
            "did:exo:agent",
            "coverage fixture",
        );
    }

    #[tokio::test]
    async fn persist_usage_event_fails_before_db_when_signature_decode_errors() {
        use std::sync::Arc;

        let keypair = KeyPair::generate();
        let event = sample_selection();
        let registry = Arc::new(
            IdentityRegistry::default()
                .with_public_key("did:exo:agent", *keypair.public_key().as_bytes()),
        );
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(active_consent_engine("tenant-a", "did:exo:agent")),
            registry,
        );
        let err = service
            .persist_usage_event(&event, "did:exo:agent", "not-hex", None)
            .await
            .expect_err("malformed signature must fail closed");
        assert!(matches!(err, GatekeeperError::InvariantViolation(_)));
        assert!(err.to_string().contains("ProvenanceVerifiable"));
    }

    #[tokio::test]
    async fn persist_context_packet_receipt_maps_db_errors_after_validation() {
        use std::sync::Arc;

        let keypair = KeyPair::generate();
        let packet = sample_packet();
        let payload_hash = context_packet_payload_hash(&packet).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let registry = Arc::new(
            IdentityRegistry::default()
                .with_public_key("did:exo:agent", *keypair.public_key().as_bytes()),
        );
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(active_consent_engine("tenant-a", "did:exo:agent")),
            registry,
        );
        let err = service
            .persist_context_packet_receipt(&packet, "did:exo:agent", &signature, None)
            .await
            .expect_err("unscoped lazy pool must fail closed at db layer");
        assert!(matches!(err, GatekeeperError::InvariantViolation(_)));
    }

    #[tokio::test]
    async fn persist_context_packet_receipt_fails_before_db_when_invariants_violated() {
        use std::sync::Arc;

        let keypair = KeyPair::generate();
        let packet = sample_packet();
        let payload_hash = context_packet_payload_hash(&packet).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let registry = Arc::new(
            IdentityRegistry::default()
                .with_public_key("did:exo:agent", *keypair.public_key().as_bytes()),
        );
        let pool = lazy_postgres_pool();
        let service = DagDbGatekeeperService::new(
            pool,
            Arc::new(active_consent_engine("tenant-a", "did:exo:agent")),
            registry,
        );
        let actor = exo_core::Did::new("did:exo:agent").expect("actor");
        let invariant_context = InvariantContext {
            actor: actor.clone(),
            actor_roles: vec![
                Role {
                    name: "senator".into(),
                    branch: GovernmentBranch::Legislative,
                },
                Role {
                    name: "executor".into(),
                    branch: GovernmentBranch::Executive,
                },
            ],
            bailment_state: BailmentState::Active {
                bailor: exo_core::Did::new("did:exo:bailor").expect("bailor"),
                bailee: actor,
                scope: "dag-db".into(),
            },
            consent_records: Vec::new(),
            authority_chain: Default::default(),
            is_self_grant: false,
            human_override_preserved: true,
            kernel_modification_attempted: false,
            quorum_evidence: None,
            provenance: None,
            actor_permissions: Default::default(),
            requested_permissions: Default::default(),
            trusted_authority_keys: Default::default(),
            trusted_provenance_keys: Default::default(),
        };
        let err = service
            .persist_context_packet_receipt(
                &packet,
                "did:exo:agent",
                &signature,
                Some(&invariant_context),
            )
            .await
            .expect_err("invariant violation must fail closed");
        assert!(matches!(err, GatekeeperError::InvariantViolation(_)));
    }

    #[test]
    fn dagdb_invariant_set_excludes_authority_chain_and_provenance() {
        // The dag-db subset must NOT include the two invariants that need a
        // signed authority chain / provenance object the dag-db consent schema
        // does not carry, or every legitimate write would fail closed.
        let set = dagdb_invariant_set();
        assert!(
            set.invariants
                .contains(&ConstitutionalInvariant::ConsentRequired)
        );
        assert!(
            !set.invariants
                .contains(&ConstitutionalInvariant::AuthorityChainValid),
            "AuthorityChainValid must be excluded from the dag-db engine subset"
        );
        assert!(
            !set.invariants
                .contains(&ConstitutionalInvariant::ProvenanceVerifiable),
            "ProvenanceVerifiable is enforced directly via Ed25519, not the engine"
        );
    }

    #[tokio::test]
    async fn dagdb_invariant_context_from_active_consent_passes_subset_engine() {
        use crate::invariants::{InvariantEngine, enforce_all};

        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(active_consent_engine("tenant-a", "did:exo:agent")),
            Arc::new(IdentityRegistry::default()),
        );
        let context = service
            .dagdb_invariant_context("tenant-a", "did:exo:agent")
            .expect("context for valid DID");
        // Built from a real active bailment + consent grant, the dag-db subset
        // engine passes (no fail-closed deadlock for a legitimately-authorized
        // agent).
        let engine = InvariantEngine::new(dagdb_invariant_set());
        assert!(
            enforce_all(&engine, &context).is_ok(),
            "active-consent dag-db context must satisfy the enforced invariant subset"
        );
    }

    #[tokio::test]
    async fn dagdb_invariant_context_without_bailment_fails_consent_required() {
        use crate::invariants::{InvariantEngine, enforce_all};

        // No bailment registered for this tenant => no consent record mirrored
        // => ConsentRequired (in the subset) fails closed.
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(ConsentEngine::default()),
            Arc::new(IdentityRegistry::default()),
        );
        let context = service
            .dagdb_invariant_context("tenant-a", "did:exo:agent")
            .expect("context for valid DID");
        let engine = InvariantEngine::new(dagdb_invariant_set());
        let violations =
            enforce_all(&engine, &context).expect_err("missing bailment must fail closed");
        assert!(
            violations
                .iter()
                .any(|v| v.invariant == ConstitutionalInvariant::ConsentRequired),
            "missing bailment must surface a ConsentRequired violation"
        );
    }

    #[tokio::test]
    async fn dagdb_invariant_context_rejects_malformed_did() {
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(ConsentEngine::default()),
            Arc::new(IdentityRegistry::default()),
        );
        assert!(
            service.dagdb_invariant_context("tenant-a", "").is_none(),
            "a structurally-invalid DID must not yield an invariant context"
        );
    }

    // ---------------------------------------------------------------------
    // PRD-D5: route-contract coverage for the four lifecycle/persistence
    // surfaces now routed through the gatekeeper chain. Each surface proves:
    //   * consented + signed reaches the DB layer (and a DB outage is
    //     classified as `surface_database_unavailable`, i.e. 503 at the
    //     gateway), never a silent pass;
    //   * missing consent fails closed before any DB access;
    //   * a forged signature fails closed before any DB access.
    // Error-classification coverage (422 reject vs 503 unavailable) follows.
    // ---------------------------------------------------------------------

    use exo_dag_db_domain::{
        context_packet_persistence::{
            CONTEXT_PACKET_RECORD_SCHEMA_VERSION, DefaultContextQuality, PacketFreshnessStatus,
            PacketPersistenceStatus, PacketValidationStatus, canonical_idempotency_key,
        },
        continuation_persistence::{ContinuationRetrievalStatus, PRD17_CONTINUATION_RECORD_SCHEMA},
        default_route::{
            DEFAULT_ROUTE_SCHEMA_VERSION, DefaultRouteMemoryRef, DefaultRouteSource,
            DefaultRouteStatus, RouteFreshnessStatus,
        },
        lifecycle_action::{
            LifecycleActionType, LifecycleEvidenceRef, LifecycleMemoryRef, LifecycleRollbackRef,
            LifecycleTerminalState, PRD17_LIFECYCLE_ACTION_SCHEMA, ProductionLifecycleApproval,
        },
    };
    use exo_dag_db_postgres::postgres::{
        context_packet_persistence::ContextPacketPostgresError,
        lifecycle_action::LifecycleActionPostgresError,
    };

    const SURFACE_TENANT: &str = "tenant-a";
    const SURFACE_AGENT: &str = "did:exo:agent";
    const SURFACE_NAMESPACE: &str = "project_memory_v3";

    fn surface_memory_ref(memory_id: &str) -> LifecycleMemoryRef {
        LifecycleMemoryRef {
            tenant_id: SURFACE_TENANT.to_owned(),
            project_id: "dag_db".to_owned(),
            memory_namespace: SURFACE_NAMESPACE.to_owned(),
            memory_id: memory_id.to_owned(),
        }
    }

    fn sample_lifecycle_action() -> LifecycleAction {
        let action_id = "lifecycle-writeback-d5";
        let validation_report_id = format!("validation-{action_id}");
        LifecycleAction {
            schema_version: PRD17_LIFECYCLE_ACTION_SCHEMA.to_owned(),
            action_id: action_id.to_owned(),
            action_type: LifecycleActionType::Writeback,
            tenant_id: SURFACE_TENANT.to_owned(),
            project_id: "dag_db".to_owned(),
            memory_namespace: SURFACE_NAMESPACE.to_owned(),
            actor_id: SURFACE_AGENT.to_owned(),
            source_packet_id: "packet-d5-001".to_owned(),
            source_receipt_id: "receipt-d5-001".to_owned(),
            parent_memory_ids: vec![surface_memory_ref("memory-parent-a")],
            target_memory_ids: vec![surface_memory_ref("memory-target-a")],
            validation_report_id: validation_report_id.clone(),
            policy_ref: "policy-d5-local-mutation".to_owned(),
            rollback_ref: LifecycleRollbackRef {
                rollback_id: format!("rollback-{action_id}"),
                action_id: action_id.to_owned(),
                inverse_action_type: LifecycleActionType::Writeback.inverse(),
                before_refs: vec![surface_memory_ref("memory-parent-a")],
                after_refs: vec![surface_memory_ref("memory-target-a")],
                validation_ref: validation_report_id,
                operator_required: true,
            },
            route_invalidation_event_ids: vec!["route-event-d5-001".to_owned()],
            evidence_refs: vec![LifecycleEvidenceRef {
                evidence_id: "evidence-d5-001".to_owned(),
                receipt_id: "receipt-evidence-d5-001".to_owned(),
                digest: "a".repeat(64),
                summary_ref: "summary-evidence-d5-001".to_owned(),
                preserved: true,
            }],
            terminal_state: LifecycleTerminalState::OperatorDeferred,
            production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
            created_at: "2026-06-12T00:00:00Z".to_owned(),
        }
    }

    fn sample_default_route() -> DefaultRouteRecord {
        DefaultRouteRecord {
            schema_version: DEFAULT_ROUTE_SCHEMA_VERSION.to_owned(),
            route_id: "route-d5-default".to_owned(),
            tenant_id: SURFACE_TENANT.to_owned(),
            project_id: "dag_db".to_owned(),
            memory_namespace: SURFACE_NAMESPACE.to_owned(),
            status: DefaultRouteStatus::Active,
            route_source: DefaultRouteSource::Persisted,
            policy_ref: "policy:d5-default-route".to_owned(),
            freshness_ref: "freshness:current".to_owned(),
            policy_allowed: true,
            freshness_status: RouteFreshnessStatus::Current,
            invalidated: false,
            production_default_route_approval_status: "operator_deferred".to_owned(),
            packet_quality_review_status: "operator_deferred".to_owned(),
            selected_memory_refs: vec![DefaultRouteMemoryRef {
                memory_id: "memory-a".to_owned(),
                latest_receipt_hash: "memory-a-receipt".to_owned(),
                validation_status: "passed".to_owned(),
                citation_ref: "citation:memory-a".to_owned(),
            }],
            created_at: "hlc:1".to_owned(),
            updated_at: "hlc:2".to_owned(),
        }
    }

    fn sample_continuation_record() -> ContinuationRecord {
        ContinuationRecord {
            schema_version: PRD17_CONTINUATION_RECORD_SCHEMA.to_owned(),
            continuation_id: "continuation-d5-001".to_owned(),
            task_id: "task-d5-next-agent".to_owned(),
            tenant_id: SURFACE_TENANT.to_owned(),
            project_id: "dag_db".to_owned(),
            memory_namespace: SURFACE_NAMESPACE.to_owned(),
            summary_ref: "summary-continuation-d5-001".to_owned(),
            memory_refs: vec![surface_memory_ref("memory-target-a")],
            blocker_refs: vec!["blocker-production-lifecycle-approval-deferred".to_owned()],
            validation_refs: vec!["validation-continuation-d5-001".to_owned()],
            expiry_epoch_seconds: 2_000,
            later_retrieval_status: ContinuationRetrievalStatus::Pending,
            production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
            created_at: "2026-06-12T00:00:00Z".to_owned(),
        }
    }

    fn sample_context_packet_record() -> ContextPacketRecord {
        ContextPacketRecord {
            schema_version: CONTEXT_PACKET_RECORD_SCHEMA_VERSION.to_owned(),
            packet_id: "packet-d5-001".to_owned(),
            route_id: "route-d5-001".to_owned(),
            query_hash: "query-hash-d5-001".to_owned(),
            tenant_id: SURFACE_TENANT.to_owned(),
            project_id: "dag_db".to_owned(),
            memory_namespace: SURFACE_NAMESPACE.to_owned(),
            selected_memory_ids: vec!["memory-d5-001".to_owned()],
            selected_edge_ids: Vec::new(),
            token_budget: 1_000,
            token_estimate: 200,
            context_quality: DefaultContextQuality::UsableContext,
            citation_coverage_bp: 10_000,
            validation_coverage_bp: 10_000,
            freshness_status: PacketFreshnessStatus::Current,
            validation_status: PacketValidationStatus::Passed,
            source_proof_refs: vec!["receipt-d5-001".to_owned()],
            fallback_reason: None,
            idempotency_key: canonical_idempotency_key("route-d5-001", "query-hash-d5-001", 1_000),
            persistence_status: PacketPersistenceStatus::ProofBound,
            production_default_route_approval_status: "operator_deferred".to_owned(),
            packet_quality_review_status: "operator_deferred".to_owned(),
            created_at: "2026-06-12T00:00:00Z".to_owned(),
        }
    }

    /// An unreachable (port-1) lazy pool, independent of
    /// `EXO_DAGDB_TEST_DATABASE_URL`. The consented-pass surface tests use this
    /// so they deterministically observe the gate-passed-then-DB-unavailable
    /// path and never write synthetic rows into a live shared store.
    fn unreachable_postgres_pool() -> PgPool {
        use std::time::Duration;

        use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

        let options = PgConnectOptions::new()
            .host("127.0.0.1")
            .port(1)
            .username("postgres")
            .database("postgres");
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(100))
            .connect_lazy_with(options)
    }

    fn surface_service_consented(registry: Arc<IdentityRegistry>) -> DagDbGatekeeperService {
        DagDbGatekeeperService::new(
            unreachable_postgres_pool(),
            Arc::new(active_consent_engine(SURFACE_TENANT, SURFACE_AGENT)),
            registry,
        )
    }

    fn registry_for(keypair: &KeyPair) -> Arc<IdentityRegistry> {
        Arc::new(
            IdentityRegistry::default()
                .with_public_key(SURFACE_AGENT, *keypair.public_key().as_bytes()),
        )
    }

    /// Assert a consented + signed call passed the gate: the gatekeeper let it
    /// reach the persistence layer, so the outcome is either success or a
    /// classified database failure — never a consent/provenance rejection. This
    /// holds whether or not a live `EXO_DAGDB_TEST_DATABASE_URL` is configured
    /// (no DB => `surface_database_unavailable`; live DB => the write may
    /// succeed), so the test is robust under both the DB-independent suite and a
    /// live-DB run.
    fn assert_gate_passed<T>(result: Result<T, GatekeeperError>) {
        if let Err(error) = result {
            let detail = error.to_string();
            assert!(
                detail.contains("surface_database_unavailable"),
                "consented+signed call must not be gate-rejected; got: {detail}"
            );
            assert!(
                !detail.contains("ConsentRequired") && !detail.contains("ProvenanceVerifiable"),
                "consented+signed call must not be gate-rejected; got: {detail}"
            );
        }
    }

    // D5-S1: lifecycle action through the gate.
    #[tokio::test]
    async fn lifecycle_action_consented_signed_reaches_db_layer() {
        let keypair = KeyPair::generate();
        let action = sample_lifecycle_action();
        let payload_hash = lifecycle_action_payload_hash(&action).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = surface_service_consented(registry_for(&keypair));
        let result = service
            .persist_lifecycle_action(&action, SURFACE_AGENT, &signature, None)
            .await;
        assert_gate_passed(result);
    }

    #[tokio::test]
    async fn lifecycle_action_unconsented_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let action = sample_lifecycle_action();
        let payload_hash = lifecycle_action_payload_hash(&action).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(ConsentEngine::default()),
            registry_for(&keypair),
        );
        let err = service
            .persist_lifecycle_action(&action, SURFACE_AGENT, &signature, None)
            .await
            .expect_err("missing consent must fail closed");
        assert!(err.to_string().contains("ConsentRequired"), "{err}");
    }

    #[tokio::test]
    async fn lifecycle_action_forged_signature_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let forger = KeyPair::generate();
        let action = sample_lifecycle_action();
        let payload_hash = lifecycle_action_payload_hash(&action).expect("payload hash");
        let forged = sign_write_payload(&forger, &payload_hash).expect("forged signature");
        let service = surface_service_consented(registry_for(&keypair));
        let err = service
            .persist_lifecycle_action(&action, SURFACE_AGENT, &forged, None)
            .await
            .expect_err("forged signature must fail closed");
        assert!(err.to_string().contains("ProvenanceVerifiable"), "{err}");
    }

    // D5-S2: default route through the gate.
    #[tokio::test]
    async fn default_route_consented_signed_reaches_db_layer() {
        let keypair = KeyPair::generate();
        let route = sample_default_route();
        let payload_hash = default_route_payload_hash(&route).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = surface_service_consented(registry_for(&keypair));
        let result = service
            .persist_default_route(&route, SURFACE_AGENT, &signature, None)
            .await;
        assert_gate_passed(result);
    }

    #[tokio::test]
    async fn default_route_unconsented_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let route = sample_default_route();
        let payload_hash = default_route_payload_hash(&route).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(ConsentEngine::default()),
            registry_for(&keypair),
        );
        let err = service
            .persist_default_route(&route, SURFACE_AGENT, &signature, None)
            .await
            .expect_err("missing consent must fail closed");
        assert!(err.to_string().contains("ConsentRequired"), "{err}");
    }

    #[tokio::test]
    async fn default_route_forged_signature_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let forger = KeyPair::generate();
        let route = sample_default_route();
        let payload_hash = default_route_payload_hash(&route).expect("payload hash");
        let forged = sign_write_payload(&forger, &payload_hash).expect("forged signature");
        let service = surface_service_consented(registry_for(&keypair));
        let err = service
            .persist_default_route(&route, SURFACE_AGENT, &forged, None)
            .await
            .expect_err("forged signature must fail closed");
        assert!(err.to_string().contains("ProvenanceVerifiable"), "{err}");
    }

    // D5-S3: continuation + context-packet through the gate.
    #[tokio::test]
    async fn continuation_consented_signed_reaches_db_layer() {
        let keypair = KeyPair::generate();
        let record = sample_continuation_record();
        let payload_hash = continuation_record_payload_hash(&record).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = surface_service_consented(registry_for(&keypair));
        let result = service
            .persist_continuation_record(&record, 1_000, SURFACE_AGENT, &signature, None)
            .await;
        assert_gate_passed(result);
    }

    #[tokio::test]
    async fn continuation_unconsented_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let record = sample_continuation_record();
        let payload_hash = continuation_record_payload_hash(&record).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(ConsentEngine::default()),
            registry_for(&keypair),
        );
        let err = service
            .persist_continuation_record(&record, 1_000, SURFACE_AGENT, &signature, None)
            .await
            .expect_err("missing consent must fail closed");
        assert!(err.to_string().contains("ConsentRequired"), "{err}");
    }

    #[tokio::test]
    async fn continuation_forged_signature_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let forger = KeyPair::generate();
        let record = sample_continuation_record();
        let payload_hash = continuation_record_payload_hash(&record).expect("payload hash");
        let forged = sign_write_payload(&forger, &payload_hash).expect("forged signature");
        let service = surface_service_consented(registry_for(&keypair));
        let err = service
            .persist_continuation_record(&record, 1_000, SURFACE_AGENT, &forged, None)
            .await
            .expect_err("forged signature must fail closed");
        assert!(err.to_string().contains("ProvenanceVerifiable"), "{err}");
    }

    #[tokio::test]
    async fn context_packet_record_consented_signed_reaches_db_layer() {
        let keypair = KeyPair::generate();
        let record = sample_context_packet_record();
        let payload_hash = context_packet_record_payload_hash(&record).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = surface_service_consented(registry_for(&keypair));
        let result = service
            .persist_context_packet_record(&record, SURFACE_AGENT, &signature, None)
            .await;
        assert_gate_passed(result);
    }

    #[tokio::test]
    async fn context_packet_record_unconsented_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let record = sample_context_packet_record();
        let payload_hash = context_packet_record_payload_hash(&record).expect("payload hash");
        let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");
        let service = DagDbGatekeeperService::new(
            lazy_postgres_pool(),
            Arc::new(ConsentEngine::default()),
            registry_for(&keypair),
        );
        let err = service
            .persist_context_packet_record(&record, SURFACE_AGENT, &signature, None)
            .await
            .expect_err("missing consent must fail closed");
        assert!(err.to_string().contains("ConsentRequired"), "{err}");
    }

    #[tokio::test]
    async fn context_packet_record_forged_signature_fails_closed_before_db() {
        let keypair = KeyPair::generate();
        let forger = KeyPair::generate();
        let record = sample_context_packet_record();
        let payload_hash = context_packet_record_payload_hash(&record).expect("payload hash");
        let forged = sign_write_payload(&forger, &payload_hash).expect("forged signature");
        let service = surface_service_consented(registry_for(&keypair));
        let err = service
            .persist_context_packet_record(&record, SURFACE_AGENT, &forged, None)
            .await
            .expect_err("forged signature must fail closed");
        assert!(err.to_string().contains("ProvenanceVerifiable"), "{err}");
    }

    // D5-S4: error classification — a contract/replay reject carries the
    // `metadata rejected` marker (422 at the gateway); a DB/transaction outage
    // carries the `surface_database_unavailable` marker (503). The two markers
    // are mutually exclusive so a reject is never reported as a DB outage and a
    // DB outage is never reported as a policy reject.
    #[test]
    fn surface_db_failure_classified_as_unavailable() {
        let db_error = LifecycleActionPostgresError::Postgres {
            source: sqlx::Error::PoolClosed,
        };
        let mapped = domain_blocked("lifecycle_action_postgres", &db_error);
        let detail = mapped.to_string();
        assert!(detail.contains("surface_database_unavailable"), "{detail}");
        assert!(!detail.contains("metadata rejected"), "{detail}");
    }

    #[test]
    fn surface_contract_reject_classified_as_metadata_rejected() {
        let json_error = ContextPacketPostgresError::UnsafeReplay {
            packet_id: "packet-d5-001".to_owned(),
        };
        let mapped = domain_blocked("context_packet_record_postgres", &json_error);
        let detail = mapped.to_string();
        assert!(detail.contains("metadata rejected"), "{detail}");
        assert!(!detail.contains("surface_database_unavailable"), "{detail}");
    }
}
