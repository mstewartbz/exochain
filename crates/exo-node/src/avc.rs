// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Node API surface for Autonomous Volition Credentials.
//!
//! Routes are merged into the gateway's axum router via the same
//! `serve_with_extra_routes` pattern used by governance, passport, and
//! provenance. Mutating endpoints (`POST`) inherit bearer-token auth
//! from the merged write guard; ordinary AVC denials return HTTP `200`
//! with `decision: Deny` rather than `403` so callers see structured
//! reason codes.
//!
//! ## Routes
//!
//! | Method | Path | Purpose |
//! |--------|------|---------|
//! | `POST` | `/api/v1/avc/issue` | Register a signed credential. |
//! | `POST` | `/api/v1/avc/validate` | Validate a credential and optional action. |
//! | `POST` | `/api/v1/avc/receipts/emit` | Validate a subject-signed action and mint a node-signed receipt. |
//! | `POST` | `/api/v1/avc/delegate` | Register a signed child credential. |
//! | `POST` | `/api/v1/avc/revoke` | Register a signed revocation. |
//! | `GET`  | `/api/v1/avc/:id` | Fetch a registered credential by hex ID. |
//! | `GET`  | `/api/v1/agents/:did/avcs` | List credentials for a subject DID. |
//!
//! ## State
//!
//! The handlers operate against a shared [`AvcApiState`] that wraps an
//! `InMemoryAvcRegistry` behind `Arc<Mutex<_>>`. All synchronous store
//! access is performed inside `tokio::task::spawn_blocking` so async
//! workers are not held under the registry lock.

use std::{
    path::Path as FsPath,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    routing::{get, post},
};
use exo_avc::{
    AutonomousVolitionCredential, AvcActionRequest, AvcDecision, AvcRegistryRead, AvcRegistryWrite,
    AvcRevocation, AvcTrustReceipt, AvcValidationRequest, AvcValidationResult, InMemoryAvcRegistry,
    avc_action_signature_payload, create_trust_receipt, validate_avc,
};
use exo_core::{Did, Hash256, PublicKey, Signature, crypto};
use exo_root::{RootTrustBundle, verify_root_bundle};
use serde::{Deserialize, Serialize};
use tower::limit::ConcurrencyLimitLayer;

const MAX_AVC_API_BODY_BYTES: usize = 64 * 1024;
const MAX_AVC_API_CONCURRENT_REQUESTS: usize = 64;
pub const AVC_ROOT_TRUST_BUNDLE_ENV: &str = "EXO_AVC_ROOT_TRUST_BUNDLE";
pub const AVC_ROOT_TRUST_CEREMONY_ID: &str = "avc-exo-ceremony-2026";
pub const AVC_ROOT_TRUST_BUNDLE_ID_HEX: &str =
    "7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58";
pub const AVC_ROOT_TRUST_ISSUER_DID: &str = "did:exo:8EVGmqLo15JEnrbcrLo9r84qX1mtrVeBdPjHLUtb1sXX";
pub const AVC_ROOT_TRUST_ISSUER_PUBLIC_KEY_HEX: &str =
    "6b765381964de7f74e77e4f9d265105f415e58722d19ff71603f62c31d5aff32";

pub type AvcReceiptSigner = Arc<dyn Fn(&[u8]) -> Signature + Send + Sync>;

/// Shared state for AVC route handlers.
#[derive(Clone)]
pub struct AvcApiState {
    pub registry: Arc<Mutex<InMemoryAvcRegistry>>,
    validator_did: Did,
    receipt_signer: AvcReceiptSigner,
}

