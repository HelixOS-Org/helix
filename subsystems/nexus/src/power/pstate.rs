//! Intelligent P-State governor.

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::types::{PState, PowerMode};
use super::workload::WorkloadPredictor;

// ============================================================================
// GOVERNOR ALGORITHM
// ============================================================================

/// Governor algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorAlgorithm {
    /// Ondemand-like
    OnDemand,
    /// Conservative (gradual changes)
    Conservative,
    /// Schedutil (scheduler-integrated)
    Schedutil,
    /// Powersave (minimum)
    Powersave,
    /// Performance (maximum)
    Performance,
}

// ============================================================================
// P-STATE STATISTICS
// ============================================================================

/// P-State statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PStateStats {
    /// Time in each state
    pub time_in_state: ArrayMap<u64, 32>, // freq -> nanoseconds
    /// State transitions
    pub transitions: u64,
    /// Energy saved estimate
    pub energy_saved: f64,
}

// ============================================================================
// P-STATE GOVERNOR
// ============================================================================

/// Intelligent P-State governor
#[repr(align(64))]
pub struct PStateGovernor {
    /// Available P-States
    available_states: Vec<PState>,
    /// Current state index
    current_idx: usize,
    /// Workload predictor
    workload: WorkloadPredictor,
    /// Power mode
    power_mode: PowerMode,
    /// Governor algorithm
    algorithm: GovernorAlgorithm,
    /// Hysteresis counter
    hysteresis: u32,
    /// Statistics
    stats: PStateStats,
}

impl PStateGovernor {
    /// Create new governor
    pub fn new(available_states: Vec<PState>) -> Self {
        let max_idx = available_states.len().saturating_sub(1);
        Self {
            available_states,
            current_idx: max_idx,
            workload: WorkloadPredictor::new(),
            power_mode: PowerMode::Balanced,
            algorithm: GovernorAlgorithm::Schedutil,
            hysteresis: 0,
            stats: PStateStats::default(),
        }
    }

    /// Update with current load and get recommended state
    pub fn update(&mut self, current_load: f64, minute: u8) -> &PState {
        self.workload.record(current_load, minute);

        let target_idx = match self.algorithm {
            GovernorAlgorithm::Performance => self.available_states.len() - 1,
            GovernorAlgorithm::Powersave => 0,
            GovernorAlgorithm::OnDemand => self.select_ondemand(current_load),
            GovernorAlgorithm::Conservative => self.select_conservative(current_load),
            GovernorAlgorithm::Schedutil => self.select_schedutil(current_load),
        };

        // Apply hysteresis to prevent oscillation
        if target_idx != self.current_idx {
            self.hysteresis += 1;
            if self.hysteresis >= 3 {
                // Transition
                self.stats.transitions += 1;
                self.current_idx = target_idx;
                self.hysteresis = 0;
            }
        } else {
            self.hysteresis = 0;
        }

        &self.available_states[self.current_idx]
    }

    /// OnDemand algorithm
    fn select_ondemand(&self, load: f64) -> usize {
        let target_perf = self.power_mode.target_performance();

        if load > 0.8 {
            // Go to max
            self.available_states.len() - 1
        } else {
            // Scale with load
            let desired = load * 1.2 * target_perf;
            self.find_state_for_perf(desired)
        }
    }

    /// Conservative algorithm
    fn select_conservative(&self, load: f64) -> usize {
        let target = self.select_ondemand(load);

        // Only move one step at a time
        match target.cmp(&self.current_idx) {
            Ordering::Greater => self.current_idx.saturating_add(1).min(target),
            Ordering::Less => self.current_idx.saturating_sub(1).max(target),
            Ordering::Equal => self.current_idx,
        }
    }

    /// Schedutil algorithm (scheduler-integrated)
    fn select_schedutil(&self, _load: f64) -> usize {
        let predicted = self.workload.predict();
        let target_perf = self.power_mode.target_performance();

        // Use prediction with margin
        let target_load = if self.workload.is_increasing() {
            (predicted * 1.3).min(1.0) * target_perf
        } else {
            predicted * target_perf
        };

        self.find_state_for_perf(target_load)
    }

    /// Find P-State for target performance
    fn find_state_for_perf(&self, target_perf: f64) -> usize {
        for (i, state) in self.available_states.iter().enumerate() {
            if state.relative_perf >= target_perf {
                return i;
            }
        }
        self.available_states.len() - 1
    }

    /// Set algorithm
    #[inline(always)]
    pub fn set_algorithm(&mut self, algo: GovernorAlgorithm) {
        self.algorithm = algo;
    }

    /// Set power mode
    #[inline(always)]
    pub fn set_power_mode(&mut self, mode: PowerMode) {
        self.power_mode = mode;
    }

    /// Get current state
    #[inline(always)]
    pub fn current_state(&self) -> &PState {
        &self.available_states[self.current_idx]
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &PStateStats {
        &self.stats
    }

    /// Get workload predictor
    #[inline(always)]
    pub fn workload_predictor(&self) -> &WorkloadPredictor {
        &self.workload
    }
}
