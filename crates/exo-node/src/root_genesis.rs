//! Root genesis portal adapter.

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use exo_core::Hash256;
use exo_root::{CeremonyEnvelope, GenesisCeremonyConfig, PortalStore, RootError};
use serde::Serialize;

/// Shared root genesis portal state.
#[derive(Clone)]
pub struct RootGenesisApiState {
    portal: Arc<Mutex<PortalStore>>,
    config: GenesisCeremonyConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PortalStatusResponse {
    ceremony_id: String,
    threshold: u16,
    max_signers: u16,
    accepted_envelopes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct EnvelopeAcceptedResponse {
    envelope_id: Hash256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ErrorResponse {
    error: String,
}

impl RootGenesisApiState {
    /// Create portal state for one ceremony configuration.
    #[must_use]
    pub fn new(config: GenesisCeremonyConfig) -> Self {
        Self {
            portal: Arc::new(Mutex::new(PortalStore::new(config.clone()))),
            config,
        }
    }
}

/// Router for the server-backed root genesis portal.
pub fn root_genesis_router(state: RootGenesisApiState) -> Router {
    Router::new()
        .route("/api/v1/root-genesis/portal", get(handle_portal_status))
        .route(
            "/api/v1/root-genesis/portal/envelopes",
            post(handle_portal_envelope),
        )
        .with_state(state)
}

async fn handle_portal_status(
    State(state): State<RootGenesisApiState>,
) -> Result<Json<PortalStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let portal = state.portal.lock().map_err(|_| {
        portal_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "portal store lock failed",
        )
    })?;
    Ok(Json(PortalStatusResponse {
        ceremony_id: state.config.ceremony_id,
        threshold: state.config.threshold,
        max_signers: state.config.max_signers,
        accepted_envelopes: portal.envelope_count(),
    }))
}

async fn handle_portal_envelope(
    State(state): State<RootGenesisApiState>,
    Json(envelope): Json<CeremonyEnvelope>,
) -> Result<(StatusCode, Json<EnvelopeAcceptedResponse>), (StatusCode, Json<ErrorResponse>)> {
    let mut portal = state.portal.lock().map_err(|_| {
        portal_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "portal store lock failed",
        )
    })?;
    match portal.submit(envelope) {
        Ok(envelope_id) => Ok((
            StatusCode::CREATED,
            Json(EnvelopeAcceptedResponse { envelope_id }),
        )),
        Err(error) => Err(root_error_to_response(error)),
    }
}

