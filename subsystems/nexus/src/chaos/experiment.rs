//! Chaos experiment definitions

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::config::FaultConfig;
use crate::core::NexusTimestamp;

// ============================================================================
// EXPERIMENT RESULTS
// ============================================================================

/// Results of a chaos experiment
#[derive(Debug, Clone, Default)]
pub struct ExperimentResults {
    /// Total faults injected
    pub faults_injected: u32,
    /// System survived
    pub survived: bool,
    /// Healing triggered
    pub healing_triggered: u32,
    /// Rollbacks triggered
    pub rollbacks_triggered: u32,
    /// Errors observed
    pub errors_observed: u32,
    /// Performance degradation percentage
    pub performance_degradation: f32,
    /// Notes
    pub notes: Vec<String>,
}

// ============================================================================
// CHAOS EXPERIMENT
// ============================================================================

/// A chaos experiment (collection of faults)
#[derive(Debug, Clone)]
pub struct ChaosExperiment {
    /// Unique experiment ID
    pub id: u64,
    /// Experiment name
    pub name: String,
    /// Description
    pub description: String,
    /// Fault configurations
    pub faults: Vec<FaultConfig>,
    /// Start timestamp
    pub started: Option<NexusTimestamp>,
    /// End timestamp
    pub ended: Option<NexusTimestamp>,
    /// Duration (cycles)
    pub duration_cycles: Option<u64>,
    /// Is experiment running
    pub running: bool,
    /// Results
    pub results: Option<ExperimentResults>,
}

impl ChaosExperiment {
    /// Create a new experiment
    pub fn new(name: impl Into<String>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            description: String::new(),
            faults: Vec::new(),
            started: None,
            ended: None,
            duration_cycles: None,
            running: false,
            results: None,
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a fault
    #[inline(always)]
    pub fn add_fault(&mut self, fault: FaultConfig) {
        self.faults.push(fault);
    }

    /// Set duration
    #[inline(always)]
    pub fn with_duration(mut self, cycles: u64) -> Self {
        self.duration_cycles = Some(cycles);
        self
    }

    /// Start experiment
    #[inline]
    pub fn start(&mut self) {
        self.started = Some(NexusTimestamp::now());
        self.running = true;
        self.results = Some(ExperimentResults::default());
    }

    /// Stop experiment
    #[inline(always)]
    pub fn stop(&mut self) {
        self.ended = Some(NexusTimestamp::now());
        self.running = false;
    }

    /// Check if experiment should end
    #[inline]
    pub fn should_end(&self) -> bool {
        if !self.running {
            return true;
        }

        if let (Some(started), Some(duration)) = (self.started, self.duration_cycles) {
            NexusTimestamp::now().duration_since(started) >= duration
        } else {
            false
        }
    }

    /// Record a fault injection
    #[inline]
    pub fn record_injection(&mut self) {
        if let Some(ref mut results) = self.results {
            results.faults_injected += 1;
        }
    }

    /// Record an error
    #[inline]
    pub fn record_error(&mut self) {
        if let Some(ref mut results) = self.results {
            results.errors_observed += 1;
        }
    }

    /// Record healing
    #[inline]
    pub fn record_healing(&mut self) {
        if let Some(ref mut results) = self.results {
            results.healing_triggered += 1;
        }
    }
}
