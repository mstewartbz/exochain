//! Council and human approval scope checks for ExoChain DAG DB.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    CouncilDecisionStatus, CouncilReviewStatus, DAGDB_COUNCIL_DECISION_RESPONSE_SCHEMA_VERSION,
    DagDbCouncilDecisionRequest, DagDbCouncilDecisionResponse, DecisionSource, ReceiptEventType,
    RiskClass, SubjectKind, ValidationStatus,
};
use serde::Serialize;
use thiserror::Error;

use crate::{
    hash::{CouncilDecisionIdMaterial, ReceiptHashMaterial},
    metadata::{MetadataError, MetadataField, sanitize_runtime_metadata},
    model::CouncilDecision,
};

/// Errors produced by council approval validation and response construction.
#[derive(Debug, Error)]
pub enum CouncilError {
    /// Request shape or field content is invalid.
    #[error("invalid_request_shape: {0}")]
    InvalidRequestShape(&'static str),
    /// Approval is required and no usable durable approval is available.
    #[error("approval_required")]
    ApprovalRequired,
    /// A durable approval was denied.
    #[error("approval_denied")]
    ApprovalDenied,
    /// A durable approval was escalated and cannot authorize the action.
    #[error("council_escalation_required")]
    CouncilEscalationRequired,
    /// Durable approval scope does not match the requested action.
    #[error("approval_scope_mismatch")]
    ApprovalScopeMismatch,
    /// Metadata sanitizer rejected the council notes.
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    /// Canonical hash material could not be serialized.
    #[error("hash_material_failed: {0}")]
    Hash(String),
}

/// Council result alias.
pub type Result<T> = std::result::Result<T, CouncilError>;

/// Scope that an approval must cover before an R3-R5 action may proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalScope {
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub requested_action: String,
    pub approved_scope_hash: Hash256,
    pub risk_class: RiskClass,
    pub council_decision_id: Option<Hash256>,
}

/// Built council decision plus its API response and event body hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CouncilDecisionRecord {
    pub decision: CouncilDecision,
    pub response: DagDbCouncilDecisionResponse,
    pub event_body_hash: Hash256,
}

#[derive(Debug, Serialize)]
struct CouncilDecisionEventBody<'a> {
    decision_id: Hash256,
    subject_kind: SubjectKind,
    subject_id: Hash256,
    requested_action: &'a str,
    approved_scope_hash: Hash256,
    risk_class: RiskClass,
    approver_did: &'a str,
    decision_source: DecisionSource,
    decision_status: CouncilDecisionStatus,
    reason_code: &'a str,
    expires_at: Timestamp,
    notes_hash: Option<Hash256>,
}

/// Return whether a risk class requires durable council or human approval.
#[must_use]
pub const fn risk_requires_council(risk_class: RiskClass) -> bool {
    matches!(risk_class, RiskClass::R3 | RiskClass::R4 | RiskClass::R5)
}

/// Check whether a stored council decision exactly matches the requested scope.
#[must_use]
pub fn approval_scope_matches(scope: &ApprovalScope, decision: &CouncilDecision) -> bool {
    scope.tenant_id == decision.tenant_id
        && scope.namespace == decision.namespace
        && scope.subject_kind == decision.subject_kind
        && scope.subject_id == decision.subject_id
        && scope.requested_action == decision.requested_action
        && scope.approved_scope_hash == decision.approved_scope_hash
        && scope.risk_class == decision.risk_class
        && scope
            .council_decision_id
            .is_none_or(|decision_id| decision_id == decision.decision_id)
}

