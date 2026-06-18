//! Deterministic canonicalization decisions for memory graph placement.

use exo_core::Hash256;
use exo_dag_db_api::{
    CanonicalizationDecision, CanonicalizationDecisionKind, GraphEdgeRef, MemoryEdgeKind,
    RiskClass, SimilarityResult, SimilarityType, ValidationStatus,
};
use serde::Serialize;

use crate::scoring::{DomainResult, hash_event_body};

const NEAR_DUPLICATE_BP: u16 = 8_500;
const RELATED_BP: u16 = 5_000;

/// Canonicalization request after similarity overlay scoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalizationRequest {
    pub input_memory_id: Hash256,
    pub risk_class: RiskClass,
    pub validator_status: ValidationStatus,
    pub similarity_results: Vec<SimilarityResult>,
    pub requested_decision: Option<CanonicalizationDecisionKind>,
}

#[derive(Debug, Serialize)]
struct DecisionIdMaterial<'a> {
    input_memory_id: Hash256,
    decision_kind: CanonicalizationDecisionKind,
    matched_memory_ids: &'a [String],
    confidence_bp: u16,
}

/// Decide how incoming memory relates to the current canonical graph.
pub fn decide_canonicalization(
    request: CanonicalizationRequest,
) -> DomainResult<CanonicalizationDecision> {
    let best_similarity = request.similarity_results.first();
    let decision_kind = request
        .requested_decision
        .unwrap_or_else(|| infer_decision(best_similarity));
    let matched_memory_ids = matched_memory_ids(&request.similarity_results, decision_kind);
    let canonical_memory_id = canonical_memory_id(&request.similarity_results, decision_kind);
    let confidence_bp = confidence_bp(best_similarity, decision_kind);
    let required_edges_to_create = required_edges(
        request.input_memory_id,
        canonical_memory_id.as_deref(),
        decision_kind,
    );
    let decision_id = hash_event_body(&DecisionIdMaterial {
        input_memory_id: request.input_memory_id,
        decision_kind,
        matched_memory_ids: &matched_memory_ids,
        confidence_bp,
    })?;
    Ok(CanonicalizationDecision {
        decision_id: decision_id.to_string(),
        input_memory_id: request.input_memory_id.to_string(),
        canonical_memory_id,
        matched_memory_ids,
        decision_kind,
        decision_reason: decision_reason(decision_kind).into(),
        confidence_bp,
        risk_class: request.risk_class,
        validator_status: request.validator_status,
        required_edges_to_create,
        receipt_intent: "canonicalization_decided".into(),
        receipt_id: None,
    })
}

fn infer_decision(best_similarity: Option<&SimilarityResult>) -> CanonicalizationDecisionKind {
    let Some(best) = best_similarity else {
        return CanonicalizationDecisionKind::NewCanonical;
    };
    match best.similarity_type {
        SimilarityType::ExactHash => CanonicalizationDecisionKind::ExactDuplicate,
        SimilarityType::NearDuplicate if best.similarity_bp >= NEAR_DUPLICATE_BP => {
            CanonicalizationDecisionKind::NearDuplicate
        }
        SimilarityType::ConceptOverlap if best.similarity_bp >= RELATED_BP => {
            CanonicalizationDecisionKind::Related
        }
        SimilarityType::WeakRelated
        | SimilarityType::ConceptOverlap
        | SimilarityType::NearDuplicate => CanonicalizationDecisionKind::NewCanonical,
    }
}

