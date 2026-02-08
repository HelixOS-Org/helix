//! # Cooperative Priority Negotiation
//!
//! Priority negotiation between cooperating processes:
//! - Priority inheritance chains
//! - Priority ceiling protocols
//! - Dynamic priority adjustment via negotiation
//! - Fairness-aware priority boosting
//! - Anti-starvation guarantees

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// PRIORITY TYPES
// ============================================================================

/// Priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopPriorityClass {
    /// Idle priority
    Idle,
    /// Background
    Background,
    /// Normal
    Normal,
    /// Above normal
    AboveNormal,
    /// High
    High,
    /// Realtime
    Realtime,
    /// Critical
    Critical,
}

/// Priority negotiation action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiationAction {
    /// Request boost
    RequestBoost,
    /// Offer boost
    OfferBoost,
    /// Accept boost
    AcceptBoost,
    /// Decline boost
    DeclineBoost,
    /// Donate priority
    Donate,
    /// Return donated priority
    ReturnDonated,
    /// Inherit from blocked holder
    Inherit,
}

/// Boost reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopBoostReason {
    /// Priority inheritance (holding resource needed by high-prio)
    Inheritance,
    /// Priority ceiling
    Ceiling,
    /// Cooperative donation
    Donation,
    /// Anti-starvation
    AntiStarvation,
    /// Deadline approaching
    Deadline,
    /// System request
    System,
}

// ============================================================================
// PRIORITY STATE
// ============================================================================

/// Active boost
#[derive(Debug, Clone)]
pub struct ActiveBoost {
    /// Boost reason
    pub reason: CoopBoostReason,
    /// Boosted to class
    pub boosted_to: CoopPriorityClass,
    /// Granted by (pid)
    pub granted_by: u64,
    /// Grant timestamp
    pub granted_at: u64,
    /// Expiry
    pub expires_at: u64,
}

/// Per-process priority state
#[derive(Debug)]
pub struct ProcessPriority {
    /// Process id
    pub pid: u64,
    /// Base priority
    pub base: CoopPriorityClass,
    /// Effective (boosted) priority
    pub effective: CoopPriorityClass,
    /// Active boosts
    boosts: Vec<ActiveBoost>,
    /// Max simultaneous boosts
    max_boosts: usize,
    /// Total boosts received
    pub total_boosts: u64,
    /// Total boosts donated
    pub total_donations: u64,
    /// Time at elevated priority (ns)
    pub elevated_time_ns: u64,
    /// Last elevation start
    elevation_start: Option<u64>,
    /// Starvation counter (scheduling rounds without running)
    pub starvation_count: u64,
    /// Starvation threshold
    starvation_threshold: u64,
}

impl ProcessPriority {
    pub fn new(pid: u64, base: CoopPriorityClass) -> Self {
        Self {
            pid,
            base,
            effective: base,
            boosts: Vec::new(),
            max_boosts: 8,
            total_boosts: 0,
            total_donations: 0,
            elevated_time_ns: 0,
            elevation_start: None,
            starvation_count: 0,
            starvation_threshold: 100,
        }
    }

    /// Apply boost
    pub fn apply_boost(&mut self, boost: ActiveBoost, now: u64) -> bool {
        if self.boosts.len() >= self.max_boosts {
            return false;
        }
        let new_class = boost.boosted_to;
        self.boosts.push(boost);
        self.total_boosts += 1;
        self.recalculate_effective(now);
        new_class > self.base
    }

    /// Remove expired boosts
    pub fn expire_boosts(&mut self, now: u64) {
        self.boosts.retain(|b| now < b.expires_at);
        self.recalculate_effective(now);
    }

    /// Remove boosts from a specific grantor
    pub fn remove_boosts_from(&mut self, grantor: u64, now: u64) {
        self.boosts.retain(|b| b.granted_by != grantor);
        self.recalculate_effective(now);
    }

    fn recalculate_effective(&mut self, now: u64) {
        let old_effective = self.effective;
        self.effective = self.base;
        for boost in &self.boosts {
            if boost.boosted_to > self.effective {
                self.effective = boost.boosted_to;
            }
        }
        // Track elevation time
        if self.effective > self.base {
            if self.elevation_start.is_none() {
                self.elevation_start = Some(now);
            }
        } else if let Some(start) = self.elevation_start.take() {
            self.elevated_time_ns += now.saturating_sub(start);
        }
        let _ = old_effective; // suppress warning
    }

    /// Is currently boosted?
    pub fn is_boosted(&self) -> bool {
        self.effective > self.base
    }

    /// Is starving?
    pub fn is_starving(&self) -> bool {
        self.starvation_count >= self.starvation_threshold
    }

