// SPDX-License-Identifier: GPL-2.0
//! Bridge perf_hw_bridge — hardware performance monitoring unit bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// PMU event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmuEventType {
    HwCycles,
    HwInstructions,
    HwCacheReferences,
    HwCacheMisses,
    HwBranchInstructions,
    HwBranchMisses,
    HwBusCycles,
    HwStalledFrontend,
    HwStalledBackend,
    SwCpuClock,
    SwTaskClock,
    SwPageFaults,
    SwContextSwitches,
    SwCpuMigrations,
    SwPageFaultsMin,
    SwPageFaultsMaj,
    Raw(u64),
}

/// Sampling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingMode {
    CountOnly,
    FrequencyBased,
    PeriodBased,
}

/// Event scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventScope {
    User,
    Kernel,
    Hypervisor,
    All,
}

/// PMU counter state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterState {
    Disabled,
    Enabled,
    Running,
    Multiplexed,
    Error,
}

/// Performance counter descriptor
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PerfCounter {
    pub id: u64,
    pub event_type: PmuEventType,
    pub scope: EventScope,
    pub state: CounterState,
    pub sampling: SamplingMode,
    pub sample_period: u64,
    pub count: u64,
    pub time_enabled_ns: u64,
    pub time_running_ns: u64,
    pub cpu_id: Option<u32>,
    pub pid: Option<u64>,
    pub overflows: u64,
}

impl PerfCounter {
    pub fn new(id: u64, event: PmuEventType, scope: EventScope) -> Self {
        Self {
            id, event_type: event, scope, state: CounterState::Disabled,
            sampling: SamplingMode::CountOnly, sample_period: 0,
            count: 0, time_enabled_ns: 0, time_running_ns: 0,
            cpu_id: None, pid: None, overflows: 0,
        }
    }

    #[inline(always)]
    pub fn enable(&mut self) { self.state = CounterState::Enabled; }
    #[inline(always)]
    pub fn disable(&mut self) { self.state = CounterState::Disabled; }

    #[inline]
    pub fn increment(&mut self, delta: u64) {
        self.count = self.count.wrapping_add(delta);
        if self.sample_period > 0 && self.count >= self.sample_period {
            self.overflows += self.count / self.sample_period;
            self.count %= self.sample_period;
        }
    }

    #[inline(always)]
    pub fn multiplexing_ratio(&self) -> f64 {
        if self.time_enabled_ns == 0 { return 0.0; }
        self.time_running_ns as f64 / self.time_enabled_ns as f64
    }

    #[inline]
    pub fn scaled_count(&self) -> u64 {
        let ratio = self.multiplexing_ratio();
        if ratio == 0.0 { return self.count; }
        (self.count as f64 / ratio) as u64
    }
}

/// Event group (leader + members)
#[derive(Debug)]
pub struct PerfEventGroup {
    pub leader_id: u64,
    pub members: Vec<u64>,
    pub pinned: bool,
    pub exclusive: bool,
}

impl PerfEventGroup {
    pub fn new(leader: u64) -> Self {
        Self { leader_id: leader, members: alloc::vec![leader], pinned: false, exclusive: false }
    }

    #[inline(always)]
    pub fn add_member(&mut self, counter_id: u64) { self.members.push(counter_id); }
}

/// Sample record
#[derive(Debug, Clone)]
pub struct PerfSample {
    pub counter_id: u64,
    pub ip: u64,
    pub pid: u64,
    pub tid: u64,
    pub time: u64,
    pub cpu: u32,
    pub period: u64,
}

/// Bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PerfHwBridgeStats {
    pub total_counters: u32,
    pub active_counters: u32,
    pub total_samples: u64,
    pub total_overflows: u64,
    pub multiplexed_counters: u32,
    pub avg_mux_ratio: f64,
}

/// Main perf hardware bridge
#[repr(align(64))]
pub struct BridgePerfHw {
    counters: BTreeMap<u64, PerfCounter>,
    groups: Vec<PerfEventGroup>,
    samples: Vec<PerfSample>,
    next_id: u64,
    max_samples: usize,
}

impl BridgePerfHw {
    pub fn new() -> Self {
        Self { counters: BTreeMap::new(), groups: Vec::new(), samples: Vec::new(), next_id: 1, max_samples: 8192 }
    }

    #[inline]
    pub fn create_counter(&mut self, event: PmuEventType, scope: EventScope) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.counters.insert(id, PerfCounter::new(id, event, scope));
        id
    }

    #[inline(always)]
    pub fn create_group(&mut self, leader_id: u64) {
        self.groups.push(PerfEventGroup::new(leader_id));
    }

    #[inline(always)]
    pub fn enable(&mut self, id: u64) {
        if let Some(c) = self.counters.get_mut(&id) { c.enable(); }
    }

    #[inline(always)]
    pub fn disable(&mut self, id: u64) {
        if let Some(c) = self.counters.get_mut(&id) { c.disable(); }
    }

    #[inline(always)]
    pub fn record_sample(&mut self, sample: PerfSample) {
        if self.samples.len() >= self.max_samples { self.samples.drain(..self.max_samples / 4); }
        self.samples.push(sample);
    }

    #[inline(always)]
    pub fn read_counter(&self, id: u64) -> Option<u64> {
        self.counters.get(&id).map(|c| c.scaled_count())
    }

    pub fn stats(&self) -> PerfHwBridgeStats {
        let active = self.counters.values().filter(|c| c.state == CounterState::Running || c.state == CounterState::Enabled).count() as u32;
        let mux = self.counters.values().filter(|c| c.state == CounterState::Multiplexed).count() as u32;
        let overflows: u64 = self.counters.values().map(|c| c.overflows).sum();
        let ratios: Vec<f64> = self.counters.values().filter(|c| c.time_enabled_ns > 0).map(|c| c.multiplexing_ratio()).collect();
        let avg_mux = if ratios.is_empty() { 1.0 } else { ratios.iter().sum::<f64>() / ratios.len() as f64 };
        PerfHwBridgeStats {
            total_counters: self.counters.len() as u32, active_counters: active,
            total_samples: self.samples.len() as u64, total_overflows: overflows,
            multiplexed_counters: mux, avg_mux_ratio: avg_mux,
        }
    }
}