impl AvcApiState {
    /// Wrap a fresh registry in the standard `Arc<Mutex<_>>` envelope.
    #[must_use]
    pub fn new(validator_did: Did, receipt_signer: AvcReceiptSigner) -> Self {
        Self {
            registry: Arc::new(Mutex::new(InMemoryAvcRegistry::new())),
            validator_did,
            receipt_signer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootTrustIssuerRegistration {
    pub bundle_id: Hash256,
    pub ceremony_id: String,
    pub issuer_did: Did,
    pub issuer_public_key: PublicKey,
}

fn parse_expected_hash(hex_value: &str, label: &str) -> anyhow::Result<Hash256> {
    let bytes = hex::decode(hex_value)
        .map_err(|error| anyhow::anyhow!("invalid {label} hex constant: {error}"))?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "invalid {label} hex constant: expected 32 bytes, got {}",
            bytes.len()
        );
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    Ok(Hash256::from_bytes(buf))
}

fn parse_expected_public_key(hex_value: &str, label: &str) -> anyhow::Result<PublicKey> {
    let bytes = hex::decode(hex_value)
        .map_err(|error| anyhow::anyhow!("invalid {label} hex constant: {error}"))?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "invalid {label} hex constant: expected 32 bytes, got {}",
            bytes.len()
        );
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    Ok(PublicKey::from_bytes(buf))
}

/// Load the configured AVC root trust bundle, verify it in-process, and
/// register the delegated operational issuer public key.
///
/// If `EXO_AVC_ROOT_TRUST_BUNDLE` is absent, no registration is performed.
/// If it is present, any read, parse, verification, or expected-identity
/// mismatch error is fatal to preserve fail-closed production startup.
pub fn load_configured_root_trust_bundle(
    state: &AvcApiState,
) -> anyhow::Result<Option<RootTrustIssuerRegistration>> {
    let Some(path) = std::env::var_os(AVC_ROOT_TRUST_BUNDLE_ENV) else {
        return Ok(None);
    };
    if path.is_empty() {
        anyhow::bail!("{AVC_ROOT_TRUST_BUNDLE_ENV} is set but empty");
    }
    load_root_trust_bundle_from_path(state, FsPath::new(&path)).map(Some)
}

pub fn load_root_trust_bundle_from_path(
    state: &AvcApiState,
    path: &FsPath,
) -> anyhow::Result<RootTrustIssuerRegistration> {
    let bytes = std::fs::read(path).map_err(|error| {
        anyhow::anyhow!(
            "failed to read AVC root trust bundle at {}: {error}",
            path.display()
        )
    })?;
    let bundle: RootTrustBundle = serde_json::from_slice(&bytes).map_err(|error| {
        anyhow::anyhow!(
            "failed to parse AVC root trust bundle at {}: {error}",
            path.display()
        )
    })?;
    verify_root_bundle(&bundle).map_err(|error| {
        anyhow::anyhow!(
            "AVC root trust bundle verification failed for {}: {error}",
            path.display()
        )
    })?;

    if bundle.config.ceremony_id != AVC_ROOT_TRUST_CEREMONY_ID {
        anyhow::bail!(
            "AVC root trust bundle ceremony mismatch: expected {}, got {}",
            AVC_ROOT_TRUST_CEREMONY_ID,
            bundle.config.ceremony_id
        );
    }

    let expected_bundle_id =
        parse_expected_hash(AVC_ROOT_TRUST_BUNDLE_ID_HEX, "AVC root trust bundle id")?;
    if bundle.bundle_id != expected_bundle_id {
        anyhow::bail!(
            "AVC root trust bundle id mismatch: expected {}, got {}",
            expected_bundle_id,
            bundle.bundle_id
        );
    }

    let expected_issuer_did = Did::new(AVC_ROOT_TRUST_ISSUER_DID)
        .map_err(|error| anyhow::anyhow!("invalid AVC root trust issuer DID constant: {error}"))?;
    if bundle.issuer_delegation.issuer_did != expected_issuer_did {
        anyhow::bail!(
            "AVC root trust issuer DID mismatch: expected {}, got {}",
            expected_issuer_did,
            bundle.issuer_delegation.issuer_did
        );
    }

    let expected_public_key = parse_expected_public_key(
        AVC_ROOT_TRUST_ISSUER_PUBLIC_KEY_HEX,
        "AVC root trust issuer public key",
    )?;
    if bundle.issuer_delegation.issuer_public_key != expected_public_key {
        anyhow::bail!(
            "AVC root trust issuer public key mismatch for {}",
            expected_issuer_did
        );
    }

    let registration = RootTrustIssuerRegistration {
        bundle_id: bundle.bundle_id,
        ceremony_id: bundle.config.ceremony_id.clone(),
        issuer_did: bundle.issuer_delegation.issuer_did.clone(),
        issuer_public_key: bundle.issuer_delegation.issuer_public_key,
    };

    let mut registry = state.registry.lock().map_err(|_| {
        anyhow::anyhow!("AVC registry unavailable while registering root trust issuer")
    })?;
    registry.put_public_key(
        registration.issuer_did.clone(),
        registration.issuer_public_key,
    );

    Ok(registration)
}

// ---------------------------------------------------------------------------
// Request / Response shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueRequest {
    pub credential: AutonomousVolitionCredential,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueResponse {
    pub credential_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelegateRequest {
    pub child_credential: AutonomousVolitionCredential,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelegateResponse {
    pub credential_id: String,
    pub parent_avc_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeRequest {
    pub revocation: AvcRevocation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeResponse {
    pub credential_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmitReceiptRequest {
    pub validation: AvcValidationRequest,
    pub subject_signature: Signature,
    /// Optional subject public key for did:exo values derived from a key.
    /// If the registry already has a trusted key for the actor DID, that
    /// registered key wins and this field is ignored.
    pub subject_public_key: Option<PublicKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmitReceiptResponse {
    pub receipt_hash: String,
    pub receipt: AvcTrustReceipt,
    pub validation: AvcValidationResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvcSummary {
    pub credential_id: String,
    pub subject_did: String,
    pub issuer_did: String,
    pub principal_did: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListAvcResponse {
    pub did: String,
    pub credentials: Vec<AvcSummary>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, String);
type ApiResult<T> = Result<T, ApiError>;

fn parse_did(raw: &str) -> ApiResult<Did> {
    Did::new(raw).map_err(|err| {
        tracing::warn!(%err, "rejected malformed AVC DID");
        (StatusCode::BAD_REQUEST, "Invalid DID".into())
    })
}

fn parse_hash(raw: &str) -> ApiResult<Hash256> {
    let bytes = hex::decode(raw).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "credential id must be lowercase hex".into(),
        )
    })?;
    if bytes.len() != 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            "credential id must be 32 bytes (64 hex chars)".into(),
        ));
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    Ok(Hash256::from_bytes(buf))
}

async fn with_registry_blocking<T, F>(state: Arc<AvcApiState>, op: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce(&mut InMemoryAvcRegistry) -> ApiResult<T> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut guard = state.registry.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "AVC registry unavailable".into(),
            )
        })?;
        op(&mut guard)
    })
    .await
    .map_err(|err| {
        tracing::error!(err = %err, "AVC registry task failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AVC registry task failed".into(),
        )
    })?
}

