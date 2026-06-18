//! Canonical hash materials for ExoChain DAG DB IDs and receipts.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    ConsentPurpose, DecisionSource, MemoryEdgeType, MemoryNodeType, RiskClass, SourceType,
    SubjectKind,
};
use serde::{Deserialize, Serialize};

use crate::error::{DagDbError, Result};

const SCHEMA_VERSION: u16 = 1;

const MEMORY_ID_DOMAIN: &str = "exo.dagdb.memory_id";
const CATALOG_ID_DOMAIN: &str = "exo.dagdb.catalog_id";
const ROUTE_ID_DOMAIN: &str = "exo.dagdb.route_id";
const CONTEXT_PACKET_ID_DOMAIN: &str = "exo.dagdb.context_packet_id";
const VALIDATION_REPORT_ID_DOMAIN: &str = "exo.dagdb.validation_report_id";
const SAFETY_SCORE_ID_DOMAIN: &str = "exo.dagdb.agent_safety_score_id";
const CREDENTIAL_ID_DOMAIN: &str = "exo.dagdb.inbound_agent_credential_id";
const COUNCIL_DECISION_ID_DOMAIN: &str = "exo.dagdb.council_decision_id";
const RECEIPT_HASH_DOMAIN: &str = "exo.dagdb.receipt_hash";
const REQUEST_HASH_DOMAIN: &str = "exo.dagdb.request_hash";

/// Parent-link material used inside memory ID hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ParentLink {
    pub memory_id: Hash256,
    pub edge_type: MemoryEdgeType,
}

impl ParentLink {
    /// Construct a parent link from a memory hash and edge type.
    #[must_use]
    pub const fn new(memory_id: Hash256, edge_type: MemoryEdgeType) -> Self {
        Self {
            memory_id,
            edge_type,
        }
    }
}

/// Entity ID material for receipt memory objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptMemoryObjectIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub node_type: MemoryNodeType,
    pub source_type: SourceType,
    pub source_hash: Hash256,
    pub payload_hash: Hash256,
    pub owner_did: String,
    pub controller_did: String,
    pub consent_purpose: ConsentPurpose,
    pub parent_links: Vec<ParentLink>,
}

impl ReceiptMemoryObjectIdMaterial {
    /// Build memory ID material with deterministic parent-link order.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        namespace: String,
        node_type: MemoryNodeType,
        source_type: SourceType,
        source_hash: Hash256,
        payload_hash: Hash256,
        owner_did: String,
        controller_did: String,
        consent_purpose: ConsentPurpose,
        parent_links: Vec<ParentLink>,
    ) -> Self {
        let mut parent_links = parent_links;
        parent_links.sort();
        parent_links.dedup();
        Self {
            tenant_id,
            namespace,
            node_type,
            source_type,
            source_hash,
            payload_hash,
            owner_did,
            controller_did,
            consent_purpose,
            parent_links,
        }
    }

    /// Compute the canonical memory ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            MEMORY_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                self.node_type,
                self.source_type,
                self.source_hash,
                self.payload_hash,
                &self.owner_did,
                &self.controller_did,
                self.consent_purpose,
                &self.parent_links,
            ),
        )
    }
}

/// Entity ID material for catalog entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntryIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub memory_id: Option<Hash256>,
    pub parent_catalog_id: Option<Hash256>,
    pub catalog_level: u32,
    pub payload_hash: Hash256,
    pub source_hash: Hash256,
}

impl CatalogEntryIdMaterial {
    /// Build catalog ID material.
    #[must_use]
    pub fn new(
        tenant_id: String,
        namespace: String,
        memory_id: Option<Hash256>,
        parent_catalog_id: Option<Hash256>,
        catalog_level: u32,
        payload_hash: Hash256,
        source_hash: Hash256,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            memory_id,
            parent_catalog_id,
            catalog_level,
            payload_hash,
            source_hash,
        }
    }

    /// Compute the canonical catalog ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            CATALOG_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                self.memory_id,
                self.parent_catalog_id,
                self.catalog_level,
                self.payload_hash,
                self.source_hash,
            ),
        )
    }
}

