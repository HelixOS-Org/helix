//! # Holistic Perf Counter
//!
//! Hardware performance counter management:
//! - PMU event configuration and multiplexing
//! - Per-CPU and per-task counter tracking
//! - Event groups with leader-follower scheduling
//! - Sampling with overflow interrupt tracking
//! - Derived metrics (IPC, cache miss ratio, branch mispredict)
//! - Counter overflow management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Hardware event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwEvent {
    Cycles,
    Instructions,
    CacheReferences,
    CacheMisses,
    BranchInstructions,
    BranchMisses,
    BusCycles,
    StalledCyclesFrontend,
    StalledCyclesBackend,
    RefCycles,
    L1DReadMiss,
    L1DWriteMiss,
    L1IReadMiss,
    LlcReadMiss,
    LlcWriteMiss,
    DtlbReadMiss,
    DtlbWriteMiss,
    ItlbReadMiss,
    ContextSwitches,
    PageFaults,
    Custom(u32),
}

/// Counter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterMode {
    Counting,
    Sampling,
    Multiplexed,
}

/// A configured performance counter
#[derive(Debug, Clone)]
pub struct PerfCounter {
    pub id: u64,
    pub event: HwEvent,
    pub mode: CounterMode,
    pub value: u64,
    pub enabled_ns: u64,
    pub running_ns: u64,
    pub overflows: u64,
    pub sample_period: u64,
    pub last_read_ts: u64,
    pub cpu: Option<u32>,
    pub task_id: Option<u64>,
    pub group_leader: Option<u64>,
    pub active: bool,
}

impl PerfCounter {
    pub fn new(id: u64, event: HwEvent, mode: CounterMode) -> Self {
        Self {
            id, event, mode, value: 0, enabled_ns: 0, running_ns: 0,
            overflows: 0, sample_period: 0, last_read_ts: 0,
            cpu: None, task_id: None, group_leader: None, active: false,
        }
    }

    pub fn read(&mut self, ts: u64) -> u64 {
        self.last_read_ts = ts;
        if self.enabled_ns > 0 && self.running_ns > 0 && self.running_ns < self.enabled_ns {
            // Scale value for multiplexing
            (self.value as u128 * self.enabled_ns as u128 / self.running_ns as u128) as u64
        } else {
            self.value
        }
    }

    pub fn update(&mut self, delta: u64, enabled_delta: u64, running_delta: u64) {
        self.value = self.value.wrapping_add(delta);
        self.enabled_ns += enabled_delta;
        self.running_ns += running_delta;
    }

    pub fn overflow(&mut self) { self.overflows += 1; }
    pub fn reset(&mut self) { self.value = 0; self.overflows = 0; self.enabled_ns = 0; self.running_ns = 0; }
    pub fn multiplex_ratio(&self) -> f64 { if self.enabled_ns == 0 { 1.0 } else { self.running_ns as f64 / self.enabled_ns as f64 } }
}

/// Counter group
#[derive(Debug, Clone)]
pub struct CounterGroup {
    pub leader_id: u64,
    pub members: Vec<u64>,
    pub pinned: bool,
    pub exclusive: bool,
}

/// Derived metric
#[derive(Debug, Clone)]
pub struct DerivedMetric {
    pub name_hash: u64,
    pub value: f64,
    pub ts: u64,
}

/// Per-CPU PMU state
#[derive(Debug, Clone)]
pub struct PmuState {
    pub cpu_id: u32,
    pub hw_counters: u32,
    pub active_counters: u32,
    pub fixed_counters: u32,
    pub multiplex_switches: u64,
}

impl PmuState {
    pub fn new(cpu: u32, hw: u32, fixed: u32) -> Self {
        Self { cpu_id: cpu, hw_counters: hw, active_counters: 0, fixed_counters: fixed, multiplex_switches: 0 }
    }
}

/// Sample record
#[derive(Debug, Clone)]
pub struct PerfSample {
    pub counter_id: u64,
    pub ip: u64,
    pub pid: u64,
    pub cpu: u32,
    pub ts: u64,
    pub value: u64,
}

/// Perf counter stats
#[derive(Debug, Clone, Default)]
pub struct PerfCounterStats {
    pub counters: usize,
    pub groups: usize,
    pub samples: u64,
    pub overflows: u64,
    pub ipc: f64,
    pub cache_miss_ratio: f64,
    pub branch_miss_ratio: f64,
}

/// Holistic performance counter manager
pub struct HolisticPerfCounter {
    counters: BTreeMap<u64, PerfCounter>,
    groups: Vec<CounterGroup>,
    pmus: BTreeMap<u32, PmuState>,
    samples: Vec<PerfSample>,
    derived: Vec<DerivedMetric>,
    stats: PerfCounterStats,
    next_id: u64,
}