fn map_avc_error(err: exo_avc::AvcError) -> ApiError {
    tracing::warn!(?err, "AVC operation rejected");
    match err {
        exo_avc::AvcError::EmptyField { .. }
        | exo_avc::AvcError::UnsupportedSchema { .. }
        | exo_avc::AvcError::BasisPointOutOfRange { .. }
        | exo_avc::AvcError::InvalidTimestamp { .. }
        | exo_avc::AvcError::DelegationWidens { .. }
        | exo_avc::AvcError::DelegationRejected { .. }
        | exo_avc::AvcError::InvalidInput { .. } => (StatusCode::BAD_REQUEST, err.to_string()),
        exo_avc::AvcError::Registry { .. } | exo_avc::AvcError::Serialization { .. } => {
            (StatusCode::INTERNAL_SERVER_ERROR, "AVC error".into())
        }
    }
}

fn summary_of(credential: &AutonomousVolitionCredential) -> ApiResult<AvcSummary> {
    let id = credential.id().map_err(map_avc_error)?;
    Ok(AvcSummary {
        credential_id: format!("{id}"),
        subject_did: credential.subject_did.to_string(),
        issuer_did: credential.issuer_did.to_string(),
        principal_did: credential.principal_did.to_string(),
    })
}

fn require_action(request: &AvcValidationRequest) -> ApiResult<&AvcActionRequest> {
    request.action.as_ref().ok_or((
        StatusCode::BAD_REQUEST,
        "receipt emission requires an action".into(),
    ))
}

fn resolve_subject_public_key(
    registry: &InMemoryAvcRegistry,
    action: &AvcActionRequest,
    supplied_public_key: Option<PublicKey>,
) -> ApiResult<PublicKey> {
    if let Some(public_key) = registry.resolve_public_key(&action.actor_did) {
        return Ok(public_key);
    }

    let Some(public_key) = supplied_public_key else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "subject public key is unresolved".into(),
        ));
    };
    let derived_did = crate::identity::did_from_public_key(&public_key).map_err(|err| {
        tracing::warn!(%err, "rejected AVC receipt action public key");
        (
            StatusCode::UNAUTHORIZED,
            "subject public key is not a valid did:exo actor key".into(),
        )
    })?;
    if derived_did != action.actor_did {
        return Err((
            StatusCode::UNAUTHORIZED,
            "subject public key does not match action actor DID".into(),
        ));
    }
    Ok(public_key)
}

fn verify_subject_action_signature(
    registry: &InMemoryAvcRegistry,
    request: &AvcValidationRequest,
    subject_signature: &Signature,
    subject_public_key: Option<PublicKey>,
) -> ApiResult<()> {
    if subject_signature.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "subject action signature must not be empty".into(),
        ));
    }
    let action = require_action(request)?;
    let public_key = resolve_subject_public_key(registry, action, subject_public_key)?;
    let payload = avc_action_signature_payload(&request.credential, action, &request.now)
        .map_err(map_avc_error)?;
    if !crypto::verify(&payload, subject_signature, &public_key) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "subject action signature is invalid".into(),
        ));
    }
    Ok(())
}

fn require_registered_credential(
    registry: &InMemoryAvcRegistry,
    request: &AvcValidationRequest,
) -> ApiResult<Hash256> {
    let credential_id = request.credential.id().map_err(map_avc_error)?;
    let registered = registry
        .get_credential(&credential_id)
        .ok_or((StatusCode::NOT_FOUND, "credential is not registered".into()))?;
    if registered != request.credential {
        return Err((
            StatusCode::BAD_REQUEST,
            "credential does not match registered AVC".into(),
        ));
    }
    Ok(credential_id)
}