/// Enforce the R3-R5 approval rule for an action at `now`.
pub fn require_approval_for_risk(
    scope: &ApprovalScope,
    decision: Option<&CouncilDecision>,
    now: Timestamp,
) -> Result<CouncilReviewStatus> {
    if !risk_requires_council(scope.risk_class) {
        return Ok(CouncilReviewStatus::NotRequired);
    }

    let decision = decision.ok_or(CouncilError::ApprovalRequired)?;
    if !approval_scope_matches(scope, decision) {
        return Err(CouncilError::ApprovalScopeMismatch);
    }

    match decision.decision_status {
        CouncilDecisionStatus::Approved if decision.expires_at.is_expired(&now) => {
            Err(CouncilError::ApprovalRequired)
        }
        CouncilDecisionStatus::Approved => Ok(CouncilReviewStatus::Approved),
        CouncilDecisionStatus::Denied => Err(CouncilError::ApprovalDenied),
        CouncilDecisionStatus::Escalated => Err(CouncilError::CouncilEscalationRequired),
        CouncilDecisionStatus::Expired | CouncilDecisionStatus::Revoked => {
            Err(CouncilError::ApprovalRequired)
        }
    }
}

/// Validate retry-after-approval material for a previously blocked mutation.
pub fn validate_retry_after_approval(
    scope: &ApprovalScope,
    decision: &CouncilDecision,
    now: Timestamp,
) -> Result<()> {
    require_approval_for_risk(scope, Some(decision), now).map(|_| ())
}

/// Build the durable council decision record and its receipt-backed response.
pub fn build_council_decision_record(
    request: DagDbCouncilDecisionRequest,
) -> Result<CouncilDecisionRecord> {
    validate_common_text("tenant_id", &request.tenant_id)?;
    validate_common_text("namespace", &request.namespace)?;
    validate_common_text("idempotency_key", &request.idempotency_key)?;
    validate_common_text("requested_action", &request.requested_action)?;
    validate_common_text("reason_code", &request.reason_code)?;
    validate_did(&request.approver_did)?;

    let subject_id = parse_hash256("subject_id", &request.subject_id)?;
    let approved_scope_hash = parse_hash256("approved_scope_hash", &request.approved_scope_hash)?;
    let validation_report_id = parse_optional_hash(
        "validation_report_id",
        request.validation_report_id.as_deref(),
    )?;
    let route_id = parse_optional_hash("route_id", request.route_id.as_deref())?;
    let context_packet_id =
        parse_optional_hash("context_packet_id", request.context_packet_id.as_deref())?;
    let created_at = parse_hlc("created_at", &request.created_at)?;
    let expires_at = parse_hlc("expires_at", &request.expires_at)?;
    if expires_at <= created_at {
        return Err(CouncilError::InvalidRequestShape("expires_at"));
    }

    let notes = request
        .notes_text
        .as_deref()
        .map(|notes| sanitize_runtime_metadata(MetadataField::CouncilNotes, notes))
        .transpose()?;
    let notes_hash = notes
        .as_ref()
        .map(hash_serializable)
        .transpose()
        .map_err(|err| CouncilError::Hash(err.to_string()))?;

    let id_material = CouncilDecisionIdMaterial::new(
        request.tenant_id.clone(),
        request.namespace.clone(),
        request.subject_kind,
        subject_id,
        request.requested_action.clone(),
        approved_scope_hash,
        request.risk_class,
        request.approver_did.clone(),
        request.decision_source,
        created_at,
        expires_at,
    );
    let decision_id = id_material
        .hash()
        .map_err(|err| CouncilError::Hash(err.to_string()))?;

    let event_body_hash = hash_serializable(&CouncilDecisionEventBody {
        decision_id,
        subject_kind: request.subject_kind,
        subject_id,
        requested_action: &request.requested_action,
        approved_scope_hash,
        risk_class: request.risk_class,
        approver_did: &request.approver_did,
        decision_source: request.decision_source,
        decision_status: request.decision_status,
        reason_code: &request.reason_code,
        expires_at,
        notes_hash,
    })
    .map_err(|err| CouncilError::Hash(err.to_string()))?;

    let receipt_hash = ReceiptHashMaterial {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        subject_kind: SubjectKind::CouncilDecision,
        subject_id: decision_id,
        prev_receipt_hash: Hash256::ZERO,
        seq: 1,
        event_type: ReceiptEventType::CouncilDecisionRecorded,
        actor_did: request.approver_did.clone(),
        event_hlc: created_at,
        event_body_hash,
    }
    .hash()
    .map_err(|err| CouncilError::Hash(err.to_string()))?;

    let council_status = council_status_for_decision(request.decision_status);
    let decision = CouncilDecision {
        decision_id,
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        subject_kind: request.subject_kind,
        subject_id,
        requested_action: request.requested_action.clone(),
        approved_scope_hash,
        risk_class: request.risk_class,
        approver_did: request.approver_did.clone(),
        decision_source: request.decision_source,
        decision_status: request.decision_status,
        reason_code: request.reason_code.clone(),
        created_at,
        expires_at,
        receipt_hash,
        validation_report_id,
        route_id,
        context_packet_id,
        notes: notes.clone(),
    };
    let response = DagDbCouncilDecisionResponse {
        schema_version: DAGDB_COUNCIL_DECISION_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        decision_id: decision_id.to_string(),
        subject_kind: decision.subject_kind,
        subject_id: subject_id.to_string(),
        receipt_hash: receipt_hash.to_string(),
        validation_status: ValidationStatus::NeedsCouncil,
        council_status,
        decision_status: decision.decision_status,
        approved_scope_hash: approved_scope_hash.to_string(),
        risk_class: decision.risk_class,
        expires_at: expires_at.to_string(),
        created_new: true,
        validation_report_id: decision.validation_report_id.map(|hash| hash.to_string()),
        route_id: decision.route_id.map(|hash| hash.to_string()),
        context_packet_id: decision.context_packet_id.map(|hash| hash.to_string()),
        notes,
    };

    Ok(CouncilDecisionRecord {
        decision,
        response,
        event_body_hash,
    })
}

