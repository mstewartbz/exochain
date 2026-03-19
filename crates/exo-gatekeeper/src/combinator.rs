//! Combinator algebra engine.
//!
//! Provides a deterministic algebra for composing governance operations.
//! Every reduction is pure: same input always produces same output.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::GatekeeperError;

// ---------------------------------------------------------------------------
// Combinator types
// ---------------------------------------------------------------------------

/// A predicate that guards combinator execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predicate {
    /// Name of the predicate (for tracing).
    pub name: String,
    /// The key in the input that must exist and be truthy.
    pub required_key: String,
    /// Expected value (if None, just check existence).
    pub expected_value: Option<String>,
}

impl Predicate {
    pub fn evaluate(&self, input: &CombinatorInput) -> bool {
        match input.fields.get(&self.required_key) {
            None => false,
            Some(val) => match &self.expected_value {
                None => true,
                Some(expected) => val == expected,
            },
        }
    }
}

/// A transform function applied to combinator output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformFn {
    /// Name of the transform (for tracing).
    pub name: String,
    /// Key to add to the output.
    pub output_key: String,
    /// Value to set.
    pub output_value: String,
}

/// Policy for retrying a combinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Current attempt (used during reduction).
    pub current_attempt: u32,
}

/// A checkpoint identifier for resumable combinators.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(pub String);

/// Duration in milliseconds (deterministic, no floating-point).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Duration(pub u64);

/// The combinator algebra terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Combinator {
    /// Pass-through: returns the input as output.
    Identity,
    /// All must succeed in order.
    Sequence(Vec<Combinator>),
    /// All must succeed (order-independent; we process left-to-right).
    Parallel(Vec<Combinator>),
    /// First success wins.
    Choice(Vec<Combinator>),
    /// Proceed only if predicate holds.
    Guard(Box<Combinator>, Predicate),
    /// Modify the result of the inner combinator.
    Transform(Box<Combinator>, TransformFn),
    /// Retry with policy.
    Retry(Box<Combinator>, RetryPolicy),
    /// Time-bounded (simulated in deterministic mode).
    Timeout(Box<Combinator>, Duration),
    /// Resumable checkpoint.
    Checkpoint(Box<Combinator>, CheckpointId),
}

// ---------------------------------------------------------------------------
// Input / Output envelopes
// ---------------------------------------------------------------------------

/// Typed input envelope for combinator reduction.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombinatorInput {
    /// Key-value fields.
    pub fields: BTreeMap<String, String>,
}

impl CombinatorInput {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.fields.insert(key.into(), value.into());
    }

    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.set(key, value);
        self
    }
}

/// Typed output envelope from combinator reduction.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombinatorOutput {
    /// Key-value fields.
    pub fields: BTreeMap<String, String>,
    /// Checkpoint data if a checkpoint was reached.
    pub checkpoint: Option<CheckpointId>,
}

