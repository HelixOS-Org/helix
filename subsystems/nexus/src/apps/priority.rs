//! # Application Dynamic Priority Management
//!
//! Per-application priority analysis and tuning:
//! - Priority class system (real-time → idle)
//! - Dynamic priority adjustment based on behavior
//! - Priority inversion detection
//! - Niceness-aware scheduling hints
//! - Priority inheritance tracking
//! - Deadline-aware boosting

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// PRIORITY CLASSES
// ============================================================================

/// Priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityClass {
    /// Real-time critical
    RealTime    = 0,
    /// System services
    System      = 1,
    /// High priority user
    HighUser    = 2,
    /// Normal
    Normal      = 3,
    /// Below normal
    BelowNormal = 4,
    /// Low background
    Background  = 5,
    /// Idle only
    Idle        = 6,
}

impl PriorityClass {
    /// Base nice value for class
    pub fn base_nice(&self) -> i32 {
        match self {
            Self::RealTime => -20,
            Self::System => -15,
            Self::HighUser => -10,
            Self::Normal => 0,
            Self::BelowNormal => 5,
            Self::Background => 10,
            Self::Idle => 19,
        }
    }

    /// Timeslice multiplier
    pub fn timeslice_multiplier(&self) -> u32 {
        match self {
            Self::RealTime => 8,
            Self::System => 6,
            Self::HighUser => 4,
            Self::Normal => 2,
            Self::BelowNormal => 1,
            Self::Background => 1,
            Self::Idle => 1,
        }
    }

    /// Whether preemption is allowed
    pub fn preemptible(&self) -> bool {
        !matches!(self, Self::RealTime)
    }
}

// ============================================================================
// PRIORITY ADJUSTMENT
// ============================================================================

/// Reason for priority adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentReason {
    /// Interactive behavior detected
    Interactive,
    /// CPU-bound detected
    CpuBound,
    /// I/O-bound detected
    IoBound,
    /// Deadline approaching
    DeadlineUrgent,
    /// Priority inversion fix
    InversionFix,
    /// Starvation prevention
    StarvationFix,
    /// User request
    UserRequest,
    /// Energy saving
    EnergySaving,
    /// Thermal throttle
    ThermalThrottle,
    /// Aging bonus
    Aging,
}

/// Priority adjustment record
#[derive(Debug, Clone)]
pub struct PriorityAdjustment {
    /// Process ID
    pub pid: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Old effective priority
    pub old_priority: i32,
    /// New effective priority
    pub new_priority: i32,
    /// Reason
    pub reason: AdjustmentReason,
    /// Duration (0 = permanent)
    pub duration_ms: u64,
    /// Expiry timestamp
    pub expires_at: u64,
}

// ============================================================================
// PRIORITY INVERSION
// ============================================================================

/// Priority inversion event
#[derive(Debug, Clone)]
pub struct InversionEvent {
    /// High priority process blocked
    pub blocked_pid: u64,
    /// Low priority process holding resource
    pub holder_pid: u64,
    /// Resource identifier
    pub resource_id: u64,
    /// Detected timestamp
    pub detected_at: u64,
    /// Duration so far (us)
    pub duration_us: u64,
    /// Whether inheritance applied
    pub inheritance_applied: bool,
}

/// Priority inheritance state
#[derive(Debug, Clone)]
pub struct InheritanceState {
    /// Process that received boosted priority
    pub boosted_pid: u64,
    /// Original priority
    pub original_priority: i32,
    /// Boosted priority
    pub boosted_priority: i32,
    /// Chain of blocked processes
    pub blocked_chain: Vec<u64>,
    /// Applied timestamp
    pub applied_at: u64,
}

// ============================================================================
// DEADLINE
// ============================================================================

/// Deadline info for a process
#[derive(Debug, Clone)]
pub struct DeadlineInfo {
    /// Process ID
    pub pid: u64,
    /// Deadline timestamp
    pub deadline: u64,
    /// Period (ms)
    pub period_ms: u64,
    /// Worst-case execution time (ms)
    pub wcet_ms: u64,
    /// Running budget remaining (ms)
    pub budget_remaining_ms: u64,
    /// Deadlines met
    pub deadlines_met: u64,
    /// Deadlines missed
    pub deadlines_missed: u64,
}

impl DeadlineInfo {
    /// Utilization (WCET / period)
    pub fn utilization(&self) -> f64 {
        if self.period_ms == 0 {
            return 0.0;
        }
        self.wcet_ms as f64 / self.period_ms as f64
    }

    /// Miss rate
    pub fn miss_rate(&self) -> f64 {
        let total = self.deadlines_met + self.deadlines_missed;
        if total == 0 {
            return 0.0;
        }
        self.deadlines_missed as f64 / total as f64
    }