/// Build only the response for gateway callers that do not persist yet.
pub fn build_council_decision_response(
    request: DagDbCouncilDecisionRequest,
) -> Result<DagDbCouncilDecisionResponse> {
    build_council_decision_record(request).map(|record| record.response)
}

fn council_status_for_decision(status: CouncilDecisionStatus) -> CouncilReviewStatus {
    match status {
        CouncilDecisionStatus::Approved => CouncilReviewStatus::Approved,
        CouncilDecisionStatus::Denied | CouncilDecisionStatus::Revoked => {
            CouncilReviewStatus::Denied
        }
        CouncilDecisionStatus::Expired => CouncilReviewStatus::Expired,
        CouncilDecisionStatus::Escalated => CouncilReviewStatus::Escalated,
    }
}

fn validate_common_text(field: &'static str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(CouncilError::InvalidRequestShape(field));
    }
    Ok(())
}

fn validate_did(value: &str) -> Result<()> {
    if value.starts_with("did:") && value.len() > 4 {
        return Ok(());
    }
    Err(CouncilError::InvalidRequestShape("approver_did"))
}

fn parse_optional_hash(field: &'static str, value: Option<&str>) -> Result<Option<Hash256>> {
    value.map(|text| parse_hash256(field, text)).transpose()
}

fn parse_hlc(field: &'static str, value: &str) -> Result<Timestamp> {
    let (physical, logical) = value
        .split_once(':')
        .ok_or(CouncilError::InvalidRequestShape(field))?;
    let physical_ms = physical
        .parse::<u64>()
        .map_err(|_| CouncilError::InvalidRequestShape(field))?;
    let logical = logical
        .parse::<u32>()
        .map_err(|_| CouncilError::InvalidRequestShape(field))?;
    Ok(Timestamp::new(physical_ms, logical))
}

