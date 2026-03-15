//! decision.forum — The sovereign governance engine
//! Every decision is a first-class cryptographically verifiable object.
//! Runtime-enforced here: TNC controls, advanced policy thresholds, and constitution hashing.
//! The GOV-001..013 and LEG-001..013 catalogs live in `constitution.rs` and are hash-bound
//! into every decision, but not every catalog item is a standalone runtime check yet.

pub mod advanced_policy;
pub mod authority;
pub mod cli;
pub mod constitution;
pub mod decision_object;
pub mod fiduciary_package;
pub mod requirements;
pub mod tnc_enforcer;

pub use decision_object::DecisionObject;
pub use fiduciary_package::FiduciaryDefensePackage;
pub use tnc_enforcer::TNCEnforcer;

/// Birth a new Decision Object (example entry point)
pub fn create_genesis_decision(title: &str) -> Result<DecisionObject, Box<dyn std::error::Error>> {
    #[cfg(test)]
    requirements::Requirement::GenesisDecision.mark_covered();

    let mut obj = DecisionObject::new(title);
    obj.authority_chain.push(authority::AuthorityLink {
        pubkey: "GenesisPubKey0001".into(),
        signature: "GenesisSignature0001".into(),
        actor_kind: authority::ActorKind::Human,
        expires_at: None,
        conflict_disclosure: Some(authority::ConflictDisclosure {
            has_conflict: false,
            description: None,
            disclosed_at: chrono::Utc::now(),
        }),
    });
    obj.human_review = advanced_policy::HumanReviewStatus::approved_by("council:genesis");
    obj.seal()?;
    Ok(obj)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_create_genesis_decision() {
        let obj = create_genesis_decision("Genesis Test").expect("Should create genesis decision");
        assert_eq!(obj.title, "Genesis Test");
        assert_eq!(obj.status, decision_object::Status::Approved);
    }

    #[test]
    pub fn test_100_percent_requirements_coverage() {
        crate::decision_object::tests::test_decision_object_creation();
        crate::decision_object::tests::test_decision_object_seal_success();
        crate::decision_object::tests::test_decision_object_seal_failure_records_tnc_audit_event();
        crate::tnc_enforcer::tests::test_enforce_all_happy_path();
        crate::fiduciary_package::tests::test_fiduciary_package();
        crate::cli::tests::test_cli_run();
        crate::advanced_policy::tests::test_valid_advanced_policy();
        test_create_genesis_decision();
        crate::requirements::assert_all_requirements_covered();
    }
}