/// Entity ID material for routes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub requesting_agent_did: String,
    pub task_signature_hash: Hash256,
    pub approved_scope_hash: Hash256,
    pub selected_memory_ids_ordered: Vec<Hash256>,
    pub token_budget: u32,
}

impl RouteIdMaterial {
    /// Build route ID material. The selected memory list is ordered input.
    #[must_use]
    pub fn new(
        tenant_id: String,
        namespace: String,
        requesting_agent_did: String,
        task_signature_hash: Hash256,
        approved_scope_hash: Hash256,
        selected_memory_ids_ordered: Vec<Hash256>,
        token_budget: u32,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            requesting_agent_did,
            task_signature_hash,
            approved_scope_hash,
            selected_memory_ids_ordered,
            token_budget,
        }
    }

    /// Compute the canonical route ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            ROUTE_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                &self.requesting_agent_did,
                self.task_signature_hash,
                self.approved_scope_hash,
                &self.selected_memory_ids_ordered,
                self.token_budget,
            ),
        )
    }
}

/// Entity ID material for context packets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPacketIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub route_id: Hash256,
    pub task_hash: Hash256,
    pub memory_refs_ordered: Vec<Hash256>,
    pub token_budget: u32,
}

impl ContextPacketIdMaterial {
    /// Build context packet ID material. The memory refs list is ordered input.
    #[must_use]
    pub fn new(
        tenant_id: String,
        namespace: String,
        request_id: String,
        route_id: Hash256,
        task_hash: Hash256,
        memory_refs_ordered: Vec<Hash256>,
        token_budget: u32,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            request_id,
            route_id,
            task_hash,
            memory_refs_ordered,
            token_budget,
        }
    }

    /// Compute the canonical context packet ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            CONTEXT_PACKET_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                &self.request_id,
                self.route_id,
                self.task_hash,
                &self.memory_refs_ordered,
                self.token_budget,
            ),
        )
    }
}

/// Entity ID material for validation reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReportIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub validator_did: String,
    pub input_hash: Hash256,
    pub policy_hash: Hash256,
}

impl ValidationReportIdMaterial {
    /// Build validation report ID material.
    #[must_use]
    pub fn new(
        tenant_id: String,
        namespace: String,
        subject_kind: SubjectKind,
        subject_id: Hash256,
        validator_did: String,
        input_hash: Hash256,
        policy_hash: Hash256,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            subject_kind,
            subject_id,
            validator_did,
            input_hash,
            policy_hash,
        }
    }

    /// Compute the canonical validation report ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            VALIDATION_REPORT_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                self.subject_kind,
                self.subject_id,
                &self.validator_did,
                self.input_hash,
                self.policy_hash,
            ),
        )
    }
}

/// Entity ID material for agent memory safety scores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMemorySafetyScoreIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub agent_did: String,
    pub operator_did: String,
    pub window_start: Timestamp,
    pub window_end: Timestamp,
    pub evidence_hash: Hash256,
}

impl AgentMemorySafetyScoreIdMaterial {
    /// Build safety score ID material.
    #[must_use]
    pub fn new(
        tenant_id: String,
        namespace: String,
        agent_did: String,
        operator_did: String,
        window_start: Timestamp,
        window_end: Timestamp,
        evidence_hash: Hash256,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            agent_did,
            operator_did,
            window_start,
            window_end,
            evidence_hash,
        }
    }

    /// Compute the canonical safety score ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            SAFETY_SCORE_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                &self.agent_did,
                &self.operator_did,
                self.window_start,
                self.window_end,
                self.evidence_hash,
            ),
        )
    }
}

/// Entity ID material for inbound agent credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboundAgentCredentialIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub agent_did: String,
    pub operator_did: String,
    pub model_name: String,
    pub model_version: String,
    pub provider_or_builder: String,
    pub requested_action: String,
    pub requested_scope_hash: Hash256,
    pub purpose: ConsentPurpose,
    pub autonomy_level: String,
    pub nonce: String,
    pub expires_at: Timestamp,
}

