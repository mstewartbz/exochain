//! Deterministic similarity overlay scoring.

use std::collections::BTreeSet;

use exo_core::Hash256;
use exo_dag_db_api::{SimilarityResult, SimilarityType};

/// Inputs compared before canonicalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimilarityInput {
    pub candidate_memory_id: Hash256,
    pub candidate_payload_hash: Hash256,
    pub candidate_summary: String,
    pub matched_memory_id: Hash256,
    pub matched_payload_hash: Hash256,
    pub matched_summary: String,
}

/// Compare a candidate with an existing memory node using integer basis points.
#[must_use]
pub fn compare_similarity(input: &SimilarityInput) -> SimilarityResult {
    if input.candidate_payload_hash == input.matched_payload_hash {
        return SimilarityResult {
            candidate_memory_id: input.matched_memory_id.to_string(),
            similarity_type: SimilarityType::ExactHash,
            similarity_bp: 10_000,
            matched_fields: vec!["payload_hash".into()],
            reason: "payload hashes match exactly".into(),
        };
    }

    let candidate_tokens = normalized_tokens(&input.candidate_summary);
    let matched_tokens = normalized_tokens(&input.matched_summary);
    let overlap_bp = overlap_basis_points(&candidate_tokens, &matched_tokens);
    if overlap_bp >= 8_500 {
        return SimilarityResult {
            candidate_memory_id: input.matched_memory_id.to_string(),
            similarity_type: SimilarityType::NearDuplicate,
            similarity_bp: overlap_bp,
            matched_fields: vec!["summary".into()],
            reason: "summary token overlap is above near-duplicate threshold".into(),
        };
    }
    if overlap_bp >= 5_000 {
        return SimilarityResult {
            candidate_memory_id: input.matched_memory_id.to_string(),
            similarity_type: SimilarityType::ConceptOverlap,
            similarity_bp: overlap_bp,
            matched_fields: vec!["summary".into()],
            reason: "summary token overlap indicates a related concept".into(),
        };
    }
    SimilarityResult {
        candidate_memory_id: input.matched_memory_id.to_string(),
        similarity_type: SimilarityType::WeakRelated,
        similarity_bp: overlap_bp,
        matched_fields: Vec::new(),
        reason: "weak token overlap".into(),
    }
}

/// Compare a candidate against existing memory in deterministic order.
#[must_use]
pub fn compare_all_similarity(inputs: &[SimilarityInput]) -> Vec<SimilarityResult> {
    let mut results: Vec<SimilarityResult> = inputs.iter().map(compare_similarity).collect();
    results.sort_by(|left, right| {
        right
            .similarity_bp
            .cmp(&left.similarity_bp)
            .then_with(|| left.candidate_memory_id.cmp(&right.candidate_memory_id))
            .then_with(|| left.reason.cmp(&right.reason))
    });
    results
}

fn normalized_tokens(input: &str) -> BTreeSet<String> {
    input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn overlap_basis_points(left: &BTreeSet<String>, right: &BTreeSet<String>) -> u16 {
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let shared = left.intersection(right).count();
    let denominator = left.len().min(right.len());
    u16::try_from(shared.saturating_mul(10_000) / denominator).unwrap_or(10_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    #[test]
    fn similarity_precedes_canonicalization_vectors() {
        let exact = compare_similarity(&SimilarityInput {
            candidate_memory_id: h(0x01),
            candidate_payload_hash: h(0x10),
            candidate_summary: "EXOCHAIN graph routing".into(),
            matched_memory_id: h(0x02),
            matched_payload_hash: h(0x10),
            matched_summary: "unrelated".into(),
        });
        assert_eq!(exact.similarity_type, SimilarityType::ExactHash);
        assert_eq!(exact.similarity_bp, 10_000);

        let near = compare_similarity(&SimilarityInput {
            candidate_memory_id: h(0x01),
            candidate_payload_hash: h(0x11),
            candidate_summary: "graph routing canonical memory".into(),
            matched_memory_id: h(0x03),
            matched_payload_hash: h(0x12),
            matched_summary: "canonical graph routing memory".into(),
        });
        assert_eq!(near.similarity_type, SimilarityType::NearDuplicate);
        assert_eq!(near.similarity_bp, 10_000);

        let related = compare_similarity(&SimilarityInput {
            candidate_memory_id: h(0x01),
            candidate_payload_hash: h(0x13),
            candidate_summary: "graph routing canonical memory".into(),
            matched_memory_id: h(0x04),
            matched_payload_hash: h(0x14),
            matched_summary: "routing graph planner".into(),
        });
        assert_eq!(related.similarity_type, SimilarityType::ConceptOverlap);
        assert!(related.similarity_bp >= 5_000);
    }

    #[test]
    fn similarity_vectors_cover_weak_empty_and_sorted_results() {
        let weak = compare_similarity(&SimilarityInput {
            candidate_memory_id: h(0x01),
            candidate_payload_hash: h(0x20),
            candidate_summary: "alpha beta".into(),
            matched_memory_id: h(0x05),
            matched_payload_hash: h(0x21),
            matched_summary: "gamma delta".into(),
        });
        assert_eq!(weak.similarity_type, SimilarityType::WeakRelated);
        assert_eq!(weak.similarity_bp, 0);

        let empty = compare_similarity(&SimilarityInput {
            candidate_memory_id: h(0x01),
            candidate_payload_hash: h(0x22),
            candidate_summary: String::new(),
            matched_memory_id: h(0x06),
            matched_payload_hash: h(0x23),
            matched_summary: "gamma".into(),
        });
        assert_eq!(empty.similarity_type, SimilarityType::WeakRelated);
        assert_eq!(empty.similarity_bp, 0);

        let sorted = compare_all_similarity(&[
            SimilarityInput {
                candidate_memory_id: h(0x01),
                candidate_payload_hash: h(0x24),
                candidate_summary: "graph routing canonical memory".into(),
                matched_memory_id: h(0x09),
                matched_payload_hash: h(0x25),
                matched_summary: "graph routing".into(),
            },
            SimilarityInput {
                candidate_memory_id: h(0x01),
                candidate_payload_hash: h(0x24),
                candidate_summary: "graph routing canonical memory".into(),
                matched_memory_id: h(0x08),
                matched_payload_hash: h(0x26),
                matched_summary: "graph routing".into(),
            },
        ]);
        assert_eq!(sorted[0].candidate_memory_id, h(0x08).to_string());
        assert_eq!(sorted[1].candidate_memory_id, h(0x09).to_string());
    }
}
