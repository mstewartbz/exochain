use std::collections::BTreeSet;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntitlementPlan {
    BasicFree,
    FamilyPaid,
    TeamPaid,
    FrontlineBasicFamily,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BillingBinding {
    None,
    StripeCatalog {
        product_ref: String,
        price_ref: String,
    },
    CustomContract {
        contract_ref: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilityCode {
    AdvancedAiGuidance,
    MarketplaceAutomation,
    PrecisionTrialMatching,
    StorageExpansion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrialState {
    NotInTrial,
    Active { trial_ref: String },
    Expired { trial_ref: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GiftState {
    NotGifted,
    Pending { gift_ref: String },
    Redeemed { gift_ref: String },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FrontlineCohort {
    Firefighter,
    Emt,
    LawEnforcement,
    Sheriff,
    EmergencyRoomPersonnel,
    HospitalStaff,
    FemaResponder,
    NimsWorker,
    PowerlineUtilityWorker,
    ActiveDutyMilitary,
    ReserveMilitary,
    TacticalWorker,
    IntelligenceWorker,
    PressOperative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontlineVerificationMethod {
    DeterministicMetadata,
    ManualReview,
    RawDocument,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontlineEligibility {
    pub cohort: Option<FrontlineCohort>,
    pub verification_method: FrontlineVerificationMethod,
    pub evidence_ref: Option<String>,
    pub stores_raw_proof_document: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketplaceRuleScope {
    Unspecified,
    GoldenHourOutreach,
    FamilyPreparedness,
    DisasterReadiness,
    AmbientContextPack,
    DecisionForumRulePack,
    SyntaxisRulePack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketplacePlanGate {
    Unspecified,
    BasicOrHigher,
    FamilyOrHigher,
    TeamOrHigher,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemplateConsentRequirement {
    Unspecified,
    None,
    EmergencyOutreachAcknowledgement,
    HouseholdCoordinationAcknowledgement,
    GovernancePackAcknowledgement,
    AmbientSignalAcknowledgement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketplaceAuditBehavior {
    Unspecified,
    AccessLogOnly,
    RuleExecutionAudit,
    GovernanceAuditTrail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemplateDisablementBehavior {
    Unspecified,
    DisableFutureRunsRetainAudit,
    FreezeRulesRetainAudit,
    RevokeScheduledActionsRetainAudit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketplaceTemplate {
    pub template_ref: String,
    pub display_name: &'static str,
    pub rule_scope: MarketplaceRuleScope,
    pub plan_gate: MarketplacePlanGate,
    pub required_consent: TemplateConsentRequirement,
    pub audit_behavior: MarketplaceAuditBehavior,
    pub disablement_behavior: TemplateDisablementBehavior,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialEntitlement {
    pub subscriber_ref: String,
    pub plan: EntitlementPlan,
    pub billing_binding: BillingBinding,
    pub trial_state: TrialState,
    pub gift_state: GiftState,
    pub frontline_eligibility: Option<FrontlineEligibility>,
    pub paid_capabilities: Vec<CapabilityCode>,
    pub marketplace_template_refs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntitlementDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub static LIVESAFE_MARKETPLACE_TEMPLATES: LazyLock<Vec<MarketplaceTemplate>> =
    LazyLock::new(|| {
        vec![
            MarketplaceTemplate {
                template_ref: "template:golden-hour-outreach".into(),
                display_name: "Golden-Hour Outreach",
                rule_scope: MarketplaceRuleScope::GoldenHourOutreach,
                plan_gate: MarketplacePlanGate::BasicOrHigher,
                required_consent: TemplateConsentRequirement::EmergencyOutreachAcknowledgement,
                audit_behavior: MarketplaceAuditBehavior::AccessLogOnly,
                disablement_behavior: TemplateDisablementBehavior::DisableFutureRunsRetainAudit,
            },
            MarketplaceTemplate {
                template_ref: "template:family-preparedness".into(),
                display_name: "Family Preparedness",
                rule_scope: MarketplaceRuleScope::FamilyPreparedness,
                plan_gate: MarketplacePlanGate::FamilyOrHigher,
                required_consent: TemplateConsentRequirement::HouseholdCoordinationAcknowledgement,
                audit_behavior: MarketplaceAuditBehavior::RuleExecutionAudit,
                disablement_behavior: TemplateDisablementBehavior::DisableFutureRunsRetainAudit,
            },
            MarketplaceTemplate {
                template_ref: "template:disaster-plan".into(),
                display_name: "Disaster Plan",
                rule_scope: MarketplaceRuleScope::DisasterReadiness,
                plan_gate: MarketplacePlanGate::FamilyOrHigher,
                required_consent: TemplateConsentRequirement::HouseholdCoordinationAcknowledgement,
                audit_behavior: MarketplaceAuditBehavior::RuleExecutionAudit,
                disablement_behavior:
                    TemplateDisablementBehavior::RevokeScheduledActionsRetainAudit,
            },
            MarketplaceTemplate {
                template_ref: "template:ambient-context-pack".into(),
                display_name: "Ambient Context Pack",
                rule_scope: MarketplaceRuleScope::AmbientContextPack,
                plan_gate: MarketplacePlanGate::BasicOrHigher,
                required_consent: TemplateConsentRequirement::AmbientSignalAcknowledgement,
                audit_behavior: MarketplaceAuditBehavior::AccessLogOnly,
                disablement_behavior: TemplateDisablementBehavior::DisableFutureRunsRetainAudit,
            },
            MarketplaceTemplate {
                template_ref: "template:decision-forum-rule-pack".into(),
                display_name: "Decision Forum Rule Pack",
                rule_scope: MarketplaceRuleScope::DecisionForumRulePack,
                plan_gate: MarketplacePlanGate::TeamOrHigher,
                required_consent: TemplateConsentRequirement::GovernancePackAcknowledgement,
                audit_behavior: MarketplaceAuditBehavior::GovernanceAuditTrail,
                disablement_behavior: TemplateDisablementBehavior::FreezeRulesRetainAudit,
            },
            MarketplaceTemplate {
                template_ref: "template:syntaxis-rule-pack".into(),
                display_name: "Syntaxis Rule Pack",
                rule_scope: MarketplaceRuleScope::SyntaxisRulePack,
                plan_gate: MarketplacePlanGate::TeamOrHigher,
                required_consent: TemplateConsentRequirement::GovernancePackAcknowledgement,
                audit_behavior: MarketplaceAuditBehavior::GovernanceAuditTrail,
                disablement_behavior: TemplateDisablementBehavior::FreezeRulesRetainAudit,
            },
        ]
    });

pub fn validate_marketplace_templates(templates: &[MarketplaceTemplate]) -> EntitlementDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();
    let mut refs_seen = BTreeSet::new();

    for template in templates {
        if template.template_ref.trim().is_empty()
            || !refs_seen.insert(template.template_ref.clone())
        {
            reasons.insert(
                "Marketplace templates must declare unique synthetic template references."
                    .to_string(),
            );
            required_evidence.insert(
                "Synthetic marketplace template references for every template.".to_string(),
            );
        }

        if matches!(template.rule_scope, MarketplaceRuleScope::Unspecified)
            || matches!(template.plan_gate, MarketplacePlanGate::Unspecified)
            || matches!(
                template.required_consent,
                TemplateConsentRequirement::Unspecified
            )
            || matches!(
                template.audit_behavior,
                MarketplaceAuditBehavior::Unspecified
            )
            || matches!(
                template.disablement_behavior,
                TemplateDisablementBehavior::Unspecified
            )
        {
            reasons.insert(
                "Marketplace templates must declare a rule scope, plan gate, consent requirement, audit behavior, and disablement behavior."
                    .to_string(),
            );
            required_evidence.insert(
                "Marketplace template metadata with plan gate, consent, audit, and disablement declarations."
                    .to_string(),
            );
        }
    }

    entitlement_decision(reasons, required_evidence)
}

pub fn validate_entitlement(entitlement: &CommercialEntitlement) -> EntitlementDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if entitlement.subscriber_ref.trim().is_empty() {
        reasons.insert("Entitlement records require a subscriber reference.".to_string());
        required_evidence
            .insert("Synthetic subscriber reference for plan and capability state.".to_string());
    }

    if requires_paid_binding(entitlement)
        && !matches!(
            entitlement.billing_binding,
            BillingBinding::StripeCatalog { .. } | BillingBinding::CustomContract { .. }
        )
    {
        reasons.insert(
            "Paid plans and paid capabilities require Stripe catalog binding or custom-contract classification."
                .to_string(),
        );
        required_evidence.insert(
            "Synthetic Stripe catalog refs or custom-contract classification for paid state."
                .to_string(),
        );
    }

    if !trial_state_is_well_formed(&entitlement.trial_state) {
        reasons.insert("Trial states must carry a synthetic trial reference.".to_string());
        required_evidence
            .insert("Synthetic trial reference for active or expired trial state.".to_string());
    }

    if !gift_state_is_well_formed(&entitlement.gift_state) {
        reasons.insert("Gift states must carry a synthetic gift reference.".to_string());
        required_evidence
            .insert("Synthetic gift reference for pending or redeemed gift state.".to_string());
    }

    validate_frontline_eligibility(entitlement, &mut reasons, &mut required_evidence);
    validate_marketplace_assignments(entitlement, &mut reasons, &mut required_evidence);

    entitlement_decision(reasons, required_evidence)
}

fn requires_paid_binding(entitlement: &CommercialEntitlement) -> bool {
    matches!(
        entitlement.plan,
        EntitlementPlan::FamilyPaid | EntitlementPlan::TeamPaid
    ) || !entitlement.paid_capabilities.is_empty()
}

fn trial_state_is_well_formed(trial_state: &TrialState) -> bool {
    match trial_state {
        TrialState::NotInTrial => true,
        TrialState::Active { trial_ref } | TrialState::Expired { trial_ref } => {
            !trial_ref.trim().is_empty()
        }
    }
}

fn gift_state_is_well_formed(gift_state: &GiftState) -> bool {
    match gift_state {
        GiftState::NotGifted => true,
        GiftState::Pending { gift_ref } | GiftState::Redeemed { gift_ref } => {
            !gift_ref.trim().is_empty()
        }
    }
}

fn validate_frontline_eligibility(
    entitlement: &CommercialEntitlement,
    reasons: &mut BTreeSet<String>,
    required_evidence: &mut BTreeSet<String>,
) {
    if let Some(eligibility) = &entitlement.frontline_eligibility
        && (eligibility.stores_raw_proof_document
            || eligibility.verification_method == FrontlineVerificationMethod::RawDocument)
    {
        reasons.insert("Frontline eligibility must not store raw proof documents.".to_string());
        required_evidence.insert(
            "Deterministic frontline metadata without uploaded eligibility documents.".to_string(),
        );
    }

    if entitlement.plan != EntitlementPlan::FrontlineBasicFamily {
        return;
    }

    let Some(eligibility) = &entitlement.frontline_eligibility else {
        reasons.insert(
            "Frontline family entitlements require a declared cohort and deterministic metadata evidence."
                .to_string(),
        );
        required_evidence.insert(
            "Frontline cohort classification and deterministic metadata evidence reference."
                .to_string(),
        );
        return;
    };

    if eligibility.cohort.is_none()
        || eligibility.verification_method != FrontlineVerificationMethod::DeterministicMetadata
        || eligibility
            .evidence_ref
            .as_ref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
    {
        reasons.insert(
            "Frontline family entitlements require a declared cohort and deterministic metadata evidence."
                .to_string(),
        );
        required_evidence.insert(
            "Frontline cohort classification and deterministic metadata evidence reference."
                .to_string(),
        );
    }
}

fn validate_marketplace_assignments(
    entitlement: &CommercialEntitlement,
    reasons: &mut BTreeSet<String>,
    required_evidence: &mut BTreeSet<String>,
) {
    for template_ref in &entitlement.marketplace_template_refs {
        let Some(template) = LIVESAFE_MARKETPLACE_TEMPLATES
            .iter()
            .find(|template| &template.template_ref == template_ref)
        else {
            reasons
                .insert("Marketplace entitlements must reference a declared template.".to_string());
            required_evidence.insert(
                "Synthetic marketplace template reference from the approved catalog.".to_string(),
            );
            continue;
        };

        if !plan_satisfies_gate(entitlement.plan, template.plan_gate) {
            reasons.insert(format!(
                "{} is not allowed for the selected entitlement plan.",
                template.display_name
            ));
            required_evidence.insert(format!(
                "Plan-gate review for marketplace template {}.",
                template.template_ref
            ));
        }
    }
}

fn plan_satisfies_gate(plan: EntitlementPlan, gate: MarketplacePlanGate) -> bool {
    match gate {
        MarketplacePlanGate::Unspecified => false,
        MarketplacePlanGate::BasicOrHigher => true,
        MarketplacePlanGate::FamilyOrHigher => matches!(
            plan,
            EntitlementPlan::FamilyPaid
                | EntitlementPlan::TeamPaid
                | EntitlementPlan::FrontlineBasicFamily
        ),
        MarketplacePlanGate::TeamOrHigher => matches!(plan, EntitlementPlan::TeamPaid),
    }
}

fn entitlement_decision(
    reasons: BTreeSet<String>,
    required_evidence: BTreeSet<String>,
) -> EntitlementDecision {
    EntitlementDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}
