//! Intelligent C-State selection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::types::{CState, PowerMode};
use crate::math;

// ============================================================================
// IDLE TIME PREDICTOR
// ============================================================================

/// Idle time predictor
#[derive(Debug, Clone)]
pub struct IdleTimePredictor {
    /// Recent idle durations
    history: VecDeque<u32>, // microseconds
    /// Moving average
    avg_idle: f64,
    /// Standard deviation
    std_dev: f64,
}

impl IdleTimePredictor {
    /// Create new idle time predictor
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            avg_idle: 100.0,
            std_dev: 50.0,
        }
    }

    /// Record idle duration
    pub fn record(&mut self, duration_us: u32) {
        self.history.push_back(duration_us);
        if self.history.len() > 100 {
            self.history.pop_front();
        }

        // Update statistics
        if !self.history.is_empty() {
            self.avg_idle =
                self.history.iter().map(|&d| d as f64).sum::<f64>() / self.history.len() as f64;

            let variance: f64 = self
                .history
                .iter()
                .map(|&d| math::powi(d as f64 - self.avg_idle, 2))
                .sum::<f64>()
                / self.history.len() as f64;

            self.std_dev = math::sqrt(variance);
        }
    }

    /// Predict next idle duration
    #[inline(always)]
    pub fn predict(&self) -> u32 {
        // Conservative estimate: mean - 0.5 * std_dev
        (self.avg_idle - 0.5 * self.std_dev).max(0.0) as u32
    }
}

impl Default for IdleTimePredictor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// C-STATE STATISTICS
// ============================================================================

/// C-State statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CStateStats {
    /// Time spent in each C-State (nanoseconds)
    pub time_in_state: BTreeMap<u8, u64>,
    /// Entries into each state
    pub entries: BTreeMap<u8, u64>,
    /// Wakeups that were too early
    pub early_wakeups: u64,
}

// ============================================================================
// C-STATE SELECTOR
// ============================================================================

/// Intelligent C-State selector
#[repr(align(64))]
pub struct CStateSelector {
    /// Available C-States
    available_states: Vec<CState>,
    /// Expected idle time predictor
    idle_predictor: IdleTimePredictor,
    /// Exit latency tolerance (microseconds)
    latency_tolerance: u32,
    /// Current power mode
    power_mode: PowerMode,
    /// Statistics
    stats: CStateStats,
}

impl CStateSelector {
    /// Create new C-State selector
    pub fn new(available_states: Vec<CState>) -> Self {
        Self {
            available_states,
            idle_predictor: IdleTimePredictor::new(),
            latency_tolerance: 100, // 100us default
            power_mode: PowerMode::Balanced,
            stats: CStateStats::default(),
        }
    }

    /// Select optimal C-State for expected idle
    pub fn select(&self) -> CState {
        let predicted_idle = self.idle_predictor.predict();
        let max_allowed = self.power_mode.max_cstate();

        // Find deepest state that can wake up in time
        let mut best_state = CState::C0;

        for &state in &self.available_states {
            if state > max_allowed {
                break;
            }

            // State is viable if we can wake up within tolerance
            let break_even = state.wakeup_latency() * 2; // Rule of thumb
            if predicted_idle >= break_even + self.latency_tolerance {
                best_state = state;
            }
        }

        best_state
    }

    /// Record actual idle duration
    pub fn record_idle(&mut self, predicted_state: CState, actual_duration_us: u32) {
        self.idle_predictor.record(actual_duration_us);

        // Update statistics
        let state_depth = match predicted_state {
            CState::C0 => 0,
            CState::C1 => 1,
            CState::C1E => 2,
            CState::C3 => 3,
            CState::C6 => 6,
            CState::C7 => 7,
            CState::C10 => 10,
        };

        *self.stats.entries.entry(state_depth).or_insert(0) += 1;
        *self.stats.time_in_state.entry(state_depth).or_insert(0) +=
            actual_duration_us as u64 * 1000;

        // Check for early wakeup
        if actual_duration_us < predicted_state.wakeup_latency() {
            self.stats.early_wakeups += 1;
        }
    }

    /// Set latency tolerance
    #[inline(always)]
    pub fn set_latency_tolerance(&mut self, tolerance_us: u32) {
        self.latency_tolerance = tolerance_us;
    }

    /// Set power mode
    #[inline(always)]
    pub fn set_power_mode(&mut self, mode: PowerMode) {
        self.power_mode = mode;
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &CStateStats {
        &self.stats
    }
}
