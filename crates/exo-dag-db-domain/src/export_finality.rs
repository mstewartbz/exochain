use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PRD17_EXPORT_FINALITY_SCHEMA_VERSION: &str = "dagdb_prd17_export_finality_record_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFinalityRecord {
    pub schema_version: String,
    pub export_id: String,
    pub artifact_digest: String,
    pub metadata_digest: String,
    pub receipt_id: String,
    pub local_outbox_ref: Option<String>,
    pub production_finality_ref: Option<String>,
    pub finality_state: String,
    pub reimport_id: String,
    pub retrieval_reuse_status: String,
    pub leakage_status: String,
    pub reimport_continuity: ReimportContinuity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReimportContinuity {
    pub memory_ids: Vec<String>,
    pub catalog_entry_ids: Vec<String>,
    pub graph_node_ids: Vec<String>,
    pub layer_membership_ids: Vec<String>,
    pub validation_report_ids: Vec<String>,
    pub citation_locator_ids: Vec<String>,
    pub provenance_preserved: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExportFinalityError {
    #[error("export finality schema_version is invalid")]
    SchemaVersion,
    #[error("{0} is missing")]
    MissingField(&'static str),
    #[error("{0} is unsafe")]
    UnsafeField(&'static str),
    #[error("{0} must be a lowercase sha256 hex digest")]
    InvalidDigest(&'static str),
    #[error("artifact digest does not match the export artifact")]
    ArtifactDigestMismatch,
    #[error("metadata digest does not match export metadata")]
    MetadataDigestMismatch,
    #[error("finality_state is invalid")]
    FinalityStateInvalid,
    #[error("local outbox acceptance requires a local_outbox_ref")]
    MissingLocalOutboxRef,
    #[error("production finality acceptance requires a production_finality_ref")]
    MissingProductionFinalityRef,
    #[error("operator-deferred finality cannot carry a production finality receipt")]
    OperatorDeferredHasProductionRef,
    #[error("reimport continuity is incomplete")]
    ReimportContinuityIncomplete,
    #[error("retrieval reuse did not pass")]
    RetrievalReuseInvalid,
    #[error("leakage status did not pass zero leakage")]
    LeakageStatusInvalid,
}

pub fn validate_export_finality_record(
    record: &ExportFinalityRecord,
    computed_artifact_digest: &str,
    computed_metadata_digest: &str,
) -> Result<(), ExportFinalityError> {
    if record.schema_version != PRD17_EXPORT_FINALITY_SCHEMA_VERSION {
        return Err(ExportFinalityError::SchemaVersion);
    }
    require_safe_id("export_id", &record.export_id)?;
    require_hex_digest("artifact_digest", &record.artifact_digest)?;
    require_hex_digest("metadata_digest", &record.metadata_digest)?;
    require_safe_id("receipt_id", &record.receipt_id)?;
    require_safe_id("reimport_id", &record.reimport_id)?;
    require_hex_digest("computed_artifact_digest", computed_artifact_digest)?;
    require_hex_digest("computed_metadata_digest", computed_metadata_digest)?;
    if record.artifact_digest != computed_artifact_digest {
        return Err(ExportFinalityError::ArtifactDigestMismatch);
    }
    if record.metadata_digest != computed_metadata_digest {
        return Err(ExportFinalityError::MetadataDigestMismatch);
    }

    match record.finality_state.as_str() {
        "local_outbox_accepted" => {
            validate_ref(
                record.local_outbox_ref.as_deref(),
                ExportFinalityError::MissingLocalOutboxRef,
            )?;
            if let Some(ref value) = record.production_finality_ref {
                validate_safe_ref("production_finality_ref", value)?;
            }
        }
        "production_finality_accepted" => {
            validate_ref(
                record.local_outbox_ref.as_deref(),
                ExportFinalityError::MissingLocalOutboxRef,
            )?;
            validate_ref(
                record.production_finality_ref.as_deref(),
                ExportFinalityError::MissingProductionFinalityRef,
            )?;
        }
        "operator_deferred" => {
            if record.production_finality_ref.is_some() {
                return Err(ExportFinalityError::OperatorDeferredHasProductionRef);
            }
            if let Some(ref value) = record.local_outbox_ref {
                validate_safe_ref("local_outbox_ref", value)?;
            }
        }
        "honest_blocked" => {
            if let Some(ref value) = record.local_outbox_ref {
                validate_safe_ref("local_outbox_ref", value)?;
            }
            if let Some(ref value) = record.production_finality_ref {
                validate_safe_ref("production_finality_ref", value)?;
            }
        }
        _ => return Err(ExportFinalityError::FinalityStateInvalid),
    }

    if record.retrieval_reuse_status != "retrieval_reuse_passed" {
        return Err(ExportFinalityError::RetrievalReuseInvalid);
    }
    if record.leakage_status != "passed_zero_leakage" {
        return Err(ExportFinalityError::LeakageStatusInvalid);
    }
    validate_reimport_continuity(&record.reimport_continuity)?;
    Ok(())
}

fn validate_reimport_continuity(
    continuity: &ReimportContinuity,
) -> Result<(), ExportFinalityError> {
    if !continuity.provenance_preserved {
        return Err(ExportFinalityError::ReimportContinuityIncomplete);
    }
    validate_id_list("memory_ids", &continuity.memory_ids)?;
    validate_id_list("catalog_entry_ids", &continuity.catalog_entry_ids)?;
    validate_id_list("graph_node_ids", &continuity.graph_node_ids)?;
    validate_id_list("layer_membership_ids", &continuity.layer_membership_ids)?;
    validate_id_list("validation_report_ids", &continuity.validation_report_ids)?;
    validate_id_list("citation_locator_ids", &continuity.citation_locator_ids)?;
    Ok(())
}

fn validate_id_list(field: &'static str, values: &[String]) -> Result<(), ExportFinalityError> {
    if values.is_empty() {
        return Err(ExportFinalityError::ReimportContinuityIncomplete);
    }
    let mut previous = "";
    for value in values {
        require_safe_id(field, value)?;
        if !previous.is_empty() && value.as_str() <= previous {
            return Err(ExportFinalityError::UnsafeField(field));
        }
        previous = value;
    }
    Ok(())
}

fn validate_ref(
    raw: Option<&str>,
    missing: ExportFinalityError,
) -> Result<(), ExportFinalityError> {
    let Some(value) = raw else {
        return Err(missing);
    };
    validate_safe_ref("finality_ref", value)
}

fn validate_safe_ref(field: &'static str, value: &str) -> Result<(), ExportFinalityError> {
    if value.trim().is_empty() {
        return Err(ExportFinalityError::MissingField(field));
    }
    if value.starts_with('/')
        || value.starts_with("~/")
        || value.contains('\\')
        || value.contains('\0')
        || value.split('/').any(|part| part.is_empty() || part == "..")
    {
        return Err(ExportFinalityError::UnsafeField(field));
    }
    Ok(())
}

fn require_safe_id(field: &'static str, value: &str) -> Result<(), ExportFinalityError> {
    if value.trim().is_empty() {
        return Err(ExportFinalityError::MissingField(field));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(ExportFinalityError::UnsafeField(field));
    }
    Ok(())
}

fn require_hex_digest(field: &'static str, value: &str) -> Result<(), ExportFinalityError> {
    if value.len() != 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    {
        return Err(ExportFinalityError::InvalidDigest(field));
    }
    Ok(())
}
