//! Economy bindings: HonorGood object validation and deterministic anchors.

use exo_core::Hash256;
use exo_economy::{
    EconomyObjectKind, EconomyRecordAnchor, HonorGoodRuleset, LegacyReceipt, Mission,
    ValueContributionNode,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::serde_bridge::{from_json_str, to_js_value};

const HASH256_HEX_LEN: usize = 64;
const EXOCHAIN_SETTLEMENT_AUTHORITY: &str = "EXOCHAIN";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WasmEconomyAnchor<T> {
    pub settlement_authority: &'static str,
    pub local_settlement_authority: bool,
    pub object: T,
    pub anchor: EconomyRecordAnchor,
}

fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}

fn parse_anchor_hash_hex(value: &str) -> Result<Hash256, JsValue> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Hash256::ZERO);
    }
    if trimmed.len() != HASH256_HEX_LEN {
        return Err(js_error("previous anchor hash must be 64 hex characters"));
    }
    let bytes = hex::decode(trimmed).map_err(|_| js_error("previous anchor hash must be hex"))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| js_error("previous anchor hash must decode to 32 bytes"))?;
    Ok(Hash256::from_bytes(array))
}

fn economy_anchor<T: Serialize>(
    object: T,
    object_kind: EconomyObjectKind,
    object_id: Hash256,
    object_hash: Hash256,
    created_at: exo_core::Timestamp,
    previous_anchor_hash: Hash256,
) -> Result<WasmEconomyAnchor<T>, exo_economy::EconomyError> {
    let anchor = EconomyRecordAnchor {
        anchor_hash: Hash256::ZERO,
        previous_anchor_hash,
        object_kind,
        object_id,
        object_hash,
        created_at,
    }
    .anchor()?;

    Ok(WasmEconomyAnchor {
        settlement_authority: EXOCHAIN_SETTLEMENT_AUTHORITY,
        local_settlement_authority: false,
        object,
        anchor,
    })
}

pub(crate) fn mission_anchor(
    mission: Mission,
    previous_anchor_hash: Hash256,
) -> Result<WasmEconomyAnchor<Mission>, exo_economy::EconomyError> {
    let anchored = mission.anchor()?;
    economy_anchor(
        anchored.clone(),
        EconomyObjectKind::Mission,
        anchored.mission_id,
        anchored.content_hash,
        anchored.created_at,
        previous_anchor_hash,
    )
}

pub(crate) fn legacy_receipt_anchor(
    receipt: LegacyReceipt,
    previous_anchor_hash: Hash256,
) -> Result<WasmEconomyAnchor<LegacyReceipt>, exo_economy::EconomyError> {
    let anchored = receipt.anchor()?;
    economy_anchor(
        anchored.clone(),
        EconomyObjectKind::LegacyReceipt,
        anchored.legacy_receipt_id,
        anchored.content_hash,
        anchored.created_at,
        previous_anchor_hash,
    )
}

pub(crate) fn ruleset_anchor(
    ruleset: HonorGoodRuleset,
    previous_anchor_hash: Hash256,
) -> Result<WasmEconomyAnchor<HonorGoodRuleset>, exo_economy::EconomyError> {
    let anchored = ruleset.anchor()?;
    economy_anchor(
        anchored.clone(),
        EconomyObjectKind::HonorGoodRuleset,
        anchored.ruleset_id,
        anchored.content_hash,
        anchored.created_at,
        previous_anchor_hash,
    )
}

pub(crate) fn value_contribution_node_anchor(
    node: ValueContributionNode,
    previous_anchor_hash: Hash256,
) -> Result<WasmEconomyAnchor<ValueContributionNode>, exo_economy::EconomyError> {
    let anchored = node.anchor()?;
    economy_anchor(
        anchored.clone(),
        EconomyObjectKind::ValueContributionNode,
        anchored.contribution_node_id,
        anchored.content_hash,
        anchored.created_at_hlc,
        previous_anchor_hash,
    )
}

#[wasm_bindgen]
pub fn wasm_anchor_economy_mission(
    mission_json: &str,
    previous_anchor_hash_hex: &str,
) -> Result<JsValue, JsValue> {
    let mission: Mission = from_json_str(mission_json)?;
    let previous = parse_anchor_hash_hex(previous_anchor_hash_hex)?;
    let report = mission_anchor(mission, previous)
        .map_err(|err| js_error(&format!("economy mission rejected: {err}")))?;
    to_js_value(&report)
}

#[wasm_bindgen]
pub fn wasm_anchor_economy_legacy_receipt(
    legacy_receipt_json: &str,
    previous_anchor_hash_hex: &str,
) -> Result<JsValue, JsValue> {
    let receipt: LegacyReceipt = from_json_str(legacy_receipt_json)?;
    let previous = parse_anchor_hash_hex(previous_anchor_hash_hex)?;
    let report = legacy_receipt_anchor(receipt, previous)
        .map_err(|err| js_error(&format!("economy legacy receipt rejected: {err}")))?;
    to_js_value(&report)
}