    /// Tick starvation counter
    pub fn tick_starvation(&mut self) {
        self.starvation_count += 1;
    }

    /// Reset starvation (process ran)
    pub fn reset_starvation(&mut self) {
        self.starvation_count = 0;
    }
}

// ============================================================================
// NEGOTIATION
// ============================================================================

/// Negotiation request
#[derive(Debug, Clone)]
pub struct NegotiationRequest {
    /// Requester pid
    pub requester: u64,
    /// Target pid
    pub target: u64,
    /// Action
    pub action: NegotiationAction,
    /// Desired class
    pub desired_class: CoopPriorityClass,
    /// Reason
    pub reason: CoopBoostReason,
    /// Timestamp
    pub timestamp: u64,
    /// Duration (ns)
    pub duration_ns: u64,
}

/// Negotiation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiationResult {
    /// Accepted
    Accepted,
    /// Declined
    Declined,
    /// Counter-offered
    CounterOffer,
    /// Timed out
    TimedOut,
    /// Invalid
    Invalid,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Priority negotiation stats
#[derive(Debug, Clone, Default)]
pub struct CoopPriorityStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Currently boosted
    pub boosted_count: usize,
    /// Starving count
    pub starving_count: usize,
    /// Total negotiations
    pub total_negotiations: u64,
    /// Accepted negotiations
    pub accepted: u64,
    /// Declined negotiations
    pub declined: u64,
}

/// Cooperative priority engine
pub struct CoopPriorityEngine {
    /// Per-process state
    processes: BTreeMap<u64, ProcessPriority>,
    /// Pending requests
    pending: Vec<NegotiationRequest>,
    /// Max pending
    max_pending: usize,
    /// Stats
    stats: CoopPriorityStats,
}

impl CoopPriorityEngine {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            pending: Vec::new(),
            max_pending: 256,
            stats: CoopPriorityStats::default(),
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64, base: CoopPriorityClass) {
        self.processes.insert(pid, ProcessPriority::new(pid, base));
        self.update_stats();
    }

    /// Remove process
    pub fn remove(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    /// Submit negotiation request
    pub fn negotiate(&mut self, request: NegotiationRequest) -> NegotiationResult {
        self.stats.total_negotiations += 1;

        // Validate
        if !self.processes.contains_key(&request.requester)
            || !self.processes.contains_key(&request.target)
        {
            return NegotiationResult::Invalid;
        }

        match request.action {
            NegotiationAction::RequestBoost | NegotiationAction::Donate => {
                // Auto-accept reasonable boosts
                let boost = ActiveBoost {
                    reason: request.reason,
                    boosted_to: request.desired_class,
                    granted_by: request.requester,
                    granted_at: request.timestamp,
                    expires_at: request.timestamp + request.duration_ns,
                };
                if let Some(target) = self.processes.get_mut(&request.target) {
                    if target.apply_boost(boost, request.timestamp) {
                        self.stats.accepted += 1;
                        self.update_stats();
                        return NegotiationResult::Accepted;
                    }
                }
                self.stats.declined += 1;
                NegotiationResult::Declined
            },
            NegotiationAction::Inherit => {
                let boost = ActiveBoost {
                    reason: CoopBoostReason::Inheritance,
                    boosted_to: request.desired_class,
                    granted_by: request.requester,
                    granted_at: request.timestamp,
                    expires_at: request.timestamp + request.duration_ns,
                };
                if let Some(target) = self.processes.get_mut(&request.target) {
                    target.apply_boost(boost, request.timestamp);
                    self.stats.accepted += 1;
                    self.update_stats();
                    NegotiationResult::Accepted
                } else {
                    NegotiationResult::Invalid
                }
            },
            NegotiationAction::ReturnDonated => {
                if let Some(target) = self.processes.get_mut(&request.target) {
                    target.remove_boosts_from(request.requester, request.timestamp);
                    self.update_stats();
                    NegotiationResult::Accepted
                } else {
                    NegotiationResult::Invalid
                }
            },
            _ => {
                if self.pending.len() < self.max_pending {
                    self.pending.push(request);
                }
                NegotiationResult::CounterOffer
            },
        }
    }

    /// Anti-starvation sweep
    pub fn anti_starvation_sweep(&mut self, now: u64) {
        let starving: Vec<u64> = self
            .processes
            .values()
            .filter(|p| p.is_starving())
            .map(|p| p.pid)
            .collect();

        for pid in starving {
            if let Some(proc) = self.processes.get_mut(&pid) {
                let boost = ActiveBoost {
                    reason: CoopBoostReason::AntiStarvation,
                    boosted_to: CoopPriorityClass::AboveNormal,
                    granted_by: 0, // system
                    granted_at: now,
                    expires_at: now + 10_000_000, // 10ms
                };
                proc.apply_boost(boost, now);
                proc.reset_starvation();
            }
        }
        self.update_stats();
    }

    /// Expire all outdated boosts
    pub fn expire_all(&mut self, now: u64) {
        for proc in self.processes.values_mut() {
            proc.expire_boosts(now);
        }
        self.update_stats();
    }

    /// Get effective priority
    pub fn effective_priority(&self, pid: u64) -> Option<CoopPriorityClass> {
        self.processes.get(&pid).map(|p| p.effective)
    }

    /// Get process state
    pub fn process(&self, pid: u64) -> Option<&ProcessPriority> {
        self.processes.get(&pid)
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.boosted_count = self.processes.values().filter(|p| p.is_boosted()).count();
        self.stats.starving_count = self.processes.values().filter(|p| p.is_starving()).count();
    }

    /// Stats
    pub fn stats(&self) -> &CoopPriorityStats {
        &self.stats
    }
}