impl InboundAgentCredentialIdMaterial {
    /// Build credential ID material.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        namespace: String,
        agent_did: String,
        operator_did: String,
        model_name: String,
        model_version: String,
        provider_or_builder: String,
        requested_action: String,
        requested_scope_hash: Hash256,
        purpose: ConsentPurpose,
        autonomy_level: String,
        nonce: String,
        expires_at: Timestamp,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            agent_did,
            operator_did,
            model_name,
            model_version,
            provider_or_builder,
            requested_action,
            requested_scope_hash,
            purpose,
            autonomy_level,
            nonce,
            expires_at,
        }
    }

    /// Compute the canonical credential ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            CREDENTIAL_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                &self.agent_did,
                &self.operator_did,
                &self.model_name,
                &self.model_version,
                &self.provider_or_builder,
                &self.requested_action,
                self.requested_scope_hash,
                self.purpose,
                &self.autonomy_level,
                &self.nonce,
                self.expires_at,
            ),
        )
    }
}

/// Entity ID material for council decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouncilDecisionIdMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub requested_action: String,
    pub approved_scope_hash: Hash256,
    pub risk_class: RiskClass,
    pub approver_did: String,
    pub decision_source: DecisionSource,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

impl CouncilDecisionIdMaterial {
    /// Build council decision ID material.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        namespace: String,
        subject_kind: SubjectKind,
        subject_id: Hash256,
        requested_action: String,
        approved_scope_hash: Hash256,
        risk_class: RiskClass,
        approver_did: String,
        decision_source: DecisionSource,
        created_at: Timestamp,
        expires_at: Timestamp,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            subject_kind,
            subject_id,
            requested_action,
            approved_scope_hash,
            risk_class,
            approver_did,
            decision_source,
            created_at,
            expires_at,
        }
    }

    /// Compute the canonical council decision ID.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            COUNCIL_DECISION_ID_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                self.subject_kind,
                self.subject_id,
                &self.requested_action,
                self.approved_scope_hash,
                self.risk_class,
                &self.approver_did,
                self.decision_source,
                self.created_at,
                self.expires_at,
            ),
        )
    }
}

/// Receipt hash material for subject event receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptHashMaterial {
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub prev_receipt_hash: Hash256,
    pub seq: u64,
    pub event_type: exo_dag_db_api::ReceiptEventType,
    pub actor_did: String,
    pub event_hlc: Timestamp,
    pub event_body_hash: Hash256,
}

impl ReceiptHashMaterial {
    /// Compute the canonical receipt hash.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            RECEIPT_HASH_DOMAIN,
            &(
                &self.tenant_id,
                &self.namespace,
                self.subject_kind,
                self.subject_id,
                self.prev_receipt_hash,
                self.seq,
                self.event_type,
                &self.actor_did,
                self.event_hlc,
                self.event_body_hash,
            ),
        )
    }
}

/// Idempotency request hash material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestHashMaterial {
    pub route_name: String,
    pub tenant_id: String,
    pub namespace: String,
    pub canonical_redacted_request_body: Vec<u8>,
}

impl RequestHashMaterial {
    /// Compute the canonical idempotency request hash.
    pub fn hash(&self) -> Result<Hash256> {
        hash_tagged(
            REQUEST_HASH_DOMAIN,
            &(
                &self.route_name,
                &self.tenant_id,
                &self.namespace,
                &self.canonical_redacted_request_body,
            ),
        )
    }
}

/// Convert a 64-character lowercase hex string into `Hash256`.
pub fn parse_hash256_hex(field: &str, value: &str) -> Result<Hash256> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DagDbError::Serialization(format!(
            "{field} must be a lowercase sha256 hex digest"
        )));
    }

    let mut bytes = [0_u8; 32];
    let raw = value.as_bytes();
    for index in 0..32 {
        let high = hex_nibble(raw[index * 2]);
        let low = hex_nibble(raw[index * 2 + 1]);
        bytes[index] = (high << 4) | low;
    }
    Ok(Hash256::from_bytes(bytes))
}

/// Compute stable hash material using canonical EXOCHAIN CBOR.
pub fn stable_hash_parts(domain: &str, parts: &[&str]) -> Result<Hash256> {
    hash_tagged(domain, parts)
}

