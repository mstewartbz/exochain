#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

pub const NUM_REQUIREMENTS: usize = 15;

#[cfg(test)]
static COVERAGE: [AtomicUsize; NUM_REQUIREMENTS] = [
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
];

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum Requirement {
    Tnc01AuthorityChain = 0,
    Tnc02HumanGate = 1,
    Tnc03AuditContinuity = 2,
    Tnc04SyncConstraints = 3,
    Tnc05DelegationExpiry = 4,
    Tnc06ConflictDisclosure = 5,
    Tnc07Quorum = 6,
    Tnc08Immutability = 7,
    Tnc09AiCeiling = 8,
    Tnc10Ratification = 9,
    DecisionObjectCreation = 10,
    DecisionObjectSealing = 11,
    FiduciaryPackageGeneration = 12,
    CliRun = 13,
    GenesisDecision = 14,
}

impl Requirement {
    pub const ALL: &'static [Requirement] = &[
        Requirement::Tnc01AuthorityChain,
        Requirement::Tnc02HumanGate,
        Requirement::Tnc03AuditContinuity,
        Requirement::Tnc04SyncConstraints,
        Requirement::Tnc05DelegationExpiry,
        Requirement::Tnc06ConflictDisclosure,
        Requirement::Tnc07Quorum,
        Requirement::Tnc08Immutability,
        Requirement::Tnc09AiCeiling,
        Requirement::Tnc10Ratification,
        Requirement::DecisionObjectCreation,
        Requirement::DecisionObjectSealing,
        Requirement::FiduciaryPackageGeneration,
        Requirement::CliRun,
        Requirement::GenesisDecision,
    ];

    #[cfg(test)]
    pub fn mark_covered(self) {
        COVERAGE[self as usize].fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
pub fn assert_all_requirements_covered() {
    let mut missing = Vec::new();
    for &req in Requirement::ALL {
        if COVERAGE[req as usize].load(Ordering::SeqCst) == 0 {
            missing.push(req);
        }
    }
    assert!(missing.is_empty(), "Missing test coverage for requirements: {:?}", missing);
}