impl CombinatorOutput {
    #[must_use]
    pub fn from_input(input: &CombinatorInput) -> Self {
        Self {
            fields: input.fields.clone(),
            checkpoint: None,
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.fields.insert(key.into(), value.into());
    }

    pub fn merge(&mut self, other: &CombinatorOutput) {
        for (k, v) in &other.fields {
            self.fields.insert(k.clone(), v.clone());
        }
        if other.checkpoint.is_some() {
            self.checkpoint.clone_from(&other.checkpoint);
        }
    }
}

// ---------------------------------------------------------------------------
// Reduction engine
// ---------------------------------------------------------------------------

/// Reduce a combinator with the given input.
///
/// Reduction is deterministic: same combinator + same input = same output.
pub fn reduce(
    combinator: &Combinator,
    input: &CombinatorInput,
) -> Result<CombinatorOutput, GatekeeperError> {
    match combinator {
        Combinator::Identity => Ok(CombinatorOutput::from_input(input)),

        Combinator::Sequence(combinators) => {
            let mut current_input = input.clone();
            let mut last_output = CombinatorOutput::from_input(input);

            for (i, c) in combinators.iter().enumerate() {
                match reduce(c, &current_input) {
                    Ok(output) => {
                        // Feed output as next input.
                        current_input = CombinatorInput {
                            fields: output.fields.clone(),
                        };
                        last_output = output;
                    }
                    Err(e) => {
                        return Err(GatekeeperError::CombinatorError(format!(
                            "Sequence step {} failed: {}",
                            i, e
                        )));
                    }
                }
            }
            Ok(last_output)
        }

        Combinator::Parallel(combinators) => {
            let mut merged = CombinatorOutput::from_input(input);

            for (i, c) in combinators.iter().enumerate() {
                match reduce(c, input) {
                    Ok(output) => {
                        merged.merge(&output);
                    }
                    Err(e) => {
                        return Err(GatekeeperError::CombinatorError(format!(
                            "Parallel branch {} failed: {}",
                            i, e
                        )));
                    }
                }
            }
            Ok(merged)
        }

        Combinator::Choice(combinators) => {
            for c in combinators {
                match reduce(c, input) {
                    Ok(output) => return Ok(output),
                    Err(_) => continue,
                }
            }
            Err(GatekeeperError::CombinatorError(
                "Choice: all alternatives failed".into(),
            ))
        }

        Combinator::Guard(inner, predicate) => {
            if !predicate.evaluate(input) {
                return Err(GatekeeperError::CombinatorError(format!(
                    "Guard predicate '{}' failed",
                    predicate.name
                )));
            }
            reduce(inner, input)
        }

        Combinator::Transform(inner, transform) => {
            let mut output = reduce(inner, input)?;
            output.set(
                transform.output_key.clone(),
                transform.output_value.clone(),
            );
            Ok(output)
        }

        Combinator::Retry(inner, policy) => {
            let mut last_err = None;
            for attempt in 0..=policy.max_retries {
                match reduce(inner, input) {
                    Ok(mut output) => {
                        output.set("retry_attempts", attempt.to_string());
                        return Ok(output);
                    }
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }
            Err(last_err.unwrap_or_else(|| {
                GatekeeperError::CombinatorError("Retry exhausted".into())
            }))
        }

        Combinator::Timeout(inner, duration) => {
            // In deterministic mode, we simulate timeout by simply running.
            // Real timeout enforcement is at the Holon runtime level.
            let mut output = reduce(inner, input)?;
            output.set("timeout_budget_ms", duration.0.to_string());
            Ok(output)
        }

        Combinator::Checkpoint(inner, checkpoint_id) => {
            let mut output = reduce(inner, input)?;
            output.checkpoint = Some(checkpoint_id.clone());
            Ok(output)
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> CombinatorInput {
        CombinatorInput::new()
            .with("name", "alice")
            .with("role", "judge")
    }

    // --- Identity ---

    #[test]
    fn identity_passes_through() {
        let input = sample_input();
        let output = reduce(&Combinator::Identity, &input).unwrap();
        assert_eq!(output.fields, input.fields);
    }

    // --- Sequence ---

    #[test]
    fn sequence_empty_returns_input() {
        let input = sample_input();
        let output = reduce(&Combinator::Sequence(vec![]), &input).unwrap();
        assert_eq!(output.fields, input.fields);
    }

    #[test]
    fn sequence_chains_results() {
        let input = sample_input();
        let seq = Combinator::Sequence(vec![
            Combinator::Transform(
                Box::new(Combinator::Identity),
                TransformFn {
                    name: "add_step1".into(),
                    output_key: "step1".into(),
                    output_value: "done".into(),
                },
            ),
            Combinator::Transform(
                Box::new(Combinator::Identity),
                TransformFn {
                    name: "add_step2".into(),
                    output_key: "step2".into(),
                    output_value: "done".into(),
                },
            ),
        ]);
        let output = reduce(&seq, &input).unwrap();
        assert_eq!(output.fields.get("step1"), Some(&"done".to_string()));
        assert_eq!(output.fields.get("step2"), Some(&"done".to_string()));
    }

    #[test]
    fn sequence_fails_if_any_step_fails() {
        let input = sample_input();
        let seq = Combinator::Sequence(vec![
            Combinator::Identity,
            Combinator::Guard(
                Box::new(Combinator::Identity),
                Predicate {
                    name: "requires_admin".into(),
                    required_key: "admin".into(),
                    expected_value: None,
                },
            ),
        ]);
        let result = reduce(&seq, &input);
        assert!(result.is_err());
    }

    // --- Parallel ---

    #[test]
    fn parallel_merges_results() {
        let input = sample_input();
        let par = Combinator::Parallel(vec![
            Combinator::Transform(
                Box::new(Combinator::Identity),
                TransformFn {
                    name: "branch_a".into(),
                    output_key: "a".into(),
                    output_value: "1".into(),
                },
            ),
            Combinator::Transform(
                Box::new(Combinator::Identity),
                TransformFn {
                    name: "branch_b".into(),
                    output_key: "b".into(),
                    output_value: "2".into(),
                },
            ),
        ]);
        let output = reduce(&par, &input).unwrap();
        assert_eq!(output.fields.get("a"), Some(&"1".to_string()));
        assert_eq!(output.fields.get("b"), Some(&"2".to_string()));
    }

    #[test]
    fn parallel_fails_if_any_branch_fails() {
        let input = sample_input();
        let par = Combinator::Parallel(vec![
            Combinator::Identity,
            Combinator::Guard(
                Box::new(Combinator::Identity),
                Predicate {
                    name: "impossible".into(),
                    required_key: "nonexistent".into(),
                    expected_value: None,
                },
            ),
        ]);
        assert!(reduce(&par, &input).is_err());
    }

    // --- Choice ---

    #[test]
    fn choice_returns_first_success() {
        let input = sample_input();
        let choice = Combinator::Choice(vec![
            Combinator::Guard(
                Box::new(Combinator::Transform(
                    Box::new(Combinator::Identity),
                    TransformFn {
                        name: "fail_branch".into(),
                        output_key: "branch".into(),
                        output_value: "first".into(),
                    },
                )),
                Predicate {
                    name: "impossible".into(),
                    required_key: "nonexistent".into(),
                    expected_value: None,
                },
            ),
            Combinator::Transform(
                Box::new(Combinator::Identity),
                TransformFn {
                    name: "success_branch".into(),
                    output_key: "branch".into(),
                    output_value: "second".into(),
                },
            ),
        ]);
        let output = reduce(&choice, &input).unwrap();
        assert_eq!(output.fields.get("branch"), Some(&"second".to_string()));
    }

    #[test]
    fn choice_fails_if_all_alternatives_fail() {
        let input = sample_input();
        let guard = |key: &str| Combinator::Guard(
            Box::new(Combinator::Identity),
            Predicate {
                name: "fail".into(),
                required_key: key.into(),
                expected_value: None,
            },
        );
        let choice = Combinator::Choice(vec![guard("x"), guard("y"), guard("z")]);
        assert!(reduce(&choice, &input).is_err());
    }

    // --- Guard ---

    #[test]
    fn guard_passes_when_predicate_holds() {
        let input = sample_input();
        let guarded = Combinator::Guard(
            Box::new(Combinator::Identity),
            Predicate {
                name: "has_name".into(),
                required_key: "name".into(),
                expected_value: None,
            },
        );
        assert!(reduce(&guarded, &input).is_ok());
    }

    #[test]
    fn guard_fails_when_predicate_does_not_hold() {
        let input = sample_input();
        let guarded = Combinator::Guard(
            Box::new(Combinator::Identity),
            Predicate {
                name: "has_admin".into(),
                required_key: "admin".into(),
                expected_value: None,
            },
        );
        assert!(reduce(&guarded, &input).is_err());
    }

    #[test]
    fn guard_checks_expected_value() {
        let input = sample_input();
        let guarded = Combinator::Guard(
            Box::new(Combinator::Identity),
            Predicate {
                name: "name_is_alice".into(),
                required_key: "name".into(),
                expected_value: Some("alice".into()),
            },
        );
        assert!(reduce(&guarded, &input).is_ok());

        let guarded_wrong = Combinator::Guard(
            Box::new(Combinator::Identity),
            Predicate {
                name: "name_is_bob".into(),
                required_key: "name".into(),
                expected_value: Some("bob".into()),
            },
        );
        assert!(reduce(&guarded_wrong, &input).is_err());
    }

    // --- Transform ---

    #[test]
    fn transform_adds_key_to_output() {
        let input = sample_input();
        let transformed = Combinator::Transform(
            Box::new(Combinator::Identity),
            TransformFn {
                name: "add_status".into(),
                output_key: "status".into(),
                output_value: "verified".into(),
            },
        );
        let output = reduce(&transformed, &input).unwrap();
        assert_eq!(output.fields.get("status"), Some(&"verified".to_string()));
        // Original fields preserved.
        assert_eq!(output.fields.get("name"), Some(&"alice".to_string()));
    }

    // --- Retry ---

    #[test]
    fn retry_succeeds_on_first_attempt_for_identity() {
        let input = sample_input();
        let retried = Combinator::Retry(
            Box::new(Combinator::Identity),
            RetryPolicy { max_retries: 3, current_attempt: 0 },
        );
        let output = reduce(&retried, &input).unwrap();
        assert_eq!(output.fields.get("retry_attempts"), Some(&"0".to_string()));
    }

    #[test]
    fn retry_exhausts_on_permanent_failure() {
        let input = sample_input();
        let retried = Combinator::Retry(
            Box::new(Combinator::Guard(
                Box::new(Combinator::Identity),
                Predicate {
                    name: "impossible".into(),
                    required_key: "nonexistent".into(),
                    expected_value: None,
                },
            )),
            RetryPolicy { max_retries: 2, current_attempt: 0 },
        );
        assert!(reduce(&retried, &input).is_err());
    }

    // --- Timeout ---

    #[test]
    fn timeout_runs_inner_and_records_budget() {
        let input = sample_input();
        let timed = Combinator::Timeout(
            Box::new(Combinator::Identity),
            Duration(5000),
        );
        let output = reduce(&timed, &input).unwrap();
        assert_eq!(output.fields.get("timeout_budget_ms"), Some(&"5000".to_string()));
    }

    // --- Checkpoint ---

    #[test]
    fn checkpoint_records_id_in_output() {
        let input = sample_input();
        let cp = Combinator::Checkpoint(
            Box::new(Combinator::Identity),
            CheckpointId("cp-001".into()),
        );
        let output = reduce(&cp, &input).unwrap();
        assert_eq!(output.checkpoint, Some(CheckpointId("cp-001".into())));
    }

    // --- Determinism ---

    #[test]
    fn reduction_is_deterministic() {
        let input = sample_input();
        let combinator = Combinator::Sequence(vec![
            Combinator::Transform(
                Box::new(Combinator::Identity),
                TransformFn {
                    name: "step1".into(),
                    output_key: "x".into(),
                    output_value: "1".into(),
                },
            ),
            Combinator::Checkpoint(
                Box::new(Combinator::Identity),
                CheckpointId("cp".into()),
            ),
        ]);

        let output1 = reduce(&combinator, &input).unwrap();
        let output2 = reduce(&combinator, &input).unwrap();
        assert_eq!(output1.fields, output2.fields);
        assert_eq!(output1.checkpoint, output2.checkpoint);
    }

    // --- Composition ---

    #[test]
    fn complex_composition() {
        let input = CombinatorInput::new()
            .with("authorized", "true")
            .with("user", "alice");

        let program = Combinator::Sequence(vec![
            Combinator::Guard(
                Box::new(Combinator::Identity),
                Predicate {
                    name: "is_authorized".into(),
                    required_key: "authorized".into(),
                    expected_value: Some("true".into()),
                },
            ),
            Combinator::Parallel(vec![
                Combinator::Transform(
                    Box::new(Combinator::Identity),
                    TransformFn {
                        name: "audit".into(),
                        output_key: "audited".into(),
                        output_value: "yes".into(),
                    },
                ),
                Combinator::Transform(
                    Box::new(Combinator::Identity),
                    TransformFn {
                        name: "log".into(),
                        output_key: "logged".into(),
                        output_value: "yes".into(),
                    },
                ),
            ]),
            Combinator::Checkpoint(
                Box::new(Combinator::Identity),
                CheckpointId("final".into()),
            ),
        ]);

        let output = reduce(&program, &input).unwrap();
        assert_eq!(output.fields.get("audited"), Some(&"yes".to_string()));
        assert_eq!(output.fields.get("logged"), Some(&"yes".to_string()));
        assert_eq!(output.checkpoint, Some(CheckpointId("final".into())));
    }

    // --- CombinatorInput helpers ---

    #[test]
    fn combinator_input_new_is_empty() {
        let input = CombinatorInput::new();
        assert!(input.fields.is_empty());
    }

    #[test]
    fn combinator_input_with_chaining() {
        let input = CombinatorInput::new().with("a", "1").with("b", "2");
        assert_eq!(input.fields.len(), 2);
    }

    // --- CombinatorOutput helpers ---

    #[test]
    fn combinator_output_merge() {
        let mut out1 = CombinatorOutput::default();
        out1.set("a", "1");
        let mut out2 = CombinatorOutput::default();
        out2.set("b", "2");
        out2.checkpoint = Some(CheckpointId("cp".into()));

        out1.merge(&out2);
        assert_eq!(out1.fields.get("a"), Some(&"1".to_string()));
        assert_eq!(out1.fields.get("b"), Some(&"2".to_string()));
        assert_eq!(out1.checkpoint, Some(CheckpointId("cp".into())));
    }

    // --- Predicate ---

    #[test]
    fn predicate_evaluate_missing_key() {
        let input = CombinatorInput::new();
        let pred = Predicate {
            name: "test".into(),
            required_key: "missing".into(),
            expected_value: None,
        };
        assert!(!pred.evaluate(&input));
    }

    #[test]
    fn predicate_evaluate_key_exists_no_value_check() {
        let input = CombinatorInput::new().with("key", "anything");
        let pred = Predicate {
            name: "test".into(),
            required_key: "key".into(),
            expected_value: None,
        };
        assert!(pred.evaluate(&input));
    }

    #[test]
    fn predicate_evaluate_value_mismatch() {
        let input = CombinatorInput::new().with("key", "actual");
        let pred = Predicate {
            name: "test".into(),
            required_key: "key".into(),
            expected_value: Some("expected".into()),
        };
        assert!(!pred.evaluate(&input));
    }

    // --- Parallel empty ---

    #[test]
    fn parallel_empty_returns_input() {
        let input = sample_input();
        let output = reduce(&Combinator::Parallel(vec![]), &input).unwrap();
        assert_eq!(output.fields, input.fields);
    }

    // --- Choice single ---

    #[test]
    fn choice_single_returns_it() {
        let input = sample_input();
        let choice = Combinator::Choice(vec![Combinator::Identity]);
        let output = reduce(&choice, &input).unwrap();
        assert_eq!(output.fields, input.fields);
    }
}
