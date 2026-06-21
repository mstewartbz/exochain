//! Bounded control-state transitions for ExoChain DAG DB.

use exo_dag_db_api::{
    CouncilReviewStatus, DagFinalityStatus, MemoryStatus, ReceiptEventType, RouteStatus,
    ValidationStatus,
};
use thiserror::Error;

/// Control-state machines covered by the DAG DB transition table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateMachine {
    /// Memory object lifecycle status.
    Memory,
    /// Validation report status.
    Validation,
    /// Route receipt status.
    Route,
    /// Council review status.
    CouncilReview,
    /// DAG finality status.
    DagFinality,
}

/// Rejected control-state transition.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("undeclared {machine:?} transition {from} -> {to}")]
pub struct StateTransitionError {
    /// State machine whose transition was rejected.
    pub machine: StateMachine,
    /// Stable source-state label.
    pub from: &'static str,
    /// Stable destination-state label.
    pub to: &'static str,
}

/// Result alias for control-state transition checks.
pub type Result<T> = std::result::Result<T, StateTransitionError>;

/// Validate a memory status transition and return its receipt event.
pub fn memory_transition_event(from: MemoryStatus, to: MemoryStatus) -> Result<ReceiptEventType> {
    match (from, to) {
        (MemoryStatus::Pending, MemoryStatus::Approved) => Ok(ReceiptEventType::MemoryApproved),
        (MemoryStatus::Approved, MemoryStatus::Routable) => Ok(ReceiptEventType::MemoryRoutable),
        (MemoryStatus::Pending, MemoryStatus::Blocked) => Ok(ReceiptEventType::ValidationFailed),
        (MemoryStatus::Pending, MemoryStatus::Rejected) => Ok(ReceiptEventType::DuplicateRejected),
        (MemoryStatus::Routable, MemoryStatus::Revoked) => Ok(ReceiptEventType::MemoryRevoked),
        (MemoryStatus::Routable, MemoryStatus::Superseded) => {
            Ok(ReceiptEventType::MemorySuperseded)
        }
        _ => Err(undeclared(
            StateMachine::Memory,
            memory_label(from),
            memory_label(to),
        )),
    }
}

/// Validate a validation status transition and return its receipt event.
pub fn validation_transition_event(
    from: ValidationStatus,
    to: ValidationStatus,
) -> Result<ReceiptEventType> {
    match (from, to) {
        (ValidationStatus::Pending, ValidationStatus::Passed) => {
            Ok(ReceiptEventType::ValidationPassed)
        }
        (ValidationStatus::Pending, ValidationStatus::Failed) => {
            Ok(ReceiptEventType::ValidationFailed)
        }
        (ValidationStatus::Pending, ValidationStatus::Contradictory)
        | (ValidationStatus::Pending, ValidationStatus::NeedsCouncil)
        | (ValidationStatus::Pending, ValidationStatus::Expired) => {
            Ok(ReceiptEventType::ValidationCreated)
        }
        _ => Err(undeclared(
            StateMachine::Validation,
            validation_label(from),
            validation_label(to),
        )),
    }
}

/// Validate a route status transition and return its receipt event.
pub fn route_transition_event(from: RouteStatus, to: RouteStatus) -> Result<ReceiptEventType> {
    match (from, to) {
        (RouteStatus::Pending, RouteStatus::Active) => Ok(ReceiptEventType::RouteActivated),
        (RouteStatus::Active, RouteStatus::Stale) => Ok(ReceiptEventType::RouteStale),
        (RouteStatus::Active, RouteStatus::Invalidated)
        | (RouteStatus::Stale, RouteStatus::Invalidated) => Ok(ReceiptEventType::RouteInvalidated),
        (RouteStatus::Pending, RouteStatus::Blocked) => Ok(ReceiptEventType::ValidationFailed),
        _ => Err(undeclared(
            StateMachine::Route,
            route_label(from),
            route_label(to),
        )),
    }
}

/// Validate a council review status transition and return its receipt event.
pub fn council_review_transition_event(
    from: CouncilReviewStatus,
    to: CouncilReviewStatus,
) -> Result<ReceiptEventType> {
    match (from, to) {
        (CouncilReviewStatus::Required, CouncilReviewStatus::Pending) => {
            Ok(ReceiptEventType::ValidationCreated)
        }
        (CouncilReviewStatus::Pending, CouncilReviewStatus::Approved)
        | (CouncilReviewStatus::Pending, CouncilReviewStatus::Denied)
        | (CouncilReviewStatus::Pending, CouncilReviewStatus::Expired)
        | (CouncilReviewStatus::Pending, CouncilReviewStatus::Escalated) => {
            Ok(ReceiptEventType::CouncilDecisionRecorded)
        }
        _ => Err(undeclared(
            StateMachine::CouncilReview,
            council_review_label(from),
            council_review_label(to),
        )),
    }
}