fn result_supports_decision(
    result: &SimilarityResult,
    decision_kind: CanonicalizationDecisionKind,
) -> bool {
    match decision_kind {
        CanonicalizationDecisionKind::NewCanonical
        | CanonicalizationDecisionKind::RejectedNeedsReview => false,
        CanonicalizationDecisionKind::ExactDuplicate => {
            result.similarity_type == SimilarityType::ExactHash
        }
        CanonicalizationDecisionKind::NearDuplicate => match result.similarity_type {
            SimilarityType::ExactHash => true,
            SimilarityType::NearDuplicate => result.similarity_bp >= NEAR_DUPLICATE_BP,
            SimilarityType::ConceptOverlap | SimilarityType::WeakRelated => false,
        },
        CanonicalizationDecisionKind::Related
        | CanonicalizationDecisionKind::Replacement
        | CanonicalizationDecisionKind::Contradiction
        | CanonicalizationDecisionKind::Supersession
        | CanonicalizationDecisionKind::AlternateSummary => match result.similarity_type {
            SimilarityType::ExactHash => true,
            SimilarityType::NearDuplicate => result.similarity_bp >= NEAR_DUPLICATE_BP,
            SimilarityType::ConceptOverlap => result.similarity_bp >= RELATED_BP,
            SimilarityType::WeakRelated => false,
        },
    }
}

fn matched_memory_ids(
    similarity_results: &[SimilarityResult],
    decision_kind: CanonicalizationDecisionKind,
) -> Vec<String> {
    let mut matched: Vec<String> = similarity_results
        .iter()
        .filter(|result| result_supports_decision(result, decision_kind))
        .map(|result| result.candidate_memory_id.clone())
        .collect();
    matched.sort();
    matched.dedup();
    matched
}

fn canonical_memory_id(
    similarity_results: &[SimilarityResult],
    decision_kind: CanonicalizationDecisionKind,
) -> Option<String> {
    similarity_results
        .iter()
        .find(|result| result_supports_decision(result, decision_kind))
        .map(|result| result.candidate_memory_id.clone())
}

fn confidence_bp(
    best_similarity: Option<&SimilarityResult>,
    decision_kind: CanonicalizationDecisionKind,
) -> u16 {
    match decision_kind {
        CanonicalizationDecisionKind::NewCanonical => 10_000,
        CanonicalizationDecisionKind::RejectedNeedsReview => 0,
        CanonicalizationDecisionKind::Replacement
        | CanonicalizationDecisionKind::Contradiction
        | CanonicalizationDecisionKind::Supersession
        | CanonicalizationDecisionKind::AlternateSummary => {
            best_similarity.map_or(7_500, |similarity| similarity.similarity_bp)
        }
        CanonicalizationDecisionKind::ExactDuplicate
        | CanonicalizationDecisionKind::NearDuplicate
        | CanonicalizationDecisionKind::Related => {
            best_similarity.map_or(0, |similarity| similarity.similarity_bp)
        }
    }
}

fn required_edges(
    input_memory_id: Hash256,
    canonical_memory_id: Option<&str>,
    decision_kind: CanonicalizationDecisionKind,
) -> Vec<GraphEdgeRef> {
    let Some(to_memory_id) = canonical_memory_id else {
        return Vec::new();
    };
    let edge_kind = match decision_kind {
        CanonicalizationDecisionKind::ExactDuplicate => MemoryEdgeKind::DuplicateOf,
        CanonicalizationDecisionKind::NearDuplicate => MemoryEdgeKind::NearDuplicateOf,
        CanonicalizationDecisionKind::Related => MemoryEdgeKind::RelatedTo,
        CanonicalizationDecisionKind::Replacement => MemoryEdgeKind::Replaces,
        CanonicalizationDecisionKind::Contradiction => MemoryEdgeKind::Contradicts,
        CanonicalizationDecisionKind::Supersession => MemoryEdgeKind::Supersedes,
        CanonicalizationDecisionKind::AlternateSummary => MemoryEdgeKind::AlternativeSummaryOf,
        CanonicalizationDecisionKind::NewCanonical
        | CanonicalizationDecisionKind::RejectedNeedsReview => return Vec::new(),
    };
    vec![GraphEdgeRef {
        from_memory_id: input_memory_id.to_string(),
        to_memory_id: to_memory_id.to_owned(),
        edge_kind,
    }]
}

