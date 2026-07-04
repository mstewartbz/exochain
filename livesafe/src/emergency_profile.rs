use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisclosureScope {
    ResponderEmergency,
    ResponderExpandedRequest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmergencyFieldBoundary {
    EmergencyCore,
    ReleaseBoundEmergency,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyProfileField {
    pub field_name: String,
    pub value_ref: String,
    pub contains_raw_payload: bool,
    pub contains_direct_contact_data: bool,
    pub contains_location_trace: bool,
    pub contains_qr_secret: bool,
    pub explicit_release_accepted: bool,
    pub effective_date_ref: Option<String>,
    pub revocation_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyProfileDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyProfileProjection {
    pub scope: DisclosureScope,
    pub fields: Vec<EmergencyProfileField>,
    pub requested_field_names: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyProfileProjectionDecision {
    pub allowed: bool,
    pub boundary: EmergencyFieldBoundary,
    pub projected_field_names: Vec<String>,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

const ALLOWED_FIELD_NAMES: [&str; 9] = [
    "preferred_name",
    "preferred_language",
    "blood_type",
    "allergy_summary",
    "medication_summary",
    "condition_summary",
    "medical_directive_summary",
    "emergency_contact_summary",
    "pace_contact_summary",
];

const CORE_EMERGENCY_FIELDS: [&str; 5] = [
    "preferred_name",
    "blood_type",
    "allergy_summary",
    "medication_summary",
    "condition_summary",
];

pub fn evaluate_emergency_profile(fields: &[EmergencyProfileField]) -> EmergencyProfileDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    for field in fields {
        if !ALLOWED_FIELD_NAMES.contains(&field.field_name.as_str()) {
            reasons.insert(
                "Emergency profile fields must use the approved LiveSafe field vocabulary."
                    .to_string(),
            );
            required_evidence.insert(
                "Approved emergency-profile field vocabulary mapped to current product meaning."
                    .to_string(),
            );
        }

        if field.value_ref.trim().is_empty() {
            reasons.insert(
                "Emergency profile fields require synthetic value references instead of inline data."
                    .to_string(),
            );
            required_evidence
                .insert("Synthetic reference for every emergency-profile field value.".to_string());
        }

        if field.contains_raw_payload {
            reasons.insert(
                "Emergency profile fixtures must not embed raw sensitive payloads.".to_string(),
            );
            required_evidence.insert(
                "Synthetic emergency-profile fixtures without raw medical, identity, or safety content."
                    .to_string(),
            );
        }

        if field.contains_direct_contact_data {
            reasons.insert(
                "Emergency profile metadata must not contain direct contact details.".to_string(),
            );
            required_evidence.insert(
                "Redacted or referenced contact summaries rather than direct contact details."
                    .to_string(),
            );
        }

        if field.contains_location_trace {
            reasons
                .insert("Emergency profile metadata must not contain location traces.".to_string());
            required_evidence.insert(
                "Location data excluded from emergency-profile fixtures and metadata.".to_string(),
            );
        }

        if field.contains_qr_secret {
            reasons.insert(
                "Emergency profile metadata must not contain QR secrets or raw activation payloads."
                    .to_string(),
            );
            required_evidence.insert(
                "QR activation metadata kept out of the emergency-profile field contract."
                    .to_string(),
            );
        }
    }

    EmergencyProfileDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

pub fn project_emergency_profile(
    projection: EmergencyProfileProjection,
) -> EmergencyProfileProjectionDecision {
    let validation = evaluate_emergency_profile(&projection.fields);
    let mut reasons: BTreeSet<String> = validation.reasons.into_iter().collect();
    let mut required_evidence: BTreeSet<String> =
        validation.required_evidence.into_iter().collect();
    let indexed_fields: BTreeMap<&str, &EmergencyProfileField> = projection
        .fields
        .iter()
        .map(|field| (field.field_name.as_str(), field))
        .collect();
    let mut projected_field_names = Vec::new();
    let mut boundary = EmergencyFieldBoundary::Denied;

    if projection.scope == DisclosureScope::ResponderExpandedRequest {
        reasons.insert(
            "Expanded responder disclosure remains blocked until Bob approves the live responder-access scope."
                .to_string(),
        );
        required_evidence.insert(
            "Bob-approved responder-access scope before any expanded responder disclosure."
                .to_string(),
        );
    }

    for requested_field_name in &projection.requested_field_names {
        let Some(field) = indexed_fields.get(requested_field_name.as_str()) else {
            reasons.insert(
                "Emergency-profile projection requests must reference fields present in the validated profile."
                    .to_string(),
            );
            required_evidence.insert(
                "Projection request field names bound to the validated emergency-profile inventory."
                    .to_string(),
            );
            continue;
        };

        if projection.scope == DisclosureScope::ResponderExpandedRequest {
            continue;
        }

        if CORE_EMERGENCY_FIELDS.contains(&requested_field_name.as_str()) {
            projected_field_names.push(requested_field_name.clone());
            if boundary == EmergencyFieldBoundary::Denied {
                boundary = EmergencyFieldBoundary::EmergencyCore;
            }
            continue;
        }

        if is_release_bound_field(requested_field_name.as_str()) {
            if release_requirements_met(field) {
                projected_field_names.push(requested_field_name.clone());
                boundary = EmergencyFieldBoundary::ReleaseBoundEmergency;
            } else {
                reasons.insert(
                    "Release-bound emergency profile fields require explicit acceptance, an effective-date reference, and a revocation reference."
                        .to_string(),
                );
                required_evidence.insert(
                    "Release acceptance, effective-date reference, and revocation reference for every release-bound emergency field."
                        .to_string(),
                );
            }
            continue;
        }

        reasons.insert(
            "Responder emergency projection excludes fields outside the approved emergency subset."
                .to_string(),
        );
        required_evidence.insert(
            "Approved responder emergency subset with explicit exclusions for P.A.C.E. and contact-detail fields."
                .to_string(),
        );
    }

    EmergencyProfileProjectionDecision {
        allowed: reasons.is_empty(),
        boundary,
        projected_field_names,
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn is_release_bound_field(field_name: &str) -> bool {
    matches!(field_name, "medical_directive_summary")
}

fn release_requirements_met(field: &EmergencyProfileField) -> bool {
    field.explicit_release_accepted
        && !empty_option(&field.effective_date_ref)
        && !empty_option(&field.revocation_ref)
}

fn empty_option(value: &Option<String>) -> bool {
    value
        .as_ref()
        .map(|current| current.trim().is_empty())
        .unwrap_or(true)
}