impl HolisticPerfCounter {
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(), groups: Vec::new(),
            pmus: BTreeMap::new(), samples: Vec::new(),
            derived: Vec::new(), stats: PerfCounterStats::default(),
            next_id: 1,
        }
    }

    pub fn add_pmu(&mut self, cpu: u32, hw_counters: u32, fixed: u32) {
        self.pmus.insert(cpu, PmuState::new(cpu, hw_counters, fixed));
    }

    pub fn create_counter(&mut self, event: HwEvent, mode: CounterMode) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.counters.insert(id, PerfCounter::new(id, event, mode));
        id
    }

    pub fn bind_cpu(&mut self, counter_id: u64, cpu: u32) {
        if let Some(c) = self.counters.get_mut(&counter_id) { c.cpu = Some(cpu); }
    }

    pub fn bind_task(&mut self, counter_id: u64, task: u64) {
        if let Some(c) = self.counters.get_mut(&counter_id) { c.task_id = Some(task); }
    }

    pub fn create_group(&mut self, leader: u64, members: Vec<u64>) {
        for &m in &members {
            if let Some(c) = self.counters.get_mut(&m) { c.group_leader = Some(leader); }
        }
        self.groups.push(CounterGroup { leader_id: leader, members, pinned: false, exclusive: false });
    }

    pub fn enable(&mut self, id: u64) { if let Some(c) = self.counters.get_mut(&id) { c.active = true; } }
    pub fn disable(&mut self, id: u64) { if let Some(c) = self.counters.get_mut(&id) { c.active = false; } }

    pub fn update(&mut self, id: u64, delta: u64, enabled: u64, running: u64) {
        if let Some(c) = self.counters.get_mut(&id) { c.update(delta, enabled, running); }
    }

    pub fn read(&mut self, id: u64, ts: u64) -> Option<u64> {
        self.counters.get_mut(&id).map(|c| c.read(ts))
    }

    pub fn record_sample(&mut self, counter_id: u64, ip: u64, pid: u64, cpu: u32, ts: u64, value: u64) {
        self.samples.push(PerfSample { counter_id, ip, pid, cpu, ts, value });
        self.stats.samples += 1;
    }

    pub fn compute_derived(&mut self, ts: u64) {
        let cycles = self.counters.values().filter(|c| matches!(c.event, HwEvent::Cycles)).map(|c| c.value).sum::<u64>();
        let insns = self.counters.values().filter(|c| matches!(c.event, HwEvent::Instructions)).map(|c| c.value).sum::<u64>();
        let cache_refs = self.counters.values().filter(|c| matches!(c.event, HwEvent::CacheReferences)).map(|c| c.value).sum::<u64>();
        let cache_misses = self.counters.values().filter(|c| matches!(c.event, HwEvent::CacheMisses)).map(|c| c.value).sum::<u64>();
        let br_insns = self.counters.values().filter(|c| matches!(c.event, HwEvent::BranchInstructions)).map(|c| c.value).sum::<u64>();
        let br_misses = self.counters.values().filter(|c| matches!(c.event, HwEvent::BranchMisses)).map(|c| c.value).sum::<u64>();

        if cycles > 0 {
            self.stats.ipc = insns as f64 / cycles as f64;
            self.derived.push(DerivedMetric { name_hash: 0x1, value: self.stats.ipc, ts });
        }
        if cache_refs > 0 {
            self.stats.cache_miss_ratio = cache_misses as f64 / cache_refs as f64;
            self.derived.push(DerivedMetric { name_hash: 0x2, value: self.stats.cache_miss_ratio, ts });
        }
        if br_insns > 0 {
            self.stats.branch_miss_ratio = br_misses as f64 / br_insns as f64;
            self.derived.push(DerivedMetric { name_hash: 0x3, value: self.stats.branch_miss_ratio, ts });
        }
    }

    pub fn recompute(&mut self) {
        self.stats.counters = self.counters.len();
        self.stats.groups = self.groups.len();
        self.stats.overflows = self.counters.values().map(|c| c.overflows).sum();
    }

    pub fn counter(&self, id: u64) -> Option<&PerfCounter> { self.counters.get(&id) }
    pub fn pmu(&self, cpu: u32) -> Option<&PmuState> { self.pmus.get(&cpu) }
    pub fn stats(&self) -> &PerfCounterStats { &self.stats }
    pub fn derived(&self) -> &[DerivedMetric] { &self.derived }
}
