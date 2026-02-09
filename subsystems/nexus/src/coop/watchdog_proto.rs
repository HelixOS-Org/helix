//! # Coop Watchdog Protocol
//!
//! Distributed watchdog for cooperative process liveness:
//! - Heartbeat monitoring with adaptive intervals
//! - Phi accrual failure detector
//! - Partitioned failure detection
//! - Recovery orchestration
//! - Watchdog escalation chains

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// WATCHDOG TYPES
// ============================================================================

/// Process liveness state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    /// Alive and responsive
    Alive,
    /// Suspected failure
    Suspected,
    /// Confirmed dead
    Dead,
    /// Unreachable (network partition)
    Unreachable,
    /// Recovering
    Recovering,
}

/// Recovery action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogRecovery {
    /// No action needed
    None,
    /// Restart process
    Restart,
    /// Failover to backup
    Failover,
    /// Notify dependent processes
    Notify,
    /// Escalate to administrator
    Escalate,
    /// Fence (isolate) process
    Fence,
}

/// Escalation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscalationLevel {
    /// Info
    Info,
    /// Warning
    Warning,
    /// Alert
    Alert,
    /// Critical
    Critical,
    /// Emergency
    Emergency,
}

// ============================================================================
// PHI ACCRUAL FAILURE DETECTOR
// ============================================================================

/// Phi accrual failure detector
#[derive(Debug)]
pub struct PhiDetector {
    /// Heartbeat intervals (ns)
    intervals: VecDeque<u64>,
    /// Max samples
    max_samples: usize,
    /// Last heartbeat (ns)
    pub last_heartbeat_ns: u64,
    /// Threshold phi for suspected
    pub suspected_threshold: f64,
    /// Threshold phi for dead
    pub dead_threshold: f64,
}

impl PhiDetector {
    pub fn new(suspected_threshold: f64, dead_threshold: f64) -> Self {
        Self {
            intervals: VecDeque::new(),
            max_samples: 64,
            last_heartbeat_ns: 0,
            suspected_threshold,
            dead_threshold,
        }
    }

    /// Record heartbeat
    #[inline]
    pub fn heartbeat(&mut self, now: u64) {
        if self.last_heartbeat_ns > 0 {
            let interval = now.saturating_sub(self.last_heartbeat_ns);
            if self.intervals.len() >= self.max_samples {
                self.intervals.pop_front();
            }
            self.intervals.push_back(interval);
        }
        self.last_heartbeat_ns = now;
    }

    /// Compute mean interval
    fn mean(&self) -> f64 {
        if self.intervals.is_empty() {
            return 1_000_000_000.0; // Default 1s
        }
        let sum: u64 = self.intervals.iter().sum();
        sum as f64 / self.intervals.len() as f64
    }

    /// Compute variance
    fn variance(&self) -> f64 {
        if self.intervals.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let sum_sq: f64 = self
            .intervals
            .iter()
            .map(|&i| {
                let d = i as f64 - mean;
                d * d
            })
            .sum();
        sum_sq / (self.intervals.len() - 1) as f64
    }

    /// Compute phi value
    pub fn phi(&self, now: u64) -> f64 {
        if self.last_heartbeat_ns == 0 || self.intervals.is_empty() {
            return 0.0;
        }
        let elapsed = now.saturating_sub(self.last_heartbeat_ns) as f64;
        let mean = self.mean();
        let variance = self.variance();
        let stddev = libm::sqrt(variance.max(1.0));

        // Phi = -log10(P(next_heartbeat > elapsed))
        // Using normal distribution approximation
        let z = (elapsed - mean) / stddev;
        // Approximate CDF using logistic function
        let p = 1.0 / (1.0 + libm::exp(-1.7 * z));
        if p >= 1.0 {
            16.0 // Cap at high phi
        } else {
            -libm::log(1.0 - p) / core::f64::consts::LN_10
        }
    }

    /// Determine state
    #[inline]
    pub fn state(&self, now: u64) -> LivenessState {
        let phi = self.phi(now);
        if phi >= self.dead_threshold {
            LivenessState::Dead
        } else if phi >= self.suspected_threshold {
            LivenessState::Suspected
        } else {
            LivenessState::Alive
        }
    }
}

// ============================================================================
// WATCHED PROCESS
// ============================================================================

