//! Pure trust-check domain service.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{ConsentPurpose, CouncilReviewStatus, CredentialStatus, ValidationStatus};

use crate::{
    model::{AgentMemorySafetyScore, DagDbAuthorizedScope, InboundAgentCredential},
    scoring::{
        AgentSafetyComponents, AgentSafetyDecision, DomainError, DomainGateContext, DomainResult,
        compute_agent_memory_safety_score, ensure_authority_and_consent, ensure_tenant_scope,
        hash_error,
    },
};

/// Trust-check request material after gateway scope verification.
#[derive(Debug, Clone)]
pub struct TrustCheckDomainInput {
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
    pub signature: Vec<u8>,
    pub checkpoint_hash: Option<Hash256>,
    pub attestation_hash: Option<Hash256>,
    pub prior_trust_receipt_hash: Option<Hash256>,
    pub evidence_hash: Hash256,
    pub window_start: Timestamp,
    pub window_end: Timestamp,
    pub safety_components: AgentSafetyComponents,
    pub credential_created_at: Timestamp,
    pub score_created_at: Timestamp,
    pub score_latest_receipt_hash: Hash256,
}

/// Trust-check output records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustCheckDomainResult {
    pub credential: InboundAgentCredential,
    pub safety_score: AgentMemorySafetyScore,
    pub decision: AgentSafetyDecision,
}

/// Build or update inbound agent credential and safety score records.
pub fn run_trust_check(
    scope: &DagDbAuthorizedScope,
    gate: &DomainGateContext,
    input: TrustCheckDomainInput,
    now: Timestamp,
) -> DomainResult<TrustCheckDomainResult> {
    ensure_tenant_scope(scope, &input.tenant_id, &input.namespace)?;
    ensure_authority_and_consent(scope, gate)?;
    if input.signature.is_empty() {
        return Err(DomainError::InvalidSignature);
    }
    if input.expires_at.is_expired(&now) {
        return Err(DomainError::ExpiredCredential);
    }

    let signature_hash = Hash256::digest(&input.signature);
    let mut credential = InboundAgentCredential {
        credential_id: Hash256::ZERO,
        tenant_id: input.tenant_id.clone(),
        namespace: input.namespace.clone(),
        agent_did: input.agent_did.clone(),
        operator_did: input.operator_did.clone(),
        model_name: input.model_name,
        model_version: input.model_version,
        provider_or_builder: input.provider_or_builder,
        requested_action: input.requested_action,
        requested_scope_hash: input.requested_scope_hash,
        purpose: input.purpose,
        autonomy_level: input.autonomy_level,
        nonce: input.nonce,
        expires_at: input.expires_at,
        signature_hash,
        credential_status: CredentialStatus::Pending,
        created_at: input.credential_created_at,
        checkpoint_hash: input.checkpoint_hash,
        attestation_hash: input.attestation_hash,
        prior_trust_receipt_hash: input.prior_trust_receipt_hash,
    };
    credential.credential_id = credential.id_material().hash().map_err(hash_error)?;

    let score_result = compute_agent_memory_safety_score(input.safety_components)?;
    credential.credential_status = match score_result.decision {
        AgentSafetyDecision::Pass => CredentialStatus::Active,
        AgentSafetyDecision::NeedsCouncil => CredentialStatus::Pending,
        AgentSafetyDecision::Block => CredentialStatus::Blocked,
    };
    let validation_status = match score_result.decision {
        AgentSafetyDecision::Pass => ValidationStatus::Passed,
        AgentSafetyDecision::NeedsCouncil => ValidationStatus::NeedsCouncil,
        AgentSafetyDecision::Block => ValidationStatus::Failed,
    };
    let council_status = match score_result.decision {
        AgentSafetyDecision::NeedsCouncil => CouncilReviewStatus::Required,
        AgentSafetyDecision::Pass | AgentSafetyDecision::Block => CouncilReviewStatus::NotRequired,
    };
    let mut safety_score = AgentMemorySafetyScore {
        safety_score_id: Hash256::ZERO,
        tenant_id: input.tenant_id,
        namespace: input.namespace,
        agent_did: input.agent_did,
        operator_did: input.operator_did,
        window_start: input.window_start,
        window_end: input.window_end,
        evidence_hash: input.evidence_hash,
        identity_bp: input.safety_components.identity_bp,
        authority_bp: input.safety_components.authority_bp,
        consent_bp: input.safety_components.consent_bp,
        provenance_bp: input.safety_components.provenance_bp,
        validation_bp: input.safety_components.validation_bp,
        recency_bp: input.safety_components.recency_bp,
        revocation_bp: input.safety_components.revocation_bp,
        route_quality_bp: input.safety_components.route_quality_bp,
        incident_penalty_bp: input.safety_components.incident_penalty_bp,
        total_score_bp: score_result.total_score_bp,
        validation_status,
        council_status,
        latest_receipt_hash: input.score_latest_receipt_hash,
        created_at: input.score_created_at,
    };
    safety_score.safety_score_id = safety_score.id_material().hash().map_err(hash_error)?;

    Ok(TrustCheckDomainResult {
        credential,
        safety_score,
        decision: score_result.decision,
    })
}

