//! Model definition for verification
//!
//! This module provides the Model type for defining systems
//! to verify with initial states, transitions, and properties.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::predicate::Predicate;
use super::property::Property;
use super::state::State;
use super::transition::Transition;

/// A model to verify
pub struct Model {
    /// Model name
    pub name: String,
    /// Initial state generator
    initial_state: fn() -> State,
    /// Transitions
    transitions: Vec<Transition>,
    /// Properties to verify
    properties: Vec<(Property, Predicate)>,
    /// Invariants (checked on all states)
    invariants: Vec<Predicate>,
}

impl Model {
    /// Create a new model
    pub fn new(name: impl Into<String>, initial_state: fn() -> State) -> Self {
        Self {
            name: name.into(),
            initial_state,
            transitions: Vec::new(),
            properties: Vec::new(),
            invariants: Vec::new(),
        }
    }

    /// Add transition
    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    /// Add property
    pub fn add_property(&mut self, property: Property, predicate: Predicate) {
        self.properties.push((property, predicate));
    }

    /// Add invariant
    pub fn add_invariant(&mut self, predicate: Predicate) {
        self.invariants.push(predicate);
    }

    /// Get initial state
    pub fn initial(&self) -> State {
        (self.initial_state)()
    }

    /// Get enabled transitions
    pub fn enabled_transitions(&self, state: &State) -> Vec<&Transition> {
        self.transitions
            .iter()
            .filter(|t| t.is_enabled(state))
            .collect()
    }

    /// Check invariants
    pub fn check_invariants(&self, state: &State) -> Vec<&Predicate> {
        self.invariants
            .iter()
            .filter(|inv| !inv.check(state))
            .collect()
    }

    /// Get all properties
    pub fn properties(&self) -> &[(Property, Predicate)] {
        &self.properties
    }

    /// Get all invariants
    pub fn invariants(&self) -> &[Predicate] {
        &self.invariants
    }

    /// Get transitions count
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }
}
