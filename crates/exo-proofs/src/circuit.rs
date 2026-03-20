//! Circuit abstraction -- R1CS constraint system.
//!
//! Provides the `Circuit` trait and a `ConstraintSystem` for expressing
//! arithmetic circuits as rank-1 constraint systems (A * B = C).

use serde::{Deserialize, Serialize};

use crate::error::Result;

// ---------------------------------------------------------------------------
// Variable
// ---------------------------------------------------------------------------

/// A variable in the constraint system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    /// Index into the constraint system's variable list.
    pub index: usize,
    /// The witness value (populated during proving, None during setup).
    pub value: Option<u64>,
}

// ---------------------------------------------------------------------------
// LinearCombination
// ---------------------------------------------------------------------------

/// A linear combination of variables: sum of (coefficient, variable_index) pairs.
/// The constant term uses index `usize::MAX` as a sentinel for the "one" variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinearCombination {
    pub terms: Vec<(u64, usize)>,
}

impl LinearCombination {
    /// Create an empty linear combination.
    #[must_use]
    pub fn zero() -> Self {
        Self { terms: Vec::new() }
    }

    /// Create a linear combination with a single variable.
    #[must_use]
    pub fn from_variable(var: Variable) -> Self {
        Self {
            terms: vec![(1, var.index)],
        }
    }

    /// Create a constant linear combination.
    #[must_use]
    pub fn constant(value: u64) -> Self {
        Self {
            terms: vec![(value, usize::MAX)],
        }
    }

    /// Add a term.
    pub fn add_term(&mut self, coeff: u64, var_index: usize) {
        self.terms.push((coeff, var_index));
    }

    /// Evaluate the linear combination given variable assignments.
    /// `vars[i]` = value of variable i. The "one" variable (index MAX) = 1.
    pub fn evaluate(&self, vars: &[u64]) -> u64 {
        let mut sum: u64 = 0;
        for &(coeff, idx) in &self.terms {
            let val = if idx == usize::MAX {
                1u64
            } else if idx < vars.len() {
                vars[idx]
            } else {
                0
            };
            sum = sum.wrapping_add(coeff.wrapping_mul(val));
        }
        sum
    }
}

// ---------------------------------------------------------------------------
// Constraint
// ---------------------------------------------------------------------------

/// An R1CS constraint: A * B = C, where A, B, C are linear combinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constraint {
    pub a_terms: LinearCombination,
    pub b_terms: LinearCombination,
    pub c_terms: LinearCombination,
}

// ---------------------------------------------------------------------------
// ConstraintSystem
// ---------------------------------------------------------------------------

/// A constraint system accumulating R1CS constraints.
#[derive(Debug, Clone)]
pub struct ConstraintSystem {
    /// All allocated variables.
    pub variables: Vec<Variable>,
    /// All constraints.
    pub constraints: Vec<Constraint>,
    /// Number of public input variables.
    pub num_public_inputs: usize,
    /// Indices of public input variables.
    pub public_input_indices: Vec<usize>,
}

impl ConstraintSystem {
    /// Create a new empty constraint system.
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            constraints: Vec::new(),
            num_public_inputs: 0,
            public_input_indices: Vec::new(),
        }
    }

    /// Number of variables.
    #[must_use]
    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }

    /// Number of constraints.
    #[must_use]
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// Check if all constraints are satisfied by the current variable assignments.
    pub fn is_satisfied(&self) -> bool {
        let vals: Vec<u64> = self
            .variables
            .iter()
            .map(|v| v.value.unwrap_or(0))
            .collect();
        for c in &self.constraints {
            let a = c.a_terms.evaluate(&vals);
            let b = c.b_terms.evaluate(&vals);
            let c_val = c.c_terms.evaluate(&vals);
            if a.wrapping_mul(b) != c_val {
                return false;
            }
        }
        true
    }
}

impl Default for ConstraintSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocate a variable in the constraint system.
pub fn allocate(cs: &mut ConstraintSystem, value: Option<u64>) -> Variable {
    let index = cs.variables.len();
    let var = Variable { index, value };
    cs.variables.push(var);
    var
}

