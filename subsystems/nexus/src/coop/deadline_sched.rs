// SPDX-License-Identifier: GPL-2.0
//! Coop deadline_sched — deadline-based cooperative scheduling.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Deadline task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlTaskState {
    Ready,
    Running,
    Blocked,
    Throttled,
    DeadlineMiss,
}

/// Deadline parameters
#[derive(Debug, Clone, Copy)]
pub struct DlParams {
    pub runtime_ns: u64,
    pub deadline_ns: u64,
    pub period_ns: u64,
}

impl DlParams {
    pub fn new(runtime: u64, deadline: u64, period: u64) -> Self { Self { runtime_ns: runtime, deadline_ns: deadline, period_ns: period } }
    pub fn utilization(&self) -> f64 { if self.period_ns == 0 { 0.0 } else { self.runtime_ns as f64 / self.period_ns as f64 } }
}

/// Deadline task
#[derive(Debug)]
pub struct DlTask {
    pub id: u64,
    pub state: DlTaskState,
    pub params: DlParams,
    pub absolute_deadline: u64,
    pub runtime_remaining: u64,
    pub total_runtime: u64,
    pub deadline_misses: u64,
    pub periods_completed: u64,
    pub last_activation: u64,
}

impl DlTask {
    pub fn new(id: u64, params: DlParams) -> Self {
        Self { id, state: DlTaskState::Ready, params, absolute_deadline: 0, runtime_remaining: params.runtime_ns, total_runtime: 0, deadline_misses: 0, periods_completed: 0, last_activation: 0 }
    }

    pub fn activate(&mut self, now: u64) {
        self.state = DlTaskState::Ready;
        self.absolute_deadline = now + self.params.deadline_ns;
        self.runtime_remaining = self.params.runtime_ns;
        self.last_activation = now;
    }

    pub fn run(&mut self, elapsed: u64) {
        self.state = DlTaskState::Running;
        let consumed = elapsed.min(self.runtime_remaining);
        self.runtime_remaining -= consumed;
        self.total_runtime += consumed;
        if self.runtime_remaining == 0 { self.state = DlTaskState::Throttled; }
    }

    pub fn check_deadline(&mut self, now: u64) -> bool {
        if now > self.absolute_deadline && self.runtime_remaining > 0 { self.deadline_misses += 1; self.state = DlTaskState::DeadlineMiss; true }
        else { false }
    }

    pub fn complete_period(&mut self) { self.periods_completed += 1; }
    pub fn miss_rate(&self) -> f64 { if self.periods_completed == 0 { 0.0 } else { self.deadline_misses as f64 / self.periods_completed as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct DeadlineSchedStats {
    pub total_tasks: u32,
    pub total_utilization: f64,
    pub total_misses: u64,
    pub total_periods: u64,
    pub miss_rate: f64,
}

/// Main deadline scheduler
pub struct CoopDeadlineSched {
    tasks: BTreeMap<u64, DlTask>,
    next_id: u64,
}

impl CoopDeadlineSched {
    pub fn new() -> Self { Self { tasks: BTreeMap::new(), next_id: 1 } }

    pub fn add_task(&mut self, params: DlParams) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.tasks.insert(id, DlTask::new(id, params));
        id
    }

    pub fn pick_next(&self) -> Option<u64> {
        self.tasks.values().filter(|t| t.state == DlTaskState::Ready)
            .min_by_key(|t| t.absolute_deadline).map(|t| t.id)
    }

    pub fn stats(&self) -> DeadlineSchedStats {
        let util: f64 = self.tasks.values().map(|t| t.params.utilization()).sum();
        let misses: u64 = self.tasks.values().map(|t| t.deadline_misses).sum();
        let periods: u64 = self.tasks.values().map(|t| t.periods_completed).sum();
        let rate = if periods == 0 { 0.0 } else { misses as f64 / periods as f64 };
        DeadlineSchedStats { total_tasks: self.tasks.len() as u32, total_utilization: util, total_misses: misses, total_periods: periods, miss_rate: rate }
    }
}
