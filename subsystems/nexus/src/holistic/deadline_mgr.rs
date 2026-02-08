//! # Holistic Deadline Manager
//!
//! System-wide deadline management for real-time and soft-real-time tasks:
//! - Deadline-based priority boosting
//! - Admission control (utilization bound)
//! - EDF (Earliest Deadline First) ordering
//! - Deadline miss tracking per-task and system-wide
//! - Slack reclamation for non-RT tasks
//! - Overrun detection and throttling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Deadline class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineClass {
    /// Hard real-time (must never miss)
    HardRealtime,
    /// Firm real-time (occasional miss tolerated)
    FirmRealtime,
    /// Soft real-time (best-effort deadline)
    SoftRealtime,
    /// Best-effort (no deadline)
    BestEffort,
}

/// Deadline miss severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissSeverity {
    /// Minor — within tolerance
    Minor,
    /// Major — noticeable impact
    Major,
    /// Critical — system-level effect
    Critical,
}

/// Task deadline parameters (SCHED_DEADLINE style)
#[derive(Debug, Clone)]
pub struct DeadlineParams {
    pub task_id: u64,
    pub class: DeadlineClass,
    /// Runtime budget per period (ns)
    pub runtime_ns: u64,
    /// Period (ns)
    pub period_ns: u64,
    /// Relative deadline from period start (ns)
    pub deadline_ns: u64,
}

impl DeadlineParams {
    pub fn utilization(&self) -> f64 {
        if self.period_ns == 0 {
            return 0.0;
        }
        self.runtime_ns as f64 / self.period_ns as f64
    }

    /// Density (runtime / deadline)
    pub fn density(&self) -> f64 {
        if self.deadline_ns == 0 {
            return 0.0;
        }
        self.runtime_ns as f64 / self.deadline_ns as f64
    }
}

/// Tracked deadline task state
#[derive(Debug, Clone)]
pub struct DeadlineTaskState {
    pub params: DeadlineParams,
    pub absolute_deadline: u64,
    pub remaining_runtime_ns: u64,
    pub total_periods: u64,
    pub deadlines_met: u64,
    pub deadlines_missed: u64,
    pub worst_miss_ns: u64,
    pub total_overrun_ns: u64,
    pub throttled: bool,
    pub cpu_affinity: Option<u32>,
}

impl DeadlineTaskState {
    pub fn new(params: DeadlineParams, now: u64) -> Self {
        let abs_dl = now + params.deadline_ns;
        let remaining = params.runtime_ns;
        Self {
            params,
            absolute_deadline: abs_dl,
            remaining_runtime_ns: remaining,
            total_periods: 0,
            deadlines_met: 0,
            deadlines_missed: 0,
            worst_miss_ns: 0,
            total_overrun_ns: 0,
            throttled: false,
            cpu_affinity: None,
        }
    }

    /// Miss ratio
    pub fn miss_ratio(&self) -> f64 {
        let total = self.deadlines_met + self.deadlines_missed;
        if total == 0 {
            return 0.0;
        }
        self.deadlines_missed as f64 / total as f64
    }

    /// Consume runtime
    pub fn consume(&mut self, ns: u64) {
        if ns > self.remaining_runtime_ns {
            let overrun = ns - self.remaining_runtime_ns;
            self.remaining_runtime_ns = 0;
            self.total_overrun_ns += overrun;
            self.throttled = true;
        } else {
            self.remaining_runtime_ns -= ns;
        }
    }

    /// Start new period
    pub fn new_period(&mut self, now: u64) {
        self.total_periods += 1;
        if now <= self.absolute_deadline {
            self.deadlines_met += 1;
        } else {
            self.deadlines_missed += 1;
            let miss_by = now - self.absolute_deadline;
            if miss_by > self.worst_miss_ns {
                self.worst_miss_ns = miss_by;
            }
        }
        self.absolute_deadline = now + self.params.deadline_ns;
        self.remaining_runtime_ns = self.params.runtime_ns;
        self.throttled = false;
    }
}

/// Admission result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionResult {
    /// Admitted
    Admitted,
    /// Rejected: would exceed utilization bound
    RejectedUtilization,
    /// Rejected: density too high
    RejectedDensity,
    /// Rejected: too many RT tasks
    RejectedCapacity,
}

/// System slack info
#[derive(Debug, Clone)]
pub struct SlackInfo {
    pub total_slack_ns: u64,
    pub reclaimable_cpus: u32,
    pub next_deadline_ns: u64,
}

/// Holistic Deadline Manager stats
#[derive(Debug, Clone, Default)]
pub struct HolisticDeadlineMgrStats {
    pub total_dl_tasks: usize,
    pub hard_rt_tasks: usize,
    pub total_utilization: f64,
    pub system_miss_ratio: f64,
    pub throttled_tasks: usize,
    pub total_overrun_ns: u64,
    pub admissions_rejected: u64,
}

