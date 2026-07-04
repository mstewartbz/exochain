use livesafe::medical_jacket_custody::{
    ConsentScope, MedicalJacketRecord, MedicalJacketRecordClass, ProjectionRequest,
    TrialMatchingRequest, evaluate_emergency_projection, evaluate_medical_jacket_records,
    evaluate_trial_matching,
};

fn phenotypical_record(record_ref: &str) -> MedicalJacketRecord {
    MedicalJacketRecord {
        record_ref: record_ref.into(),
        class: MedicalJacketRecordClass::Phenotypical,
        encrypted_blob_ref: "vault:blob:synthetic".into(),
        contains_raw_payload: false,
        consent_scopes: vec![ConsentScope::Custody, ConsentScope::EmergencyProjection],
        custody_receipt_ref: Some("custody:receipt:synthetic".into()),
        external_source_ref: None,
    }
}

#[test]
fn medical_jacket_records_require_safe_classification_and_custody_metadata() {
    let mut unsafe_phenotypical = phenotypical_record("record:phenotypical");
    unsafe_phenotypical.custody_receipt_ref = None;
    unsafe_phenotypical.contains_raw_payload = true;

    let genotypical_without_import_consent = MedicalJacketRecord {
        record_ref: "record:genotypical".into(),
        class: MedicalJacketRecordClass::GenotypicalImport,
        encrypted_blob_ref: "vault:blob:genotypical".into(),
        contains_raw_payload: false,
        consent_scopes: vec![ConsentScope::Custody],
        custody_receipt_ref: Some("custody:receipt:genotypical".into()),
        external_source_ref: None,
    };

    let decision =
        evaluate_medical_jacket_records(&[unsafe_phenotypical, genotypical_without_import_consent]);

    assert!(!decision.allowed);
    assert!(decision.reasons.contains(
        &"Medical jacket custody records must not embed raw sensitive payloads in fixtures or metadata.".into()
    ));
    assert!(
        decision
            .reasons
            .contains(&"Phenotypical custody records require a custody receipt reference.".into())
    );
    assert!(decision.reasons.contains(
        &"Genotypical imports require explicit import consent before custody, export, or matching.".into()
    ));
    assert!(
        decision
            .reasons
            .contains(&"Genotypical imports require an external-source reference.".into())
    );
}

#[test]
fn emergency_projection_allows_only_authorized_phenotypical_records() {
    let allowed_record = phenotypical_record("record:allowed");

    let mut no_projection_consent = phenotypical_record("record:no-projection");
    no_projection_consent
        .consent_scopes
        .retain(|scope| *scope != ConsentScope::EmergencyProjection);

    let genotypical_record = MedicalJacketRecord {
        record_ref: "record:genotypical".into(),
        class: MedicalJacketRecordClass::GenotypicalImport,
        encrypted_blob_ref: "vault:blob:genotypical".into(),
        contains_raw_payload: false,
        consent_scopes: vec![ConsentScope::Custody, ConsentScope::GenotypicalImport],
        custody_receipt_ref: Some("custody:receipt:genotypical".into()),
        external_source_ref: Some("import:lab:synthetic".into()),
    };

    let denied = evaluate_emergency_projection(ProjectionRequest {
        records: vec![
            allowed_record.clone(),
            no_projection_consent,
            genotypical_record,
        ],
        requested_record_refs: vec![
            "record:allowed".into(),
            "record:no-projection".into(),
            "record:genotypical".into(),
        ],
    });

    assert!(!denied.allowed);
    assert!(denied.reasons.contains(
        &"Emergency projection requires explicit emergency-projection consent for every requested record.".into()
    ));
    assert!(denied.reasons.contains(
        &"Emergency projection is limited to phenotypical medical-jacket records.".into()
    ));

    let allowed = evaluate_emergency_projection(ProjectionRequest {
        records: vec![allowed_record],
        requested_record_refs: vec!["record:allowed".into()],
    });

    assert!(allowed.allowed, "{allowed:?}");
    assert_eq!(
        allowed.projected_record_refs,
        vec!["record:allowed".to_string()]
    );
}

#[test]
fn trial_matching_remains_inactive_until_opt_in_and_eligibility_contracts_pass() {
    let denied = evaluate_trial_matching(TrialMatchingRequest {
        enabled: true,
        opted_in: false,
        data_class_validated: false,
        eligibility_contract_passed: false,
    });

    assert!(!denied.allowed);
    assert!(denied.reasons.contains(
        &"Precision-medicine clinical-trial matching remains inactive until the subscriber explicitly opts in.".into()
    ));
    assert!(denied.reasons.contains(
        &"Precision-medicine clinical-trial matching requires validated phenotypical and genotypical data classes.".into()
    ));
    assert!(denied.reasons.contains(
        &"Precision-medicine clinical-trial matching requires a passing eligibility contract before activation.".into()
    ));

    let allowed = evaluate_trial_matching(TrialMatchingRequest {
        enabled: true,
        opted_in: true,
        data_class_validated: true,
        eligibility_contract_passed: true,
    });

    assert!(allowed.allowed, "{allowed:?}");
}
