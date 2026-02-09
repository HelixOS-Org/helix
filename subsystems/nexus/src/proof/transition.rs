//! State transitions
//!
//! This module provides the Transition type for defining
//! state transitions with guards and effects.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use super::state::State;

/// A state transition
pub struct Transition {
    /// Transition name
    pub name: String,
    /// Guard condition
    guard: Option<fn(&State) -> bool>,
    /// Effect
    effect: fn(&State) -> State,
}

impl Transition {
    /// Create a new transition
    pub fn new(name: impl Into<String>, effect: fn(&State) -> State) -> Self {
        Self {
            name: name.into(),
            guard: None,
            effect,
        }
    }

    /// Add guard
    #[inline(always)]
    pub fn with_guard(mut self, guard: fn(&State) -> bool) -> Self {
        self.guard = Some(guard);
        self
    }

    /// Is enabled in state?
    #[inline(always)]
    pub fn is_enabled(&self, state: &State) -> bool {
        self.guard.map(|g| g(state)).unwrap_or(true)
    }

    /// Apply transition
    #[inline]
    pub fn apply(&self, state: &State) -> State {
        let mut next = (self.effect)(state);
        next.parent = Some(state.id);
        next.transition = Some(self.name.clone());
        next
    }
}

impl core::fmt::Debug for Transition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Transition")
            .field("name", &self.name)
            .field("has_guard", &self.guard.is_some())
            .finish()
    }
}
