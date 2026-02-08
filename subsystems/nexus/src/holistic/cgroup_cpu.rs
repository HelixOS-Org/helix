// SPDX-License-Identifier: GPL-2.0
//! Holistic cgroup_cpu — CPU cgroup controller management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// CPU cgroup policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupCpuPolicy {
    Normal,
    Batch,
    Idle,
    Deadline,
}

/// CPU cgroup
#[derive(Debug)]
pub struct CpuCgroup {
    pub id: u64,
    pub parent_id: u64,
    pub shares: u32,
    pub quota_us: i64,
    pub period_us: u64,
    pub runtime_us: u64,
    pub nr_throttled: u64,
    pub throttled_us: u64,
    pub nr_tasks: u32,
    pub policy: CgroupCpuPolicy,
    pub weight: u32,
}

impl CpuCgroup {
    pub fn new(id: u64, parent: u64) -> Self {
        Self { id, parent_id: parent, shares: 1024, quota_us: -1, period_us: 100000, runtime_us: 0, nr_throttled: 0, throttled_us: 0, nr_tasks: 0, policy: CgroupCpuPolicy::Normal, weight: 100 }
    }

    pub fn set_bandwidth(&mut self, quota: i64, period: u64) {
        self.quota_us = quota;
        self.period_us = period;
    }

    pub fn account_runtime(&mut self, us: u64) {
        self.runtime_us += us;
        if self.quota_us > 0 && self.runtime_us > self.quota_us as u64 {
            self.nr_throttled += 1;
            let overage = self.runtime_us - self.quota_us as u64;
            self.throttled_us += overage;
        }
    }

    pub fn reset_period(&mut self) { self.runtime_us = 0; }

    pub fn throttle_ratio(&self) -> f64 {
        if self.quota_us <= 0 { return 0.0; }
        self.runtime_us as f64 / self.quota_us as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct CgroupCpuStats {
    pub total_cgroups: u32,
    pub total_tasks: u32,
    pub total_throttled: u64,
    pub total_throttled_us: u64,
    pub avg_shares: u32,
}

/// Main holistic cgroup CPU
pub struct HolisticCgroupCpu {
    cgroups: BTreeMap<u64, CpuCgroup>,
    next_id: u64,
}

impl HolisticCgroupCpu {
    pub fn new() -> Self { Self { cgroups: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, parent: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.cgroups.insert(id, CpuCgroup::new(id, parent));
        id
    }

    pub fn set_shares(&mut self, id: u64, shares: u32) {
        if let Some(cg) = self.cgroups.get_mut(&id) { cg.shares = shares; }
    }

    pub fn set_bandwidth(&mut self, id: u64, quota: i64, period: u64) {
        if let Some(cg) = self.cgroups.get_mut(&id) { cg.set_bandwidth(quota, period); }
    }

    pub fn account(&mut self, id: u64, us: u64) {
        if let Some(cg) = self.cgroups.get_mut(&id) { cg.account_runtime(us); }
    }

    pub fn destroy(&mut self, id: u64) { self.cgroups.remove(&id); }

    pub fn stats(&self) -> CgroupCpuStats {
        let tasks: u32 = self.cgroups.values().map(|c| c.nr_tasks).sum();
        let throttled: u64 = self.cgroups.values().map(|c| c.nr_throttled).sum();
        let throttled_us: u64 = self.cgroups.values().map(|c| c.throttled_us).sum();
        let avg_shares = if self.cgroups.is_empty() { 0 }
            else { (self.cgroups.values().map(|c| c.shares as u64).sum::<u64>() / self.cgroups.len() as u64) as u32 };
        CgroupCpuStats { total_cgroups: self.cgroups.len() as u32, total_tasks: tasks, total_throttled: throttled, total_throttled_us: throttled_us, avg_shares }
    }
}
