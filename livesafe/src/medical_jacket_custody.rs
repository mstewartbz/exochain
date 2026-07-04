use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MedicalJacketRecordClass {
    Phenotypical,
    GenotypicalImport,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConsentScope {
    Custody,
    EmergencyProjection,
    Export,
    GenotypicalImport,
    TrialMatching,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MedicalJacketRecord {
    pub record_ref: String,
    pub class: MedicalJacketRecordClass,
    pub encrypted_blob_ref: String,
    pub contains_raw_payload: bool,
    pub consent_scopes: Vec<ConsentScope>,
    pub custody_receipt_ref: Option<String>,
    pub external_source_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MedicalJacketDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionRequest {
    pub records: Vec<MedicalJacketRecord>,
    pub requested_record_refs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionDecision {
    pub allowed: bool,
    pub projected_record_refs: Vec<String>,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialMatchingRequest {
    pub enabled: bool,
    pub opted_in: bool,
    pub data_class_validated: bool,
    pub eligibility_contract_passed: bool,
}

pub fn evaluate_medical_jacket_records(records: &[MedicalJacketRecord]) -> MedicalJacketDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    for record in records {
        if record.record_ref.trim().is_empty() {
            reasons
                .insert("Medical jacket records require a synthetic record reference.".to_string());
            required_evidence
                .insert("Synthetic record references for every medical-jacket record.".to_string());
        }

        if record.encrypted_blob_ref.trim().is_empty() {
            reasons.insert(
                "Medical jacket records require an encrypted blob reference before custody evaluation."
                    .to_string(),
            );
            required_evidence.insert(
                "Encrypted blob reference for each phenotypical or genotypical record.".to_string(),
            );
        }

        if record.contains_raw_payload {
            reasons.insert(
                "Medical jacket custody records must not embed raw sensitive payloads in fixtures or metadata."
                    .to_string(),
            );
            required_evidence.insert(
                "Synthetic fixtures with opaque references instead of raw medical or genetic payloads."
                    .to_string(),
            );
        }

        if !record.consent_scopes.contains(&ConsentScope::Custody) {
            reasons.insert(
                "Medical jacket custody evaluation requires explicit custody consent.".to_string(),
            );
            required_evidence
                .insert("Custody-consent state for each medical-jacket record class.".to_string());
        }

        match record.class {
            MedicalJacketRecordClass::Phenotypical => {
                if empty_option(&record.custody_receipt_ref) {
                    reasons.insert(
                        "Phenotypical custody records require a custody receipt reference."
                            .to_string(),
                    );
                    required_evidence.insert(
                        "Custody receipt reference for each phenotypical medical record class."
                            .to_string(),
                    );
                }
            }
            MedicalJacketRecordClass::GenotypicalImport => {
                if !record
                    .consent_scopes
                    .contains(&ConsentScope::GenotypicalImport)
                {
                    reasons.insert(
                        "Genotypical imports require explicit import consent before custody, export, or matching."
                            .to_string(),
                    );
                    required_evidence.insert(
                        "Separate genotypical import consent and source classification."
                            .to_string(),
                    );
                }

                if empty_option(&record.external_source_ref) {
                    reasons.insert(
                        "Genotypical imports require an external-source reference.".to_string(),
                    );
                    required_evidence.insert(
                        "Synthetic source reference for each genotypical import.".to_string(),
                    );
                }
            }
        }
    }

    decision(reasons, required_evidence)
}

pub fn evaluate_emergency_projection(request: ProjectionRequest) -> ProjectionDecision {
    let validation = evaluate_medical_jacket_records(&request.records);
    let mut reasons: BTreeSet<String> = validation.reasons.into_iter().collect();
    let mut required_evidence: BTreeSet<String> =
        validation.required_evidence.into_iter().collect();
    let indexed_records: BTreeMap<&str, &MedicalJacketRecord> = request
        .records
        .iter()
        .map(|record| (record.record_ref.as_str(), record))
        .collect();
    let mut projected_record_refs = Vec::new();

    for requested_ref in &request.requested_record_refs {
        let Some(record) = indexed_records.get(requested_ref.as_str()) else {
            reasons.insert(
                "Emergency projection requests must reference existing medical-jacket records."
                    .to_string(),
            );
            required_evidence.insert(
                "Requested emergency projection references bound to the medical-jacket inventory."
                    .to_string(),
            );
            continue;
        };

        if record.class != MedicalJacketRecordClass::Phenotypical {
            reasons.insert(
                "Emergency projection is limited to phenotypical medical-jacket records."
                    .to_string(),
            );
            required_evidence.insert(
                "Emergency projection rules separating phenotypical records from genotypical imports."
                    .to_string(),
            );
            continue;
        }

        if !record
            .consent_scopes
            .contains(&ConsentScope::EmergencyProjection)
        {
            reasons.insert(
                "Emergency projection requires explicit emergency-projection consent for every requested record."
                    .to_string(),
            );
            required_evidence.insert(
                "Emergency-projection consent for each projected phenotypical record.".to_string(),
            );
            continue;
        }

        projected_record_refs.push(requested_ref.clone());
    }

    ProjectionDecision {
        allowed: reasons.is_empty(),
        projected_record_refs,
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

pub fn evaluate_trial_matching(request: TrialMatchingRequest) -> MedicalJacketDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if !request.enabled {
        reasons.insert(
            "Precision-medicine clinical-trial matching remains inactive until explicitly enabled."
                .to_string(),
        );
        required_evidence
            .insert("Explicit product activation decision for trial matching.".to_string());
    }

    if !request.opted_in {
        reasons.insert(
            "Precision-medicine clinical-trial matching remains inactive until the subscriber explicitly opts in."
                .to_string(),
        );
        required_evidence.insert("Subscriber opt-in state for trial matching.".to_string());
    }

    if !request.data_class_validated {
        reasons.insert(
            "Precision-medicine clinical-trial matching requires validated phenotypical and genotypical data classes."
                .to_string(),
        );
        required_evidence
            .insert("Validated phenotypical and genotypical classification evidence.".to_string());
    }

    if !request.eligibility_contract_passed {
        reasons.insert(
            "Precision-medicine clinical-trial matching requires a passing eligibility contract before activation."
                .to_string(),
        );
        required_evidence
            .insert("Eligibility-contract result for trial matching activation.".to_string());
    }

    decision(reasons, required_evidence)
}

fn decision(
    reasons: BTreeSet<String>,
    required_evidence: BTreeSet<String>,
) -> MedicalJacketDecision {
    MedicalJacketDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn empty_option(value: &Option<String>) -> bool {
    value
        .as_ref()
        .map(|current| current.trim().is_empty())
        .unwrap_or(true)
}
