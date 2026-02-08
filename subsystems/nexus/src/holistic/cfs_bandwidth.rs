// SPDX-License-Identifier: GPL-2.0
//! Holistic cfs_bandwidth — CFS bandwidth throttling.

extern crate alloc;

use alloc::collections::BTreeMap;

/// CFS group throttle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfsBwState {
    Running,
    Throttled,
    Expired,
    Distributing,
}

/// CFS bandwidth group
#[derive(Debug)]
pub struct CfsBwGroup {
    pub id: u64,
    pub quota_ns: u64,
    pub period_ns: u64,
    pub runtime_remaining: u64,
    pub state: CfsBwState,
    pub nr_throttled: u64,
    pub throttled_time_ns: u64,
    pub nr_periods: u64,
    pub nr_burst: u64,
    pub burst_ns: u64,
}

impl CfsBwGroup {
    pub fn new(id: u64, quota_ns: u64, period_ns: u64) -> Self {
        Self { id, quota_ns, period_ns, runtime_remaining: quota_ns, state: CfsBwState::Running, nr_throttled: 0, throttled_time_ns: 0, nr_periods: 0, nr_burst: 0, burst_ns: 0 }
    }

    pub fn consume(&mut self, ns: u64) {
        if ns >= self.runtime_remaining { self.runtime_remaining = 0; self.state = CfsBwState::Throttled; self.nr_throttled += 1; }
        else { self.runtime_remaining -= ns; }
    }

    pub fn refill(&mut self) {
        self.runtime_remaining = self.quota_ns;
        self.nr_periods += 1;
        if self.state == CfsBwState::Throttled { self.state = CfsBwState::Running; }
    }

    pub fn utilization(&self) -> f64 {
        if self.quota_ns == 0 { 0.0 } else { 1.0 - self.runtime_remaining as f64 / self.quota_ns as f64 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct CfsBandwidthStats {
    pub total_groups: u32,
    pub throttled_groups: u32,
    pub total_throttle_events: u64,
    pub total_throttled_time_ns: u64,
    pub avg_utilization: f64,
}

/// Main holistic CFS bandwidth
pub struct HolisticCfsBandwidth {
    groups: BTreeMap<u64, CfsBwGroup>,
}

impl HolisticCfsBandwidth {
    pub fn new() -> Self { Self { groups: BTreeMap::new() } }

    pub fn add_group(&mut self, id: u64, quota_ns: u64, period_ns: u64) { self.groups.insert(id, CfsBwGroup::new(id, quota_ns, period_ns)); }

    pub fn consume(&mut self, id: u64, ns: u64) { if let Some(g) = self.groups.get_mut(&id) { g.consume(ns); } }

    pub fn tick(&mut self) { for g in self.groups.values_mut() { g.refill(); } }

    pub fn stats(&self) -> CfsBandwidthStats {
        let throttled = self.groups.values().filter(|g| g.state == CfsBwState::Throttled).count() as u32;
        let events: u64 = self.groups.values().map(|g| g.nr_throttled).sum();
        let time: u64 = self.groups.values().map(|g| g.throttled_time_ns).sum();
        let util: f64 = if self.groups.is_empty() { 0.0 } else { self.groups.values().map(|g| g.utilization()).sum::<f64>() / self.groups.len() as f64 };
        CfsBandwidthStats { total_groups: self.groups.len() as u32, throttled_groups: throttled, total_throttle_events: events, total_throttled_time_ns: time, avg_utilization: util }
    }
}

// ============================================================================
// Merged from cfs_bandwidth_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfsBwV2State {
    Uncapped,
    Active,
    Throttled,
    Expired,
}

/// CFS bandwidth group v2
#[derive(Debug)]
pub struct CfsBwV2Group {
    pub id: u64,
    pub quota_us: u64,
    pub period_us: u64,
    pub runtime_us: u64,
    pub state: CfsBwV2State,
    pub nr_throttled: u64,
    pub throttled_time_us: u64,
    pub nr_periods: u64,
    pub burst_us: u64,
}

impl CfsBwV2Group {
    pub fn new(id: u64, quota: u64, period: u64) -> Self {
        Self { id, quota_us: quota, period_us: period, runtime_us: 0, state: CfsBwV2State::Active, nr_throttled: 0, throttled_time_us: 0, nr_periods: 0, burst_us: 0 }
    }

    pub fn consume(&mut self, us: u64) {
        self.runtime_us += us;
        if self.runtime_us >= self.quota_us + self.burst_us {
            self.state = CfsBwV2State::Throttled;
            self.nr_throttled += 1;
        }
    }

    pub fn reset_period(&mut self) {
        if self.state == CfsBwV2State::Throttled {
            self.throttled_time_us += self.runtime_us.saturating_sub(self.quota_us);
        }
        self.runtime_us = 0;
        self.state = CfsBwV2State::Active;
        self.nr_periods += 1;
    }

    pub fn utilization(&self) -> f64 {
        if self.quota_us == 0 { return 0.0; }
        self.runtime_us as f64 / self.quota_us as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct CfsBwV2Stats {
    pub total_groups: u32,
    pub throttled: u32,
    pub total_throttled: u64,
    pub total_throttle_time: u64,
}

/// Main holistic CFS bandwidth v2
pub struct HolisticCfsBandwidthV2 {
    groups: BTreeMap<u64, CfsBwV2Group>,
    next_id: u64,
}

impl HolisticCfsBandwidthV2 {
    pub fn new() -> Self { Self { groups: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, quota: u64, period: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.groups.insert(id, CfsBwV2Group::new(id, quota, period));
        id
    }

    pub fn consume(&mut self, id: u64, us: u64) {
        if let Some(g) = self.groups.get_mut(&id) { g.consume(us); }
    }

    pub fn tick_period(&mut self) {
        for g in self.groups.values_mut() { g.reset_period(); }
    }

    pub fn destroy(&mut self, id: u64) { self.groups.remove(&id); }

    pub fn stats(&self) -> CfsBwV2Stats {
        let throttled = self.groups.values().filter(|g| g.state == CfsBwV2State::Throttled).count() as u32;
        let tot_throt: u64 = self.groups.values().map(|g| g.nr_throttled).sum();
        let tot_time: u64 = self.groups.values().map(|g| g.throttled_time_us).sum();
        CfsBwV2Stats { total_groups: self.groups.len() as u32, throttled, total_throttled: tot_throt, total_throttle_time: tot_time }
    }
}
