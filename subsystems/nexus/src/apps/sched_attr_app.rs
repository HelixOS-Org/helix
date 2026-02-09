// SPDX-License-Identifier: GPL-2.0
//! Apps sched_attr_app — extended scheduling attributes.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedPolicy {
    Normal,
    Fifo,
    RoundRobin,
    Batch,
    Idle,
    Deadline,
}

/// Scheduler attribute
#[derive(Debug, Clone)]
pub struct SchedAttr {
    pub size: u32,
    pub policy: SchedPolicy,
    pub flags: u64,
    pub nice: i8,
    pub priority: u32,
    pub runtime_ns: u64,
    pub deadline_ns: u64,
    pub period_ns: u64,
    pub utilization_hint: u32,
}

impl SchedAttr {
    #[inline(always)]
    pub fn normal(nice: i8) -> Self {
        Self { size: 56, policy: SchedPolicy::Normal, flags: 0, nice, priority: 0, runtime_ns: 0, deadline_ns: 0, period_ns: 0, utilization_hint: 0 }
    }

    #[inline(always)]
    pub fn deadline(runtime: u64, deadline: u64, period: u64) -> Self {
        Self { size: 56, policy: SchedPolicy::Deadline, flags: 0, nice: 0, priority: 0, runtime_ns: runtime, deadline_ns: deadline, period_ns: period, utilization_hint: 0 }
    }

    #[inline(always)]
    pub fn is_realtime(&self) -> bool { matches!(self.policy, SchedPolicy::Fifo | SchedPolicy::RoundRobin | SchedPolicy::Deadline) }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.period_ns == 0 { 0.0 } else { self.runtime_ns as f64 / self.period_ns as f64 }
    }
}

/// Process sched state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessSchedState {
    pub pid: u64,
    pub attr: SchedAttr,
    pub change_count: u64,
    pub last_change_ns: u64,
}

impl ProcessSchedState {
    pub fn new(pid: u64) -> Self { Self { pid, attr: SchedAttr::normal(0), change_count: 0, last_change_ns: 0 } }

    #[inline(always)]
    pub fn set_attr(&mut self, attr: SchedAttr, now: u64) {
        self.attr = attr; self.change_count += 1; self.last_change_ns = now;
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SchedAttrAppStats {
    pub tracked_processes: u32,
    pub realtime_processes: u32,
    pub deadline_processes: u32,
    pub total_changes: u64,
    pub total_utilization: f64,
}

/// Main sched_attr app
pub struct AppSchedAttr {
    processes: BTreeMap<u64, ProcessSchedState>,
}

impl AppSchedAttr {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }
    #[inline(always)]
    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessSchedState::new(pid)); }

    #[inline(always)]
    pub fn set_attr(&mut self, pid: u64, attr: SchedAttr, now: u64) {
        if let Some(p) = self.processes.get_mut(&pid) { p.set_attr(attr, now); }
    }

    #[inline]
    pub fn stats(&self) -> SchedAttrAppStats {
        let rt = self.processes.values().filter(|p| p.attr.is_realtime()).count() as u32;
        let dl = self.processes.values().filter(|p| p.attr.policy == SchedPolicy::Deadline).count() as u32;
        let changes: u64 = self.processes.values().map(|p| p.change_count).sum();
        let util: f64 = self.processes.values().filter(|p| p.attr.policy == SchedPolicy::Deadline).map(|p| p.attr.utilization()).sum();
        SchedAttrAppStats { tracked_processes: self.processes.len() as u32, realtime_processes: rt, deadline_processes: dl, total_changes: changes, total_utilization: util }
    }
}
