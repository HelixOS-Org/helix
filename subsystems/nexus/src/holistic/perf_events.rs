// SPDX-License-Identifier: GPL-2.0
//! Holistic perf_events — performance event monitoring and PMU management.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfEventType {
    Hardware,
    Software,
    Tracepoint,
    HwCache,
    Raw,
    Breakpoint,
}

/// Hardware event id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HwEventId {
    CpuCycles,
    Instructions,
    CacheReferences,
    CacheMisses,
    BranchInstructions,
    BranchMisses,
    BusCycles,
    StalledCyclesFrontend,
    StalledCyclesBackend,
    RefCpuCycles,
}

impl HwEventId {
    #[inline(always)]
    pub fn is_cache_related(&self) -> bool {
        matches!(self, Self::CacheReferences | Self::CacheMisses)
    }

    #[inline(always)]
    pub fn is_branch_related(&self) -> bool {
        matches!(self, Self::BranchInstructions | Self::BranchMisses)
    }
}

/// Software event id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwEventId {
    CpuClock,
    TaskClock,
    PageFaults,
    ContextSwitches,
    CpuMigrations,
    PageFaultsMajor,
    PageFaultsMinor,
    AlignmentFaults,
    EmulationFaults,
}

/// PMU type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmuType {
    Core,
    Uncore,
    Software,
    Tracepoint,
}

/// A PMU descriptor
#[derive(Debug, Clone)]
pub struct PmuDesc {
    pub id: u32,
    pub name: String,
    pub pmu_type: PmuType,
    pub nr_counters: u32,
    pub nr_fixed: u32,
    pub counter_width: u8,
    pub available: bool,
}

impl PmuDesc {
    #[inline(always)]
    pub fn total_counters(&self) -> u32 {
        self.nr_counters + self.nr_fixed
    }

    #[inline(always)]
    pub fn max_count(&self) -> u64 {
        if self.counter_width >= 64 { u64::MAX }
        else { (1u64 << self.counter_width) - 1 }
    }
}

/// Event configuration
#[derive(Debug, Clone)]
pub struct PerfEventConfig {
    pub event_id: u64,
    pub event_type: PerfEventType,
    pub sample_period: u64,
    pub sample_type: u32,
    pub exclude_kernel: bool,
    pub exclude_user: bool,
    pub exclude_idle: bool,
    pub pinned: bool,
    pub inherit: bool,
    pub cpu: Option<u32>,
    pub pid: Option<u32>,
}

/// A live perf event
#[derive(Debug)]
pub struct PerfEvent {
    pub id: u64,
    pub config: PerfEventConfig,
    pub count: u64,
    pub enabled_time_ns: u64,
    pub running_time_ns: u64,
    pub pmu_id: u32,
    pub state: PerfEventState,
    pub overflow_count: u64,
    pub last_sample_timestamp: u64,
}

impl PerfEvent {
    pub fn new(id: u64, config: PerfEventConfig, pmu_id: u32) -> Self {
        Self {
            id, config, count: 0,
            enabled_time_ns: 0, running_time_ns: 0,
            pmu_id, state: PerfEventState::Inactive,
            overflow_count: 0, last_sample_timestamp: 0,
        }
    }

    #[inline(always)]
    pub fn multiplexing_ratio(&self) -> f64 {
        if self.enabled_time_ns == 0 { return 0.0; }
        self.running_time_ns as f64 / self.enabled_time_ns as f64
    }

    #[inline]
    pub fn scaled_count(&self) -> u64 {
        let ratio = self.multiplexing_ratio();
        if ratio < 0.001 { return 0; }
        (self.count as f64 / ratio) as u64
    }

    #[inline(always)]
    pub fn is_multiplexed(&self) -> bool {
        self.multiplexing_ratio() < 0.99
    }
}

/// Event state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfEventState {
    Inactive,
    Active,
    Error,
    Off,
}

/// Per-CPU perf state
#[derive(Debug)]
#[repr(align(64))]
pub struct CpuPerfState {
    pub cpu_id: u32,
    pub active_events: Vec<u64>,
    pub gp_counters_used: u32,
    pub fixed_counters_used: u32,
    pub total_gp: u32,
    pub total_fixed: u32,
}

impl CpuPerfState {
    pub fn new(cpu_id: u32, gp: u32, fixed: u32) -> Self {
        Self {
            cpu_id, active_events: Vec::new(),
            gp_counters_used: 0, fixed_counters_used: 0,
            total_gp: gp, total_fixed: fixed,
        }
    }