    /// Is urgent (deadline approaching)
    pub fn is_urgent(&self, now: u64) -> bool {
        now + self.wcet_ms as u64 >= self.deadline
    }
}

// ============================================================================
// PROCESS PRIORITY STATE
// ============================================================================

/// Complete priority state for a process
#[derive(Debug, Clone)]
pub struct ProcessPriorityState {
    /// Process ID
    pub pid: u64,
    /// Static class
    pub class: PriorityClass,
    /// User-set nice value
    pub nice: i32,
    /// Effective priority (lower = higher priority)
    pub effective: i32,
    /// Whether temporarily boosted
    pub boosted: bool,
    /// Boost expiry
    pub boost_expires: u64,
    /// Active inversions
    pub inversion_count: u32,
    /// Deadline info (if real-time)
    pub deadline: Option<DeadlineInfo>,
    /// CPU time since last adjustment (ms)
    pub cpu_since_adjust: u64,
    /// Wait time since last adjustment (ms)
    pub wait_since_adjust: u64,
    /// Recent adjustments
    pub adjustment_history: Vec<PriorityAdjustment>,
    /// Max history
    max_history: usize,
}

impl ProcessPriorityState {
    pub fn new(pid: u64, class: PriorityClass) -> Self {
        Self {
            pid,
            class,
            nice: class.base_nice(),
            effective: class.base_nice(),
            boosted: false,
            boost_expires: 0,
            inversion_count: 0,
            deadline: None,
            cpu_since_adjust: 0,
            wait_since_adjust: 0,
            adjustment_history: Vec::new(),
            max_history: 32,
        }
    }

    /// Apply adjustment
    pub fn apply_adjustment(
        &mut self,
        reason: AdjustmentReason,
        new_priority: i32,
        now: u64,
        duration_ms: u64,
    ) {
        let adj = PriorityAdjustment {
            pid: self.pid,
            timestamp: now,
            old_priority: self.effective,
            new_priority,
            reason,
            duration_ms,
            expires_at: if duration_ms > 0 {
                now + duration_ms
            } else {
                0
            },
        };

        self.effective = new_priority;
        if duration_ms > 0 {
            self.boosted = true;
            self.boost_expires = now + duration_ms;
        }

        self.adjustment_history.push(adj);
        if self.adjustment_history.len() > self.max_history {
            self.adjustment_history.remove(0);
        }

        self.cpu_since_adjust = 0;
        self.wait_since_adjust = 0;
    }

    /// Check if boost expired
    pub fn check_expiry(&mut self, now: u64) {
        if self.boosted && now >= self.boost_expires {
            self.boosted = false;
            self.effective = self.nice;
        }
    }

    /// CPU/wait ratio
    pub fn cpu_wait_ratio(&self) -> f64 {
        let total = self.cpu_since_adjust + self.wait_since_adjust;
        if total == 0 {
            return 0.5;
        }
        self.cpu_since_adjust as f64 / total as f64
    }
}

// ============================================================================
// PRIORITY MANAGER
// ============================================================================

/// Priority analyzer stats
#[derive(Debug, Clone, Default)]
pub struct PriorityStats {
    /// Total processes managed
    pub total_processes: usize,
    /// Active boosts
    pub active_boosts: u32,
    /// Total adjustments made
    pub total_adjustments: u64,
    /// Inversions detected
    pub inversions_detected: u64,
    /// Inversions resolved
    pub inversions_resolved: u64,
    /// Starvation events prevented
    pub starvation_prevented: u64,
}

/// Application priority analyzer
pub struct AppPriorityAnalyzer {
    /// Per-process states
    states: BTreeMap<u64, ProcessPriorityState>,
    /// Active inversions
    inversions: Vec<InversionEvent>,
    /// Active inheritances
    inheritances: Vec<InheritanceState>,
    /// Stats
    stats: PriorityStats,
    /// Interactive threshold (CPU ratio below this = interactive)
    interactive_threshold: f64,
    /// Starvation threshold (ms without running)
    starvation_threshold_ms: u64,
    /// Aging interval (ms)
    aging_interval_ms: u64,
}

