//! Non-mutating M53 writeback hygiene proposal contracts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    kg_import::hash_from_hex,
    kg_writeback::{KgWritebackError, Result},
};

/// Non-mutating hygiene proposal schema used for M53 preparation.
pub const KG_WRITEBACK_HYGIENE_PROPOSAL_SCHEMA: &str = "dagdb_kg_writeback_hygiene_proposal_v1";

const FORBIDDEN_ROLLBACK_NOTE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "/tmp/",
    "~/.",
    "~/",
    "/var/folders/",
    "authorization",
    "begin private key",
    "database_url",
    ".env",
    "mongodb://",
    "mysql://",
    "password",
    "postgres://",
    "postgresql://",
    "private key-----",
    "raw_body",
    "raw_document_body",
    "raw_markdown",
    "raw_markdown_body",
    "raw_model_output",
    "raw_private_payload",
    "raw_prompt_body",
    "redis://",
    "secret",
    "sk-proj-",
    "sqlite://",
    "source_excerpt",
];

const RAW_ROLLBACK_NOTE_MARKERS: &[&str] = &[
    "agent transcript",
    "assistant:",
    "human:",
    "raw source body",
    "raw transcript",
    "user:",
    "<|im_start|>",
];

/// M53 hygiene case type prepared without mutating live records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KgWritebackHygieneCaseType {
    StaleImport,
    LowValueRecord,
}

/// Proposed M53 hygiene action. These actions are advisory until policy gates pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KgWritebackHygieneProposedAction {
    Relink,
    Supersede,
    Retain,
    Review,
}

/// Non-mutating M53 hygiene proposal bound to evidence and rollback notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackHygieneProposal {
    pub schema_version: String,
    pub case_id: String,
    pub record_id: String,
    pub case_type: KgWritebackHygieneCaseType,
    pub evidence_refs: Vec<String>,
    pub proposed_action: KgWritebackHygieneProposedAction,
    pub rollback_note: String,
    pub mutation_status: String,
    pub live_route_mutation: bool,
    pub live_deletion_approved: bool,
    pub exo_dag_promotion: bool,
}

impl KgWritebackHygieneProposal {
    /// Validate M53 hygiene evidence as proposed-only preparation.
    pub fn validate_proposed_only(&self) -> Result<()> {
        if self.schema_version != KG_WRITEBACK_HYGIENE_PROPOSAL_SCHEMA {
            return invalid_hint(format!(
                "unsupported hygiene schema_version: {}",
                self.schema_version
            ));
        }
        hash_from_hex("case_id", &self.case_id)?;
        hash_from_hex("record_id", &self.record_id)?;
        validate_evidence_refs(&self.evidence_refs)?;
        validate_rollback_note(&self.rollback_note)?;
        if self.mutation_status != "proposed_only" {
            return invalid_hint("mutation_status must be one of: proposed_only");
        }
        if self.live_route_mutation {
            return invalid_hint("hygiene proposal must not mutate live routes");
        }
        if self.live_deletion_approved {
            return invalid_hint("hygiene proposal must not approve live deletion");
        }
        if self.exo_dag_promotion {
            return invalid_hint("hygiene proposal must not promote exo-dag");
        }
        Ok(())
    }
}

fn validate_evidence_refs(values: &[String]) -> Result<()> {
    if values.is_empty() {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "hygiene proposal requires evidence_refs".to_owned(),
        });
    }

    let mut seen = BTreeSet::new();
    for value in values {
        if value.is_empty() {
            return invalid_hint("evidence_ref must not be empty");
        }
        hash_from_hex("evidence_ref", value)?;
        if !seen.insert(value) {
            return invalid_hint("duplicate evidence_ref");
        }
    }

    Ok(())
}