fn hash_tagged<T: Serialize + ?Sized>(
    domain_tag: &str,
    fields_by_declared_index: &T,
) -> Result<Hash256> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(
        &(domain_tag, SCHEMA_VERSION, fields_by_declared_index),
        &mut buf,
    )
    .map_err(|err| DagDbError::Serialization(err.to_string()))?;
    Ok(Hash256::digest(&buf))
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        _ => byte - b'a' + 10,
    }
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::{ConsentPurpose, MemoryNodeType, RiskClass, SourceType};

    use super::*;
    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(physical_ms: u64, logical: u32) -> Timestamp {
        Timestamp::new(physical_ms, logical)
    }

    #[test]
    fn canonical_hash_vectors() {
        let memory = ReceiptMemoryObjectIdMaterial::new(
            "tenant-a".into(),
            "primary".into(),
            MemoryNodeType::Source,
            SourceType::PublicWeb,
            h(0x11),
            h(0x22),
            "did:exo:owner".into(),
            "did:exo:controller".into(),
            ConsentPurpose::Retrieval,
            vec![
                ParentLink::new(h(0x44), MemoryEdgeType::Parent),
                ParentLink::new(h(0x33), MemoryEdgeType::Parent),
                ParentLink::new(h(0x33), MemoryEdgeType::Parent),
            ],
        );
        let route = RouteIdMaterial::new(
            "tenant-a".into(),
            "primary".into(),
            "did:exo:agent".into(),
            h(0x10),
            h(0x20),
            vec![h(0x30), h(0x31)],
            4096,
        );
        let receipt = ReceiptHashMaterial {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            subject_kind: SubjectKind::Memory,
            subject_id: h(0x99),
            prev_receipt_hash: Hash256::ZERO,
            seq: 1,
            event_type: exo_dag_db_api::ReceiptEventType::IntakeCreated,
            actor_did: "did:exo:agent".into(),
            event_hlc: ts(2_000, 1),
            event_body_hash: h(0x98),
        };
        let request = RequestHashMaterial {
            route_name: "dagdb.intake".into(),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            canonical_redacted_request_body: br#"{"title":{"text":"Allowed"}}"#.to_vec(),
        };

        assert_eq!(
            memory.hash().expect("memory hash").to_string(),
            "0a2b3ee8384a4c7bb6749935e4476b9e9c091b3b9f7e58a6efb8899657b45361"
        );
        assert_eq!(
            route.hash().expect("route hash").to_string(),
            "982c68dbf608f41a4e3b7b9bd1057958005d04c22a6ab0b8eb496319605ad5a7"
        );
        assert_eq!(
            receipt.hash().expect("receipt hash").to_string(),
            "9b97027d1e06aa1b29ef3e6ddf22a180991c7299be0945a9d6f42ac95b26b73b"
        );
        assert_eq!(
            request.hash().expect("request hash").to_string(),
            "87532d689a9a6f38e0551f42fb6c40abb98e8c80b4decf8f6d174d60dc113096"
        );
    }

    #[test]
    fn every_id_material_hashes_with_distinct_domains() {
        let materials = [
            ReceiptMemoryObjectIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                MemoryNodeType::Source,
                SourceType::PublicWeb,
                h(0x11),
                h(0x22),
                "did:exo:owner".into(),
                "did:exo:controller".into(),
                ConsentPurpose::Retrieval,
                vec![ParentLink::new(h(0x33), MemoryEdgeType::Parent)],
            )
            .hash()
            .expect("memory id hash"),
            CatalogEntryIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                Some(h(0x34)),
                Some(h(0x35)),
                1,
                h(0x22),
                h(0x11),
            )
            .hash()
            .expect("catalog id hash"),
            RouteIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                "did:exo:agent".into(),
                h(0x36),
                h(0x37),
                vec![h(0x38), h(0x39)],
                2048,
            )
            .hash()
            .expect("route id hash"),
            ContextPacketIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                "request-1".into(),
                h(0x40),
                h(0x41),
                vec![h(0x38), h(0x39)],
                2048,
            )
            .hash()
            .expect("context packet id hash"),
            ValidationReportIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                SubjectKind::Memory,
                h(0x42),
                "did:exo:validator".into(),
                h(0x43),
                h(0x44),
            )
            .hash()
            .expect("validation report id hash"),
            AgentMemorySafetyScoreIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                "did:exo:agent".into(),
                "did:exo:operator".into(),
                ts(1_000, 0),
                ts(2_000, 0),
                h(0x45),
            )
            .hash()
            .expect("safety score id hash"),
            InboundAgentCredentialIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                "did:exo:agent".into(),
                "did:exo:operator".into(),
                "exo-agent".into(),
                "1.0.0".into(),
                "exo".into(),
                "memory:route".into(),
                h(0x46),
                ConsentPurpose::TrustCheck,
                "supervised".into(),
                "nonce-1".into(),
                ts(3_000, 0),
            )
            .hash()
            .expect("credential id hash"),
            CouncilDecisionIdMaterial::new(
                "tenant-a".into(),
                "primary".into(),
                SubjectKind::Memory,
                h(0x47),
                "memory:routable".into(),
                h(0x48),
                RiskClass::R3,
                "did:exo:council".into(),
                DecisionSource::Human,
                ts(1_000, 0),
                ts(2_000, 0),
            )
            .hash()
            .expect("council decision id hash"),
            ReceiptHashMaterial {
                tenant_id: "tenant-a".into(),
                namespace: "primary".into(),
                subject_kind: SubjectKind::Memory,
                subject_id: h(0x49),
                prev_receipt_hash: Hash256::ZERO,
                seq: 1,
                event_type: exo_dag_db_api::ReceiptEventType::IntakeCreated,
                actor_did: "did:exo:agent".into(),
                event_hlc: ts(4_000, 0),
                event_body_hash: h(0x50),
            }
            .hash()
            .expect("receipt hash"),
            RequestHashMaterial {
                route_name: "dagdb.intake".into(),
                tenant_id: "tenant-a".into(),
                namespace: "primary".into(),
                canonical_redacted_request_body: b"{}".to_vec(),
            }
            .hash()
            .expect("request hash"),
        ];

        for hash in materials {
            assert_ne!(hash, Hash256::ZERO);
        }
        let mut sorted = materials.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), materials.len());
    }

    #[test]
    fn id_material_excludes_generated_fields() {
        let parent_links = vec![ParentLink::new(h(0x44), MemoryEdgeType::Parent)];
        let base_hash = ReceiptMemoryObjectIdMaterial::new(
            "tenant-a".into(),
            "primary".into(),
            MemoryNodeType::Source,
            SourceType::PublicWeb,
            h(0x11),
            h(0x22),
            "did:exo:owner".into(),
            "did:exo:controller".into(),
            ConsentPurpose::Retrieval,
            parent_links.clone(),
        )
        .hash()
        .expect("base ID hash");
        let changed_hash = ReceiptMemoryObjectIdMaterial::new(
            "tenant-a".into(),
            "primary".into(),
            MemoryNodeType::Source,
            SourceType::PublicWeb,
            h(0x11),
            h(0x22),
            "did:exo:owner".into(),
            "did:exo:controller".into(),
            ConsentPurpose::Retrieval,
            parent_links,
        )
        .hash()
        .expect("changed ID hash");
        assert_eq!(base_hash, changed_hash);

        let changed_payload_hash = ReceiptMemoryObjectIdMaterial::new(
            "tenant-a".into(),
            "primary".into(),
            MemoryNodeType::Source,
            SourceType::PublicWeb,
            h(0x11),
            h(0xdd),
            "did:exo:owner".into(),
            "did:exo:controller".into(),
            ConsentPurpose::Retrieval,
            vec![ParentLink::new(h(0x44), MemoryEdgeType::Parent)],
        )
        .hash()
        .expect("changed payload hash");
        assert_ne!(base_hash, changed_payload_hash);
    }

    #[test]
    fn parses_lowercase_hash_hex() {
        let hash = parse_hash256_hex("fixture", &"0a".repeat(32)).expect("hash parses");
        assert_eq!(hash.to_string(), "0a".repeat(32));
        assert!(parse_hash256_hex("fixture", &"0A".repeat(32)).is_err());
        assert!(parse_hash256_hex("fixture", "not-hex").is_err());
    }
}
