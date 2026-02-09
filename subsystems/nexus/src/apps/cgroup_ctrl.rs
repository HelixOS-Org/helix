//! # Apps Cgroup Controller
//!
//! Application-level cgroup resource controller:
//! - Per-app resource limits (CPU, memory, I/O)
//! - Hierarchical cgroup tree management
//! - Burst and throttle management
//! - Resource usage accounting
//! - Automatic limit adjustment based on behavior

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Resource type for cgroup limits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupResource {
    CpuQuota,
    CpuShares,
    CpusetCpus,
    MemHard,
    MemSoft,
    MemSwap,
    IoWeight,
    IoMax,
    PidsMax,
}

/// Cgroup enforcement state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementState {
    Normal,
    Throttled,
    Burst,
    OverLimit,
    OomRisk,
}

/// Per-app cgroup
#[derive(Debug, Clone)]
pub struct AppCgroup {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub pid_count: u32,
    pub cpu_quota_us: i64,
    pub cpu_period_us: u64,
    pub cpu_shares: u32,
    pub mem_limit: u64,
    pub mem_soft: u64,
    pub mem_swap_limit: u64,
    pub pids_max: u32,
    pub io_weight: u16,
    pub cpu_usage_us: u64,
    pub mem_usage: u64,
    pub io_bytes: u64,
    pub nr_throttled: u64,
    pub throttled_time_us: u64,
    pub state: EnforcementState,
    pub burst_budget_us: u64,
    pub burst_used_us: u64,
}

impl AppCgroup {
    pub fn new(id: u64, name: String, parent: Option<u64>) -> Self {
        Self {
            id, name, parent_id: parent, children: Vec::new(), pid_count: 0,
            cpu_quota_us: -1, cpu_period_us: 100_000, cpu_shares: 1024,
            mem_limit: u64::MAX, mem_soft: u64::MAX, mem_swap_limit: u64::MAX,
            pids_max: u32::MAX, io_weight: 100,
            cpu_usage_us: 0, mem_usage: 0, io_bytes: 0,
            nr_throttled: 0, throttled_time_us: 0,
            state: EnforcementState::Normal,
            burst_budget_us: 0, burst_used_us: 0,
        }
    }

    #[inline(always)]
    pub fn set_cpu_quota(&mut self, quota_us: i64, period_us: u64) { self.cpu_quota_us = quota_us; self.cpu_period_us = period_us; }
    #[inline(always)]
    pub fn set_mem_limit(&mut self, hard: u64, soft: u64) { self.mem_limit = hard; self.mem_soft = soft; }

    pub fn charge_cpu(&mut self, us: u64) {
        self.cpu_usage_us += us;
        if self.cpu_quota_us > 0 && self.cpu_usage_us > self.cpu_quota_us as u64 {
            if self.burst_used_us < self.burst_budget_us {
                self.burst_used_us += us;
                self.state = EnforcementState::Burst;
            } else {
                self.state = EnforcementState::Throttled;
                self.nr_throttled += 1;
            }
        }
    }

    #[inline]
    pub fn charge_mem(&mut self, bytes: u64) -> bool {
        if self.mem_usage + bytes > self.mem_limit { self.state = EnforcementState::OverLimit; return false; }
        self.mem_usage += bytes;
        if self.mem_usage > self.mem_soft { self.state = EnforcementState::OverLimit; }
        true
    }

    #[inline(always)]
    pub fn uncharge_mem(&mut self, bytes: u64) { self.mem_usage = self.mem_usage.saturating_sub(bytes); }

    #[inline]
    pub fn reset_period(&mut self) {
        self.cpu_usage_us = 0;
        self.burst_used_us = 0;
        if self.state == EnforcementState::Throttled || self.state == EnforcementState::Burst { self.state = EnforcementState::Normal; }
    }

    #[inline(always)]
    pub fn cpu_util_pct(&self) -> f64 {
        if self.cpu_quota_us <= 0 { 0.0 } else { self.cpu_usage_us as f64 / self.cpu_quota_us as f64 * 100.0 }
    }