/// Holistic Deadline Manager
pub struct HolisticDeadlineMgr {
    tasks: BTreeMap<u64, DeadlineTaskState>,
    nr_cpus: u32,
    utilization_cap: f64,
    max_dl_tasks: usize,
    total_rejected: u64,
    stats: HolisticDeadlineMgrStats,
}

impl HolisticDeadlineMgr {
    pub fn new(nr_cpus: u32) -> Self {
        Self {
            tasks: BTreeMap::new(),
            nr_cpus,
            utilization_cap: nr_cpus as f64 * 0.95, // 95% per CPU
            max_dl_tasks: (nr_cpus as usize) * 64,
            total_rejected: 0,
            stats: HolisticDeadlineMgrStats::default(),
        }
    }

    /// Total system utilization
    pub fn total_utilization(&self) -> f64 {
        self.tasks.values().map(|t| t.params.utilization()).sum()
    }

    /// Admission control
    pub fn admit(&mut self, params: DeadlineParams, now: u64) -> AdmissionResult {
        if self.tasks.len() >= self.max_dl_tasks {
            self.total_rejected += 1;
            return AdmissionResult::RejectedCapacity;
        }

        let new_util = self.total_utilization() + params.utilization();
        if new_util > self.utilization_cap {
            self.total_rejected += 1;
            return AdmissionResult::RejectedUtilization;
        }

        if params.density() > 1.0 {
            self.total_rejected += 1;
            return AdmissionResult::RejectedDensity;
        }

        let task_id = params.task_id;
        let state = DeadlineTaskState::new(params, now);
        self.tasks.insert(task_id, state);
        self.recompute();
        AdmissionResult::Admitted
    }

    /// Remove a DL task
    pub fn remove(&mut self, task_id: u64) -> bool {
        let removed = self.tasks.remove(&task_id).is_some();
        if removed {
            self.recompute();
        }
        removed
    }

    /// Consume runtime for a task
    pub fn consume_runtime(&mut self, task_id: u64, ns: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.consume(ns);
        }
    }

    /// Advance period for a task
    pub fn tick_period(&mut self, task_id: u64, now: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.new_period(now);
        }
        self.recompute();
    }

    /// Get EDF ordering (earliest deadline first)
    pub fn edf_order(&self) -> Vec<u64> {
        let mut entries: Vec<(u64, u64)> = self
            .tasks
            .iter()
            .filter(|(_, t)| !t.throttled)
            .map(|(&id, t)| (t.absolute_deadline, id))
            .collect();
        entries.sort_by_key(|&(dl, _)| dl);
        entries.into_iter().map(|(_, id)| id).collect()
    }

    /// Compute system slack
    pub fn slack(&self, now: u64) -> SlackInfo {
        let used_util = self.total_utilization();
        let slack_frac = (self.nr_cpus as f64 - used_util).max(0.0);
        let slack_ns = (slack_frac * 1_000_000.0) as u64;

        let next_dl = self
            .tasks
            .values()
            .map(|t| t.absolute_deadline)
            .filter(|&dl| dl > now)
            .min()
            .unwrap_or(u64::MAX);

        let reclaimable = (slack_frac.floor()) as u32;

        SlackInfo {
            total_slack_ns: slack_ns,
            reclaimable_cpus: reclaimable,
            next_deadline_ns: next_dl,
        }
    }

    /// Get tasks that missed their deadline
    pub fn missed_tasks(&self) -> Vec<u64> {
        self.tasks
            .iter()
            .filter(|(_, t)| t.miss_ratio() > 0.0)
            .map(|(&id, _)| id)
            .collect()
    }

    fn recompute(&mut self) {
        self.stats.total_dl_tasks = self.tasks.len();
        self.stats.hard_rt_tasks = self
            .tasks
            .values()
            .filter(|t| t.params.class == DeadlineClass::HardRealtime)
            .count();
        self.stats.total_utilization = self.total_utilization();
        self.stats.throttled_tasks = self.tasks.values().filter(|t| t.throttled).count();

        let total_met: u64 = self.tasks.values().map(|t| t.deadlines_met).sum();
        let total_missed: u64 = self.tasks.values().map(|t| t.deadlines_missed).sum();
        let total = total_met + total_missed;
        self.stats.system_miss_ratio = if total > 0 {
            total_missed as f64 / total as f64
        } else {
            0.0
        };
        self.stats.total_overrun_ns = self.tasks.values().map(|t| t.total_overrun_ns).sum();
        self.stats.admissions_rejected = self.total_rejected;
    }

    pub fn stats(&self) -> &HolisticDeadlineMgrStats {
        &self.stats
    }

    pub fn task(&self, task_id: u64) -> Option<&DeadlineTaskState> {
        self.tasks.get(&task_id)
    }
}
