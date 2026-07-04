use livesafe::entitlement_marketplace::{
    BillingBinding, CapabilityCode, CommercialEntitlement, EntitlementPlan, FrontlineCohort,
    FrontlineEligibility, FrontlineVerificationMethod, GiftState, LIVESAFE_MARKETPLACE_TEMPLATES,
    MarketplaceAuditBehavior, MarketplacePlanGate, MarketplaceRuleScope, MarketplaceTemplate,
    TemplateConsentRequirement, TemplateDisablementBehavior, TrialState, validate_entitlement,
    validate_marketplace_templates,
};

fn stripe_binding(product_ref: &str, price_ref: &str) -> BillingBinding {
    BillingBinding::StripeCatalog {
        product_ref: product_ref.into(),
        price_ref: price_ref.into(),
    }
}

#[test]
fn free_paid_trial_gift_and_marketplace_states_are_explicit() {
    let basic = CommercialEntitlement {
        subscriber_ref: "subscriber:synthetic-001".into(),
        plan: EntitlementPlan::BasicFree,
        billing_binding: BillingBinding::None,
        trial_state: TrialState::NotInTrial,
        gift_state: GiftState::NotGifted,
        frontline_eligibility: None,
        paid_capabilities: Vec::new(),
        marketplace_template_refs: vec!["template:golden-hour-outreach".into()],
    };
    assert!(validate_entitlement(&basic).allowed);

    let paid_trial_gift = CommercialEntitlement {
        subscriber_ref: "subscriber:synthetic-002".into(),
        plan: EntitlementPlan::FamilyPaid,
        billing_binding: stripe_binding("stripe:product:family", "stripe:price:family"),
        trial_state: TrialState::Active {
            trial_ref: "trial:family-30-day".into(),
        },
        gift_state: GiftState::Redeemed {
            gift_ref: "gift:family-launch".into(),
        },
        frontline_eligibility: None,
        paid_capabilities: vec![CapabilityCode::AdvancedAiGuidance],
        marketplace_template_refs: vec![
            "template:golden-hour-outreach".into(),
            "template:family-preparedness".into(),
        ],
    };

    let decision = validate_entitlement(&paid_trial_gift);
    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.reasons, Vec::<String>::new());
}

#[test]
fn paid_capabilities_and_paid_plans_require_synthetic_billing_binding() {
    let decision = validate_entitlement(&CommercialEntitlement {
        subscriber_ref: "subscriber:synthetic-003".into(),
        plan: EntitlementPlan::TeamPaid,
        billing_binding: BillingBinding::None,
        trial_state: TrialState::NotInTrial,
        gift_state: GiftState::NotGifted,
        frontline_eligibility: None,
        paid_capabilities: vec![CapabilityCode::MarketplaceAutomation],
        marketplace_template_refs: vec!["template:decision-forum-rule-pack".into()],
    });

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"Paid plans and paid capabilities require Stripe catalog binding or custom-contract classification.".into())
    );
}

#[test]
fn frontline_family_plan_requires_deterministic_metadata_and_no_raw_proof_documents() {
    let heroes_cohorts = [
        FrontlineCohort::Firefighter,
        FrontlineCohort::Emt,
        FrontlineCohort::LawEnforcement,
        FrontlineCohort::Sheriff,
        FrontlineCohort::EmergencyRoomPersonnel,
        FrontlineCohort::FemaResponder,
        FrontlineCohort::NimsWorker,
        FrontlineCohort::PowerlineUtilityWorker,
        FrontlineCohort::ActiveDutyMilitary,
        FrontlineCohort::ReserveMilitary,
    ];

    assert!(heroes_cohorts.contains(&FrontlineCohort::PowerlineUtilityWorker));
    assert!(heroes_cohorts.contains(&FrontlineCohort::FemaResponder));

    let missing_metadata = validate_entitlement(&CommercialEntitlement {
        subscriber_ref: "subscriber:synthetic-004".into(),
        plan: EntitlementPlan::FrontlineBasicFamily,
        billing_binding: BillingBinding::None,
        trial_state: TrialState::NotInTrial,
        gift_state: GiftState::NotGifted,
        frontline_eligibility: Some(FrontlineEligibility {
            cohort: None,
            verification_method: FrontlineVerificationMethod::DeterministicMetadata,
            evidence_ref: None,
            stores_raw_proof_document: true,
        }),
        paid_capabilities: Vec::new(),
        marketplace_template_refs: vec!["template:family-preparedness".into()],
    });

    assert!(!missing_metadata.allowed);
    assert!(
        missing_metadata
            .reasons
            .contains(&"Frontline family entitlements require a declared cohort and deterministic metadata evidence.".into())
    );
    assert!(
        missing_metadata
            .reasons
            .contains(&"Frontline eligibility must not store raw proof documents.".into())
    );

    let allowed = validate_entitlement(&CommercialEntitlement {
        subscriber_ref: "subscriber:synthetic-005".into(),
        plan: EntitlementPlan::FrontlineBasicFamily,
        billing_binding: BillingBinding::None,
        trial_state: TrialState::NotInTrial,
        gift_state: GiftState::NotGifted,
        frontline_eligibility: Some(FrontlineEligibility {
            cohort: Some(FrontlineCohort::Emt),
            verification_method: FrontlineVerificationMethod::DeterministicMetadata,
            evidence_ref: Some("frontline:metadata:emt-verified".into()),
            stores_raw_proof_document: false,
        }),
        paid_capabilities: Vec::new(),
        marketplace_template_refs: vec!["template:family-preparedness".into()],
    });

    assert!(allowed.allowed, "{allowed:?}");
}

#[test]
fn marketplace_templates_require_scope_plan_gate_consent_audit_and_disablement() {
    let mut templates = LIVESAFE_MARKETPLACE_TEMPLATES.to_vec();
    templates.push(MarketplaceTemplate {
        template_ref: "template:broken".into(),
        display_name: "Broken Template",
        rule_scope: MarketplaceRuleScope::Unspecified,
        plan_gate: MarketplacePlanGate::Unspecified,
        required_consent: TemplateConsentRequirement::Unspecified,
        audit_behavior: MarketplaceAuditBehavior::Unspecified,
        disablement_behavior: TemplateDisablementBehavior::Unspecified,
    });

    let decision = validate_marketplace_templates(&templates);

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"Marketplace templates must declare a rule scope, plan gate, consent requirement, audit behavior, and disablement behavior.".into())
    );
}