    #[inline(always)]
    pub fn mem_util_pct(&self) -> f64 {
        if self.mem_limit == u64::MAX { 0.0 } else { self.mem_usage as f64 / self.mem_limit as f64 * 100.0 }
    }
}

/// Cgroup event
#[derive(Debug, Clone)]
pub struct AppCgroupEvent {
    pub cgroup_id: u64,
    pub kind: AppCgroupEventKind,
    pub ts: u64,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCgroupEventKind {
    Throttled,
    OverMemLimit,
    BurstStart,
    BurstEnd,
    OomRisk,
    LimitChanged,
    PeriodReset,
}

/// Cgroup controller stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppCgroupStats {
    pub total_cgroups: usize,
    pub throttled_count: usize,
    pub over_limit_count: usize,
    pub total_cpu_usage_us: u64,
    pub total_mem_usage: u64,
    pub total_throttle_events: u64,
}

/// Apps cgroup controller
pub struct AppsCgroupCtrl {
    cgroups: BTreeMap<u64, AppCgroup>,
    events: Vec<AppCgroupEvent>,
    stats: AppCgroupStats,
    next_id: u64,
}

impl AppsCgroupCtrl {
    pub fn new() -> Self { Self { cgroups: BTreeMap::new(), events: Vec::new(), stats: AppCgroupStats::default(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, name: String, parent: Option<u64>) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let cg = AppCgroup::new(id, name, parent);
        self.cgroups.insert(id, cg);
        if let Some(pid) = parent { if let Some(p) = self.cgroups.get_mut(&pid) { p.children.push(id); } }
        id
    }

    #[inline]
    pub fn charge_cpu(&mut self, id: u64, us: u64, ts: u64) {
        if let Some(cg) = self.cgroups.get_mut(&id) {
            let old = cg.state;
            cg.charge_cpu(us);
            if cg.state == EnforcementState::Throttled && old != EnforcementState::Throttled {
                self.events.push(AppCgroupEvent { cgroup_id: id, kind: AppCgroupEventKind::Throttled, ts, value: cg.cpu_usage_us });
            }
        }
    }

    #[inline]
    pub fn charge_mem(&mut self, id: u64, bytes: u64, ts: u64) -> bool {
        if let Some(cg) = self.cgroups.get_mut(&id) {
            let ok = cg.charge_mem(bytes);
            if !ok { self.events.push(AppCgroupEvent { cgroup_id: id, kind: AppCgroupEventKind::OverMemLimit, ts, value: cg.mem_usage }); }
            ok
        } else { false }
    }

    #[inline(always)]
    pub fn uncharge_mem(&mut self, id: u64, bytes: u64) { if let Some(cg) = self.cgroups.get_mut(&id) { cg.uncharge_mem(bytes); } }

    #[inline]
    pub fn reset_periods(&mut self, ts: u64) {
        let ids: Vec<u64> = self.cgroups.keys().copied().collect();
        for id in ids { if let Some(cg) = self.cgroups.get_mut(&id) { cg.reset_period(); } }
        self.events.push(AppCgroupEvent { cgroup_id: 0, kind: AppCgroupEventKind::PeriodReset, ts, value: 0 });
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_cgroups = self.cgroups.len();
        self.stats.throttled_count = self.cgroups.values().filter(|c| c.state == EnforcementState::Throttled).count();
        self.stats.over_limit_count = self.cgroups.values().filter(|c| c.state == EnforcementState::OverLimit).count();
        self.stats.total_cpu_usage_us = self.cgroups.values().map(|c| c.cpu_usage_us).sum();
        self.stats.total_mem_usage = self.cgroups.values().map(|c| c.mem_usage).sum();
        self.stats.total_throttle_events = self.cgroups.values().map(|c| c.nr_throttled).sum();
    }

    #[inline(always)]
    pub fn cgroup(&self, id: u64) -> Option<&AppCgroup> { self.cgroups.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &AppCgroupStats { &self.stats }
}
