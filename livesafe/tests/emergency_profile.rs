use livesafe::emergency_profile::{
    DisclosureScope, EmergencyFieldBoundary, EmergencyProfileField, EmergencyProfileProjection,
    evaluate_emergency_profile, project_emergency_profile,
};

fn safe_field(field_name: &str) -> EmergencyProfileField {
    EmergencyProfileField {
        field_name: field_name.into(),
        value_ref: format!("synthetic:{field_name}"),
        contains_raw_payload: false,
        contains_direct_contact_data: false,
        contains_location_trace: false,
        contains_qr_secret: false,
        explicit_release_accepted: false,
        effective_date_ref: None,
        revocation_ref: None,
    }
}

#[test]
fn emergency_profile_fields_require_allowed_names_and_redacted_boundaries() {
    let mut unsupported = safe_field("unknown_field");
    unsupported.value_ref = String::new();

    let mut unsafe_contact = safe_field("emergency_contact_summary");
    unsafe_contact.contains_direct_contact_data = true;

    let mut unsafe_location = safe_field("preferred_language");
    unsafe_location.contains_location_trace = true;

    let mut unsafe_qr = safe_field("allergy_summary");
    unsafe_qr.contains_qr_secret = true;
    unsafe_qr.contains_raw_payload = true;

    let decision =
        evaluate_emergency_profile(&[unsupported, unsafe_contact, unsafe_location, unsafe_qr]);

    assert!(!decision.allowed);
    assert!(decision.reasons.contains(
        &"Emergency profile fields must use the approved LiveSafe field vocabulary.".into()
    ));
    assert!(
        decision.reasons.contains(
            &"Emergency profile fields require synthetic value references instead of inline data."
                .into()
        )
    );
    assert!(
        decision
            .reasons
            .contains(&"Emergency profile fixtures must not embed raw sensitive payloads.".into())
    );
    assert!(
        decision.reasons.contains(
            &"Emergency profile metadata must not contain direct contact details.".into()
        )
    );
    assert!(
        decision
            .reasons
            .contains(&"Emergency profile metadata must not contain location traces.".into())
    );
    assert!(
        decision.reasons.contains(
            &"Emergency profile metadata must not contain QR secrets or raw activation payloads."
                .into()
        )
    );
}

#[test]
fn responder_projection_allows_only_the_emergency_subset() {
    let preferred_name = safe_field("preferred_name");
    let blood_type = safe_field("blood_type");
    let directive = safe_field("medical_directive_summary");
    let pace = safe_field("pace_contact_summary");

    let denied = project_emergency_profile(EmergencyProfileProjection {
        scope: DisclosureScope::ResponderEmergency,
        fields: vec![
            preferred_name.clone(),
            blood_type.clone(),
            directive.clone(),
            pace,
        ],
        requested_field_names: vec![
            "preferred_name".into(),
            "blood_type".into(),
            "medical_directive_summary".into(),
            "pace_contact_summary".into(),
        ],
    });

    assert!(!denied.allowed);
    assert_eq!(
        denied.projected_field_names,
        vec!["preferred_name".to_string(), "blood_type".to_string()]
    );
    assert!(denied.reasons.contains(
        &"Responder emergency projection excludes fields outside the approved emergency subset.".into()
    ));
    assert!(denied.reasons.contains(
        &"Release-bound emergency profile fields require explicit acceptance, an effective-date reference, and a revocation reference.".into()
    ));
}

#[test]
fn expanded_responder_scope_stays_fail_closed_and_release_bound_fields_need_proof() {
    let mut directive = safe_field("medical_directive_summary");
    directive.explicit_release_accepted = true;
    directive.effective_date_ref = Some("directive:effective:2026-05-25".into());
    directive.revocation_ref = Some("directive:revocation:synthetic".into());

    let expanded = project_emergency_profile(EmergencyProfileProjection {
        scope: DisclosureScope::ResponderExpandedRequest,
        fields: vec![directive.clone()],
        requested_field_names: vec!["medical_directive_summary".into()],
    });

    assert!(!expanded.allowed);
    assert!(expanded.reasons.contains(
        &"Expanded responder disclosure remains blocked until Bob approves the live responder-access scope.".into()
    ));

    let release_bound = project_emergency_profile(EmergencyProfileProjection {
        scope: DisclosureScope::ResponderEmergency,
        fields: vec![directive],
        requested_field_names: vec!["medical_directive_summary".into()],
    });

    assert!(release_bound.allowed, "{release_bound:?}");
    assert_eq!(
        release_bound.boundary,
        EmergencyFieldBoundary::ReleaseBoundEmergency
    );
    assert_eq!(
        release_bound.projected_field_names,
        vec!["medical_directive_summary".to_string()]
    );
}