#[wasm_bindgen]
pub fn wasm_anchor_economy_ruleset(
    ruleset_json: &str,
    previous_anchor_hash_hex: &str,
) -> Result<JsValue, JsValue> {
    let ruleset: HonorGoodRuleset = from_json_str(ruleset_json)?;
    let previous = parse_anchor_hash_hex(previous_anchor_hash_hex)?;
    let report = ruleset_anchor(ruleset, previous)
        .map_err(|err| js_error(&format!("economy ruleset rejected: {err}")))?;
    to_js_value(&report)
}

#[wasm_bindgen]
pub fn wasm_anchor_economy_value_contribution_node(
    node_json: &str,
    previous_anchor_hash_hex: &str,
) -> Result<JsValue, JsValue> {
    let node: ValueContributionNode = from_json_str(node_json)?;
    let previous = parse_anchor_hash_hex(previous_anchor_hash_hex)?;
    let report = value_contribution_node_anchor(node, previous)
        .map_err(|err| js_error(&format!("economy value contribution node rejected: {err}")))?;
    to_js_value(&report)
}

#[cfg(test)]
mod tests {
    use exo_core::{Did, Timestamp};
    use exo_economy::{
        MissionPurpose, MissionStatus, MissionType, ParticipantRef, ValueContributionStatus,
    };

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn did(label: &str) -> Did {
        Did::new(&format!("did:exo:{label}")).unwrap()
    }

    fn sample_mission() -> Mission {
        Mission {
            mission_id: Hash256::ZERO,
            name: "HonorGood WASM mission".into(),
            mission_type: MissionType::UpstreamRecognition,
            owner_did: did("owner"),
            principal_did: did("principal"),
            purpose: MissionPurpose {
                problem: "adapter validation".into(),
                served_party: "EXOCHAIN".into(),
                promised_outcome: "deterministic anchor".into(),
                expected_value: "stable hash".into(),
                risk_surface: "WASM boundary".into(),
                proof_required: "canonical CBOR hash".into(),
                success_condition: "same input same anchor".into(),
            },
            related_platforms: vec!["EXOCHAIN".into()],
            expected_value_micro_exo: None,
            ruleset_id: h(0x11),
            status: MissionStatus::Active,
            created_at: Timestamp::new(42_000, 0),
            content_hash: Hash256::ZERO,
        }
    }

    fn sample_node() -> ValueContributionNode {
        ValueContributionNode {
            contribution_node_id: Hash256::ZERO,
            contributor_ref: ParticipantRef::ProjectTreasury {
                project: "Archon".into(),
                treasury_ref: "public-project-treasury:Archon".into(),
            },
            contributor_type: exo_economy::ContributorType::Project,
            contribution_name: "Archon".into(),
            contribution_type: exo_economy::ContributionType::Code,
            source_uri: Some("https://github.com/coleam00/Archon".into()),
            evidence_hash: h(0x21),
            provenance_hash: h(0x22),
            license_or_compact_ref: "MIT".into(),
            honor_good_terms_hash: h(0x23),
            bailment_terms_hash: h(0x24),
            settlement_ruleset_id: h(0x25),
            beneficiary_ref: ParticipantRef::ProjectTreasury {
                project: "Archon".into(),
                treasury_ref: "public-project-treasury:Archon".into(),
            },
            materiality_policy_id: h(0x26),
            adoption_policy_id: h(0x27),
            revocation_policy_id: h(0x28),
            dispute_policy_id: h(0x29),
            status: ValueContributionStatus::Active,
            created_at_hlc: Timestamp::new(43_000, 0),
            content_hash: Hash256::ZERO,
        }
    }

    #[test]
    fn mission_anchor_is_deterministic_and_non_authoritative_locally() {
        let first = mission_anchor(sample_mission(), Hash256::ZERO).unwrap();
        let second = mission_anchor(sample_mission(), Hash256::ZERO).unwrap();

        assert_eq!(first.object.mission_id, second.object.mission_id);
        assert_eq!(first.anchor.anchor_hash, second.anchor.anchor_hash);
        assert_eq!(first.anchor.object_kind, EconomyObjectKind::Mission);
        assert_eq!(first.settlement_authority, "EXOCHAIN");
        assert!(!first.local_settlement_authority);
    }

    #[test]
    fn value_contribution_node_anchor_rejects_invalid_input() {
        let mut node = sample_node();
        node.evidence_hash = Hash256::ZERO;

        assert!(value_contribution_node_anchor(node, Hash256::ZERO).is_err());
    }
}