fn validate_rollback_note(value: &str) -> Result<()> {
    let note = value.trim();
    if note.is_empty() {
        return invalid_hint("rollback_note must not be empty");
    }

    let normalized = note.to_ascii_lowercase();
    reject_forbidden_rollback_note_material(&normalized)?;
    if is_placeholder_rollback_note(&normalized) {
        return invalid_hint("rollback_note must not be a placeholder");
    }
    if !names_retained_state(&normalized) {
        return invalid_hint("rollback_note must name retained prior route/candidate/live state");
    }
    if !names_evidence_basis(&normalized) {
        return invalid_hint("rollback_note must name evidence or receipt basis");
    }

    Ok(())
}

fn reject_forbidden_rollback_note_material(normalized: &str) -> Result<()> {
    if let Some(fragment) = FORBIDDEN_ROLLBACK_NOTE_FRAGMENTS
        .iter()
        .find(|fragment| normalized.contains(**fragment))
    {
        return invalid_hint(format!(
            "rollback_note contains forbidden fragment {fragment}"
        ));
    }

    if let Some(marker) = RAW_ROLLBACK_NOTE_MARKERS
        .iter()
        .find(|marker| normalized.contains(**marker))
    {
        return invalid_hint(format!("rollback_note contains raw source marker {marker}"));
    }

    Ok(())
}

fn is_placeholder_rollback_note(normalized: &str) -> bool {
    matches!(
        normalized,
        "tbd"
            | "todo"
            | "manual rollback later"
            | "rollback later"
            | "later"
            | "pending"
            | "none"
            | "n/a"
    )
}

fn names_retained_state(normalized: &str) -> bool {
    normalized.contains("prior route")
        || normalized.contains("prior candidate")
        || normalized.contains("retained route")
        || normalized.contains("retained candidate")
        || normalized.contains("retained live state")
        || normalized.contains("current live state")
}

fn names_evidence_basis(normalized: &str) -> bool {
    normalized.contains("evidence") || normalized.contains("receipt")
}

