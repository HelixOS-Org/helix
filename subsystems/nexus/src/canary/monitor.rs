//! Canary monitor for invariants and canary values.

#![allow(clippy::excessive_nesting)]

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::canary::Canary;
use super::invariant::{Invariant, InvariantCheck};
use crate::core::NexusTimestamp;

/// Monitor for canary values and invariants
pub struct CanaryMonitor {
    /// Invariants
    invariants: BTreeMap<u64, Invariant>,
    /// Canaries
    canaries: BTreeMap<String, Canary>,
    /// Violation history
    violations: VecDeque<InvariantCheck>,
    /// Maximum violations to keep
    max_violations: usize,
    /// Total checks
    total_checks: AtomicU64,
    /// Total violations
    total_violations: AtomicU64,
    /// Is monitoring enabled?
    enabled: AtomicBool,
}

impl CanaryMonitor {
    /// Create a new monitor
    pub fn new() -> Self {
        Self {
            invariants: BTreeMap::new(),
            canaries: BTreeMap::new(),
            violations: VecDeque::new(),
            max_violations: 1000,
            total_checks: AtomicU64::new(0),
            total_violations: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Add an invariant
    #[inline(always)]
    pub fn add_invariant(&mut self, invariant: Invariant) {
        self.invariants.insert(invariant.id, invariant);
    }

    /// Remove an invariant
    #[inline(always)]
    pub fn remove_invariant(&mut self, id: u64) -> Option<Invariant> {
        self.invariants.remove(&id)
    }

    /// Add a canary
    #[inline(always)]
    pub fn add_canary(&mut self, name: impl Into<String>, canary: Canary) {
        self.canaries.insert(name.into(), canary);
    }

    /// Check all canaries
    #[inline]
    pub fn check_canaries(&self) -> Vec<(&str, bool)> {
        self.canaries
            .iter()
            .map(|(name, canary)| (name.as_str(), canary.check()))
            .collect()
    }

    /// Check if all canaries are intact
    #[inline(always)]
    pub fn all_canaries_intact(&self) -> bool {
        self.canaries.values().all(|c| c.check())
    }

    /// Run all invariant checks
    pub fn check_all(&mut self) -> Vec<InvariantCheck> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Vec::new();
        }

        let now = NexusTimestamp::now();
        let mut results = Vec::new();

        // We need to collect IDs first due to borrow checker
        let ids: Vec<u64> = self.invariants.keys().copied().collect();

        for id in ids {
            if let Some(invariant) = self.invariants.get_mut(&id) {
                if invariant.should_check(now) {
                    let check = invariant.check();
                    self.total_checks.fetch_add(1, Ordering::Relaxed);

                    if check.violated() {
                        self.total_violations.fetch_add(1, Ordering::Relaxed);

                        if self.violations.len() >= self.max_violations {
                            self.violations.pop_front();
                        }
                        self.violations.push_back(check.clone());
                    }

                    results.push(check);
                }
            }
        }

        results
    }

    /// Check a specific invariant
    pub fn check_one(&mut self, id: u64) -> Option<InvariantCheck> {
        if let Some(invariant) = self.invariants.get_mut(&id) {
            let check = invariant.check();
            self.total_checks.fetch_add(1, Ordering::Relaxed);

            if check.violated() {
                self.total_violations.fetch_add(1, Ordering::Relaxed);
                self.violations.push_back(check.clone());
            }

            Some(check)
        } else {
            None
        }
    }

    /// Get violations
    #[inline(always)]
    pub fn violations(&self) -> &[InvariantCheck] {
        &self.violations
    }

    /// Get critical violations
    #[inline(always)]
    pub fn critical_violations(&self) -> Vec<&InvariantCheck> {
        self.violations.iter().filter(|v| v.critical).collect()
    }

    /// Get all invariants
    #[inline(always)]
    pub fn invariants(&self) -> impl Iterator<Item = &Invariant> {
        self.invariants.values()
    }

    /// Get invariant by ID
    #[inline(always)]
    pub fn get_invariant(&self, id: u64) -> Option<&Invariant> {
        self.invariants.get(&id)
    }

    /// Get invariant by name
    #[inline(always)]
    pub fn get_invariant_by_name(&self, name: &str) -> Option<&Invariant> {
        self.invariants.values().find(|i| i.name == name)
    }

    /// Enable monitoring
    #[inline(always)]
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable monitoring
    #[inline(always)]
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Clear violation history
    #[inline(always)]
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> CanaryStats {
        let critical_violations = self.violations.iter().filter(|v| v.critical).count();

        CanaryStats {
            invariant_count: self.invariants.len(),
            canary_count: self.canaries.len(),
            total_checks: self.total_checks.load(Ordering::Relaxed),
            total_violations: self.total_violations.load(Ordering::Relaxed),
            critical_violations: critical_violations as u64,
            canaries_intact: self.all_canaries_intact(),
        }
    }
}

impl Default for CanaryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Canary monitoring statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CanaryStats {
    /// Number of invariants
    pub invariant_count: usize,
    /// Number of canaries
    pub canary_count: usize,
    /// Total checks performed
    pub total_checks: u64,
    /// Total violations detected
    pub total_violations: u64,
    /// Critical violations
    pub critical_violations: u64,
    /// Are all canaries intact?
    pub canaries_intact: bool,
}
