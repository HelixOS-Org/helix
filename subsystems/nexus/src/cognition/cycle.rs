//! # Cognitive Cycle Management
//!
//! Manages the execution of cognitive cycles.
//! Each cycle processes signals through all domains.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// CYCLE TYPES
// ============================================================================

/// A cognitive cycle
#[derive(Debug, Clone)]
pub struct CognitiveCycle {
    /// Cycle ID
    pub id: u64,
    /// Cycle number
    pub number: u64,
    /// Start time
    pub start_time: Timestamp,
    /// End time (if completed)
    pub end_time: Option<Timestamp>,
    /// Current phase
    pub phase: CyclePhase,
    /// Phase results
    pub phase_results: BTreeMap<CyclePhase, PhaseResult>,
    /// Cycle mode
    pub mode: CycleMode,
    /// Total signals processed
    pub signals_processed: u64,
    /// Total patterns detected
    pub patterns_detected: u64,
    /// Actions taken
    pub actions_taken: u64,
    /// Errors encountered
    pub errors: Vec<CycleError>,
}

/// Cycle phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CyclePhase {
    /// Initialization
    Init        = 0,
    /// SENSE: Collect signals
    Sense       = 1,
    /// UNDERSTAND: Pattern detection
    Understand  = 2,
    /// REASON: Causal analysis
    Reason      = 3,
    /// DECIDE: Option evaluation
    Decide      = 4,
    /// ACT: Execute actions
    Act         = 5,
    /// REFLECT: Evaluate outcomes
    Reflect     = 6,
    /// LEARN: Update knowledge
    Learn       = 7,
    /// LTM: Memory consolidation
    Consolidate = 8,
    /// Finalization
    Finalize    = 9,
}

impl CyclePhase {
    /// Get next phase
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Init => Some(Self::Sense),
            Self::Sense => Some(Self::Understand),
            Self::Understand => Some(Self::Reason),
            Self::Reason => Some(Self::Decide),
            Self::Decide => Some(Self::Act),
            Self::Act => Some(Self::Reflect),
            Self::Reflect => Some(Self::Learn),
            Self::Learn => Some(Self::Consolidate),
            Self::Consolidate => Some(Self::Finalize),
            Self::Finalize => None,
        }
    }

    /// Get associated domain
    pub fn domain(&self) -> Option<&'static str> {
        match self {
            Self::Sense => Some("sense"),
            Self::Understand => Some("understand"),
            Self::Reason => Some("reason"),
            Self::Decide => Some("decide"),
            Self::Act => Some("act"),
            Self::Reflect => Some("reflect"),
            Self::Learn => Some("learn"),
            Self::Consolidate => Some("ltm"),
            _ => None,
        }
    }
}

