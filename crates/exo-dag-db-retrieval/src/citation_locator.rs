use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PRD17_CITATION_LOCATOR_SCHEMA_VERSION: &str = "dagdb_prd17_citation_locator_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationLocator {
    pub schema_version: String,
    pub locator_id: String,
    pub source_id: String,
    pub source_digest: String,
    pub memory_id: String,
    pub span_ref: String,
    pub citation_text_hash: String,
    pub redaction_status: String,
    pub validation_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationLocatorInput<'a> {
    pub source_id: &'a str,
    pub source_digest: &'a str,
    pub memory_id: &'a str,
    pub span_ref: &'a str,
    pub citation_text: &'a str,
    pub redaction_status: &'a str,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CitationLocatorError {
    #[error("citation locator schema_version is invalid")]
    SchemaVersion,
    #[error("{0} is missing")]
    MissingField(&'static str),
    #[error("{0} is unsafe")]
    UnsafeField(&'static str),
    #[error("{0} must be a lowercase sha256 hex digest")]
    InvalidDigest(&'static str),
    #[error("citation locator id is not deterministic")]
    LocatorIdMismatch,
    #[error("citation locator is not redacted safe")]
    RedactionInvalid,
    #[error("citation locator validation_status is invalid")]
    ValidationStatusInvalid,
}

pub fn build_citation_locator(
    input: CitationLocatorInput<'_>,
) -> Result<CitationLocator, CitationLocatorError> {
    require_safe_id("source_id", input.source_id)?;
    require_hex_digest("source_digest", input.source_digest)?;
    require_safe_id("memory_id", input.memory_id)?;
    require_safe_span_ref(input.span_ref)?;
    require_non_empty("citation_text", input.citation_text)?;
    if input.redaction_status != "redacted_safe" && input.redaction_status != "public_safe" {
        return Err(CitationLocatorError::RedactionInvalid);
    }
    let citation_text_hash = hex_sha256(input.citation_text.as_bytes());
    let locator_id = deterministic_locator_id(
        input.source_id,
        input.source_digest,
        input.memory_id,
        input.span_ref,
        &citation_text_hash,
    );

    Ok(CitationLocator {
        schema_version: PRD17_CITATION_LOCATOR_SCHEMA_VERSION.to_owned(),
        locator_id,
        source_id: input.source_id.to_owned(),
        source_digest: input.source_digest.to_owned(),
        memory_id: input.memory_id.to_owned(),
        span_ref: input.span_ref.to_owned(),
        citation_text_hash,
        redaction_status: input.redaction_status.to_owned(),
        validation_status: "validated".to_owned(),
    })
}

pub fn validate_citation_locator(locator: &CitationLocator) -> Result<(), CitationLocatorError> {
    if locator.schema_version != PRD17_CITATION_LOCATOR_SCHEMA_VERSION {
        return Err(CitationLocatorError::SchemaVersion);
    }
    require_hex_digest("locator_id", &locator.locator_id)?;
    require_safe_id("source_id", &locator.source_id)?;
    require_hex_digest("source_digest", &locator.source_digest)?;
    require_safe_id("memory_id", &locator.memory_id)?;
    require_safe_span_ref(&locator.span_ref)?;
    require_hex_digest("citation_text_hash", &locator.citation_text_hash)?;
    if locator.redaction_status != "redacted_safe" && locator.redaction_status != "public_safe" {
        return Err(CitationLocatorError::RedactionInvalid);
    }
    if locator.validation_status != "validated" {
        return Err(CitationLocatorError::ValidationStatusInvalid);
    }
    let expected = deterministic_locator_id(
        &locator.source_id,
        &locator.source_digest,
        &locator.memory_id,
        &locator.span_ref,
        &locator.citation_text_hash,
    );
    if locator.locator_id != expected {
        return Err(CitationLocatorError::LocatorIdMismatch);
    }
    Ok(())
}

fn deterministic_locator_id(
    source_id: &str,
    source_digest: &str,
    memory_id: &str,
    span_ref: &str,
    citation_text_hash: &str,
) -> String {
    hex_sha256(
        format!("{source_id}\n{source_digest}\n{memory_id}\n{span_ref}\n{citation_text_hash}")
            .as_bytes(),
    )
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), CitationLocatorError> {
    if value.trim().is_empty() {
        return Err(CitationLocatorError::MissingField(field));
    }
    Ok(())
}

fn require_safe_id(field: &'static str, value: &str) -> Result<(), CitationLocatorError> {
    require_non_empty(field, value)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(CitationLocatorError::UnsafeField(field));
    }
    Ok(())
}

fn require_safe_span_ref(value: &str) -> Result<(), CitationLocatorError> {
    require_non_empty("span_ref", value)?;
    if value.starts_with('/')
        || value.starts_with("~/")
        || value.contains('\\')
        || value.contains('\0')
        || value.split('/').any(|part| part.is_empty() || part == "..")
    {
        return Err(CitationLocatorError::UnsafeField("span_ref"));
    }
    Ok(())
}

fn require_hex_digest(field: &'static str, value: &str) -> Result<(), CitationLocatorError> {
    if value.len() != 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    {
        return Err(CitationLocatorError::InvalidDigest(field));
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
