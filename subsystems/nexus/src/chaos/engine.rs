//! Chaos engineering engine

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::config::FaultConfig;
use super::experiment::{ChaosExperiment, ExperimentResults};
use super::fault::Fault;
use super::target::FaultTarget;
use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// CHAOS SAFETY
// ============================================================================

/// Safety limits for chaos engineering
#[derive(Debug, Clone)]
pub struct ChaosSafety {
    /// Allow destructive faults
    pub allow_destructive: bool,
    /// Maximum fault duration (cycles)
    pub max_duration: u64,
    /// Maximum probability
    pub max_probability: f32,
    /// Maximum concurrent faults
    pub max_concurrent: usize,
    /// Required cooldown between experiments (cycles)
    pub cooldown: u64,
    /// Last experiment end time
    pub last_experiment: Option<NexusTimestamp>,
}

impl Default for ChaosSafety {
    fn default() -> Self {
        Self {
            allow_destructive: false,
            max_duration: 60 * 1_000_000_000, // 60 seconds
            max_probability: 0.1,
            max_concurrent: 5,
            cooldown: 10 * 1_000_000_000, // 10 seconds
            last_experiment: None,
        }
    }
}

// ============================================================================
// CHAOS ENGINE
// ============================================================================

/// The main chaos engineering engine
pub struct ChaosEngine {
    /// Active faults
    faults: Vec<Fault>,
    /// Active experiments
    experiments: Vec<ChaosExperiment>,
    /// Is engine enabled
    enabled: AtomicBool,
    /// Maximum concurrent faults
    max_faults: usize,
    /// Total faults injected
    total_injected: AtomicU64,
    /// Safety limits
    safety: ChaosSafety,
}

impl ChaosEngine {
    /// Create a new chaos engine
    pub fn new() -> Self {
        Self {
            faults: Vec::new(),
            experiments: Vec::new(),
            enabled: AtomicBool::new(false), // Disabled by default
            max_faults: 10,
            total_injected: AtomicU64::new(0),
            safety: ChaosSafety::default(),
        }
    }

    /// Enable chaos engine
    #[inline(always)]
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable chaos engine
    #[inline(always)]
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Inject a fault
    pub fn inject(&mut self, config: FaultConfig) -> Option<u64> {
        if !self.is_enabled() {
            return None;
        }

        // Safety checks
        if !self.check_safety(&config) {
            return None;
        }

        // Check max faults
        if self.faults.len() >= self.max_faults {
            return None;
        }

        let fault = Fault::new(config);
        let id = fault.id;
        self.faults.push(fault);
        self.total_injected.fetch_add(1, Ordering::Relaxed);

        Some(id)
    }

    /// Stop a fault
    #[inline]
    pub fn stop_fault(&mut self, fault_id: u64) -> bool {
        if let Some(fault) = self.faults.iter_mut().find(|f| f.id == fault_id) {
            fault.stop();
            true
        } else {
            false
        }
    }

    /// Stop all faults
    #[inline]
    pub fn stop_all_faults(&mut self) {
        for fault in &mut self.faults {
            fault.stop();
        }
    }

    /// Run an experiment
    pub fn run_experiment(&mut self, mut experiment: ChaosExperiment) -> Option<u64> {
        if !self.is_enabled() {
            return None;
        }

        // Check cooldown
        if let Some(last) = self.safety.last_experiment {
            let elapsed = NexusTimestamp::now().duration_since(last);
            if elapsed < self.safety.cooldown {
                return None;
            }
        }

        experiment.start();
        let id = experiment.id;

        // Inject faults from experiment
        for fault_config in &experiment.faults {
            if self.check_safety(fault_config) {
                self.inject(fault_config.clone());
            }
        }

        self.experiments.push(experiment);
        Some(id)
    }

    /// Stop an experiment
    #[inline]
    pub fn stop_experiment(&mut self, experiment_id: u64) -> Option<&ExperimentResults> {
        if let Some(exp) = self.experiments.iter_mut().find(|e| e.id == experiment_id) {
            exp.stop();
            self.safety.last_experiment = Some(NexusTimestamp::now());
            exp.results.as_ref()
        } else {
            None
        }
    }

    /// Check if a point should be faulted
    pub fn should_fault(&mut self, component: ComponentId) -> Option<&Fault> {
        if !self.is_enabled() {
            return None;
        }

        for fault in &mut self.faults {
            if !fault.should_trigger() {
                continue;
            }

            // Check target
            let matches = match &fault.config.target {
                FaultTarget::Global => true,
                FaultTarget::Component(c) => *c == component,
                FaultTarget::Random { probability } => *probability > 0.5, // Simplified
                _ => false,
            };

            if matches {
                fault.record_occurrence();
                return Some(fault);
            }
        }

        None
    }

    /// Safety check for a fault config
    fn check_safety(&self, config: &FaultConfig) -> bool {
        // Check destructive
        if config.fault_type.is_destructive() && !self.safety.allow_destructive {
            return false;
        }

        // Check probability
        if config.probability > self.safety.max_probability {
            return false;
        }

        // Check duration
        if let Some(duration) = config.duration_cycles {
            if duration > self.safety.max_duration {
                return false;
            }
        }

        // Check concurrent faults
        let active = self.faults.iter().filter(|f| f.active).count();
        if active >= self.safety.max_concurrent {
            return false;
        }

        true
    }

    /// Get active faults
    #[inline(always)]
    pub fn active_faults(&self) -> Vec<&Fault> {
        self.faults.iter().filter(|f| f.active).collect()
    }

    /// Get running experiments
    #[inline(always)]
    pub fn running_experiments(&self) -> Vec<&ChaosExperiment> {
        self.experiments.iter().filter(|e| e.running).collect()
    }

    /// Get total faults injected
    #[inline(always)]
    pub fn total_injected(&self) -> u64 {
        self.total_injected.load(Ordering::Relaxed)
    }

    /// Get safety settings
    #[inline(always)]
    pub fn safety(&self) -> &ChaosSafety {
        &self.safety
    }

    /// Get mutable safety settings
    #[inline(always)]
    pub fn safety_mut(&mut self) -> &mut ChaosSafety {
        &mut self.safety
    }

    /// Cleanup inactive faults
    #[inline(always)]
    pub fn cleanup(&mut self) {
        self.faults.retain(|f| f.active);
        self.experiments.retain(|e| e.running);
    }

    /// Tick - check experiments and cleanup
    pub fn tick(&mut self) {
        // Check experiments
        for exp in &mut self.experiments {
            if exp.should_end() {
                exp.stop();
            }
        }

        // Check fault durations
        let now = NexusTimestamp::now();
        for fault in &mut self.faults {
            if let Some(duration) = fault.config.duration_cycles {
                if now.duration_since(fault.started) >= duration {
                    fault.stop();
                }
            }
        }
    }
}

impl Default for ChaosEngine {
    fn default() -> Self {
        Self::new()
    }
}