/// Result of a phase
#[derive(Debug, Clone)]
pub struct PhaseResult {
    /// Phase
    pub phase: CyclePhase,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
    /// Success
    pub success: bool,
    /// Items processed
    pub items_processed: u64,
    /// Items produced
    pub items_produced: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Cycle mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleMode {
    /// Full cycle through all phases
    Full,
    /// Quick cycle - skip optional phases
    Quick,
    /// Emergency cycle - minimal processing
    Emergency,
    /// Focused cycle - specific phases only
    Focused,
    /// Learning cycle - extended learning phase
    Learning,
}

/// Cycle error
#[derive(Debug, Clone)]
pub struct CycleError {
    /// Phase where error occurred
    pub phase: CyclePhase,
    /// Error code
    pub code: u32,
    /// Error message
    pub message: String,
    /// Is recoverable
    pub recoverable: bool,
    /// Timestamp
    pub timestamp: Timestamp,
}

// ============================================================================
// CYCLE MANAGER
// ============================================================================

/// Manages cognitive cycles
pub struct CycleManager {
    /// Current cycle
    current: Option<CognitiveCycle>,
    /// Cycle history
    history: Vec<CycleSummary>,
    /// Next cycle ID
    next_id: AtomicU64,
    /// Current cycle number
    cycle_number: u64,
    /// Configuration
    config: CycleConfig,
    /// Statistics
    stats: CycleStats,
}

/// Cycle summary for history
#[derive(Debug, Clone)]
pub struct CycleSummary {
    /// Cycle ID
    pub id: u64,
    /// Cycle number
    pub number: u64,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
    /// Mode
    pub mode: CycleMode,
    /// Success
    pub success: bool,
    /// Signals processed
    pub signals: u64,
    /// Patterns detected
    pub patterns: u64,
    /// Actions taken
    pub actions: u64,
    /// Error count
    pub errors: u32,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct CycleConfig {
    /// Maximum cycle duration (nanoseconds)
    pub max_duration_ns: u64,
    /// Phase timeout (nanoseconds)
    pub phase_timeout_ns: u64,
    /// History size
    pub max_history: usize,
    /// Enable phase skipping on overrun
    pub allow_skip: bool,
    /// Phases to skip in quick mode
    pub quick_skip_phases: Vec<CyclePhase>,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            max_duration_ns: 10_000_000, // 10ms
            phase_timeout_ns: 1_000_000, // 1ms per phase
            max_history: 1000,
            allow_skip: true,
            quick_skip_phases: vec![CyclePhase::Learn, CyclePhase::Consolidate],
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct CycleStats {
    /// Total cycles run
    pub total_cycles: u64,
    /// Successful cycles
    pub successful_cycles: u64,
    /// Failed cycles
    pub failed_cycles: u64,
    /// Total processing time (ns)
    pub total_time_ns: u64,
    /// Average cycle time (ns)
    pub avg_time_ns: u64,
    /// Fastest cycle (ns)
    pub min_time_ns: u64,
    /// Slowest cycle (ns)
    pub max_time_ns: u64,
    /// Phase timeouts
    pub phase_timeouts: u64,
    /// Total signals
    pub total_signals: u64,
    /// Total patterns
    pub total_patterns: u64,
    /// Total actions
    pub total_actions: u64,
}

impl CycleManager {
    /// Create a new cycle manager
    pub fn new(config: CycleConfig) -> Self {
        Self {
            current: None,
            history: Vec::new(),
            next_id: AtomicU64::new(1),
            cycle_number: 0,
            config,
            stats: CycleStats {
                min_time_ns: u64::MAX,
                ..Default::default()
            },
        }
    }

    /// Start a new cycle
    pub fn start_cycle(&mut self, mode: CycleMode) -> u64 {
        // Complete any existing cycle
        if self.current.is_some() {
            self.abort_cycle("New cycle started".into());
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.cycle_number += 1;

        let cycle = CognitiveCycle {
            id,
            number: self.cycle_number,
            start_time: Timestamp::now(),
            end_time: None,
            phase: CyclePhase::Init,
            phase_results: BTreeMap::new(),
            mode,
            signals_processed: 0,
            patterns_detected: 0,
            actions_taken: 0,
            errors: Vec::new(),
        };

        self.current = Some(cycle);
        self.stats.total_cycles += 1;

        id
    }

    /// Advance to next phase
    pub fn advance_phase(&mut self) -> Option<CyclePhase> {
        let cycle = self.current.as_mut()?;

        let next = cycle.phase.next()?;

        // Check if should skip
        if cycle.mode == CycleMode::Quick && self.config.quick_skip_phases.contains(&next) {
            cycle.phase = next;
            return self.advance_phase();
        }

        cycle.phase = next;
        Some(next)
    }

    /// Record phase result
    pub fn record_phase_result(&mut self, result: PhaseResult) {
        if let Some(cycle) = self.current.as_mut() {
            cycle.phase_results.insert(result.phase, result);
        }
    }

    /// Record signal
    pub fn record_signal(&mut self) {
        if let Some(cycle) = self.current.as_mut() {
            cycle.signals_processed += 1;
            self.stats.total_signals += 1;
        }
    }

    /// Record pattern
    pub fn record_pattern(&mut self) {
        if let Some(cycle) = self.current.as_mut() {
            cycle.patterns_detected += 1;
            self.stats.total_patterns += 1;
        }
    }

    /// Record action
    pub fn record_action(&mut self) {
        if let Some(cycle) = self.current.as_mut() {
            cycle.actions_taken += 1;
            self.stats.total_actions += 1;
        }
    }

    /// Record error
    pub fn record_error(
        &mut self,
        phase: CyclePhase,
        code: u32,
        message: String,
        recoverable: bool,
    ) {
        if let Some(cycle) = self.current.as_mut() {
            cycle.errors.push(CycleError {
                phase,
                code,
                message,
                recoverable,
                timestamp: Timestamp::now(),
            });
        }
    }

    /// Complete the current cycle
    pub fn complete_cycle(&mut self) -> Option<CycleSummary> {
        let mut cycle = self.current.take()?;
        cycle.end_time = Some(Timestamp::now());

        let duration = cycle.end_time.unwrap().elapsed_since(cycle.start_time);
        let success = cycle.errors.iter().all(|e| e.recoverable);

        // Update stats
        self.stats.total_time_ns += duration;
        if success {
            self.stats.successful_cycles += 1;
        } else {
            self.stats.failed_cycles += 1;
        }

        if duration < self.stats.min_time_ns {
            self.stats.min_time_ns = duration;
        }
        if duration > self.stats.max_time_ns {
            self.stats.max_time_ns = duration;
        }

        self.stats.avg_time_ns = self.stats.total_time_ns / self.stats.total_cycles;

        let summary = CycleSummary {
            id: cycle.id,
            number: cycle.number,
            duration_ns: duration,
            mode: cycle.mode,
            success,
            signals: cycle.signals_processed,
            patterns: cycle.patterns_detected,
            actions: cycle.actions_taken,
            errors: cycle.errors.len() as u32,
        };

        // Add to history
        if self.history.len() >= self.config.max_history {
            self.history.remove(0);
        }
        self.history.push(summary.clone());

        Some(summary)
    }

    /// Abort the current cycle
    pub fn abort_cycle(&mut self, reason: String) {
        if let Some(cycle) = self.current.as_mut() {
            cycle.errors.push(CycleError {
                phase: cycle.phase,
                code: 999,
                message: reason,
                recoverable: false,
                timestamp: Timestamp::now(),
            });
        }
        self.complete_cycle();
    }

    /// Get current cycle
    pub fn current(&self) -> Option<&CognitiveCycle> {
        self.current.as_ref()
    }

    /// Get current phase
    pub fn current_phase(&self) -> Option<CyclePhase> {
        self.current.as_ref().map(|c| c.phase)
    }

    /// Get cycle number
    pub fn cycle_number(&self) -> u64 {
        self.cycle_number
    }

    /// Get history
    pub fn history(&self) -> &[CycleSummary] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> &CycleStats {
        &self.stats
    }

    /// Check if cycle is running
    pub fn is_running(&self) -> bool {
        self.current.is_some()
    }

    /// Get elapsed time for current cycle
    pub fn elapsed(&self) -> Option<u64> {
        self.current
            .as_ref()
            .map(|c| Timestamp::now().elapsed_since(c.start_time))
    }

    /// Check if current cycle is overrunning
    pub fn is_overrunning(&self) -> bool {
        self.elapsed()
            .map(|e| e > self.config.max_duration_ns)
            .unwrap_or(false)
    }
}

// ============================================================================
// CYCLE SCHEDULER
// ============================================================================

/// Schedules cognitive cycles
pub struct CycleScheduler {
    /// Target cycle frequency (Hz)
    target_frequency: u32,
    /// Minimum interval (nanoseconds)
    min_interval_ns: u64,
    /// Last cycle timestamp
    last_cycle: Timestamp,
    /// Cycle budget tracker
    budget: CycleBudget,
}

/// Budget for cycle
#[derive(Debug, Clone)]
pub struct CycleBudget {
    /// Total budget (nanoseconds)
    pub total_ns: u64,
    /// Per-phase budgets
    pub phase_budgets: BTreeMap<CyclePhase, u64>,
    /// Used budget
    pub used_ns: u64,
}

impl CycleScheduler {
    /// Create a new scheduler
    pub fn new(target_frequency_hz: u32) -> Self {
        let interval = if target_frequency_hz > 0 {
            1_000_000_000 / target_frequency_hz as u64
        } else {
            100_000_000 // 100ms default
        };

        Self {
            target_frequency: target_frequency_hz,
            min_interval_ns: interval,
            last_cycle: Timestamp::now(),
            budget: CycleBudget {
                total_ns: interval,
                phase_budgets: BTreeMap::new(),
                used_ns: 0,
            },
        }
    }

    /// Check if should start new cycle
    pub fn should_start(&self) -> bool {
        let now = Timestamp::now();
        now.elapsed_since(self.last_cycle) >= self.min_interval_ns
    }

    /// Mark cycle started
    pub fn cycle_started(&mut self) {
        self.last_cycle = Timestamp::now();
        self.budget.used_ns = 0;
    }

    /// Get remaining budget
    pub fn remaining_budget(&self) -> u64 {
        self.budget.total_ns.saturating_sub(self.budget.used_ns)
    }

    /// Record usage
    pub fn record_usage(&mut self, ns: u64) {
        self.budget.used_ns += ns;
    }

    /// Get phase budget
    pub fn phase_budget(&self, phase: CyclePhase) -> u64 {
        self.budget
            .phase_budgets
            .get(&phase)
            .copied()
            .unwrap_or(self.budget.total_ns / 10) // Default 10% per phase
    }

    /// Set phase budget
    pub fn set_phase_budget(&mut self, phase: CyclePhase, budget_ns: u64) {
        self.budget.phase_budgets.insert(phase, budget_ns);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_creation() {
        let config = CycleConfig::default();
        let mut manager = CycleManager::new(config);

        let id = manager.start_cycle(CycleMode::Full);
        assert!(id > 0);
        assert!(manager.is_running());
    }

    #[test]
    fn test_phase_advance() {
        let config = CycleConfig::default();
        let mut manager = CycleManager::new(config);

        manager.start_cycle(CycleMode::Full);
        assert_eq!(manager.current_phase(), Some(CyclePhase::Init));

        manager.advance_phase();
        assert_eq!(manager.current_phase(), Some(CyclePhase::Sense));

        manager.advance_phase();
        assert_eq!(manager.current_phase(), Some(CyclePhase::Understand));
    }

    #[test]
    fn test_cycle_completion() {
        let config = CycleConfig::default();
        let mut manager = CycleManager::new(config);

        manager.start_cycle(CycleMode::Full);
        manager.record_signal();
        manager.record_pattern();
        manager.record_action();

        let summary = manager.complete_cycle();
        assert!(summary.is_some());

        let s = summary.unwrap();
        assert_eq!(s.signals, 1);
        assert_eq!(s.patterns, 1);
        assert_eq!(s.actions, 1);
    }

    #[test]
    fn test_scheduler() {
        let scheduler = CycleScheduler::new(100); // 100 Hz
        assert!(scheduler.remaining_budget() > 0);
    }
}
