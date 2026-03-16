//! LiveSafe.ai GraphQL types and resolver stubs.
//!
//! Provides LiveSafe-specific types, queries, and mutations anchored
//! to the EXOCHAIN platform: subscriber identities, emergency scan
//! receipts, consent anchors, and PACE trustee shard status.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// PACE enrollment status for a LiveSafe subscriber.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaceStatus {
    /// Subscriber has not completed PACE enrollment.
    Incomplete,
    /// PACE enrollment is active and all shards confirmed.
    Active,
    /// Subscriber is in PACE recovery flow.
    Recovery,
}

/// Card issuance status for a LiveSafe subscriber.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardStatus {
    /// Card has not been issued yet.
    NotIssued,
    /// Card is active and usable.
    Active,
    /// Card has been revoked.
    Revoked,
    /// Card has expired.
    Expired,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// LiveSafe subscriber identity anchored to EXOCHAIN.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSafeIdentity {
    /// Decentralized identifier: `did:exo:subscriber:{uuid}`.
    pub did: String,
    /// 0-100 composite odentity score.
    pub odentity_composite: f64,
    /// Current PACE enrollment status.
    pub pace_status: PaceStatus,
    /// Current card issuance status.
    pub card_status: CardStatus,
    /// Creation timestamp in milliseconds since epoch.
    pub created_at_ms: u64,
    /// Optional AnchorReceipt hash from EXOCHAIN.
    pub exochain_anchor: Option<String>,
}

/// Emergency scan event anchored to EXOCHAIN.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReceipt {
    /// Unique scan identifier.
    pub scan_id: String,
    /// DID of the scanned subscriber.
    pub subscriber_did: String,
    /// DID of the responder who performed the scan.
    pub responder_did: String,
    /// Optional location description or coordinates.
    pub location: Option<String>,
    /// Scan timestamp in milliseconds since epoch.
    pub scanned_at_ms: u64,
    /// Consent expiry timestamp in milliseconds since epoch.
    pub consent_expires_at_ms: u64,
    /// Hash of the associated audit receipt.
    pub audit_receipt_hash: String,
    /// Optional AnchorReceipt hash from EXOCHAIN.
    pub anchor_receipt: Option<String>,
}

/// Consent event for provider access.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentAnchor {
    /// Unique consent identifier.
    pub consent_id: String,
    /// DID of the subscriber granting consent.
    pub subscriber_did: String,
    /// DID of the provider receiving consent.
    pub provider_did: String,
    /// Scope of access granted.
    pub scope: Vec<String>,
    /// Grant timestamp in milliseconds since epoch.
    pub granted_at_ms: u64,
    /// Optional expiry timestamp in milliseconds since epoch.
    pub expires_at_ms: Option<u64>,
    /// Optional revocation timestamp in milliseconds since epoch.
    pub revoked_at_ms: Option<u64>,
    /// Hash of the associated audit receipt.
    pub audit_receipt_hash: String,
}

/// PACE trustee shard status.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrusteeShardStatus {
    /// DID of the trustee holding this shard.
    pub trustee_did: String,
    /// Role: primary, alternate, custodial, or emergency.
    pub role: String,
    /// Whether the shard has been confirmed by the trustee.
    pub shard_confirmed: bool,
    /// Optional acceptance timestamp in milliseconds since epoch.
    pub accepted_at_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// Mutation inputs
// ---------------------------------------------------------------------------

/// Input for anchoring a scan event.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanInput {
    pub subscriber_did: String,
    pub responder_did: String,
    pub location: Option<String>,
    pub consent_expires_at_ms: u64,
}

/// Input for anchoring a consent grant or revoke.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentInput {
    pub subscriber_did: String,
    pub provider_did: String,
    pub scope: Vec<String>,
    pub expires_at_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// Query operations
// ---------------------------------------------------------------------------

/// LiveSafe-specific GraphQL query operations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LiveSafeQuery {
    /// Look up a subscriber identity by DID.
    Identity { did: String },
    /// Retrieve scan history for a subscriber.
    ScanHistory { subscriber_did: String },
    /// Retrieve consent log for a subscriber.
    ConsentLog { subscriber_did: String },
    /// Retrieve PACE trustee shard status for a subscriber.
    PaceStatus { subscriber_did: String },
}

