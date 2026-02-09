//! # Cooperative Deadline Management
//!
//! Deadline-aware cooperative scheduling:
//! - Earliest Deadline First (EDF) ordering
//! - Deadline admission control
//! - Slack reclamation
//! - Deadline inheritance
//! - Utilization tracking
//! - Deadline miss analysis

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// DEADLINE TYPES
// ============================================================================

/// Deadline urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeadlineUrgency {
    /// Plenty of time
    Relaxed  = 0,
    /// Normal pace
    Normal   = 1,
    /// Should hurry
    Hurried  = 2,
    /// Tight deadline
    Tight    = 3,
    /// Imminent
    Imminent = 4,
    /// Overdue
    Overdue  = 5,
}

/// Deadline class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineClass {
    /// Hard deadline — must never miss
    Hard,
    /// Firm deadline — occasional miss tolerable
    Firm,
    /// Soft deadline — best effort
    Soft,
}

// ============================================================================
// DEADLINE TASK
// ============================================================================

/// Deadline task definition
#[derive(Debug, Clone)]
pub struct DeadlineTask {
    /// Task ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Deadline class
    pub class: DeadlineClass,
    /// Absolute deadline (timestamp)
    pub deadline: u64,
    /// Period (ms, 0 = aperiodic)
    pub period_ms: u64,
    /// Worst-case execution time (us)
    pub wcet_us: u64,
    /// Average execution time (us)
    pub avg_et_us: u64,
    /// Remaining budget this period (us)
    pub remaining_us: u64,
    /// Admitted
    pub admitted: bool,
}

impl DeadlineTask {
    pub fn new(id: u64, pid: u64, class: DeadlineClass, deadline: u64, wcet_us: u64) -> Self {
        Self {
            id,
            pid,
            class,
            deadline,
            period_ms: 0,
            wcet_us,
            avg_et_us: wcet_us,
            remaining_us: wcet_us,
            admitted: false,
        }
    }

    /// Periodic task
    #[inline(always)]
    pub fn periodic(mut self, period_ms: u64) -> Self {
        self.period_ms = period_ms;
        self
    }

    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.period_ms == 0 {
            return 0.0;
        }
        (self.wcet_us as f64) / (self.period_ms as f64 * 1000.0)
    }

    /// Urgency at given time
    pub fn urgency(&self, now: u64) -> DeadlineUrgency {
        if now >= self.deadline {
            return DeadlineUrgency::Overdue;
        }

        let slack_us = (self.deadline - now) * 1000; // Approximate to us
        let ratio = if self.remaining_us > 0 {
            slack_us as f64 / self.remaining_us as f64
        } else {
            f64::MAX
        };

        if ratio > 10.0 {
            DeadlineUrgency::Relaxed
        } else if ratio > 5.0 {
            DeadlineUrgency::Normal
        } else if ratio > 2.0 {
            DeadlineUrgency::Hurried
        } else if ratio > 1.0 {
            DeadlineUrgency::Tight
        } else {
            DeadlineUrgency::Imminent
        }
    }

    /// Slack time (us) — how much spare time
    #[inline]
    pub fn slack_us(&self, now: u64) -> i64 {
        if now >= self.deadline {
            return -(((now - self.deadline) * 1000) as i64);
        }
        let available = (self.deadline - now) * 1000;
        available as i64 - self.remaining_us as i64
    }

    /// Consume execution time
    #[inline]
    pub fn consume(&mut self, us: u64) {
        self.remaining_us = self.remaining_us.saturating_sub(us);

        // Update running average
        let executed = self.wcet_us - self.remaining_us;
        self.avg_et_us = (self.avg_et_us * 7 + executed) / 8;
    }

    /// Reset for next period
    #[inline(always)]
    pub fn reset_period(&mut self, new_deadline: u64) {
        self.deadline = new_deadline;
        self.remaining_us = self.wcet_us;
    }
}

// ============================================================================
// ADMISSION CONTROL
// ============================================================================

/// Admission result
#[derive(Debug, Clone)]
pub struct AdmissionResult {
    /// Admitted
    pub admitted: bool,
    /// Current total utilization
    pub total_utilization: f64,
    /// Utilization after admission
    pub new_utilization: f64,
    /// Utilization bound
    pub utilization_bound: f64,
    /// Reason for rejection
    pub rejection_reason: Option<AdmissionRejection>,
}