/// Allocate a public input variable.
pub fn allocate_public(cs: &mut ConstraintSystem, value: Option<u64>) -> Variable {
    let var = allocate(cs, value);
    cs.public_input_indices.push(var.index);
    cs.num_public_inputs += 1;
    var
}

/// Enforce an R1CS constraint: A * B = C.
pub fn enforce(
    cs: &mut ConstraintSystem,
    a: &LinearCombination,
    b: &LinearCombination,
    c: &LinearCombination,
) {
    cs.constraints.push(Constraint {
        a_terms: a.clone(),
        b_terms: b.clone(),
        c_terms: c.clone(),
    });
}

// ---------------------------------------------------------------------------
// Circuit trait
// ---------------------------------------------------------------------------

/// A circuit that can synthesize constraints.
pub trait Circuit {
    /// Synthesize the circuit's constraints into the given constraint system.
    fn synthesize(&self, cs: &mut ConstraintSystem) -> Result<()>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple circuit: x * y = z
    #[derive(Debug)]
    struct MultiplyCircuit {
        x: Option<u64>,
        y: Option<u64>,
        z: Option<u64>,
    }

    impl Circuit for MultiplyCircuit {
        fn synthesize(&self, cs: &mut ConstraintSystem) -> Result<()> {
            let x = allocate_public(cs, self.x);
            let y = allocate(cs, self.y);
            let z = allocate_public(cs, self.z);

            // x * y = z
            let a = LinearCombination::from_variable(x);
            let b = LinearCombination::from_variable(y);
            let c = LinearCombination::from_variable(z);
            enforce(cs, &a, &b, &c);

            Ok(())
        }
    }

    /// A circuit that checks a + b = c using a * 1 = (c - a) pattern: (a + b) * 1 = c
    #[derive(Debug)]
    struct AddCircuit {
        a: Option<u64>,
        b: Option<u64>,
        c: Option<u64>,
    }

    impl Circuit for AddCircuit {
        fn synthesize(&self, cs: &mut ConstraintSystem) -> Result<()> {
            let a_var = allocate_public(cs, self.a);
            let b_var = allocate(cs, self.b);
            let c_var = allocate_public(cs, self.c);

            // (a + b) * 1 = c
            let mut a_lc = LinearCombination::zero();
            a_lc.add_term(1, a_var.index);
            a_lc.add_term(1, b_var.index);
            let b_lc = LinearCombination::constant(1);
            let c_lc = LinearCombination::from_variable(c_var);
            enforce(cs, &a_lc, &b_lc, &c_lc);

            Ok(())
        }
    }

    #[test]
    fn empty_constraint_system() {
        let cs = ConstraintSystem::new();
        assert_eq!(cs.num_variables(), 0);
        assert_eq!(cs.num_constraints(), 0);
        assert!(cs.is_satisfied());
    }

    #[test]
    fn default_constraint_system() {
        let cs = ConstraintSystem::default();
        assert!(cs.is_satisfied());
    }

    #[test]
    fn allocate_variable() {
        let mut cs = ConstraintSystem::new();
        let v = allocate(&mut cs, Some(42));
        assert_eq!(v.index, 0);
        assert_eq!(v.value, Some(42));
        assert_eq!(cs.num_variables(), 1);
    }

    #[test]
    fn allocate_public_input() {
        let mut cs = ConstraintSystem::new();
        let v = allocate_public(&mut cs, Some(10));
        assert_eq!(v.index, 0);
        assert_eq!(cs.num_public_inputs, 1);
    }

    #[test]
    fn multiply_circuit_satisfied() {
        let circuit = MultiplyCircuit {
            x: Some(3),
            y: Some(4),
            z: Some(12),
        };
        let mut cs = ConstraintSystem::new();
        circuit.synthesize(&mut cs).unwrap();
        assert_eq!(cs.num_variables(), 3);
        assert_eq!(cs.num_constraints(), 1);
        assert_eq!(cs.num_public_inputs, 2);
        assert!(cs.is_satisfied());
    }