fn parse_hash256(field: &'static str, value: &str) -> Result<Hash256> {
    if value.len() != 64 {
        return Err(CouncilError::InvalidRequestShape(field));
    }
    let bytes = value.as_bytes();
    let mut out = [0u8; 32];
    for index in 0..32 {
        let high = hex_nibble(bytes[index * 2]).ok_or(CouncilError::InvalidRequestShape(field))?;
        let low =
            hex_nibble(bytes[index * 2 + 1]).ok_or(CouncilError::InvalidRequestShape(field))?;
        out[index] = (high << 4) | low;
    }
    Ok(Hash256::from_bytes(out))
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn hash_serializable<T: Serialize>(value: &T) -> std::result::Result<Hash256, String> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes).map_err(|err| err.to_string())?;
    Ok(Hash256::digest(&bytes))
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::{CouncilDecisionStatus, DecisionSource, RiskClass, SubjectKind};

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn request() -> DagDbCouncilDecisionRequest {
        DagDbCouncilDecisionRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "primary".to_owned(),
            idempotency_key: "idem-council-1".to_owned(),
            subject_kind: SubjectKind::Memory,
            subject_id: h(0xf0).to_string(),
            requested_action: "memory:routable".to_owned(),
            approved_scope_hash: h(0x12).to_string(),
            risk_class: RiskClass::R3,
            approver_did: "did:exo:council".to_owned(),
            decision_source: DecisionSource::Human,
            decision_status: CouncilDecisionStatus::Approved,
            reason_code: "operator_approved".to_owned(),
            created_at: "1000:0".to_owned(),
            expires_at: "2000:0".to_owned(),
            validation_report_id: None,
            route_id: None,
            context_packet_id: None,
            notes_text: Some("Safe approval notes".to_owned()),
        }
    }

    fn scope(record: &CouncilDecisionRecord) -> ApprovalScope {
        ApprovalScope {
            tenant_id: record.decision.tenant_id.clone(),
            namespace: record.decision.namespace.clone(),
            subject_kind: record.decision.subject_kind,
            subject_id: record.decision.subject_id,
            requested_action: record.decision.requested_action.clone(),
            approved_scope_hash: record.decision.approved_scope_hash,
            risk_class: record.decision.risk_class,
            council_decision_id: Some(record.decision.decision_id),
        }
    }

    #[test]
    fn council_decision_scope_vectors() {
        let record = build_council_decision_record(request()).expect("decision record");
        assert_eq!(record.decision.decision_source, DecisionSource::Human);
        assert_eq!(
            record.response.council_status,
            CouncilReviewStatus::Approved
        );
        assert_eq!(
            record.response.decision_id,
            record.decision.decision_id.to_string()
        );
        assert_eq!(
            record.response.receipt_hash,
            record.decision.receipt_hash.to_string()
        );
        assert_ne!(record.event_body_hash, Hash256::ZERO);
        let same_record = build_council_decision_record(request()).expect("replayed record");
        assert_eq!(same_record.response, record.response);

        let approval_scope = scope(&record);
        assert!(matches!(
            require_approval_for_risk(
                &approval_scope,
                Some(&record.decision),
                Timestamp::new(1500, 0)
            ),
            Ok(CouncilReviewStatus::Approved)
        ));
        assert!(matches!(
            require_approval_for_risk(
                &ApprovalScope {
                    risk_class: RiskClass::R2,
                    ..approval_scope.clone()
                },
                None,
                Timestamp::new(1500, 0),
            ),
            Ok(CouncilReviewStatus::NotRequired)
        ));
        assert!(matches!(
            require_approval_for_risk(&approval_scope, None, Timestamp::new(1500, 0)),
            Err(CouncilError::ApprovalRequired)
        ));
        assert!(matches!(
            require_approval_for_risk(
                &approval_scope,
                Some(&record.decision),
                Timestamp::new(2000, 0)
            ),
            Err(CouncilError::ApprovalRequired)
        ));

        let mut mismatched = approval_scope.clone();
        mismatched.requested_action = "memory:writeback".to_owned();
        assert!(matches!(
            require_approval_for_risk(&mismatched, Some(&record.decision), Timestamp::new(1500, 0)),
            Err(CouncilError::ApprovalScopeMismatch)
        ));
        assert!(matches!(
            validate_retry_after_approval(&mismatched, &record.decision, Timestamp::new(1500, 0)),
            Err(CouncilError::ApprovalScopeMismatch)
        ));
        assert!(
            validate_retry_after_approval(
                &approval_scope,
                &record.decision,
                Timestamp::new(1500, 0)
            )
            .is_ok()
        );

        let denied = decision_with_status(CouncilDecisionStatus::Denied);
        assert!(matches!(
            require_approval_for_risk(
                &approval_scope,
                Some(&denied.decision),
                Timestamp::new(1500, 0)
            ),
            Err(CouncilError::ApprovalDenied)
        ));
        let escalated = decision_with_status(CouncilDecisionStatus::Escalated);
        assert!(matches!(
            require_approval_for_risk(
                &approval_scope,
                Some(&escalated.decision),
                Timestamp::new(1500, 0)
            ),
            Err(CouncilError::CouncilEscalationRequired)
        ));
        let expired = decision_with_status(CouncilDecisionStatus::Expired);
        assert!(matches!(
            require_approval_for_risk(
                &approval_scope,
                Some(&expired.decision),
                Timestamp::new(1500, 0)
            ),
            Err(CouncilError::ApprovalRequired)
        ));

        let mut bad_window = request();
        bad_window.expires_at = "1000:0".to_owned();
        assert!(matches!(
            build_council_decision_record(bad_window),
            Err(CouncilError::InvalidRequestShape("expires_at"))
        ));

        let mut forged_notes = request();
        forged_notes.notes_text = Some("fn steal_customer_payload() {}".to_owned());
        assert!(matches!(
            build_council_decision_record(forged_notes),
            Err(CouncilError::Metadata(MetadataError::Rejected { .. }))
        ));
    }

    #[test]
    fn council_decision_validation_edges_are_fail_closed() {
        assert!(!risk_requires_council(RiskClass::R0));
        assert!(!risk_requires_council(RiskClass::R1));
        assert!(!risk_requires_council(RiskClass::R2));
        assert!(risk_requires_council(RiskClass::R3));
        assert!(risk_requires_council(RiskClass::R4));
        assert!(risk_requires_council(RiskClass::R5));

        let mut rich_request = request();
        rich_request.validation_report_id = Some(h(0x41).to_string());
        rich_request.route_id = Some(h(0x42).to_string());
        rich_request.context_packet_id = Some(h(0x43).to_string());
        rich_request.notes_text = None;
        let rich_record = build_council_decision_record(rich_request).expect("rich decision");
        assert_eq!(rich_record.decision.validation_report_id, Some(h(0x41)));
        assert_eq!(rich_record.decision.route_id, Some(h(0x42)));
        assert_eq!(rich_record.decision.context_packet_id, Some(h(0x43)));
        assert!(rich_record.decision.notes.is_none());

        let revoked = decision_with_status(CouncilDecisionStatus::Revoked);
        assert_eq!(revoked.response.council_status, CouncilReviewStatus::Denied);
        let revoked_scope = ApprovalScope {
            council_decision_id: None,
            ..scope(&revoked)
        };
        assert!(approval_scope_matches(&revoked_scope, &revoked.decision));
        assert!(matches!(
            require_approval_for_risk(
                &revoked_scope,
                Some(&revoked.decision),
                Timestamp::new(1500, 0)
            ),
            Err(CouncilError::ApprovalRequired)
        ));

        assert_invalid_shape("tenant_id", |request| request.tenant_id.clear());
        assert_invalid_shape("namespace", |request| request.namespace.clear());
        assert_invalid_shape("idempotency_key", |request| request.idempotency_key.clear());
        assert_invalid_shape("requested_action", |request| {
            request.requested_action.clear();
        });
        assert_invalid_shape("reason_code", |request| request.reason_code.clear());
        assert_invalid_shape("approver_did", |request| {
            request.approver_did = "not-a-did".to_owned();
        });
        assert_invalid_shape("subject_id", |request| {
            request.subject_id = "abcd".to_owned();
        });
        assert_invalid_shape("approved_scope_hash", |request| {
            request.approved_scope_hash =
                "ZZ12121212121212121212121212121212121212121212121212121212121212".to_owned();
        });
        assert_invalid_shape("validation_report_id", |request| {
            request.validation_report_id = Some("abcd".to_owned());
        });
        assert_invalid_shape("route_id", |request| {
            request.route_id =
                Some("42424242424242424242424242424242424242424242424242424242424242ZZ".to_owned());
        });
        assert_invalid_shape("context_packet_id", |request| {
            request.context_packet_id = Some("abcd".to_owned());
        });
        assert_invalid_shape("created_at", |request| {
            request.created_at = "1000".to_owned();
        });
        assert_invalid_shape("created_at", |request| {
            request.created_at = "x:0".to_owned();
        });
        assert_invalid_shape("expires_at", |request| {
            request.expires_at = "2000:x".to_owned();
        });
    }

    #[test]
    fn approval_scope_matcher_rejects_each_scope_field() {
        let record = build_council_decision_record(request()).expect("decision record");
        let valid_scope = scope(&record);
        assert!(approval_scope_matches(&valid_scope, &record.decision));

        assert_scope_mismatch(&record, |scope| scope.tenant_id = "tenant-b".to_owned());
        assert_scope_mismatch(&record, |scope| scope.namespace = "archive".to_owned());
        assert_scope_mismatch(&record, |scope| {
            scope.subject_kind = SubjectKind::Route;
        });
        assert_scope_mismatch(&record, |scope| scope.subject_id = h(0xee));
        assert_scope_mismatch(&record, |scope| {
            scope.requested_action = "memory:writeback".to_owned();
        });
        assert_scope_mismatch(&record, |scope| scope.approved_scope_hash = h(0xdd));
        assert_scope_mismatch(&record, |scope| scope.risk_class = RiskClass::R4);
        assert_scope_mismatch(&record, |scope| scope.council_decision_id = Some(h(0xcc)));

        let no_decision_id_scope = ApprovalScope {
            council_decision_id: None,
            ..valid_scope
        };
        assert!(approval_scope_matches(
            &no_decision_id_scope,
            &record.decision
        ));
    }

    #[test]
    fn approval_status_and_response_vectors_cover_all_branches() {
        let approved = decision_with_status(CouncilDecisionStatus::Approved);
        let approved_scope = scope(&approved);

        assert!(matches!(
            require_approval_for_risk(
                &ApprovalScope {
                    risk_class: RiskClass::R0,
                    ..approved_scope.clone()
                },
                Some(&approved.decision),
                Timestamp::new(1500, 0),
            ),
            Ok(CouncilReviewStatus::NotRequired)
        ));
        assert!(matches!(
            require_approval_for_risk(
                &approved_scope,
                Some(&approved.decision),
                Timestamp::new(1999, 1),
            ),
            Ok(CouncilReviewStatus::Approved)
        ));

        let expired_status = decision_with_status(CouncilDecisionStatus::Expired);
        assert_eq!(
            expired_status.response.council_status,
            CouncilReviewStatus::Expired
        );
        let escalated_status = decision_with_status(CouncilDecisionStatus::Escalated);
        assert_eq!(
            escalated_status.response.council_status,
            CouncilReviewStatus::Escalated
        );
    }

    fn decision_with_status(status: CouncilDecisionStatus) -> CouncilDecisionRecord {
        let mut input = request();
        input.decision_status = status;
        build_council_decision_record(input).expect("decision with status")
    }

    fn assert_scope_mismatch<F>(record: &CouncilDecisionRecord, mutate: F)
    where
        F: FnOnce(&mut ApprovalScope),
    {
        let mut invalid_scope = scope(record);
        mutate(&mut invalid_scope);
        assert!(!approval_scope_matches(&invalid_scope, &record.decision));
        assert!(matches!(
            require_approval_for_risk(
                &invalid_scope,
                Some(&record.decision),
                Timestamp::new(1500, 0),
            ),
            Err(CouncilError::ApprovalScopeMismatch)
        ));
    }

    fn assert_invalid_shape<F>(field: &'static str, mutate: F)
    where
        F: FnOnce(&mut DagDbCouncilDecisionRequest),
    {
        let mut invalid = request();
        mutate(&mut invalid);
        assert!(matches!(
            build_council_decision_record(invalid),
            Err(CouncilError::InvalidRequestShape(invalid_field)) if invalid_field == field
        ));
    }
}
