//! Verification engine
//!
//! This module provides the main Verifier for performing
//! bounded model checking verification on models.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::model::Model;
use super::predicate::Predicate;
use super::property::Property;
use super::state::{Counterexample, State};
use super::types::{PropertyType, VerificationOutcome};
use crate::core::NexusTimestamp;

/// Result of verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Property verified
    pub property: Property,
    /// Verification outcome
    pub outcome: VerificationOutcome,
    /// Counterexample (if any)
    pub counterexample: Option<Counterexample>,
    /// Verification time (cycles)
    pub duration: u64,
    /// States explored
    pub states_explored: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Configuration for verification
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    /// Maximum states to explore
    pub max_states: u64,
    /// Maximum depth
    pub max_depth: u64,
    /// Timeout (cycles)
    pub timeout: u64,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            max_states: 100000,
            max_depth: 1000,
            timeout: 1000000000, // ~1 second at 1GHz
        }
    }
}

/// Verifier statistics
#[derive(Debug, Clone)]
pub struct VerifierStats {
    /// Total verifications
    pub total_verifications: u64,
    /// Verified properties
    pub verified_properties: usize,
    /// Falsified properties
    pub falsified_properties: usize,
    /// States in cache
    pub states_in_cache: usize,
}

/// The verification engine
pub struct Verifier {
    /// Configuration
    config: VerifierConfig,
    /// States explored
    explored: BTreeMap<u64, State>,
    /// Pending states
    pending: Vec<State>,
    /// Verification history
    history: Vec<VerificationResult>,
    /// Total verifications
    total_verifications: AtomicU64,
    /// Is verifier enabled?
    enabled: AtomicBool,
}

impl Verifier {
    /// Create a new verifier
    pub fn new(config: VerifierConfig) -> Self {
        Self {
            config,
            explored: BTreeMap::new(),
            pending: Vec::new(),
            history: Vec::new(),
            total_verifications: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Verify a model
    pub fn verify(&mut self, model: &Model) -> Vec<VerificationResult> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Vec::new();
        }

        self.total_verifications.fetch_add(1, Ordering::Relaxed);

        let start = NexusTimestamp::now();
        let mut results = Vec::new();

        // Reset state
        self.explored.clear();
        self.pending.clear();

        // Add initial state
        let initial = model.initial();
        self.pending.push(initial);

        // BFS exploration
        let mut states_explored = 0u64;

        while let Some(state) = self.pending.pop() {
            if states_explored >= self.config.max_states {
                break;
            }

            let hash = state.hash();
            if self.explored.contains_key(&hash) {
                continue;
            }

            // Check invariants
            let violated = model.check_invariants(&state);
            if !violated.is_empty() {
                // Invariant violation
                for inv in violated {
                    let prop = Property::new(&inv.name, PropertyType::Invariant).critical();

                    let trace = self.build_trace(&state);
                    let counterexample = Counterexample::new(trace, 0);

                    results.push(VerificationResult {
                        property: prop,
                        outcome: VerificationOutcome::Falsified,
                        counterexample: Some(counterexample),
                        duration: NexusTimestamp::now().duration_since(start),
                        states_explored,
                        timestamp: NexusTimestamp::now(),
                    });
                }
            }

            // Add to explored
            self.explored.insert(hash, state.clone());
            states_explored += 1;

            // Explore successors
            for trans in model.enabled_transitions(&state) {
                let next = trans.apply(&state);
                let next_hash = next.hash();
                if !self.explored.contains_key(&next_hash) {
                    self.pending.push(next);
                }
            }
        }

        // Check properties on all explored states
        for (prop, pred) in model.properties() {
            let mut found_violation = false;
            let mut counterexample = None;

            for state in self.explored.values() {
                if !pred.check(state) {
                    found_violation = true;
                    let trace = self.build_trace(state);
                    counterexample = Some(Counterexample::new(trace, 0));
                    break;
                }
            }

            let outcome = if found_violation {
                VerificationOutcome::Falsified
            } else {
                VerificationOutcome::Verified
            };

            results.push(VerificationResult {
                property: prop.clone(),
                outcome,
                counterexample,
                duration: NexusTimestamp::now().duration_since(start),
                states_explored,
                timestamp: NexusTimestamp::now(),
            });
        }

        // Store in history
        for result in &results {
            self.history.push(result.clone());
        }

        results
    }

    /// Build trace from state
    fn build_trace(&self, state: &State) -> Vec<State> {
        let mut trace = Vec::new();
        let mut current = Some(state);

        while let Some(s) = current {
            trace.push(s.clone());
            current = s.parent.and_then(|p| {
                // Find parent by ID (inefficient but correct)
                self.explored.values().find(|st| st.id == p)
            });
        }

        trace.reverse();
        trace
    }

    /// Quick check a predicate on current states
    pub fn quick_check(&self, pred: &Predicate) -> bool {
        self.explored.values().all(|s| pred.check(s))
    }

    /// Get verification history
    pub fn history(&self) -> &[VerificationResult] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> VerifierStats {
        let verified = self
            .history
            .iter()
            .filter(|r| r.outcome.is_success())
            .count();
        let falsified = self
            .history
            .iter()
            .filter(|r| r.outcome.is_failure())
            .count();

        VerifierStats {
            total_verifications: self.total_verifications.load(Ordering::Relaxed),
            verified_properties: verified,
            falsified_properties: falsified,
            states_in_cache: self.explored.len(),
        }
    }

    /// Enable verifier
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable verifier
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Is verifier enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.explored.clear();
        self.pending.clear();
    }

    /// Get configuration
    pub fn config(&self) -> &VerifierConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: VerifierConfig) {
        self.config = config;
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new(VerifierConfig::default())
    }
}