fn store_receipt_idempotent(
    registry: &mut InMemoryAvcRegistry,
    receipt: AvcTrustReceipt,
) -> ApiResult<()> {
    let receipt_id = receipt.receipt_id;
    match registry.put_receipt(receipt.clone()) {
        Ok(()) => Ok(()),
        Err(exo_avc::AvcError::Registry { .. })
            if registry.get_receipt(&receipt_id).as_ref() == Some(&receipt) =>
        {
            Ok(())
        }
        Err(err) => Err(map_avc_error(err)),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_issue(
    State(state): State<Arc<AvcApiState>>,
    Json(payload): Json<IssueRequest>,
) -> ApiResult<Json<IssueResponse>> {
    let credential = payload.credential;
    let id = with_registry_blocking(state, move |registry| {
        registry.put_credential(credential).map_err(map_avc_error)
    })
    .await?;
    Ok(Json(IssueResponse {
        credential_id: format!("{id}"),
        status: "registered".into(),
    }))
}

async fn handle_validate(
    State(state): State<Arc<AvcApiState>>,
    Json(request): Json<AvcValidationRequest>,
) -> ApiResult<Json<AvcValidationResult>> {
    let result = with_registry_blocking(state, move |registry| {
        validate_avc(&request, registry).map_err(map_avc_error)
    })
    .await?;
    Ok(Json(result))
}

async fn handle_emit_receipt(
    State(state): State<Arc<AvcApiState>>,
    Json(payload): Json<EmitReceiptRequest>,
) -> ApiResult<Json<EmitReceiptResponse>> {
    let validator_did = state.validator_did.clone();
    let receipt_signer = Arc::clone(&state.receipt_signer);
    let response = with_registry_blocking(state, move |registry| {
        let request = payload.validation;
        let action_id = require_action(&request)?.action_id;
        require_registered_credential(registry, &request)?;
        verify_subject_action_signature(
            registry,
            &request,
            &payload.subject_signature,
            payload.subject_public_key,
        )?;
        let validation = validate_avc(&request, registry).map_err(map_avc_error)?;
        if validation.decision != AvcDecision::Allow {
            return Err((
                StatusCode::FORBIDDEN,
                format!("AVC validation denied: {:?}", validation.reason_codes),
            ));
        }
        let receipt = create_trust_receipt(
            &validation,
            Some(action_id),
            validator_did,
            request.now,
            |bytes| (receipt_signer)(bytes),
        )
        .map_err(map_avc_error)?;
        store_receipt_idempotent(registry, receipt.clone())?;
        Ok(EmitReceiptResponse {
            receipt_hash: format!("{}", receipt.receipt_id),
            receipt,
            validation,
        })
    })
    .await?;
    Ok(Json(response))
}

async fn handle_delegate(
    State(state): State<Arc<AvcApiState>>,
    Json(payload): Json<DelegateRequest>,
) -> ApiResult<Json<DelegateResponse>> {
    let credential = payload.child_credential;
    let parent_avc_id = credential.parent_avc_id.map(|h| format!("{h}"));
    let id = with_registry_blocking(state, move |registry| {
        registry.put_credential(credential).map_err(map_avc_error)
    })
    .await?;
    Ok(Json(DelegateResponse {
        credential_id: format!("{id}"),
        parent_avc_id,
        status: "registered".into(),
    }))
}

async fn handle_revoke(
    State(state): State<Arc<AvcApiState>>,
    Json(payload): Json<RevokeRequest>,
) -> ApiResult<Json<RevokeResponse>> {
    let revocation = payload.revocation;
    let id = revocation.credential_id;
    with_registry_blocking(state, move |registry| {
        registry.put_revocation(revocation).map_err(map_avc_error)
    })
    .await?;
    Ok(Json(RevokeResponse {
        credential_id: format!("{id}"),
        status: "revoked".into(),
    }))
}

async fn handle_get(
    State(state): State<Arc<AvcApiState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<AutonomousVolitionCredential>> {
    let hash = parse_hash(&id)?;
    let credential = with_registry_blocking(state, move |registry| {
        registry
            .get_credential(&hash)
            .ok_or((StatusCode::NOT_FOUND, "credential not found".into()))
    })
    .await?;
    Ok(Json(credential))
}

async fn handle_list_for_subject(
    State(state): State<Arc<AvcApiState>>,
    Path(did_str): Path<String>,
) -> ApiResult<Json<ListAvcResponse>> {
    let did = parse_did(&did_str)?;
    let did_for_response = did.to_string();
    let did_for_lookup = did.clone();
    let credentials = with_registry_blocking(state, move |registry| {
        let creds = registry.list_credentials_for_subject(&did_for_lookup);
        let mut summaries = Vec::with_capacity(creds.len());
        for c in &creds {
            summaries.push(summary_of(c)?);
        }
        // Deterministic ordering by credential ID.
        summaries.sort_by(|a, b| a.credential_id.cmp(&b.credential_id));
        Ok(summaries)
    })
    .await?;
    Ok(Json(ListAvcResponse {
        did: did_for_response,
        credentials,
    }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the AVC API router. POST routes inherit bearer-token auth
/// from the merged write guard in `main.rs`. The validation route returns
/// ordinary AVC denials as `200 OK` with `decision: Deny`; receipt emission
/// returns `403` because no receipt is minted for a denied action.
pub fn avc_router(state: Arc<AvcApiState>) -> Router {
    Router::new()
        .route("/api/v1/avc/issue", post(handle_issue))
        .route("/api/v1/avc/validate", post(handle_validate))
        .route("/api/v1/avc/receipts/emit", post(handle_emit_receipt))
        .route("/api/v1/avc/delegate", post(handle_delegate))
        .route("/api/v1/avc/revoke", post(handle_revoke))
        .route("/api/v1/avc/:id", get(handle_get))
        .route("/api/v1/agents/:did/avcs", get(handle_list_for_subject))
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_AVC_API_BODY_BYTES))
        .layer(ConcurrencyLimitLayer::new(MAX_AVC_API_CONCURRENT_REQUESTS))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::{
        body::{self, Body},
        http::{Method, Request},
    };
    use exo_authority::permission::Permission;
    use exo_avc::{
        AVC_SCHEMA_VERSION, AuthorityScope, AutonomyLevel, AvcActionRequest, AvcConstraints,
        AvcDecision, AvcDraft, AvcReasonCode, AvcRevocationReason, AvcSubjectKind, DelegatedIntent,
        issue_avc, revoke_avc,
    };
    use exo_core::{Hash256, Signature, Timestamp, crypto, crypto::KeyPair};
    use tower::ServiceExt;

    use super::*;

    const ISSUER_SEED: [u8; 32] = [0x11; 32];
    const SUBJECT_SEED: [u8; 32] = [0x22; 32];
    const VALIDATOR_SEED: [u8; 32] = [0x33; 32];

    fn issuer_keypair() -> KeyPair {
        KeyPair::from_secret_bytes(ISSUER_SEED).expect("valid seed")
    }

    fn subject_keypair() -> KeyPair {
        KeyPair::from_secret_bytes(SUBJECT_SEED).expect("valid seed")
    }

    fn validator_keypair() -> KeyPair {
        KeyPair::from_secret_bytes(VALIDATOR_SEED).expect("valid seed")
    }

    fn validator_did() -> Did {
        Did::new("did:exo:validator").unwrap()
    }

    fn fresh_state() -> Arc<AvcApiState> {
        let signer: AvcReceiptSigner = Arc::new(|payload: &[u8]| validator_keypair().sign(payload));
        let state = AvcApiState::new(validator_did(), signer);
        // Seed the issuer key so validate paths succeed.
        let kp = issuer_keypair();
        let did = Did::new("did:exo:issuer").unwrap();
        state
            .registry
            .lock()
            .unwrap()
            .put_public_key(did, kp.public);
        state
            .registry
            .lock()
            .unwrap()
            .put_public_key(Did::new("did:exo:agent").unwrap(), subject_keypair().public);
        Arc::new(state)
    }

    fn baseline_draft() -> AvcDraft {
        AvcDraft {
            schema_version: AVC_SCHEMA_VERSION,
            issuer_did: Did::new("did:exo:issuer").unwrap(),
            principal_did: Did::new("did:exo:issuer").unwrap(),
            subject_did: Did::new("did:exo:agent").unwrap(),
            holder_did: None,
            subject_kind: AvcSubjectKind::AiAgent {
                model_id: "alpha".into(),
                agent_version: None,
            },
            created_at: Timestamp::new(1_000_000, 0),
            expires_at: Some(Timestamp::new(2_000_000, 0)),
            delegated_intent: DelegatedIntent {
                intent_id: Hash256::from_bytes([0xAA; 32]),
                purpose: "research".into(),
                allowed_objectives: vec!["primary".into()],
                prohibited_objectives: vec![],
                autonomy_level: AutonomyLevel::Draft,
                delegation_allowed: false,
            },
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Read],
                tools: vec![],
                data_classes: vec![],
                counterparties: vec![],
                jurisdictions: vec!["US".into()],
            },
            constraints: AvcConstraints::permissive(),
            authority_chain: None,
            consent_refs: vec![],
            policy_refs: vec![],
            parent_avc_id: None,
        }
    }

    fn baseline_credential() -> AutonomousVolitionCredential {
        let kp = issuer_keypair();
        issue_avc(baseline_draft(), |bytes| kp.sign(bytes)).unwrap()
    }

    fn credential_for_subject(subject_did: Did) -> AutonomousVolitionCredential {
        let mut draft = baseline_draft();
        draft.subject_did = subject_did;
        let kp = issuer_keypair();
        issue_avc(draft, |bytes| kp.sign(bytes)).unwrap()
    }

    fn baseline_action(actor_did: Did) -> AvcActionRequest {
        AvcActionRequest {
            action_id: Hash256::from_bytes([0x55; 32]),
            actor_did,
            requested_permission: Permission::Read,
            tool: None,
            target_did: None,
            data_class: None,
            estimated_budget_minor_units: None,
            estimated_risk_bp: None,
            human_approval: None,
            requires_human_approval: false,
            action_name: None,
        }
    }

    fn sign_action(request: &AvcValidationRequest, keypair: &KeyPair) -> Signature {
        let action = request.action.as_ref().unwrap();
        let payload =
            exo_avc::avc_action_signature_payload(&request.credential, action, &request.now)
                .unwrap();
        keypair.sign(&payload)
    }

    async fn read_body(response: axum::response::Response) -> Vec<u8> {
        body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec()
    }

    #[tokio::test]
    async fn issue_registers_credential() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let credential = baseline_credential();
        let cred_id = credential.id().unwrap();
        let body = serde_json::to_vec(&IssueRequest { credential }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/issue")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp_bytes = read_body(response).await;
        let parsed: IssueResponse = serde_json::from_slice(&resp_bytes).unwrap();
        assert_eq!(parsed.credential_id, format!("{cred_id}"));
        assert_eq!(parsed.status, "registered");
    }

    #[tokio::test]
    async fn issue_rejects_invalid_issuer_signature_without_storing_credential() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let mut credential = baseline_credential();
        credential.delegated_intent.purpose = "tampered after signing".into();
        let forged_id = credential.id().unwrap();
        let body = serde_json::to_vec(&IssueRequest { credential }).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/issue")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let registry = state.registry.lock().unwrap();
        assert_eq!(
            registry.credential_count(),
            0,
            "invalid issuer signatures must not be stored"
        );
        assert!(
            registry.get_credential(&forged_id).is_none(),
            "forged credential id must remain absent"
        );
    }

    #[tokio::test]
    async fn issue_rejects_unknown_issuer_without_storing_credential() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let mut credential = baseline_credential();
        credential.issuer_did = Did::new("did:exo:unknown-issuer").unwrap();
        let forged_id = credential.id().unwrap();
        let body = serde_json::to_vec(&IssueRequest { credential }).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/issue")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let registry = state.registry.lock().unwrap();
        assert_eq!(
            registry.credential_count(),
            0,
            "unresolved issuers must not be stored"
        );
        assert!(
            registry.get_credential(&forged_id).is_none(),
            "unknown-issuer credential id must remain absent"
        );
    }

    #[tokio::test]
    async fn delegate_rejects_invalid_child_signature_without_storing_credential() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let mut credential = baseline_credential();
        credential.parent_avc_id = Some(Hash256::from_bytes([0x42; 32]));
        credential.delegated_intent.purpose = "tampered delegated purpose".into();
        let forged_id = credential.id().unwrap();
        let body = serde_json::to_vec(&DelegateRequest {
            child_credential: credential,
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/delegate")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let registry = state.registry.lock().unwrap();
        assert_eq!(
            registry.credential_count(),
            0,
            "invalid delegated credentials must not be stored"
        );
        assert!(
            registry.get_credential(&forged_id).is_none(),
            "forged delegated credential id must remain absent"
        );
    }

    #[tokio::test]
    async fn validate_returns_allow_for_valid_credential() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let credential = baseline_credential();
        let request = AvcValidationRequest {
            credential,
            action: None,
            now: Timestamp::new(1_500_000, 0),
        };
        let body = serde_json::to_vec(&request).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/validate")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp_bytes = read_body(response).await;
        let parsed: AvcValidationResult = serde_json::from_slice(&resp_bytes).unwrap();
        assert_eq!(parsed.decision, AvcDecision::Allow);
    }

    #[tokio::test]
    async fn receipt_emit_mints_and_stores_node_signed_receipt_for_registered_credential() {
        let state = fresh_state();
        let credential = baseline_credential();
        let credential_id = credential.id().unwrap();
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential.clone())
            .unwrap();
        let request = AvcValidationRequest {
            credential,
            action: Some(baseline_action(Did::new("did:exo:agent").unwrap())),
            now: Timestamp::new(1_500_000, 0),
        };
        let action_id = request.action.as_ref().unwrap().action_id;
        let subject_signature = sign_action(&request, &subject_keypair());
        let body = serde_json::to_vec(&EmitReceiptRequest {
            validation: request,
            subject_signature,
            subject_public_key: None,
        })
        .unwrap();

        let app = avc_router(Arc::clone(&state));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/receipts/emit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let parsed: EmitReceiptResponse =
            serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(
            parsed.receipt_hash,
            format!("{}", parsed.receipt.receipt_id)
        );
        assert_eq!(parsed.receipt.credential_id, credential_id);
        assert_eq!(parsed.receipt.action_id, Some(action_id));
        assert_eq!(parsed.receipt.validator_did, validator_did());
        assert_eq!(parsed.validation.decision, AvcDecision::Allow);
        assert!(parsed.receipt.verify_id().unwrap());
        let signing_payload = parsed.receipt.signing_payload().unwrap();
        assert!(crypto::verify(
            &signing_payload,
            &parsed.receipt.signature,
            validator_keypair().public_key()
        ));
        assert_eq!(state.registry.lock().unwrap().receipt_count(), 1);
    }

    #[tokio::test]
    async fn receipt_emit_accepts_derived_subject_public_key_when_registry_key_absent() {
        let state = fresh_state();
        let subject = crate::identity::did_from_public_key(subject_keypair().public_key()).unwrap();
        let credential = credential_for_subject(subject.clone());
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential.clone())
            .unwrap();
        let request = AvcValidationRequest {
            credential,
            action: Some(baseline_action(subject)),
            now: Timestamp::new(1_500_000, 0),
        };
        let subject_signature = sign_action(&request, &subject_keypair());
        let body = serde_json::to_vec(&EmitReceiptRequest {
            validation: request,
            subject_signature,
            subject_public_key: Some(*subject_keypair().public_key()),
        })
        .unwrap();

        let app = avc_router(Arc::clone(&state));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/receipts/emit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.registry.lock().unwrap().receipt_count(), 1);
    }

    #[tokio::test]
    async fn receipt_emit_rejects_unregistered_credential_without_receipt() {
        let state = fresh_state();
        let credential = baseline_credential();
        let request = AvcValidationRequest {
            credential,
            action: Some(baseline_action(Did::new("did:exo:agent").unwrap())),
            now: Timestamp::new(1_500_000, 0),
        };
        let body = serde_json::to_vec(&EmitReceiptRequest {
            subject_signature: sign_action(&request, &subject_keypair()),
            validation: request,
            subject_public_key: None,
        })
        .unwrap();

        let app = avc_router(Arc::clone(&state));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/receipts/emit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(state.registry.lock().unwrap().receipt_count(), 0);
    }

    #[tokio::test]
    async fn receipt_emit_rejects_invalid_subject_signature_without_receipt() {
        let state = fresh_state();
        let credential = baseline_credential();
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential.clone())
            .unwrap();
        let request = AvcValidationRequest {
            credential,
            action: Some(baseline_action(Did::new("did:exo:agent").unwrap())),
            now: Timestamp::new(1_500_000, 0),
        };
        let body = serde_json::to_vec(&EmitReceiptRequest {
            validation: request,
            subject_signature: Signature::from_bytes([0x99; 64]),
            subject_public_key: None,
        })
        .unwrap();

        let app = avc_router(Arc::clone(&state));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/receipts/emit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(state.registry.lock().unwrap().receipt_count(), 0);
    }

    #[tokio::test]
    async fn receipt_emit_rejects_denied_validation_without_receipt() {
        let state = fresh_state();
        let credential = baseline_credential();
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential.clone())
            .unwrap();
        let mut action = baseline_action(Did::new("did:exo:agent").unwrap());
        action.requested_permission = Permission::Write;
        let request = AvcValidationRequest {
            credential,
            action: Some(action),
            now: Timestamp::new(1_500_000, 0),
        };
        let body = serde_json::to_vec(&EmitReceiptRequest {
            subject_signature: sign_action(&request, &subject_keypair()),
            validation: request,
            subject_public_key: None,
        })
        .unwrap();

        let app = avc_router(Arc::clone(&state));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/receipts/emit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(state.registry.lock().unwrap().receipt_count(), 0);
    }

    #[tokio::test]
    async fn validate_rejects_caller_claimed_human_approval_without_evidence() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let mut draft = baseline_draft();
        draft.constraints.human_approval_required = true;
        let credential = {
            let kp = issuer_keypair();
            issue_avc(draft, |bytes| kp.sign(bytes)).unwrap()
        };
        let action = AvcActionRequest {
            action_id: Hash256::from_bytes([0x55; 32]),
            actor_did: credential.subject_did.clone(),
            requested_permission: Permission::Read,
            tool: None,
            target_did: None,
            data_class: None,
            estimated_budget_minor_units: None,
            estimated_risk_bp: None,
            human_approval: None,
            requires_human_approval: true,
            action_name: None,
        };
        let request = AvcValidationRequest {
            credential,
            action: Some(action),
            now: Timestamp::new(1_500_000, 0),
        };
        let mut body_value = serde_json::to_value(&request).unwrap();
        body_value
            .get_mut("action")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap()
            .remove("human_approval");
        let body = serde_json::to_vec(&body_value).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/validate")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let parsed: AvcValidationResult =
            serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(parsed.decision, AvcDecision::HumanApprovalRequired);
        assert_eq!(
            parsed.reason_codes,
            vec![AvcReasonCode::HumanApprovalMissing]
        );
    }

    #[tokio::test]
    async fn validate_returns_200_with_deny_for_revoked() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let credential = baseline_credential();
        let id = credential.id().unwrap();
        // Pre-register and revoke.
        {
            let kp = issuer_keypair();
            let mut reg = state.registry.lock().unwrap();
            reg.put_credential(credential.clone()).unwrap();
            let revocation = revoke_avc(
                id,
                Did::new("did:exo:issuer").unwrap(),
                AvcRevocationReason::IssuerRevoked,
                Timestamp::new(2, 0),
                |bytes| kp.sign(bytes),
            )
            .unwrap();
            reg.put_revocation(revocation).unwrap();
        }
        let request = AvcValidationRequest {
            credential,
            action: None,
            now: Timestamp::new(1_500_000, 0),
        };
        let body = serde_json::to_vec(&request).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/validate")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let parsed: AvcValidationResult =
            serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(parsed.decision, AvcDecision::Deny);
    }

    #[tokio::test]
    async fn revoke_marks_credential_revoked() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let credential = baseline_credential();
        let id = credential.id().unwrap();
        // Pre-register so we can revoke.
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential)
            .unwrap();
        let kp = issuer_keypair();
        let revocation = revoke_avc(
            id,
            Did::new("did:exo:issuer").unwrap(),
            AvcRevocationReason::IssuerRevoked,
            Timestamp::new(2, 0),
            |bytes| kp.sign(bytes),
        )
        .unwrap();
        let body = serde_json::to_vec(&RevokeRequest { revocation }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/revoke")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(state.registry.lock().unwrap().is_revoked(&id));
    }

    #[tokio::test]
    async fn revoke_rejects_unsigned_revocation_without_marking_credential_revoked() {
        let state = fresh_state();
        let app = avc_router(Arc::clone(&state));
        let credential = baseline_credential();
        let id = credential.id().unwrap();
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential)
            .unwrap();
        let kp = issuer_keypair();
        let mut revocation = revoke_avc(
            id,
            Did::new("did:exo:issuer").unwrap(),
            AvcRevocationReason::IssuerRevoked,
            Timestamp::new(2, 0),
            |bytes| kp.sign(bytes),
        )
        .unwrap();
        revocation.signature = Signature::empty();

        let body = serde_json::to_vec(&RevokeRequest { revocation }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/avc/revoke")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            !state.registry.lock().unwrap().is_revoked(&id),
            "unsigned revocation must not create a tombstone"
        );
    }

    #[tokio::test]
    async fn get_returns_404_when_unknown() {
        let state = fresh_state();
        let app = avc_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/v1/avc/{}", "11".repeat(32)))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_returns_credential_when_known() {
        let state = fresh_state();
        let credential = baseline_credential();
        let id = credential.id().unwrap();
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential.clone())
            .unwrap();
        let app = avc_router(Arc::clone(&state));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/v1/avc/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let parsed: AutonomousVolitionCredential =
            serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(parsed, credential);
    }

    #[tokio::test]
    async fn get_returns_400_for_bad_hex() {
        let state = fresh_state();
        let app = avc_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/avc/not-hex")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_returns_400_for_invalid_did() {
        let state = fresh_state();
        let app = avc_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/agents/not-a-did/avcs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_returns_credentials_for_subject() {
        let state = fresh_state();
        let credential = baseline_credential();
        state
            .registry
            .lock()
            .unwrap()
            .put_credential(credential.clone())
            .unwrap();
        let app = avc_router(Arc::clone(&state));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/agents/did:exo:agent/avcs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let parsed: ListAvcResponse = serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(parsed.did, "did:exo:agent");
        assert_eq!(parsed.credentials.len(), 1);
    }

    #[test]
    fn router_uses_blocking_store_access() {
        let source = include_str!("avc.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        assert!(
            production.contains("tokio::task::spawn_blocking"),
            "AVC handlers must isolate synchronous registry access from Tokio workers"
        );
        assert!(
            production.contains("DefaultBodyLimit::max(MAX_AVC_API_BODY_BYTES)"),
            "AVC router must cap request body size locally"
        );
        assert!(
            production.contains("ConcurrencyLimitLayer::new(MAX_AVC_API_CONCURRENT_REQUESTS)"),
            "AVC router must apply local request admission control"
        );
    }
}