// ============================================================================
// Merged from priority_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopPriorityLevel {
    Idle,
    Low,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
    Critical,
}

/// Priority boost reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityBoostReason {
    Inheritance,
    IoCompletion,
    UserInteraction,
    Starvation,
    Deadline,
}

/// A priority-tracked task
#[derive(Debug, Clone)]
pub struct PriorityV2Task {
    pub tid: u64,
    pub base_priority: CoopPriorityLevel,
    pub effective_priority: CoopPriorityLevel,
    pub boosts: Vec<PriorityBoostReason>,
    pub wait_ticks: u64,
    pub run_ticks: u64,
    pub priority_inversions: u64,
}

impl PriorityV2Task {
    pub fn new(tid: u64, priority: CoopPriorityLevel) -> Self {
        Self {
            tid, base_priority: priority,
            effective_priority: priority,
            boosts: Vec::new(),
            wait_ticks: 0, run_ticks: 0,
            priority_inversions: 0,
        }
    }

    pub fn boost(&mut self, reason: PriorityBoostReason, level: CoopPriorityLevel) {
        if level > self.effective_priority {
            self.effective_priority = level;
        }
        self.boosts.push(reason);
    }

    pub fn reset_boosts(&mut self) {
        self.effective_priority = self.base_priority;
        self.boosts.clear();
    }
}

/// Statistics for priority V2 coop
#[derive(Debug, Clone)]
pub struct PriorityV2CoopStats {
    pub tasks_registered: u64,
    pub boosts_applied: u64,
    pub inversions_detected: u64,
    pub starvation_prevents: u64,
    pub deadline_misses: u64,
}

/// Main priority V2 coop manager
#[derive(Debug)]
pub struct CoopPriorityV2 {
    tasks: BTreeMap<u64, PriorityV2Task>,
    stats: PriorityV2CoopStats,
}

impl CoopPriorityV2 {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            stats: PriorityV2CoopStats {
                tasks_registered: 0, boosts_applied: 0,
                inversions_detected: 0, starvation_prevents: 0,
                deadline_misses: 0,
            },
        }
    }

    pub fn register_task(&mut self, tid: u64, priority: CoopPriorityLevel) {
        self.tasks.insert(tid, PriorityV2Task::new(tid, priority));
        self.stats.tasks_registered += 1;
    }

    pub fn boost(&mut self, tid: u64, reason: PriorityBoostReason, level: CoopPriorityLevel) -> bool {
        if let Some(task) = self.tasks.get_mut(&tid) {
            task.boost(reason, level);
            self.stats.boosts_applied += 1;
            true
        } else { false }
    }

    pub fn detect_inversion(&mut self, holder_tid: u64, waiter_tid: u64) -> bool {
        let holder_prio = self.tasks.get(&holder_tid).map(|t| t.effective_priority);
        let waiter_prio = self.tasks.get(&waiter_tid).map(|t| t.effective_priority);
        if let (Some(hp), Some(wp)) = (holder_prio, waiter_prio) {
            if wp > hp {
                self.stats.inversions_detected += 1;
                if let Some(holder) = self.tasks.get_mut(&holder_tid) {
                    holder.boost(PriorityBoostReason::Inheritance, wp);
                    holder.priority_inversions += 1;
                }
                return true;
            }
        }
        false
    }

    pub fn highest_priority_task(&self) -> Option<u64> {
        self.tasks.values()
            .max_by_key(|t| t.effective_priority)
            .map(|t| t.tid)
    }

    pub fn stats(&self) -> &PriorityV2CoopStats {
        &self.stats
    }
}