/// Validate a DAG finality status transition and return its receipt event.
pub fn dag_finality_transition_event(
    from: DagFinalityStatus,
    to: DagFinalityStatus,
) -> Result<ReceiptEventType> {
    match (from, to) {
        (DagFinalityStatus::Pending, DagFinalityStatus::Committed) => {
            Ok(ReceiptEventType::DagFinalityCommitted)
        }
        (DagFinalityStatus::Pending, DagFinalityStatus::Failed) => {
            Ok(ReceiptEventType::DagFinalityFailed)
        }
        (DagFinalityStatus::Failed, DagFinalityStatus::Pending) => {
            Ok(ReceiptEventType::DagFinalityFailed)
        }
        (DagFinalityStatus::Failed, DagFinalityStatus::Compensated) => {
            Ok(ReceiptEventType::DagFinalityCompensated)
        }
        _ => Err(undeclared(
            StateMachine::DagFinality,
            dag_finality_label(from),
            dag_finality_label(to),
        )),
    }
}

fn undeclared(machine: StateMachine, from: &'static str, to: &'static str) -> StateTransitionError {
    StateTransitionError { machine, from, to }
}

fn memory_label(status: MemoryStatus) -> &'static str {
    match status {
        MemoryStatus::Pending => "pending",
        MemoryStatus::Approved => "approved",
        MemoryStatus::Routable => "routable",
        MemoryStatus::Blocked => "blocked",
        MemoryStatus::Revoked => "revoked",
        MemoryStatus::Superseded => "superseded",
        MemoryStatus::Rejected => "rejected",
    }
}

fn validation_label(status: ValidationStatus) -> &'static str {
    match status {
        ValidationStatus::NotRequired => "not_required",
        ValidationStatus::Pending => "pending",
        ValidationStatus::Passed => "passed",
        ValidationStatus::Failed => "failed",
        ValidationStatus::Contradictory => "contradictory",
        ValidationStatus::Expired => "expired",
        ValidationStatus::NeedsCouncil => "needs_council",
    }
}

fn route_label(status: RouteStatus) -> &'static str {
    match status {
        RouteStatus::Pending => "pending",
        RouteStatus::Active => "active",
        RouteStatus::Stale => "stale",
        RouteStatus::Invalidated => "invalidated",
        RouteStatus::Blocked => "blocked",
    }
}

fn council_review_label(status: CouncilReviewStatus) -> &'static str {
    match status {
        CouncilReviewStatus::NotRequired => "not_required",
        CouncilReviewStatus::Required => "required",
        CouncilReviewStatus::Pending => "pending",
        CouncilReviewStatus::Approved => "approved",
        CouncilReviewStatus::Denied => "denied",
        CouncilReviewStatus::Expired => "expired",
        CouncilReviewStatus::Escalated => "escalated",
    }
}

