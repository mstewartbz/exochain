use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PRD17_DRIFT_REPAIR_SCHEMA_VERSION: &str = "dagdb_prd17_import_drift_repair_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftRepairRecord {
    pub schema_version: String,
    pub repair_id: String,
    pub source_id: String,
    pub old_digest: String,
    pub new_digest: String,
    pub old_digest_evidence_ref: String,
    pub affected_memory_ids: Vec<String>,
    pub repair_action: String,
    pub validation_status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftRepairInput<'a> {
    pub source_id: &'a str,
    pub old_digest: &'a str,
    pub new_digest: &'a str,
    pub old_digest_evidence_ref: &'a str,
    pub affected_memory_ids: Vec<&'a str>,
    pub repair_action: &'a str,
    pub created_at: &'a str,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DriftRepairError {
    #[error("drift repair schema_version is invalid")]
    SchemaVersion,
    #[error("{0} is missing")]
    MissingField(&'static str),
    #[error("{0} is unsafe")]
    UnsafeField(&'static str),
    #[error("{0} must be a lowercase sha256 hex digest")]
    InvalidDigest(&'static str),
    #[error("old and new source digests must differ")]
    DigestNotDrifted,
    #[error("old digest evidence ref is unsafe")]
    OldDigestEvidenceUnsafe,
    #[error("old digest evidence was not preserved")]
    OldDigestEvidenceMissing,
    #[error("affected memory ids contain a duplicate")]
    DuplicateAffectedMemory,
    #[error("affected memory ids must be sorted")]
    AffectedMemoryUnsorted,
    #[error("repair action is unsupported")]
    UnsupportedRepairAction,
    #[error("drift repair validation_status is invalid")]
    ValidationStatusInvalid,
    #[error("repair id is not deterministic")]
    RepairIdMismatch,
}

pub fn build_drift_repair_record(
    input: DriftRepairInput<'_>,
) -> Result<DriftRepairRecord, DriftRepairError> {
    require_safe_id("source_id", input.source_id)?;
    require_hex_digest("old_digest", input.old_digest)?;
    require_hex_digest("new_digest", input.new_digest)?;
    if input.old_digest == input.new_digest {
        return Err(DriftRepairError::DigestNotDrifted);
    }
    validate_old_digest_ref(input.old_digest_evidence_ref)?;
    validate_repair_action(input.repair_action)?;
    require_non_empty("created_at", input.created_at)?;

    let mut affected_memory_ids = Vec::new();
    let mut seen = BTreeSet::new();
    for memory_id in input.affected_memory_ids {
        require_safe_id("affected_memory_ids[]", memory_id)?;
        if !seen.insert(memory_id) {
            return Err(DriftRepairError::DuplicateAffectedMemory);
        }
        affected_memory_ids.push(memory_id.to_owned());
    }
    if affected_memory_ids.is_empty() {
        return Err(DriftRepairError::MissingField("affected_memory_ids"));
    }
    affected_memory_ids.sort();

    let repair_id = deterministic_repair_id(
        input.source_id,
        input.old_digest,
        input.new_digest,
        input.old_digest_evidence_ref,
        &affected_memory_ids,
        input.repair_action,
    );
    Ok(DriftRepairRecord {
        schema_version: PRD17_DRIFT_REPAIR_SCHEMA_VERSION.to_owned(),
        repair_id,
        source_id: input.source_id.to_owned(),
        old_digest: input.old_digest.to_owned(),
        new_digest: input.new_digest.to_owned(),
        old_digest_evidence_ref: input.old_digest_evidence_ref.to_owned(),
        affected_memory_ids,
        repair_action: input.repair_action.to_owned(),
        validation_status: "validated".to_owned(),
        created_at: input.created_at.to_owned(),
    })
}

pub fn validate_drift_repair_record(record: &DriftRepairRecord) -> Result<(), DriftRepairError> {
    if record.schema_version != PRD17_DRIFT_REPAIR_SCHEMA_VERSION {
        return Err(DriftRepairError::SchemaVersion);
    }
    require_hex_digest("repair_id", &record.repair_id)?;
    require_safe_id("source_id", &record.source_id)?;
    require_hex_digest("old_digest", &record.old_digest)?;
    require_hex_digest("new_digest", &record.new_digest)?;
    if record.old_digest == record.new_digest {
        return Err(DriftRepairError::DigestNotDrifted);
    }
    validate_old_digest_ref(&record.old_digest_evidence_ref)?;
    validate_repair_action(&record.repair_action)?;
    require_non_empty("created_at", &record.created_at)?;
    if record.validation_status != "validated" {
        return Err(DriftRepairError::ValidationStatusInvalid);
    }
    let mut previous = "";
    for memory_id in &record.affected_memory_ids {
        require_safe_id("affected_memory_ids[]", memory_id)?;
        if previous == memory_id {
            return Err(DriftRepairError::DuplicateAffectedMemory);
        }
        if !previous.is_empty() && memory_id.as_str() < previous {
            return Err(DriftRepairError::AffectedMemoryUnsorted);
        }
        previous = memory_id;
    }
    if record.affected_memory_ids.is_empty() {
        return Err(DriftRepairError::MissingField("affected_memory_ids"));
    }
    let expected = deterministic_repair_id(
        &record.source_id,
        &record.old_digest,
        &record.new_digest,
        &record.old_digest_evidence_ref,
        &record.affected_memory_ids,
        &record.repair_action,
    );
    if record.repair_id != expected {
        return Err(DriftRepairError::RepairIdMismatch);
    }
    Ok(())
}

fn deterministic_repair_id(
    source_id: &str,
    old_digest: &str,
    new_digest: &str,
    old_digest_evidence_ref: &str,
    affected_memory_ids: &[String],
    repair_action: &str,
) -> String {
    hex_sha256(
        format!(
            "{source_id}\n{old_digest}\n{new_digest}\n{old_digest_evidence_ref}\n{}\n{repair_action}",
            affected_memory_ids.join("\n")
        )
        .as_bytes(),
    )
}

fn validate_old_digest_ref(value: &str) -> Result<(), DriftRepairError> {
    require_non_empty("old_digest_evidence_ref", value)?;
    if value == "none" {
        return Err(DriftRepairError::OldDigestEvidenceMissing);
    }
    if value.starts_with('/')
        || value.starts_with("~/")
        || value.contains('\\')
        || value.contains('\0')
        || value.contains('\n')
        || value.contains('\r')
        || value.split('/').any(|part| part.is_empty() || part == "..")
    {
        return Err(DriftRepairError::OldDigestEvidenceUnsafe);
    }
    Ok(())
}

fn validate_repair_action(value: &str) -> Result<(), DriftRepairError> {
    require_safe_id("repair_action", value)?;
    match value {
        "catalog_locator_repair" | "citation_locator_repair" | "source_digest_repair" => Ok(()),
        _ => Err(DriftRepairError::UnsupportedRepairAction),
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), DriftRepairError> {
    if value.trim().is_empty() {
        return Err(DriftRepairError::MissingField(field));
    }
    Ok(())
}

fn require_safe_id(field: &'static str, value: &str) -> Result<(), DriftRepairError> {
    require_non_empty(field, value)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(DriftRepairError::UnsafeField(field));
    }
    Ok(())
}

fn require_hex_digest(field: &'static str, value: &str) -> Result<(), DriftRepairError> {
    if value.len() != 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    {
        return Err(DriftRepairError::InvalidDigest(field));
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
