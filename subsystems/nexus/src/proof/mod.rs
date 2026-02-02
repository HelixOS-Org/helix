//! # Formal Verification
//!
//! Lightweight formal verification for critical kernel properties.
//!
//! ## Key Features
//!
//! - **Property Specification**: Define properties to verify
//! - **Invariant Checking**: Verify invariants hold
//! - **Model Checking**: Bounded state exploration
//! - **Proof Obligation**: Generate proof obligations
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core types (PropertyType, VerificationOutcome)
//! - `property`: Property specification
//! - `state`: State representation and values
//! - `predicate`: Predicates over state
//! - `transition`: State transitions
//! - `model`: Model definition
//! - `verifier`: Verification engine

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod model;
pub mod predicate;
pub mod property;
pub mod state;
pub mod transition;
pub mod types;
pub mod verifier;

// Re-export core types
// Re-export model types
pub use model::Model;
// Re-export predicate types
pub use predicate::Predicate;
// Re-export property types
pub use property::{Property, invariant, progress_property, safety_property};
// Re-export state types
pub use state::{Counterexample, State, Value};
// Re-export transition types
pub use transition::Transition;
pub use types::{PropertyType, VerificationOutcome};
// Re-export verifier types
pub use verifier::{VerificationResult, Verifier, VerifierConfig, VerifierStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn initial_counter() -> State {
        let mut state = State::new();
        state.set("count", Value::Uint(0));
        state
    }

    fn increment(state: &State) -> State {
        let mut next = State::new();
        let count = state.get("count").and_then(|v| v.as_uint()).unwrap_or(0);
        next.set("count", Value::Uint(count + 1));
        next
    }

    fn count_is_positive(state: &State) -> bool {
        state
            .get("count")
            .and_then(|v| v.as_uint())
            .map(|c| c >= 0)
            .unwrap_or(false)
    }

    fn count_under_10(state: &State) -> bool {
        state
            .get("count")
            .and_then(|v| v.as_uint())
            .map(|c| c < 10)
            .unwrap_or(true)
    }

    #[test]
    fn test_state() {
        let mut state = State::new();
        state.set("x", Value::Int(42));
        assert_eq!(state.get("x").and_then(|v| v.as_int()), Some(42));
    }

    #[test]
    fn test_predicate() {
        let mut state = State::new();
        state.set("count", Value::Uint(5));

        let pred = Predicate::new("positive", count_is_positive);
        assert!(pred.check(&state));
    }

    #[test]
    fn test_model_and_verification() {
        let mut model = Model::new("counter", initial_counter);

        // Add increment transition (only if count < 10)
        model.add_transition(Transition::new("increment", increment).with_guard(count_under_10));

        // Add invariant: count is always positive
        model.add_invariant(Predicate::new("positive", count_is_positive));

        // Add property
        model.add_property(
            safety_property("bounded", "Count stays under limit"),
            Predicate::new("under_10", count_under_10),
        );

        // Verify
        let config = VerifierConfig {
            max_states: 100,
            max_depth: 20,
            ..Default::default()
        };
        let mut verifier = Verifier::new(config);
        let results = verifier.verify(&model);

        // All should be verified
        assert!(results.iter().all(|r| r.outcome.is_success()));
    }

    #[test]
    fn test_property() {
        let prop = Property::new("test", PropertyType::Safety)
            .requires("x > 0")
            .ensures("result >= 0")
            .critical();

        assert!(prop.critical);
        assert_eq!(prop.preconditions.len(), 1);
        assert_eq!(prop.postconditions.len(), 1);
    }
}