fn dag_finality_label(status: DagFinalityStatus) -> &'static str {
    match status {
        DagFinalityStatus::Pending => "pending",
        DagFinalityStatus::Committed => "committed",
        DagFinalityStatus::Failed => "failed",
        DagFinalityStatus::Compensated => "compensated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_state_transition_table() {
        assert_eq!(
            memory_transition_event(MemoryStatus::Pending, MemoryStatus::Approved),
            Ok(ReceiptEventType::MemoryApproved)
        );
        assert_eq!(
            memory_transition_event(MemoryStatus::Approved, MemoryStatus::Routable),
            Ok(ReceiptEventType::MemoryRoutable)
        );
        assert_eq!(
            memory_transition_event(MemoryStatus::Pending, MemoryStatus::Blocked),
            Ok(ReceiptEventType::ValidationFailed)
        );
        assert_eq!(
            memory_transition_event(MemoryStatus::Pending, MemoryStatus::Rejected),
            Ok(ReceiptEventType::DuplicateRejected)
        );
        assert_eq!(
            memory_transition_event(MemoryStatus::Routable, MemoryStatus::Revoked),
            Ok(ReceiptEventType::MemoryRevoked)
        );
        assert_eq!(
            memory_transition_event(MemoryStatus::Routable, MemoryStatus::Superseded),
            Ok(ReceiptEventType::MemorySuperseded)
        );
        assert!(matches!(
            memory_transition_event(MemoryStatus::Approved, MemoryStatus::Rejected),
            Err(StateTransitionError {
                machine: StateMachine::Memory,
                from: "approved",
                to: "rejected"
            })
        ));

        assert_eq!(
            validation_transition_event(ValidationStatus::Pending, ValidationStatus::Passed),
            Ok(ReceiptEventType::ValidationPassed)
        );
        assert_eq!(
            validation_transition_event(ValidationStatus::Pending, ValidationStatus::Failed),
            Ok(ReceiptEventType::ValidationFailed)
        );
        for status in [
            ValidationStatus::Contradictory,
            ValidationStatus::NeedsCouncil,
            ValidationStatus::Expired,
        ] {
            assert_eq!(
                validation_transition_event(ValidationStatus::Pending, status),
                Ok(ReceiptEventType::ValidationCreated)
            );
        }
        assert!(
            validation_transition_event(ValidationStatus::Passed, ValidationStatus::Failed)
                .is_err()
        );

        assert_eq!(
            route_transition_event(RouteStatus::Pending, RouteStatus::Active),
            Ok(ReceiptEventType::RouteActivated)
        );
        assert_eq!(
            route_transition_event(RouteStatus::Active, RouteStatus::Stale),
            Ok(ReceiptEventType::RouteStale)
        );
        assert_eq!(
            route_transition_event(RouteStatus::Active, RouteStatus::Invalidated),
            Ok(ReceiptEventType::RouteInvalidated)
        );
        assert_eq!(
            route_transition_event(RouteStatus::Stale, RouteStatus::Invalidated),
            Ok(ReceiptEventType::RouteInvalidated)
        );
        assert_eq!(
            route_transition_event(RouteStatus::Pending, RouteStatus::Blocked),
            Ok(ReceiptEventType::ValidationFailed)
        );
        assert!(route_transition_event(RouteStatus::Blocked, RouteStatus::Active).is_err());

        assert_eq!(
            council_review_transition_event(
                CouncilReviewStatus::Required,
                CouncilReviewStatus::Pending
            ),
            Ok(ReceiptEventType::ValidationCreated)
        );
        for status in [
            CouncilReviewStatus::Approved,
            CouncilReviewStatus::Denied,
            CouncilReviewStatus::Expired,
            CouncilReviewStatus::Escalated,
        ] {
            assert_eq!(
                council_review_transition_event(CouncilReviewStatus::Pending, status),
                Ok(ReceiptEventType::CouncilDecisionRecorded)
            );
        }
        assert!(
            council_review_transition_event(
                CouncilReviewStatus::Approved,
                CouncilReviewStatus::Pending
            )
            .is_err()
        );

        assert_eq!(
            dag_finality_transition_event(DagFinalityStatus::Pending, DagFinalityStatus::Committed),
            Ok(ReceiptEventType::DagFinalityCommitted)
        );
        assert_eq!(
            dag_finality_transition_event(DagFinalityStatus::Pending, DagFinalityStatus::Failed),
            Ok(ReceiptEventType::DagFinalityFailed)
        );
        assert_eq!(
            dag_finality_transition_event(DagFinalityStatus::Failed, DagFinalityStatus::Pending),
            Ok(ReceiptEventType::DagFinalityFailed)
        );
        assert_eq!(
            dag_finality_transition_event(
                DagFinalityStatus::Failed,
                DagFinalityStatus::Compensated
            ),
            Ok(ReceiptEventType::DagFinalityCompensated)
        );
        assert!(
            dag_finality_transition_event(
                DagFinalityStatus::Compensated,
                DagFinalityStatus::Pending
            )
            .is_err()
        );
    }

    #[test]
    fn state_transition_labels_cover_every_status_variant() {
        assert_eq!(
            [
                memory_label(MemoryStatus::Pending),
                memory_label(MemoryStatus::Approved),
                memory_label(MemoryStatus::Routable),
                memory_label(MemoryStatus::Blocked),
                memory_label(MemoryStatus::Revoked),
                memory_label(MemoryStatus::Superseded),
                memory_label(MemoryStatus::Rejected),
            ],
            [
                "pending",
                "approved",
                "routable",
                "blocked",
                "revoked",
                "superseded",
                "rejected"
            ]
        );
        assert_eq!(
            [
                validation_label(ValidationStatus::NotRequired),
                validation_label(ValidationStatus::Pending),
                validation_label(ValidationStatus::Passed),
                validation_label(ValidationStatus::Failed),
                validation_label(ValidationStatus::Contradictory),
                validation_label(ValidationStatus::Expired),
                validation_label(ValidationStatus::NeedsCouncil),
            ],
            [
                "not_required",
                "pending",
                "passed",
                "failed",
                "contradictory",
                "expired",
                "needs_council"
            ]
        );
        assert_eq!(
            [
                route_label(RouteStatus::Pending),
                route_label(RouteStatus::Active),
                route_label(RouteStatus::Stale),
                route_label(RouteStatus::Invalidated),
                route_label(RouteStatus::Blocked),
            ],
            ["pending", "active", "stale", "invalidated", "blocked"]
        );
        assert_eq!(
            [
                council_review_label(CouncilReviewStatus::NotRequired),
                council_review_label(CouncilReviewStatus::Required),
                council_review_label(CouncilReviewStatus::Pending),
                council_review_label(CouncilReviewStatus::Approved),
                council_review_label(CouncilReviewStatus::Denied),
                council_review_label(CouncilReviewStatus::Expired),
                council_review_label(CouncilReviewStatus::Escalated),
            ],
            [
                "not_required",
                "required",
                "pending",
                "approved",
                "denied",
                "expired",
                "escalated"
            ]
        );
        assert_eq!(
            [
                dag_finality_label(DagFinalityStatus::Pending),
                dag_finality_label(DagFinalityStatus::Committed),
                dag_finality_label(DagFinalityStatus::Failed),
                dag_finality_label(DagFinalityStatus::Compensated),
            ],
            ["pending", "committed", "failed", "compensated"]
        );

        let error = undeclared(StateMachine::Route, "blocked", "active");
        assert_eq!(
            error.to_string(),
            "undeclared Route transition blocked -> active"
        );
    }
}