    #[test]
    fn multiply_circuit_not_satisfied() {
        let circuit = MultiplyCircuit {
            x: Some(3),
            y: Some(4),
            z: Some(13), // wrong
        };
        let mut cs = ConstraintSystem::new();
        circuit.synthesize(&mut cs).unwrap();
        assert!(!cs.is_satisfied());
    }

    #[test]
    fn add_circuit_satisfied() {
        let circuit = AddCircuit {
            a: Some(5),
            b: Some(7),
            c: Some(12),
        };
        let mut cs = ConstraintSystem::new();
        circuit.synthesize(&mut cs).unwrap();
        assert!(cs.is_satisfied());
    }

    #[test]
    fn add_circuit_not_satisfied() {
        let circuit = AddCircuit {
            a: Some(5),
            b: Some(7),
            c: Some(11), // wrong
        };
        let mut cs = ConstraintSystem::new();
        circuit.synthesize(&mut cs).unwrap();
        assert!(!cs.is_satisfied());
    }

    #[test]
    fn linear_combination_evaluate() {
        let mut lc = LinearCombination::zero();
        lc.add_term(2, 0); // 2 * x0
        lc.add_term(3, 1); // 3 * x1
        lc.add_term(5, usize::MAX); // 5 * 1

        let vars = vec![10, 20]; // x0=10, x1=20
        assert_eq!(lc.evaluate(&vars), 2 * 10 + 3 * 20 + 5);
    }

    #[test]
    fn linear_combination_zero() {
        let lc = LinearCombination::zero();
        assert_eq!(lc.evaluate(&[1, 2, 3]), 0);
    }

    #[test]
    fn linear_combination_constant() {
        let lc = LinearCombination::constant(42);
        assert_eq!(lc.evaluate(&[]), 42);
    }

    #[test]
    fn linear_combination_from_variable() {
        let v = Variable {
            index: 2,
            value: Some(7),
        };
        let lc = LinearCombination::from_variable(v);
        assert_eq!(lc.evaluate(&[0, 0, 7]), 7);
    }

    #[test]
    fn enforce_constraint() {
        let mut cs = ConstraintSystem::new();
        let x = allocate(&mut cs, Some(5));
        let y = allocate(&mut cs, Some(5));
        let z = allocate(&mut cs, Some(25));

        let a = LinearCombination::from_variable(x);
        let b = LinearCombination::from_variable(y);
        let c = LinearCombination::from_variable(z);
        enforce(&mut cs, &a, &b, &c);

        assert!(cs.is_satisfied());
    }

    #[test]
    fn multiple_constraints() {
        let mut cs = ConstraintSystem::new();
        let x = allocate(&mut cs, Some(2));
        let y = allocate(&mut cs, Some(3));
        let z = allocate(&mut cs, Some(6)); // x*y
        let w = allocate(&mut cs, Some(36)); // z*z

        // x * y = z
        enforce(
            &mut cs,
            &LinearCombination::from_variable(x),
            &LinearCombination::from_variable(y),
            &LinearCombination::from_variable(z),
        );

        // z * z = w
        enforce(
            &mut cs,
            &LinearCombination::from_variable(z),
            &LinearCombination::from_variable(z),
            &LinearCombination::from_variable(w),
        );

        assert_eq!(cs.num_constraints(), 2);
        assert!(cs.is_satisfied());
    }

    #[test]
    fn variable_without_value() {
        let mut cs = ConstraintSystem::new();
        let _v = allocate(&mut cs, None);
        assert_eq!(cs.variables[0].value, None);
    }

    #[test]
    fn constraint_clone_eq() {
        let c = Constraint {
            a_terms: LinearCombination::constant(1),
            b_terms: LinearCombination::constant(2),
            c_terms: LinearCombination::constant(2),
        };
        let c2 = c.clone();
        assert_eq!(c, c2);
    }
}
