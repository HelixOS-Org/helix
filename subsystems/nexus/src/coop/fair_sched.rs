// SPDX-License-Identifier: GPL-2.0
//! Coop fair_sched — cooperative fair scheduler.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Task priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FairSchedClass {
    RealTime,
    Interactive,
    Normal,
    Batch,
    Idle,
}

/// Fair schedule task
#[derive(Debug)]
pub struct FairTask {
    pub id: u64,
    pub class: FairSchedClass,
    pub vruntime: u64,
    pub weight: u32,
    pub total_runtime_ns: u64,
    pub slices_used: u64,
    pub last_scheduled: u64,
    pub waiting_since: u64,
}

impl FairTask {
    pub fn new(id: u64, class: FairSchedClass, weight: u32) -> Self {
        Self { id, class, vruntime: 0, weight, total_runtime_ns: 0, slices_used: 0, last_scheduled: 0, waiting_since: 0 }
    }

    #[inline]
    pub fn account(&mut self, runtime_ns: u64) {
        let weighted = if self.weight == 0 { runtime_ns } else { runtime_ns * 1024 / self.weight as u64 };
        self.vruntime += weighted;
        self.total_runtime_ns += runtime_ns;
        self.slices_used += 1;
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FairSchedStats {
    pub total_tasks: u32,
    pub total_schedules: u64,
    pub min_vruntime: u64,
    pub max_vruntime: u64,
    pub fairness_index: f64,
}

/// Main coop fair scheduler
pub struct CoopFairSched {
    tasks: BTreeMap<u64, FairTask>,
    total_schedules: u64,
    min_granularity_ns: u64,
}

impl CoopFairSched {
    pub fn new(granularity_ns: u64) -> Self { Self { tasks: BTreeMap::new(), total_schedules: 0, min_granularity_ns: granularity_ns } }

    #[inline]
    pub fn add_task(&mut self, id: u64, class: FairSchedClass, weight: u32) {
        let min_vruntime = self.tasks.values().map(|t| t.vruntime).min().unwrap_or(0);
        let mut task = FairTask::new(id, class, weight);
        task.vruntime = min_vruntime;
        self.tasks.insert(id, task);
    }

    #[inline(always)]
    pub fn pick_next(&mut self) -> Option<u64> {
        self.total_schedules += 1;
        self.tasks.values().min_by_key(|t| t.vruntime).map(|t| t.id)
    }

    #[inline(always)]
    pub fn account(&mut self, id: u64, runtime_ns: u64) {
        if let Some(t) = self.tasks.get_mut(&id) { t.account(runtime_ns); }
    }

    #[inline(always)]
    pub fn remove_task(&mut self, id: u64) { self.tasks.remove(&id); }

    #[inline]
    pub fn stats(&self) -> FairSchedStats {
        let vruntimes: Vec<u64> = self.tasks.values().map(|t| t.vruntime).collect();
        let min = vruntimes.iter().copied().min().unwrap_or(0);
        let max = vruntimes.iter().copied().max().unwrap_or(0);
        let fairness = if max == 0 { 1.0 } else {
            let avg = vruntimes.iter().sum::<u64>() as f64 / vruntimes.len().max(1) as f64;
            let variance: f64 = vruntimes.iter().map(|&v| { let d = v as f64 - avg; d * d }).sum::<f64>() / vruntimes.len().max(1) as f64;
            1.0 / (1.0 + libm::sqrt(variance) / avg.max(1.0))
        };
        FairSchedStats { total_tasks: self.tasks.len() as u32, total_schedules: self.total_schedules, min_vruntime: min, max_vruntime: max, fairness_index: fairness }
    }
}