impl AppPriorityAnalyzer {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            inversions: Vec::new(),
            inheritances: Vec::new(),
            stats: PriorityStats::default(),
            interactive_threshold: 0.3,
            starvation_threshold_ms: 500,
            aging_interval_ms: 250,
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64, class: PriorityClass) {
        self.states
            .insert(pid, ProcessPriorityState::new(pid, class));
        self.stats.total_processes = self.states.len();
    }

    /// Set nice value
    pub fn set_nice(&mut self, pid: u64, nice: i32, now: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.nice = nice.clamp(-20, 19);
            if !state.boosted {
                state.apply_adjustment(AdjustmentReason::UserRequest, state.nice, now, 0);
            }
        }
    }

    /// Update cpu/wait times
    pub fn update_times(&mut self, pid: u64, cpu_ms: u64, wait_ms: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.cpu_since_adjust += cpu_ms;
            state.wait_since_adjust += wait_ms;
        }
    }

    /// Detect interactive process and boost
    pub fn detect_interactive(&mut self, pid: u64, now: u64) -> bool {
        let threshold = self.interactive_threshold;
        if let Some(state) = self.states.get_mut(&pid) {
            if state.cpu_wait_ratio() < threshold && !state.boosted {
                let new_prio = (state.nice - 5).max(-20);
                state.apply_adjustment(AdjustmentReason::Interactive, new_prio, now, 1000);
                self.stats.total_adjustments += 1;
                self.stats.active_boosts += 1;
                return true;
            }
        }
        false
    }

    /// Report priority inversion
    pub fn report_inversion(
        &mut self,
        blocked_pid: u64,
        holder_pid: u64,
        resource_id: u64,
        now: u64,
    ) {
        let event = InversionEvent {
            blocked_pid,
            holder_pid,
            resource_id,
            detected_at: now,
            duration_us: 0,
            inheritance_applied: false,
        };
        self.inversions.push(event);
        self.stats.inversions_detected += 1;

        // Apply priority inheritance
        let blocked_prio = self
            .states
            .get(&blocked_pid)
            .map(|s| s.effective)
            .unwrap_or(0);

        if let Some(holder) = self.states.get_mut(&holder_pid) {
            if holder.effective > blocked_prio {
                let original = holder.effective;
                holder.apply_adjustment(AdjustmentReason::InversionFix, blocked_prio, now, 100);
                holder.inversion_count += 1;

                self.inheritances.push(InheritanceState {
                    boosted_pid: holder_pid,
                    original_priority: original,
                    boosted_priority: blocked_prio,
                    blocked_chain: alloc::vec![blocked_pid],
                    applied_at: now,
                });

                self.stats.active_boosts += 1;
                self.stats.total_adjustments += 1;
            }
        }

        // Mark event
        if let Some(evt) = self.inversions.last_mut() {
            evt.inheritance_applied = true;
        }
    }

    /// Resolve inversion
    pub fn resolve_inversion(&mut self, holder_pid: u64, resource_id: u64) {
        self.inversions
            .retain(|i| !(i.holder_pid == holder_pid && i.resource_id == resource_id));
        self.inheritances.retain(|i| i.boosted_pid != holder_pid);
        self.stats.inversions_resolved += 1;

        if let Some(state) = self.states.get_mut(&holder_pid) {
            if state.inversion_count > 0 {
                state.inversion_count -= 1;
            }
        }
    }

    /// Check starvation and apply aging
    pub fn check_starvation(&mut self, now: u64) -> Vec<u64> {
        let threshold = self.starvation_threshold_ms;
        let mut starved = Vec::new();

        let pids: Vec<u64> = self.states.keys().copied().collect();
        for pid in pids {
            let should_boost = {
                if let Some(state) = self.states.get(&pid) {
                    state.wait_since_adjust > threshold && !state.boosted
                } else {
                    false
                }
            };

            if should_boost {
                if let Some(state) = self.states.get_mut(&pid) {
                    let new_prio = (state.effective - 3).max(-20);
                    state.apply_adjustment(AdjustmentReason::StarvationFix, new_prio, now, 200);
                    starved.push(pid);
                    self.stats.starvation_prevented += 1;
                    self.stats.total_adjustments += 1;
                    self.stats.active_boosts += 1;
                }
            }
        }

        starved
    }

    /// Expire old boosts
    pub fn tick(&mut self, now: u64) {
        for state in self.states.values_mut() {
            if state.boosted && now >= state.boost_expires {
                state.boosted = false;
                state.effective = state.nice;
                if self.stats.active_boosts > 0 {
                    self.stats.active_boosts -= 1;
                }
            }
        }
    }

    /// Get state
    pub fn state(&self, pid: u64) -> Option<&ProcessPriorityState> {
        self.states.get(&pid)
    }

    /// Get stats
    pub fn stats(&self) -> &PriorityStats {
        &self.stats
    }

    /// Unregister
    pub fn unregister(&mut self, pid: u64) {
        self.states.remove(&pid);
        self.inversions
            .retain(|i| i.blocked_pid != pid && i.holder_pid != pid);
        self.inheritances.retain(|i| i.boosted_pid != pid);
        self.stats.total_processes = self.states.len();
    }
}
