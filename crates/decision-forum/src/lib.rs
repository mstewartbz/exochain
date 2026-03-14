//! decision.forum — The sovereign governance engine
//! Every decision is a first-class cryptographically verifiable object.
//! Enforces all 10 TNCs + GOV-001..013 + LEG-001..013.

pub mod decision_object;
pub mod constitution;
pub mod authority;
pub mod tnc_enforcer;
pub mod fiduciary_package;
pub mod cli;
pub mod requirements;

pub use decision_object::DecisionObject;
pub use tnc_enforcer::TNCEnforcer;
pub use fiduciary_package::FiduciaryDefensePackage;

/// Birth a new Decision Object (example entry point)
pub fn create_genesis_decision(title: &str) -> Result<DecisionObject, Box<dyn std::error::Error>> {
    #[cfg(test)]
    requirements::Requirement::GenesisDecision.mark_covered();

    let mut obj = DecisionObject::new(title);
    obj.authority_chain.push(authority::AuthorityLink {
        pubkey: "GenesisPubKey".into(),
        signature: "GenesisSignature".into(),
    });
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
        // Run all component tests synchronously to avoid race conditions
        // and guarantee the COVERAGE registry is fully populated.
        crate::decision_object::tests::test_decision_object_creation();
        crate::decision_object::tests::test_decision_object_seal_success();
        crate::decision_object::tests::test_decision_object_seal_failure();
        crate::tnc_enforcer::tests::test_tnc_all();
        crate::fiduciary_package::tests::test_fiduciary_package();
        crate::cli::tests::test_cli_run();
        test_create_genesis_decision();

        // Validate that 100% of explicit requirements have been tested.
        crate::requirements::assert_all_requirements_covered();
    }
}