/// Rejection reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionRejection {
    /// Total utilization would exceed bound
    UtilizationExceeded,
    /// Too many hard deadlines
    HardDeadlineLimit,
    /// Period too short
    PeriodTooShort,
    /// WCET too large
    WcetTooLarge,
}

// ============================================================================
// DEADLINE MISS
// ============================================================================

/// Deadline miss event
#[derive(Debug, Clone)]
pub struct DeadlineMiss {
    /// Task ID
    pub task_id: u64,
    /// Process ID
    pub pid: u64,
    /// Deadline class
    pub class: DeadlineClass,
    /// Deadline that was missed
    pub deadline: u64,
    /// Actual completion time
    pub completion: u64,
    /// Overshoot (us)
    pub overshoot_us: u64,
    /// Remaining work (us)
    pub remaining_us: u64,
    /// Cause
    pub cause: MissCause,
}

/// Cause of deadline miss
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissCause {
    /// Underestimated WCET
    WcetUnderestimate,
    /// Preempted by higher priority
    Preemption,
    /// Blocked on resource
    Blocking,
    /// System overloaded
    Overload,
    /// Unknown
    Unknown,
}

// ============================================================================
// SLACK RECLAMATION
// ============================================================================

/// Reclaimable slack from a task
#[derive(Debug, Clone)]
pub struct ReclaimableSlack {
    /// Task ID
    pub task_id: u64,
    /// Available slack (us)
    pub slack_us: u64,
    /// Priority of the slack (based on donor task priority)
    pub priority: u32,
}

// ============================================================================
// DEADLINE MANAGER
// ============================================================================

/// Deadline manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct DeadlineManagerStats {
    /// Total tasks
    pub total_tasks: usize,
    /// Admitted tasks
    pub admitted_tasks: usize,
    /// Total utilization
    pub total_utilization_pct: u32,
    /// Deadline misses
    pub total_misses: u64,
    /// Hard misses
    pub hard_misses: u64,
    /// Slack reclaimed (us)
    pub slack_reclaimed_us: u64,
}

/// Cooperative deadline manager
pub struct CoopDeadlineManager {
    /// Tasks by ID
    tasks: BTreeMap<u64, DeadlineTask>,
    /// EDF order (deadline → task ID)
    edf_order: Vec<(u64, u64)>,
    /// Miss history
    misses: VecDeque<DeadlineMiss>,
    /// Stats
    stats: DeadlineManagerStats,
    /// Utilization bound (default ~69% for EDF)
    utilization_bound: f64,
    /// Max hard deadline tasks
    max_hard_tasks: usize,
    /// Max miss history
    max_misses: usize,
    /// Next task ID
    next_id: u64,
}

