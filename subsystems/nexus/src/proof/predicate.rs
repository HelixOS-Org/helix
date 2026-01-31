//! Predicates over state
//!
//! This module provides the Predicate type for checking
//! conditions on system states during verification.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use super::state::State;

/// A predicate over state
pub struct Predicate {
    /// Predicate name
    pub name: String,
    /// Check function
    checker: fn(&State) -> bool,
}

impl Predicate {
    /// Create a new predicate
    pub fn new(name: impl Into<String>, checker: fn(&State) -> bool) -> Self {
        Self {
            name: name.into(),
            checker,
        }
    }

    /// Check predicate
    pub fn check(&self, state: &State) -> bool {
        (self.checker)(state)
    }
}

impl core::fmt::Debug for Predicate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Predicate")
            .field("name", &self.name)
            .finish()
    }
}