#[cfg(test)]
mod avc_root_trust_tests {
    use exo_avc::AvcRegistryRead;

    use super::*;

    fn avc_state_for_root_trust_test() -> AvcApiState {
        let signer: AvcReceiptSigner = Arc::new(|_| Signature::empty());
        AvcApiState::new(
            Did::new("did:exo:test-validator").expect("test DID"),
            signer,
        )
    }

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("crate has workspace root")
            .to_path_buf()
    }

    fn installed_bundle_path() -> std::path::PathBuf {
        repo_root().join("artifacts/trust/avc-exo-ceremony-2026/root-trust-bundle.canonical.json")
    }

    #[test]
    fn avc_root_trust_bundle_loader_registers_expected_issuer() {
        let state = avc_state_for_root_trust_test();
        let registration = load_root_trust_bundle_from_path(&state, &installed_bundle_path())
            .expect("load bundle");
        let expected_did = Did::new(AVC_ROOT_TRUST_ISSUER_DID).expect("expected issuer DID");
        let expected_public_key =
            parse_expected_public_key(AVC_ROOT_TRUST_ISSUER_PUBLIC_KEY_HEX, "expected issuer key")
                .expect("expected issuer key");

        assert_eq!(registration.ceremony_id, AVC_ROOT_TRUST_CEREMONY_ID);
        assert_eq!(registration.issuer_did, expected_did);
        assert_eq!(registration.issuer_public_key, expected_public_key);

        let registry = state.registry.lock().expect("registry lock");
        assert_eq!(
            registry.resolve_public_key(&expected_did),
            Some(expected_public_key)
        );
    }

    #[test]
    fn avc_root_trust_bundle_tamper_fails_closed_without_registering_issuer() {
        let mut bundle: RootTrustBundle =
            serde_json::from_slice(&std::fs::read(installed_bundle_path()).expect("read bundle"))
                .expect("parse bundle");
        bundle.transcript_hash = Hash256::from_bytes([42u8; 32]);
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        serde_json::to_writer(temp.as_file(), &bundle).expect("write tampered bundle");

        let state = avc_state_for_root_trust_test();
        let error = load_root_trust_bundle_from_path(&state, temp.path())
            .expect_err("tampered bundle must fail");
        assert!(
            error
                .to_string()
                .contains("AVC root trust bundle verification failed")
        );

        let expected_did = Did::new(AVC_ROOT_TRUST_ISSUER_DID).expect("expected issuer DID");
        let registry = state.registry.lock().expect("registry lock");
        assert_eq!(registry.resolve_public_key(&expected_did), None);
    }

    #[test]
    fn avc_root_trust_bundle_missing_required_path_fails_closed() {
        let state = avc_state_for_root_trust_test();
        let missing = repo_root().join("artifacts/trust/avc-exo-ceremony-2026/missing.json");
        let error = load_root_trust_bundle_from_path(&state, &missing)
            .expect_err("missing configured bundle path must fail");
        assert!(
            error
                .to_string()
                .contains("failed to read AVC root trust bundle")
        );
    }
}