fn decision_reason(decision_kind: CanonicalizationDecisionKind) -> &'static str {
    match decision_kind {
        CanonicalizationDecisionKind::NewCanonical => "new canonical memory node",
        CanonicalizationDecisionKind::ExactDuplicate => "exact duplicate links to canonical memory",
        CanonicalizationDecisionKind::NearDuplicate => "near duplicate links to canonical memory",
        CanonicalizationDecisionKind::Related => "related concept links to canonical memory",
        CanonicalizationDecisionKind::Replacement => "replacement preserves prior node history",
        CanonicalizationDecisionKind::Contradiction => "contradiction is surfaced",
        CanonicalizationDecisionKind::Supersession => "supersession preserves prior node history",
        CanonicalizationDecisionKind::AlternateSummary => {
            "alternate summary links to canonical memory"
        }
        CanonicalizationDecisionKind::RejectedNeedsReview => "unsafe canonicalization needs review",
    }
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::SimilarityType;

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn similarity(
        memory_id: Hash256,
        similarity_type: SimilarityType,
        bp: u16,
    ) -> SimilarityResult {
        SimilarityResult {
            candidate_memory_id: memory_id.to_string(),
            similarity_type,
            similarity_bp: bp,
            matched_fields: vec!["summary".into()],
            reason: "fixture".into(),
        }
    }

    #[test]
    fn exact_duplicate_creates_duplicate_reference() {
        let decision = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x10),
            risk_class: RiskClass::R1,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![similarity(h(0x20), SimilarityType::ExactHash, 10_000)],
            requested_decision: None,
        })
        .expect("decision");
        assert_eq!(
            decision.decision_kind,
            CanonicalizationDecisionKind::ExactDuplicate
        );
        assert_eq!(
            decision.required_edges_to_create[0].edge_kind,
            MemoryEdgeKind::DuplicateOf
        );
        assert_eq!(decision.canonical_memory_id, Some(h(0x20).to_string()));
    }

    #[test]
    fn exact_duplicate_links_to_best_match_not_lexicographic_minimum() {
        let best = h(0xff);
        let weak = h(0x00);
        let decision = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x60),
            risk_class: RiskClass::R1,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![
                similarity(best, SimilarityType::ExactHash, 10_000),
                similarity(weak, SimilarityType::WeakRelated, 0),
            ],
            requested_decision: None,
        })
        .expect("decision");
        assert_eq!(
            decision.decision_kind,
            CanonicalizationDecisionKind::ExactDuplicate
        );
        assert_eq!(decision.canonical_memory_id, Some(best.to_string()));
        assert_eq!(decision.matched_memory_ids, vec![best.to_string()]);
        assert_eq!(
            decision.required_edges_to_create[0].to_memory_id,
            best.to_string()
        );
    }

    #[test]
    fn canonicalization_decision_vectors() {
        for (kind, edge) in [
            (
                CanonicalizationDecisionKind::Replacement,
                MemoryEdgeKind::Replaces,
            ),
            (
                CanonicalizationDecisionKind::Contradiction,
                MemoryEdgeKind::Contradicts,
            ),
            (
                CanonicalizationDecisionKind::Supersession,
                MemoryEdgeKind::Supersedes,
            ),
            (
                CanonicalizationDecisionKind::AlternateSummary,
                MemoryEdgeKind::AlternativeSummaryOf,
            ),
        ] {
            let decision = decide_canonicalization(CanonicalizationRequest {
                input_memory_id: h(0x11),
                risk_class: RiskClass::R2,
                validator_status: ValidationStatus::Passed,
                similarity_results: vec![similarity(
                    h(0x21),
                    SimilarityType::ConceptOverlap,
                    7_500,
                )],
                requested_decision: Some(kind),
            })
            .expect("decision");
            assert_eq!(decision.decision_kind, kind);
            assert_eq!(decision.required_edges_to_create[0].edge_kind, edge);
            assert!(decision.receipt_intent.contains("canonicalization"));
        }

        let rejected = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x12),
            risk_class: RiskClass::R5,
            validator_status: ValidationStatus::NeedsCouncil,
            similarity_results: Vec::new(),
            requested_decision: Some(CanonicalizationDecisionKind::RejectedNeedsReview),
        })
        .expect("decision");
        assert!(rejected.required_edges_to_create.is_empty());
        assert_eq!(rejected.confidence_bp, 0);
    }

    #[test]
    fn canonicalization_inference_edges_cover_thresholds() {
        let new = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x30),
            risk_class: RiskClass::R0,
            validator_status: ValidationStatus::Passed,
            similarity_results: Vec::new(),
            requested_decision: None,
        })
        .expect("new canonical");
        assert_eq!(
            new.decision_kind,
            CanonicalizationDecisionKind::NewCanonical
        );
        assert_eq!(new.confidence_bp, 10_000);
        assert!(new.required_edges_to_create.is_empty());

        let near_below_threshold = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x31),
            risk_class: RiskClass::R0,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![similarity(h(0x41), SimilarityType::NearDuplicate, 8_499)],
            requested_decision: None,
        })
        .expect("near below threshold");
        assert_eq!(
            near_below_threshold.decision_kind,
            CanonicalizationDecisionKind::NewCanonical
        );

        let near_at_threshold = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x34),
            risk_class: RiskClass::R0,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![similarity(h(0x44), SimilarityType::NearDuplicate, 8_500)],
            requested_decision: None,
        })
        .expect("near at threshold");
        assert_eq!(
            near_at_threshold.decision_kind,
            CanonicalizationDecisionKind::NearDuplicate
        );
        assert_eq!(
            near_at_threshold.required_edges_to_create[0].edge_kind,
            MemoryEdgeKind::NearDuplicateOf
        );

        let related_at_threshold = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x32),
            risk_class: RiskClass::R0,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![similarity(h(0x42), SimilarityType::ConceptOverlap, 5_000)],
            requested_decision: None,
        })
        .expect("related at threshold");
        assert_eq!(
            related_at_threshold.decision_kind,
            CanonicalizationDecisionKind::Related
        );
        assert_eq!(
            related_at_threshold.required_edges_to_create[0].edge_kind,
            MemoryEdgeKind::RelatedTo
        );

        let concept_below_threshold = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x35),
            risk_class: RiskClass::R0,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![similarity(h(0x45), SimilarityType::ConceptOverlap, 4_999)],
            requested_decision: None,
        })
        .expect("concept below threshold");
        assert_eq!(
            concept_below_threshold.decision_kind,
            CanonicalizationDecisionKind::NewCanonical
        );

        let weak = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x33),
            risk_class: RiskClass::R0,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![similarity(h(0x43), SimilarityType::WeakRelated, 4_999)],
            requested_decision: None,
        })
        .expect("weak related");
        assert_eq!(
            weak.decision_kind,
            CanonicalizationDecisionKind::NewCanonical
        );
    }

    #[test]
    fn requested_decisions_without_similarity_fail_closed_to_review_material() {
        let replacement = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x50),
            risk_class: RiskClass::R3,
            validator_status: ValidationStatus::NeedsCouncil,
            similarity_results: Vec::new(),
            requested_decision: Some(CanonicalizationDecisionKind::Replacement),
        })
        .expect("replacement without similarity");
        assert_eq!(
            replacement.decision_kind,
            CanonicalizationDecisionKind::Replacement
        );
        assert_eq!(replacement.confidence_bp, 7_500);
        assert!(replacement.canonical_memory_id.is_none());
        assert!(replacement.matched_memory_ids.is_empty());
        assert!(replacement.required_edges_to_create.is_empty());

        let related = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(0x51),
            risk_class: RiskClass::R1,
            validator_status: ValidationStatus::Passed,
            similarity_results: Vec::new(),
            requested_decision: Some(CanonicalizationDecisionKind::Related),
        })
        .expect("related without similarity");
        assert_eq!(related.confidence_bp, 0);
        assert!(related.required_edges_to_create.is_empty());
    }
}