/// LiveSafe-specific GraphQL mutation operations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LiveSafeMutation {
    /// Anchor a scan event to EXOCHAIN.
    AnchorScan { input: ScanInput },
    /// Anchor a consent grant/revoke to EXOCHAIN.
    AnchorConsent { input: ConsentInput },
    /// Register a subscriber DID on EXOCHAIN.
    RegisterIdentity { did: String },
    /// Anchor an audit receipt to EXOCHAIN.
    AnchorAuditReceipt {
        subscriber_did: String,
        receipt_hash: String,
        event_type: String,
    },
}

// ---------------------------------------------------------------------------
// Resolver stubs (mock data)
// ---------------------------------------------------------------------------

/// Resolve a `LiveSafeQuery` and return a JSON-serialisable result.
pub fn resolve_query(query: &LiveSafeQuery) -> serde_json::Value {
    match query {
        LiveSafeQuery::Identity { did } => {
            let identity = LiveSafeIdentity {
                did: did.clone(),
                odentity_composite: 72.5,
                pace_status: PaceStatus::Active,
                card_status: CardStatus::Active,
                created_at_ms: 1_700_000_000_000,
                exochain_anchor: Some("anchor_abc123".to_string()),
            };
            serde_json::to_value(identity).unwrap_or_default()
        }
        LiveSafeQuery::ScanHistory { subscriber_did } => {
            let receipt = ScanReceipt {
                scan_id: "scan-001".to_string(),
                subscriber_did: subscriber_did.clone(),
                responder_did: "did:exo:responder:42".to_string(),
                location: Some("40.7128,-74.0060".to_string()),
                scanned_at_ms: 1_700_000_100_000,
                consent_expires_at_ms: 1_700_000_200_000,
                audit_receipt_hash: "deadbeef".to_string(),
                anchor_receipt: Some("anchor_scan_001".to_string()),
            };
            serde_json::to_value(vec![receipt]).unwrap_or_default()
        }
        LiveSafeQuery::ConsentLog { subscriber_did } => {
            let consent = ConsentAnchor {
                consent_id: "consent-001".to_string(),
                subscriber_did: subscriber_did.clone(),
                provider_did: "did:exo:provider:99".to_string(),
                scope: vec!["medical".to_string(), "emergency".to_string()],
                granted_at_ms: 1_700_000_050_000,
                expires_at_ms: Some(1_700_086_400_000),
                revoked_at_ms: None,
                audit_receipt_hash: "cafebabe".to_string(),
            };
            serde_json::to_value(vec![consent]).unwrap_or_default()
        }
        LiveSafeQuery::PaceStatus { subscriber_did: _ } => {
            let shard = TrusteeShardStatus {
                trustee_did: "did:exo:trustee:primary".to_string(),
                role: "primary".to_string(),
                shard_confirmed: true,
                accepted_at_ms: Some(1_700_000_010_000),
            };
            serde_json::to_value(vec![shard]).unwrap_or_default()
        }
    }
}