/// Watched process
#[derive(Debug)]
pub struct WatchedProcess {
    /// PID
    pub pid: u64,
    /// Phi detector
    pub detector: PhiDetector,
    /// Current state
    pub state: LivenessState,
    /// Escalation level
    pub escalation: EscalationLevel,
    /// Recovery action configured
    pub recovery: WatchdogRecovery,
    /// Dependents (PIDs to notify)
    pub dependents: Vec<u64>,
    /// Failure count
    pub failure_count: u64,
    /// Last state change (ns)
    pub last_change_ns: u64,
}

impl WatchedProcess {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            detector: PhiDetector::new(4.0, 8.0),
            state: LivenessState::Alive,
            escalation: EscalationLevel::Info,
            recovery: WatchdogRecovery::Restart,
            dependents: Vec::new(),
            failure_count: 0,
            last_change_ns: 0,
        }
    }

    /// Record heartbeat
    pub fn heartbeat(&mut self, now: u64) {
        self.detector.heartbeat(now);
        let new_state = self.detector.state(now);
        if new_state != self.state {
            self.state = new_state;
            self.last_change_ns = now;
            if matches!(new_state, LivenessState::Dead) {
                self.failure_count += 1;
                self.escalate();
            }
        }
    }

    /// Check state
    #[inline]
    pub fn check(&mut self, now: u64) -> LivenessState {
        let new_state = self.detector.state(now);
        if new_state != self.state {
            self.state = new_state;
            self.last_change_ns = now;
            if matches!(new_state, LivenessState::Dead | LivenessState::Suspected) {
                self.escalate();
            }
        }
        self.state
    }

    fn escalate(&mut self) {
        self.escalation = match self.failure_count {
            0..=1 => EscalationLevel::Warning,
            2..=3 => EscalationLevel::Alert,
            4..=5 => EscalationLevel::Critical,
            _ => EscalationLevel::Emergency,
        };
    }

    /// Add dependent
    #[inline]
    pub fn add_dependent(&mut self, pid: u64) {
        if !self.dependents.contains(&pid) {
            self.dependents.push(pid);
        }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Watchdog protocol stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopWatchdogProtoStats {
    /// Watched processes
    pub watched_processes: usize,
    /// Alive
    pub alive_count: usize,
    /// Suspected
    pub suspected_count: usize,
    /// Dead
    pub dead_count: usize,
    /// Total failures detected
    pub total_failures: u64,
}

/// Coop watchdog protocol
pub struct CoopWatchdogProtocol {
    /// Watched processes
    processes: BTreeMap<u64, WatchedProcess>,
    /// Stats
    stats: CoopWatchdogProtoStats,
}

impl CoopWatchdogProtocol {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: CoopWatchdogProtoStats::default(),
        }
    }

    /// Watch process
    #[inline(always)]
    pub fn watch(&mut self, pid: u64) -> &mut WatchedProcess {
        self.processes
            .entry(pid)
            .or_insert_with(|| WatchedProcess::new(pid))
    }

    /// Record heartbeat
    #[inline]
    pub fn heartbeat(&mut self, pid: u64, now: u64) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.heartbeat(now);
        }
        self.update_stats();
    }

    /// Check all processes
    pub fn check_all(&mut self, now: u64) -> Vec<(u64, LivenessState, WatchdogRecovery)> {
        let mut actions = Vec::new();
        let pids: Vec<u64> = self.processes.keys().cloned().collect();
        for pid in pids {
            if let Some(proc) = self.processes.get_mut(&pid) {
                let prev = proc.state;
                let new_state = proc.check(now);
                if new_state != prev
                    && matches!(new_state, LivenessState::Dead | LivenessState::Suspected)
                {
                    actions.push((pid, new_state, proc.recovery));
                }
            }
        }
        self.update_stats();
        actions
    }

    /// Unwatch process
    #[inline(always)]
    pub fn unwatch(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.watched_processes = self.processes.len();
        self.stats.alive_count = self
            .processes
            .values()
            .filter(|p| p.state == LivenessState::Alive)
            .count();
        self.stats.suspected_count = self
            .processes
            .values()
            .filter(|p| p.state == LivenessState::Suspected)
            .count();
        self.stats.dead_count = self
            .processes
            .values()
            .filter(|p| p.state == LivenessState::Dead)
            .count();
        self.stats.total_failures = self.processes.values().map(|p| p.failure_count).sum();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopWatchdogProtoStats {
        &self.stats
    }
}