fn portal_error(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

fn root_error_to_response(error: RootError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match &error {
        RootError::SignatureRejected { .. } => StatusCode::UNAUTHORIZED,
        RootError::PortalRejected { reason } if reason.contains("replay") => StatusCode::CONFLICT,
        RootError::PortalRejected { reason } if reason.contains("exceeds") => {
            StatusCode::PAYLOAD_TOO_LARGE
        }
        RootError::PortalRejected { .. } | RootError::InvalidConfig { .. } => {
            StatusCode::BAD_REQUEST
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use exo_core::{Did, SecretKey, Signature, Timestamp, crypto::KeyPair};
    use exo_root::{CeremonyEnvelopeDraft, CeremonyPayloadKind, CeremonyPhase, CertifierContact};
    use tower::ServiceExt;

    use super::*;

    fn did(index: u16) -> Did {
        Did::new(&format!("did:exo:root-portal-module-{index:02}")).expect("valid DID")
    }

    fn certifier(index: u16) -> (CertifierContact, SecretKey) {
        let seed = [u8::try_from(index).expect("index fits"); 32];
        let keypair = KeyPair::from_secret_bytes(seed).expect("valid keypair");
        let transport_secret = [u8::try_from(index).expect("index fits"); 32];
        let transport_public =
            x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(transport_secret));
        (
            CertifierContact {
                did: did(index),
                frost_identifier: index,
                signing_public_key: *keypair.public_key(),
                transport_public_key: *transport_public.as_bytes(),
            },
            keypair.secret_key().clone(),
        )
    }

    fn config() -> (GenesisCeremonyConfig, SecretKey) {
        let mut certifiers = Vec::new();
        let mut first_secret = None;
        for index in 1..=13 {
            let (contact, secret) = certifier(index);
            if index == 1 {
                first_secret = Some(secret.clone());
            }
            certifiers.push(contact);
        }
        (
            GenesisCeremonyConfig {
                ceremony_id: "exo-root-portal-module-test".into(),
                network_id: "exochain-test".into(),
                repo_commit: "d8927686a34bdc28ba36d53938f665685d2c4c04".into(),
                constitution_hash: Hash256::digest(b"constitution"),
                threshold: exo_root::ROOT_GENESIS_THRESHOLD,
                max_signers: exo_root::ROOT_GENESIS_SIGNERS,
                created_at: Timestamp::new(1_785_000_000_000, 0),
                certifiers,
            },
            first_secret.expect("first certifier secret"),
        )
    }

    fn envelope(
        config: &GenesisCeremonyConfig,
        secret: &SecretKey,
        sequence: u64,
        payload_bytes: Vec<u8>,
    ) -> CeremonyEnvelope {
        CeremonyEnvelope::sign(
            CeremonyEnvelopeDraft {
                ceremony_id: config.ceremony_id.clone(),
                phase: CeremonyPhase::Round2,
                payload_kind: CeremonyPayloadKind::Round2EncryptedPackage,
                sender_did: config.certifiers[0].did.clone(),
                recipient_did: Some(config.certifiers[1].did.clone()),
                sequence,
                payload_bytes,
            },
            secret,
        )
        .expect("signed envelope")
    }

    async fn post_envelope(
        router: axum::Router,
        envelope: &CeremonyEnvelope,
    ) -> axum::response::Response {
        let body = serde_json::to_vec(envelope).expect("json body");
        router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/root-genesis/portal/envelopes")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn get_status(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/root-genesis/portal")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    fn poison_portal_lock(state: &RootGenesisApiState) {
        let portal = Arc::clone(&state.portal);
        let _ = std::thread::spawn(move || {
            let _guard = portal.lock().expect("portal lock");
            panic!("poison portal lock");
        })
        .join();
    }

    #[tokio::test]
    async fn portal_status_reports_policy_and_accepted_envelope_count() {
        let (config, secret) = config();
        let state = RootGenesisApiState::new(config.clone());
        let router = root_genesis_router(state);
        let accepted = post_envelope(
            router.clone(),
            &envelope(&config, &secret, 1, b"ct".to_vec()),
        )
        .await;
        assert_eq!(accepted.status(), StatusCode::CREATED);

        let response = get_status(router).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body bytes");
        let status: serde_json::Value = serde_json::from_slice(&body).expect("status json");
        assert_eq!(status["ceremony_id"], config.ceremony_id);
        assert_eq!(status["threshold"], u64::from(config.threshold));
        assert_eq!(status["max_signers"], u64::from(config.max_signers));
        assert_eq!(status["accepted_envelopes"], 1);
    }

    #[tokio::test]
    async fn portal_handlers_fail_closed_when_store_lock_is_poisoned() {
        let (config, secret) = config();
        let state = RootGenesisApiState::new(config.clone());
        poison_portal_lock(&state);
        let router = root_genesis_router(state);

        let status = get_status(router.clone()).await;
        assert_eq!(status.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let submitted = post_envelope(router, &envelope(&config, &secret, 1, b"ct".to_vec())).await;
        assert_eq!(submitted.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn portal_handler_maps_signature_and_payload_size_rejections() {
        let (config, secret) = config();
        let router = root_genesis_router(RootGenesisApiState::new(config.clone()));

        let mut unsigned = envelope(&config, &secret, 1, b"ct".to_vec());
        unsigned.signature = Signature::Empty;
        let unauthorized = post_envelope(router.clone(), &unsigned).await;
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let oversized = envelope(&config, &secret, 2, vec![7; 64 * 1024 + 1]);
        let too_large = post_envelope(router, &oversized).await;
        assert_eq!(too_large.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn portal_error_mapper_preserves_internal_error_status() {
        let (status, Json(body)) = root_error_to_response(RootError::CanonicalEncoding {
            detail: "encoder unavailable".to_owned(),
        });

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.error.contains("encoder unavailable"));
    }
}
