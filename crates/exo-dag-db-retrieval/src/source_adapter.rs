use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PRD17_SOURCE_MANIFEST_SCHEMA_VERSION: &str = "dagdb_prd17_source_manifest_v1";

const SUPPORTED_ADAPTERS: [&str; 4] = [
    "document_bundle",
    "operator_external_bundle",
    "repo_file_bundle",
    "structured_table_bundle",
];

const ALLOWED_REDACTION_STATUSES: [&str; 2] = ["public_safe", "redacted_safe"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub schema_version: String,
    pub source_adapter_id: String,
    pub source_id: String,
    pub source_type: String,
    pub owner: String,
    pub digest: String,
    pub redaction_status: String,
    pub citation_policy: CitationPolicy,
    pub import_policy: ImportPolicy,
    pub export_policy: ExportPolicy,
    pub leakage_scope: String,
    pub tenant_id: String,
    pub project_id: String,
    pub memory_namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationPolicy {
    pub required: bool,
    pub locator_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPolicy {
    pub source_refs: Vec<String>,
    pub chunking_policy: String,
    pub placement_policy: String,
    pub duplicate_replay_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPolicy {
    pub exportable: bool,
    pub reimport_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAdapterAcceptance {
    pub adapter_id: String,
    pub source_id: String,
    pub source_digest: String,
    pub idempotency_key: String,
    pub citation_locator_required: bool,
    pub duplicate_replay_safe: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SourceAdapterError {
    #[error("source manifest schema_version is invalid")]
    SchemaVersion,
    #[error("source adapter is unsupported")]
    UnsupportedSourceType,
    #[error("{0} is missing")]
    MissingField(&'static str),
    #[error("{0} is unsafe")]
    UnsafeField(&'static str),
    #[error("source digest must be a lowercase sha256 hex digest")]
    InvalidDigest,
    #[error("citation policy is missing or disabled")]
    MissingCitationPolicy,
    #[error("private source is not redacted")]
    UnredactedPrivateSource,
    #[error("source ref contains path traversal or an absolute path")]
    PathTraversal,
    #[error("duplicate source ref")]
    DuplicateSourceRef,
    #[error("import replay is not duplicate-safe")]
    DuplicateReplayUnsafe,
    #[error("export/reimport policy is incomplete")]
    ExportPolicyIncomplete,
}

pub fn validate_source_manifest(
    manifest: &SourceManifest,
) -> Result<SourceAdapterAcceptance, SourceAdapterError> {
    if manifest.schema_version != PRD17_SOURCE_MANIFEST_SCHEMA_VERSION {
        return Err(SourceAdapterError::SchemaVersion);
    }
    require_safe_id("source_adapter_id", &manifest.source_adapter_id)?;
    require_safe_id("source_type", &manifest.source_type)?;
    if !SUPPORTED_ADAPTERS.contains(&manifest.source_adapter_id.as_str())
        || manifest.source_type != manifest.source_adapter_id
    {
        return Err(SourceAdapterError::UnsupportedSourceType);
    }

    require_safe_id("source_id", &manifest.source_id)?;
    require_non_empty("owner", &manifest.owner)?;
    require_hex_digest(&manifest.digest)?;
    require_safe_id("redaction_status", &manifest.redaction_status)?;
    require_safe_id("leakage_scope", &manifest.leakage_scope)?;
    require_safe_id("tenant_id", &manifest.tenant_id)?;
    require_safe_id("project_id", &manifest.project_id)?;
    require_safe_id("memory_namespace", &manifest.memory_namespace)?;

    if !ALLOWED_REDACTION_STATUSES.contains(&manifest.redaction_status.as_str()) {
        return Err(SourceAdapterError::UnredactedPrivateSource);
    }
    if manifest.leakage_scope == "private" && manifest.redaction_status != "redacted_safe" {
        return Err(SourceAdapterError::UnredactedPrivateSource);
    }
    if !manifest.citation_policy.required
        || manifest.citation_policy.locator_policy.trim().is_empty()
        || manifest.citation_policy.locator_policy == "none"
    {
        return Err(SourceAdapterError::MissingCitationPolicy);
    }
    require_safe_id(
        "citation_policy.locator_policy",
        &manifest.citation_policy.locator_policy,
    )?;
    require_safe_id(
        "import_policy.chunking_policy",
        &manifest.import_policy.chunking_policy,
    )?;
    require_safe_id(
        "import_policy.placement_policy",
        &manifest.import_policy.placement_policy,
    )?;
    validate_source_refs(&manifest.import_policy.source_refs)?;
    if !manifest.import_policy.duplicate_replay_safe {
        return Err(SourceAdapterError::DuplicateReplayUnsafe);
    }
    if !manifest.export_policy.exportable || !manifest.export_policy.reimport_required {
        return Err(SourceAdapterError::ExportPolicyIncomplete);
    }

    Ok(SourceAdapterAcceptance {
        adapter_id: manifest.source_adapter_id.clone(),
        source_id: manifest.source_id.clone(),
        source_digest: manifest.digest.clone(),
        idempotency_key: idempotency_key(manifest),
        citation_locator_required: true,
        duplicate_replay_safe: true,
    })
}

pub fn idempotency_key(manifest: &SourceManifest) -> String {
    let material = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        manifest.schema_version,
        manifest.source_adapter_id,
        manifest.source_id,
        manifest.digest,
        manifest.tenant_id,
        manifest.memory_namespace
    );
    hex_sha256(material.as_bytes())
}

fn validate_source_refs(source_refs: &[String]) -> Result<(), SourceAdapterError> {
    if source_refs.is_empty() {
        return Err(SourceAdapterError::MissingField(
            "import_policy.source_refs",
        ));
    }
    let mut seen = BTreeSet::new();
    for source_ref in source_refs {
        require_non_empty("import_policy.source_refs[]", source_ref)?;
        if source_ref.starts_with('/')
            || source_ref.starts_with("~/")
            || source_ref.contains('\\')
            || source_ref
                .split('/')
                .any(|part| part.is_empty() || part == "..")
            || source_ref.contains('\0')
        {
            return Err(SourceAdapterError::PathTraversal);
        }
        if !seen.insert(source_ref) {
            return Err(SourceAdapterError::DuplicateSourceRef);
        }
    }
    Ok(())
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), SourceAdapterError> {
    if value.trim().is_empty() {
        return Err(SourceAdapterError::MissingField(field));
    }
    Ok(())
}

fn require_safe_id(field: &'static str, value: &str) -> Result<(), SourceAdapterError> {
    require_non_empty(field, value)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(SourceAdapterError::UnsafeField(field));
    }
    Ok(())
}

fn require_hex_digest(value: &str) -> Result<(), SourceAdapterError> {
    if value.len() != 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    {
        return Err(SourceAdapterError::InvalidDigest);
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