/// Resolve a `LiveSafeMutation` and return a JSON-serialisable result.
pub fn resolve_mutation(mutation: &LiveSafeMutation) -> serde_json::Value {
    match mutation {
        LiveSafeMutation::AnchorScan { input } => {
            let receipt = ScanReceipt {
                scan_id: format!("scan-{}", uuid::Uuid::new_v4()),
                subscriber_did: input.subscriber_did.clone(),
                responder_did: input.responder_did.clone(),
                location: input.location.clone(),
                scanned_at_ms: now_ms(),
                consent_expires_at_ms: input.consent_expires_at_ms,
                audit_receipt_hash: hex::encode(
                    exo_core::hash_bytes(input.subscriber_did.as_bytes()).0,
                ),
                anchor_receipt: Some(format!("anchor_{}", uuid::Uuid::new_v4())),
            };
            serde_json::to_value(receipt).unwrap_or_default()
        }
        LiveSafeMutation::AnchorConsent { input } => {
            let consent = ConsentAnchor {
                consent_id: format!("consent-{}", uuid::Uuid::new_v4()),
                subscriber_did: input.subscriber_did.clone(),
                provider_did: input.provider_did.clone(),
                scope: input.scope.clone(),
                granted_at_ms: now_ms(),
                expires_at_ms: input.expires_at_ms,
                revoked_at_ms: None,
                audit_receipt_hash: hex::encode(
                    exo_core::hash_bytes(input.subscriber_did.as_bytes()).0,
                ),
            };
            serde_json::to_value(consent).unwrap_or_default()
        }
        LiveSafeMutation::RegisterIdentity { did } => {
            let identity = LiveSafeIdentity {
                did: did.clone(),
                odentity_composite: 0.0,
                pace_status: PaceStatus::Incomplete,
                card_status: CardStatus::NotIssued,
                created_at_ms: now_ms(),
                exochain_anchor: Some(format!("anchor_{}", uuid::Uuid::new_v4())),
            };
            serde_json::to_value(identity).unwrap_or_default()
        }
        LiveSafeMutation::AnchorAuditReceipt {
            subscriber_did,
            receipt_hash,
            event_type,
        } => {
            let combined = format!("{}:{}:{}", subscriber_did, receipt_hash, event_type);
            let anchor_hash = hex::encode(exo_core::hash_bytes(combined.as_bytes()).0);
            serde_json::to_value(anchor_hash).unwrap_or_default()
        }
    }
}

/// Current time in milliseconds since epoch.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// SDL extension
// ---------------------------------------------------------------------------

/// Additional SDL for LiveSafe types, queries, and mutations.
///
/// These extend the existing `GovSchema` SDL.
pub fn livesafe_sdl() -> &'static str {
    r#"
# --- LiveSafe enums ---

enum LiveSafePaceStatus {
    INCOMPLETE
    ACTIVE
    RECOVERY
}

enum LiveSafeCardStatus {
    NOT_ISSUED
    ACTIVE
    REVOKED
    EXPIRED
}

# --- LiveSafe types ---

type LiveSafeIdentity {
    did: String!
    odentityComposite: Float!
    paceStatus: LiveSafePaceStatus!
    cardStatus: LiveSafeCardStatus!
    createdAtMs: Int!
    exochainAnchor: String
}

type ScanReceipt {
    scanId: String!
    subscriberDid: String!
    responderDid: String!
    location: String
    scannedAtMs: Int!
    consentExpiresAtMs: Int!
    auditReceiptHash: String!
    anchorReceipt: String
}

type ConsentAnchor {
    consentId: String!
    subscriberDid: String!
    providerDid: String!
    scope: [String!]!
    grantedAtMs: Int!
    expiresAtMs: Int
    revokedAtMs: Int
    auditReceiptHash: String!
}

type TrusteeShardStatus {
    trusteeDid: String!
    role: String!
    shardConfirmed: Boolean!
    acceptedAtMs: Int
}

# --- LiveSafe inputs ---

input ScanInput {
    subscriberDid: String!
    responderDid: String!
    location: String
    consentExpiresAtMs: Int!
}

input ConsentInput {
    subscriberDid: String!
    providerDid: String!
    scope: [String!]!
    expiresAtMs: Int
}

# --- LiveSafe queries (extend Query) ---

extend type Query {
    livesafeIdentity(did: String!): LiveSafeIdentity
    livesafeScanHistory(subscriberDid: String!): [ScanReceipt!]!
    livesafeConsentLog(subscriberDid: String!): [ConsentAnchor!]!
    livesafePaceStatus(subscriberDid: String!): [TrusteeShardStatus!]!
}

# --- LiveSafe mutations (extend Mutation) ---