    #[inline]
    pub fn utilization(&self) -> f64 {
        let total = self.total_gp + self.total_fixed;
        if total == 0 { return 0.0; }
        let used = self.gp_counters_used + self.fixed_counters_used;
        used as f64 / total as f64
    }

    #[inline(always)]
    pub fn can_schedule(&self) -> bool {
        self.gp_counters_used < self.total_gp || self.fixed_counters_used < self.total_fixed
    }
}

/// Sample record
#[derive(Debug, Clone)]
pub struct PerfSample {
    pub event_id: u64,
    pub ip: u64,
    pub pid: u32,
    pub tid: u32,
    pub cpu: u32,
    pub timestamp: u64,
    pub weight: u32,
}

/// Perf stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PerfEventsStats {
    pub total_events: u64,
    pub active_events: u64,
    pub total_samples: u64,
    pub lost_samples: u64,
    pub multiplexed_events: u64,
    pub pmu_count: u32,
}

/// Main perf events manager
pub struct HolisticPerfEvents {
    pmus: BTreeMap<u32, PmuDesc>,
    events: BTreeMap<u64, PerfEvent>,
    cpu_states: BTreeMap<u32, CpuPerfState>,
    samples: VecDeque<PerfSample>,
    max_samples: usize,
    next_event_id: u64,
    stats: PerfEventsStats,
}