fn invalid_hint<T>(reason: impl Into<String>) -> Result<T> {
    Err(KgWritebackError::InvalidHint {
        reason: reason.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> String {
        format!("{byte:02x}").repeat(32)
    }

    fn proposal() -> KgWritebackHygieneProposal {
        KgWritebackHygieneProposal {
            schema_version: KG_WRITEBACK_HYGIENE_PROPOSAL_SCHEMA.to_owned(),
            case_id: h(0xa0),
            record_id: h(0xb0),
            case_type: KgWritebackHygieneCaseType::StaleImport,
            evidence_refs: vec![h(0xc0)],
            proposed_action: KgWritebackHygieneProposedAction::Relink,
            rollback_note:
                "retain current live state using existing evidence receipts until policy passes"
                    .to_owned(),
            mutation_status: "proposed_only".to_owned(),
            live_route_mutation: false,
            live_deletion_approved: false,
            exo_dag_promotion: false,
        }
    }

    #[test]
    fn kg_writeback_hygiene_cases_are_proposed_only_with_rollback_notes() {
        for (case_type, proposed_action, case_byte) in [
            (
                KgWritebackHygieneCaseType::StaleImport,
                KgWritebackHygieneProposedAction::Relink,
                0xa1,
            ),
            (
                KgWritebackHygieneCaseType::LowValueRecord,
                KgWritebackHygieneProposedAction::Retain,
                0xa2,
            ),
            (
                KgWritebackHygieneCaseType::LowValueRecord,
                KgWritebackHygieneProposedAction::Supersede,
                0xa3,
            ),
            (
                KgWritebackHygieneCaseType::StaleImport,
                KgWritebackHygieneProposedAction::Review,
                0xa4,
            ),
        ] {
            let mut proposal = proposal();
            proposal.case_id = h(case_byte);
            proposal.case_type = case_type;
            proposal.proposed_action = proposed_action;

            proposal
                .validate_proposed_only()
                .expect("proposal stays non-mutating");
        }
    }

    #[test]
    fn kg_writeback_hygiene_rejects_missing_and_bad_evidence() {
        let mut proposal = proposal();
        proposal.evidence_refs.clear();
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidEvidence { .. })
        ));

        proposal.evidence_refs = vec![String::new()];
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidHint { .. })
        ));

        proposal.evidence_refs = vec!["not-a-hash".to_owned()];
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::Hash { .. })
        ));

        proposal.evidence_refs = vec![h(0xc0), h(0xc0)];
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidHint { .. })
        ));
    }

    #[test]
    fn kg_writeback_hygiene_rejects_live_route_mutation() {
        let mut proposal = proposal();
        proposal.live_route_mutation = true;

        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidHint { .. })
        ));
    }

    #[test]
    fn kg_writeback_hygiene_rejects_live_deletion_approval() {
        let mut proposal = proposal();
        proposal.live_deletion_approved = true;

        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidHint { .. })
        ));
    }

    #[test]
    fn kg_writeback_hygiene_rejects_exo_dag_promotion() {
        let mut proposal = proposal();
        proposal.exo_dag_promotion = true;

        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidHint { .. })
        ));
    }

    #[test]
    fn kg_writeback_hygiene_rejects_bad_identity_and_mutation_status() {
        let mut proposal = proposal();
        proposal.schema_version = "dagdb_other_schema".to_owned();
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidHint { .. })
        ));

        proposal = self::proposal();
        proposal.case_id = "not-a-hash".to_owned();
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::Hash { .. })
        ));

        proposal = self::proposal();
        proposal.record_id = "not-a-hash".to_owned();
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::Hash { .. })
        ));

        proposal = self::proposal();
        proposal.mutation_status = "mutated".to_owned();
        assert!(matches!(
            proposal.validate_proposed_only(),
            Err(KgWritebackError::InvalidHint { .. })
        ));
    }

    #[test]
    fn kg_writeback_hygiene_rejects_placeholder_and_ambiguous_rollback_notes() {
        for rollback_note in [
            "",
            "   ",
            "TBD",
            "todo",
            "manual rollback later",
            "rollback later",
            "later",
            "pending",
            "none",
            "n/a",
            "use existing evidence only",
            "restore retained candidate state after policy passes",
        ] {
            let mut proposal = proposal();
            proposal.rollback_note = rollback_note.to_owned();

            assert!(matches!(
                proposal.validate_proposed_only(),
                Err(KgWritebackError::InvalidHint { .. })
            ));
        }
    }

    #[test]
    fn kg_writeback_hygiene_rejects_forbidden_rollback_note_material() {
        for rollback_note in [
            "retain current live state using evidence receipts at /Users/example/source.md",
            "retain current live state using evidence receipts and RAW_BODY payload",
            "retain current live state using evidence receipts from user: pasted transcript",
            "retain current live state using evidence receipts with sk-proj-example",
            "retain current live state using evidence receipts with Authorization header",
            "retain current live state using evidence receipts with password value",
            "retain current live state using evidence receipts with secret value",
            "retain current live state using evidence receipts from mysql://example/db",
            "retain current live state using evidence receipts from sqlite://example/db",
            "retain current live state using evidence receipts from mongodb://example/db",
            "retain current live state using evidence receipts from redis://example/db",
        ] {
            let mut proposal = proposal();
            proposal.rollback_note = rollback_note.to_owned();

            assert!(
                matches!(
                    proposal.validate_proposed_only(),
                    Err(KgWritebackError::InvalidHint { .. })
                ),
                "expected forbidden rollback_note rejection for {rollback_note}"
            );
        }
    }

    #[test]
    fn kg_writeback_hygiene_accepts_retained_prior_route_candidate_or_live_state_with_evidence() {
        for rollback_note in [
            "retain prior route state using evidence basis until policy passes",
            "retain prior candidate state using receipt basis until policy passes",
            "retain retained route state using evidence receipts until policy passes",
            "retain retained candidate state using evidence receipts until policy passes",
            "retain retained live state using evidence receipts until policy passes",
            "retain current live state using evidence receipts until policy passes",
        ] {
            let mut proposal = proposal();
            proposal.rollback_note = rollback_note.to_owned();

            proposal
                .validate_proposed_only()
                .expect("rollback note names state and evidence");
        }
    }
}