extend type Mutation {
    livesafeAnchorScan(input: ScanInput!): ScanReceipt!
    livesafeAnchorConsent(input: ConsentInput!): ConsentAnchor!
    livesafeRegisterIdentity(did: String!): LiveSafeIdentity!
    livesafeAnchorAuditReceipt(subscriberDid: String!, receiptHash: String!, eventType: String!): String!
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_livesafe_sdl_not_empty() {
        let sdl = livesafe_sdl();
        assert!(!sdl.is_empty());
        assert!(sdl.contains("LiveSafeIdentity"));
        assert!(sdl.contains("ScanReceipt"));
        assert!(sdl.contains("ConsentAnchor"));
        assert!(sdl.contains("TrusteeShardStatus"));
        assert!(sdl.contains("livesafeIdentity"));
        assert!(sdl.contains("livesafeAnchorScan"));
    }

    #[test]
    fn test_query_identity() {
        let q = LiveSafeQuery::Identity {
            did: "did:exo:subscriber:abc".into(),
        };
        let result = resolve_query(&q);
        assert_eq!(result["did"], "did:exo:subscriber:abc");
        assert_eq!(result["odentityComposite"], 72.5);
    }

    #[test]
    fn test_query_scan_history() {
        let q = LiveSafeQuery::ScanHistory {
            subscriber_did: "did:exo:subscriber:abc".into(),
        };
        let result = resolve_query(&q);
        assert!(result.is_array());
        assert_eq!(result[0]["subscriberDid"], "did:exo:subscriber:abc");
    }

    #[test]
    fn test_query_consent_log() {
        let q = LiveSafeQuery::ConsentLog {
            subscriber_did: "did:exo:subscriber:abc".into(),
        };
        let result = resolve_query(&q);
        assert!(result.is_array());
        assert_eq!(result[0]["subscriberDid"], "did:exo:subscriber:abc");
    }

    #[test]
    fn test_query_pace_status() {
        let q = LiveSafeQuery::PaceStatus {
            subscriber_did: "did:exo:subscriber:abc".into(),
        };
        let result = resolve_query(&q);
        assert!(result.is_array());
        assert!(result[0]["shardConfirmed"].as_bool().unwrap());
    }

    #[test]
    fn test_mutation_anchor_scan() {
        let m = LiveSafeMutation::AnchorScan {
            input: ScanInput {
                subscriber_did: "did:exo:subscriber:abc".into(),
                responder_did: "did:exo:responder:42".into(),
                location: Some("40.7128,-74.0060".into()),
                consent_expires_at_ms: 1_700_000_200_000,
            },
        };
        let result = resolve_mutation(&m);
        assert_eq!(result["subscriberDid"], "did:exo:subscriber:abc");
        assert!(result["scanId"].as_str().unwrap().starts_with("scan-"));
    }

    #[test]
    fn test_mutation_anchor_consent() {
        let m = LiveSafeMutation::AnchorConsent {
            input: ConsentInput {
                subscriber_did: "did:exo:subscriber:abc".into(),
                provider_did: "did:exo:provider:99".into(),
                scope: vec!["medical".into()],
                expires_at_ms: Some(1_700_086_400_000),
            },
        };
        let result = resolve_mutation(&m);
        assert_eq!(result["subscriberDid"], "did:exo:subscriber:abc");
        assert!(result["consentId"]
            .as_str()
            .unwrap()
            .starts_with("consent-"));
    }

    #[test]
    fn test_mutation_register_identity() {
        let m = LiveSafeMutation::RegisterIdentity {
            did: "did:exo:subscriber:new".into(),
        };
        let result = resolve_mutation(&m);
        assert_eq!(result["did"], "did:exo:subscriber:new");
        assert_eq!(result["odentityComposite"], 0.0);
        assert_eq!(result["paceStatus"], "Incomplete");
    }

    #[test]
    fn test_mutation_anchor_audit_receipt() {
        let m = LiveSafeMutation::AnchorAuditReceipt {
            subscriber_did: "did:exo:subscriber:abc".into(),
            receipt_hash: "deadbeef".into(),
            event_type: "scan".into(),
        };
        let result = resolve_mutation(&m);
        assert!(result.is_string());
        assert!(!result.as_str().unwrap().is_empty());
    }

    #[test]
    fn test_pace_status_enum_variants() {
        let p = PaceStatus::Incomplete;
        assert_eq!(p, PaceStatus::Incomplete);
        let p = PaceStatus::Active;
        assert_eq!(p, PaceStatus::Active);
        let p = PaceStatus::Recovery;
        assert_eq!(p, PaceStatus::Recovery);
    }

    #[test]
    fn test_card_status_enum_variants() {
        assert_eq!(CardStatus::NotIssued, CardStatus::NotIssued);
        assert_eq!(CardStatus::Active, CardStatus::Active);
        assert_eq!(CardStatus::Revoked, CardStatus::Revoked);
        assert_eq!(CardStatus::Expired, CardStatus::Expired);
    }
}