impl HolisticPerfEvents {
    pub fn new() -> Self {
        Self {
            pmus: BTreeMap::new(),
            events: BTreeMap::new(),
            cpu_states: BTreeMap::new(),
            samples: VecDeque::new(),
            max_samples: 8192,
            next_event_id: 1,
            stats: PerfEventsStats {
                total_events: 0, active_events: 0, total_samples: 0,
                lost_samples: 0, multiplexed_events: 0, pmu_count: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_pmu(&mut self, pmu: PmuDesc) {
        self.stats.pmu_count += 1;
        self.pmus.insert(pmu.id, pmu);
    }

    #[inline]
    pub fn create_event(&mut self, config: PerfEventConfig, pmu_id: u32) -> u64 {
        let id = self.next_event_id;
        self.next_event_id += 1;
        let event = PerfEvent::new(id, config, pmu_id);
        self.events.insert(id, event);
        self.stats.total_events += 1;
        id
    }

    pub fn activate_event(&mut self, event_id: u64) -> bool {
        if let Some(event) = self.events.get_mut(&event_id) {
            event.state = PerfEventState::Active;
            self.stats.active_events += 1;
            if let Some(cpu) = event.config.cpu {
                if let Some(cs) = self.cpu_states.get_mut(&cpu) {
                    cs.active_events.push(event_id);
                    cs.gp_counters_used += 1;
                }
            }
            return true;
        }
        false
    }

    pub fn deactivate_event(&mut self, event_id: u64) -> bool {
        if let Some(event) = self.events.get_mut(&event_id) {
            event.state = PerfEventState::Inactive;
            if self.stats.active_events > 0 { self.stats.active_events -= 1; }
            if let Some(cpu) = event.config.cpu {
                if let Some(cs) = self.cpu_states.get_mut(&cpu) {
                    cs.active_events.retain(|&e| e != event_id);
                    if cs.gp_counters_used > 0 { cs.gp_counters_used -= 1; }
                }
            }
            return true;
        }
        false
    }

    #[inline]
    pub fn record_sample(&mut self, sample: PerfSample) {
        self.stats.total_samples += 1;
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    #[inline]
    pub fn multiplexed_events(&self) -> Vec<u64> {
        self.events.iter()
            .filter(|(_, e)| e.is_multiplexed() && e.state == PerfEventState::Active)
            .map(|(&id, _)| id)
            .collect()
    }

    #[inline]
    pub fn hottest_ips(&self, n: usize) -> Vec<(u64, u64)> {
        let mut ip_counts: LinearMap<u64, 64> = BTreeMap::new();
        for s in &self.samples {
            *ip_counts.entry(s.ip).or_insert(0) += 1;
        }
        let mut v: Vec<_> = ip_counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    #[inline(always)]
    pub fn add_cpu_state(&mut self, state: CpuPerfState) {
        self.cpu_states.insert(state.cpu_id, state);
    }

    #[inline(always)]
    pub fn stats(&self) -> &PerfEventsStats {
        &self.stats
    }
}

// ============================================================================
// Merged from perf_events_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwEventType {
    CpuCycles, Instructions, CacheMisses, CacheReferences,
    BranchMisses, BranchInstructions, BusCycles, StalledFrontend,
    StalledBackend, RefCycles, LlcLoadMisses, LlcStoreMisses,
}

/// Software event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwEventType {
    CpuClock, TaskClock, PageFaults, ContextSwitches,
    CpuMigrations, MinorFaults, MajorFaults, AlignmentFaults,
    EmulationFaults,
}

/// Event counter
#[derive(Debug)]
#[repr(align(64))]
pub struct PerfCounter {
    pub id: u64,
    pub hw_event: Option<HwEventType>,
    pub sw_event: Option<SwEventType>,
    pub value: u64,
    pub enabled_time: u64,
    pub running_time: u64,
    pub sample_period: u64,
    pub samples_collected: u64,
    pub overflow_count: u64,
    pub cpu: i32,
    pub pid: i64,
}

impl PerfCounter {
    #[inline(always)]
    pub fn hw(id: u64, event: HwEventType) -> Self {
        Self { id, hw_event: Some(event), sw_event: None, value: 0, enabled_time: 0, running_time: 0, sample_period: 0, samples_collected: 0, overflow_count: 0, cpu: -1, pid: -1 }
    }

    #[inline(always)]
    pub fn sw(id: u64, event: SwEventType) -> Self {
        Self { id, hw_event: None, sw_event: Some(event), value: 0, enabled_time: 0, running_time: 0, sample_period: 0, samples_collected: 0, overflow_count: 0, cpu: -1, pid: -1 }
    }

    #[inline(always)]
    pub fn increment(&mut self, delta: u64) { self.value += delta; }
    #[inline(always)]
    pub fn multiplexing_ratio(&self) -> f64 { if self.enabled_time == 0 { 1.0 } else { self.running_time as f64 / self.enabled_time as f64 } }
    #[inline(always)]
    pub fn scaled_value(&self) -> u64 { let ratio = self.multiplexing_ratio(); if ratio == 0.0 { 0 } else { (self.value as f64 / ratio) as u64 } }
}

/// Event group
#[derive(Debug)]
pub struct PerfEventGroup {
    pub id: u64,
    pub leader: u64,
    pub members: Vec<u64>,
    pub pinned: bool,
}

/// Sample record
#[derive(Debug, Clone)]
pub struct PerfSampleV2 {
    pub ip: u64,
    pub pid: u64,
    pub tid: u64,
    pub cpu: u32,
    pub timestamp: u64,
    pub period: u64,
    pub weight: u32,
    pub data_src: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PerfEventsV2Stats {
    pub total_counters: u32,
    pub hw_counters: u32,
    pub sw_counters: u32,
    pub total_groups: u32,
    pub total_samples: u64,
    pub avg_mux_ratio: f64,
}

/// Main perf events v2 manager
pub struct HolisticPerfEventsV2 {
    counters: BTreeMap<u64, PerfCounter>,
    groups: BTreeMap<u64, PerfEventGroup>,
    samples: VecDeque<PerfSampleV2>,
    next_id: u64,
    max_samples: usize,
}

impl HolisticPerfEventsV2 {
    pub fn new() -> Self { Self { counters: BTreeMap::new(), groups: BTreeMap::new(), samples: VecDeque::new(), next_id: 1, max_samples: 8192 } }

    #[inline]
    pub fn add_hw_counter(&mut self, event: HwEventType) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.counters.insert(id, PerfCounter::hw(id, event));
        id
    }

    #[inline]
    pub fn add_sw_counter(&mut self, event: SwEventType) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.counters.insert(id, PerfCounter::sw(id, event));
        id
    }

    #[inline(always)]
    pub fn record_sample(&mut self, sample: PerfSampleV2) {
        if self.samples.len() >= self.max_samples { self.samples.drain(..self.max_samples / 2); }
        self.samples.push_back(sample);
    }

    #[inline]
    pub fn stats(&self) -> PerfEventsV2Stats {
        let hw = self.counters.values().filter(|c| c.hw_event.is_some()).count() as u32;
        let sw = self.counters.values().filter(|c| c.sw_event.is_some()).count() as u32;
        let mux: Vec<f64> = self.counters.values().map(|c| c.multiplexing_ratio()).collect();
        let avg = if mux.is_empty() { 1.0 } else { mux.iter().sum::<f64>() / mux.len() as f64 };
        PerfEventsV2Stats { total_counters: self.counters.len() as u32, hw_counters: hw, sw_counters: sw, total_groups: self.groups.len() as u32, total_samples: self.samples.len() as u64, avg_mux_ratio: avg }
    }
}