impl CoopDeadlineManager {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            edf_order: Vec::new(),
            misses: VecDeque::new(),
            stats: DeadlineManagerStats::default(),
            utilization_bound: 0.69, // ln(2) ≈ 0.693
            max_hard_tasks: 32,
            max_misses: 256,
            next_id: 1,
        }
    }

    /// Current total utilization
    #[inline]
    pub fn total_utilization(&self) -> f64 {
        self.tasks
            .values()
            .filter(|t| t.admitted)
            .map(|t| t.utilization())
            .sum()
    }

    /// Admission control
    pub fn admit(&mut self, task: &DeadlineTask) -> AdmissionResult {
        let current_util = self.total_utilization();
        let new_util = current_util + task.utilization();

        // Check hard deadline count
        if task.class == DeadlineClass::Hard {
            let hard_count = self
                .tasks
                .values()
                .filter(|t| t.class == DeadlineClass::Hard && t.admitted)
                .count();
            if hard_count >= self.max_hard_tasks {
                return AdmissionResult {
                    admitted: false,
                    total_utilization: current_util,
                    new_utilization: new_util,
                    utilization_bound: self.utilization_bound,
                    rejection_reason: Some(AdmissionRejection::HardDeadlineLimit),
                };
            }
        }

        // Check utilization bound
        let bound = if task.class == DeadlineClass::Hard {
            self.utilization_bound
        } else {
            1.0 // Soft/firm can go up to 100%
        };

        if new_util > bound {
            return AdmissionResult {
                admitted: false,
                total_utilization: current_util,
                new_utilization: new_util,
                utilization_bound: bound,
                rejection_reason: Some(AdmissionRejection::UtilizationExceeded),
            };
        }

        AdmissionResult {
            admitted: true,
            total_utilization: current_util,
            new_utilization: new_util,
            utilization_bound: bound,
            rejection_reason: None,
        }
    }

    /// Register and admit task
    pub fn register(&mut self, mut task: DeadlineTask) -> (u64, AdmissionResult) {
        let id = self.next_id;
        self.next_id += 1;
        task.id = id;

        let result = self.admit(&task);
        task.admitted = result.admitted;

        self.tasks.insert(id, task);
        self.rebuild_edf();
        self.update_stats();

        (id, result)
    }

    /// Get next task by EDF
    pub fn next_edf(&self, now: u64) -> Option<u64> {
        self.edf_order
            .iter()
            .find(|(deadline, id)| {
                *deadline > now
                    && self
                        .tasks
                        .get(id)
                        .map(|t| t.admitted && t.remaining_us > 0)
                        .unwrap_or(false)
            })
            .map(|(_, id)| *id)
    }

    /// Consume execution time
    #[inline]
    pub fn consume(&mut self, task_id: u64, us: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.consume(us);
        }
    }

    /// Check deadline misses
    pub fn check_misses(&mut self, now: u64) -> Vec<u64> {
        let mut missed = Vec::new();

        for (id, task) in &self.tasks {
            if task.admitted && now >= task.deadline && task.remaining_us > 0 {
                let miss = DeadlineMiss {
                    task_id: *id,
                    pid: task.pid,
                    class: task.class,
                    deadline: task.deadline,
                    completion: now,
                    overshoot_us: (now - task.deadline) * 1000,
                    remaining_us: task.remaining_us,
                    cause: MissCause::Unknown,
                };

                self.misses.push_back(miss);
                if self.misses.len() > self.max_misses {
                    self.misses.pop_front();
                }

                self.stats.total_misses += 1;
                if task.class == DeadlineClass::Hard {
                    self.stats.hard_misses += 1;
                }

                missed.push(*id);
            }
        }

        missed
    }

    /// Collect reclaimable slack
    pub fn collect_slack(&self, now: u64) -> Vec<ReclaimableSlack> {
        let mut slack_list = Vec::new();

        for (_, task) in &self.tasks {
            if !task.admitted {
                continue;
            }
            let slack = task.slack_us(now);
            if slack > 0 {
                slack_list.push(ReclaimableSlack {
                    task_id: task.id,
                    slack_us: slack as u64,
                    priority: match task.class {
                        DeadlineClass::Hard => 1,
                        DeadlineClass::Firm => 2,
                        DeadlineClass::Soft => 3,
                    },
                });
            }
        }

        slack_list.sort_by(|a, b| a.priority.cmp(&b.priority));
        slack_list
    }

    /// Period reset for periodic task
    #[inline]
    pub fn period_reset(&mut self, task_id: u64, new_deadline: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.reset_period(new_deadline);
        }
        self.rebuild_edf();
    }

    /// Rebuild EDF order
    fn rebuild_edf(&mut self) {
        self.edf_order.clear();
        for (id, task) in &self.tasks {
            if task.admitted {
                self.edf_order.push((task.deadline, *id));
            }
        }
        self.edf_order.sort_by_key(|(d, _)| *d);
    }

    fn update_stats(&mut self) {
        self.stats.total_tasks = self.tasks.len();
        self.stats.admitted_tasks = self.tasks.values().filter(|t| t.admitted).count();
        self.stats.total_utilization_pct = (self.total_utilization() * 100.0) as u32;
    }

    /// Get task
    #[inline(always)]
    pub fn task(&self, id: u64) -> Option<&DeadlineTask> {
        self.tasks.get(&id)
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &DeadlineManagerStats {
        &self.stats
    }

    /// Unregister task
    #[inline]
    pub fn unregister(&mut self, task_id: u64) {
        self.tasks.remove(&task_id);
        self.rebuild_edf();
        self.update_stats();
    }
}