#[cfg(test)]
mod tests {
    use exo_authority::Permission;
    use exo_avc::AuthorityScope;
    use exo_consent::ConsentDecision;

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn scope() -> DagDbAuthorizedScope {
        DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:operator".into(),
            authority_scope_hash: h(0x90),
            consent_scope_hash: h(0x91),
            permitted_actions: vec!["dagdb:trust_check".into()],
            expires_at: ts(20_000),
        }
    }

    fn gate() -> DomainGateContext {
        DomainGateContext {
            action: "dagdb:trust_check".into(),
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Govern],
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            consent_decision: ConsentDecision::Granted { expires: None },
        }
    }

    fn components() -> AgentSafetyComponents {
        AgentSafetyComponents {
            identity_bp: 8_000,
            authority_bp: 8_000,
            consent_bp: 8_000,
            provenance_bp: 8_000,
            validation_bp: 8_000,
            recency_bp: 8_000,
            revocation_bp: 10_000,
            route_quality_bp: 8_000,
            incident_penalty_bp: 0,
        }
    }

    fn input() -> TrustCheckDomainInput {
        TrustCheckDomainInput {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            agent_did: "did:exo:agent".into(),
            operator_did: "did:exo:operator".into(),
            model_name: "exo-agent".into(),
            model_version: "1.0.0".into(),
            provider_or_builder: "exo".into(),
            requested_action: "dagdb:route".into(),
            requested_scope_hash: h(0x10),
            purpose: ConsentPurpose::TrustCheck,
            autonomy_level: "supervised".into(),
            nonce: "nonce-1".into(),
            expires_at: ts(10_000),
            signature: vec![1, 2, 3],
            checkpoint_hash: Some(h(0x11)),
            attestation_hash: Some(h(0x12)),
            prior_trust_receipt_hash: None,
            evidence_hash: h(0x13),
            window_start: ts(1_000),
            window_end: ts(2_000),
            safety_components: components(),
            credential_created_at: ts(1_500),
            score_created_at: ts(1_500),
            score_latest_receipt_hash: h(0x15),
        }
    }

    #[test]
    fn trust_check_creates_active_credential_and_score() {
        let result =
            run_trust_check(&scope(), &gate(), input(), ts(2_000)).expect("trust check succeeds");
        assert_eq!(
            result.credential.credential_status,
            CredentialStatus::Active
        );
        assert_eq!(
            result.safety_score.validation_status,
            ValidationStatus::Passed
        );
        assert_eq!(result.safety_score.total_score_bp, 8_260);
        assert_eq!(result.decision, AgentSafetyDecision::Pass);
    }

    #[test]
    fn trust_check_failure_paths_fail_closed() {
        let empty_signature = TrustCheckDomainInput {
            signature: Vec::new(),
            ..input()
        };
        assert_eq!(
            run_trust_check(&scope(), &gate(), empty_signature, ts(2_000)),
            Err(DomainError::InvalidSignature)
        );

        let expired = TrustCheckDomainInput {
            expires_at: ts(2_000),
            ..input()
        };
        assert_eq!(
            run_trust_check(&scope(), &gate(), expired, ts(2_000)),
            Err(DomainError::ExpiredCredential)
        );

        let missing_authority = DomainGateContext {
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Read],
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            ..gate()
        };
        assert!(matches!(
            run_trust_check(&scope(), &missing_authority, input(), ts(2_000)),
            Err(DomainError::AuthorityDenied { .. })
        ));
    }
}
