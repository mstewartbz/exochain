use sha2::{Digest, Sha256};

pub const GOV_RULES: [&str; 13] = [
    "GOV-001: constitutional supremacy over probabilistic and neural layers",
    "GOV-002: explicit authority chain required for terminal decisions",
    "GOV-003: quorum must be met with unique signers",
    "GOV-004: audit continuity is mandatory for terminal states",
    "GOV-005: sync version mismatch blocks approval",
    "GOV-006: delegated authority may expire",
    "GOV-007: conflicts must be disclosed before voting",
    "GOV-008: policy and sovereignty decisions require ratification",
    "GOV-009: humans retain majority control over AI participants",
    "GOV-010: advanced reasoning remains optional and advisory",
    "GOV-011: verifier mismatch forces escalation/rejection",
    "GOV-012: evidence references must be auditable",
    "GOV-013: threshold changes are governance acts",
];

pub const LEG_RULES: [&str; 13] = [
    "LEG-001: no false compliance claims in generated artifacts",
    "LEG-002: fiduciary reports must distinguish summary from legal opinion",
    "LEG-003: signatures and records must be attributable",
    "LEG-004: timestamps must be retained for auditability",
    "LEG-005: ratification records must identify human approvers",
    "LEG-006: conflict disclosures are part of the governance record",
    "LEG-007: expired delegations are not legally operative",
    "LEG-008: duplicated signatures do not increase quorum",
    "LEG-009: constitutional references must be hash-bound",
    "LEG-010: evidence identifiers must be reproducible",
    "LEG-011: advanced AI assistance does not replace human accountability",
    "LEG-012: rejected decisions retain their audit trail",
    "LEG-013: governance receipts must avoid unsupported evidentiary claims",
];

pub fn canonical_constitution_text() -> String {
    GOV_RULES
        .iter()
        .chain(LEG_RULES.iter())
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn constitution_hash() -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_constitution_text().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constitution_catalog_is_not_stub() {
        assert_eq!(GOV_RULES.len(), 13);
        assert_eq!(LEG_RULES.len(), 13);
        assert!(canonical_constitution_text().contains("GOV-001"));
        assert!(canonical_constitution_text().contains("LEG-013"));
    }

    #[test]
    fn test_constitution_hash_is_stable_length() {
        let hash = constitution_hash();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|ch| ch.is_ascii_hexdigit()));
use super::decision_object::DecisionClass;

/// Reference to the governing constitution and its governance parameters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstitutionRef {
    pub hash: String,
    pub version: String,
    pub human_gate_classes: Vec<DecisionClass>,
    pub max_delegation_depth: u32,
    pub ai_ceiling_class: DecisionClass,
}

/// Returns a sensible default constitution for genesis / testing.
pub fn default_constitution() -> ConstitutionRef {
    ConstitutionRef {
        hash: "genesis-constitution-hash".to_string(),
        version: "1.0.0".to_string(),
        human_gate_classes: vec![DecisionClass::Strategic, DecisionClass::Constitutional],
        max_delegation_depth: 5,
        ai_ceiling_class: DecisionClass::Operational,
    }
}
