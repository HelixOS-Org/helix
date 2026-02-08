// SPDX-License-Identifier: GPL-2.0
//! Bridge perf events — hardware and software performance counter proxy.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Performance event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfEventType {
    /// Hardware counter (PMU)
    Hardware,
    /// Software event
    Software,
    /// Tracepoint
    Tracepoint,
    /// Hardware cache event
    HwCache,
    /// Raw PMU event
    Raw,
    /// Breakpoint
    Breakpoint,
}

/// Hardware event IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwEvent {
    CpuCycles,
    Instructions,
    CacheReferences,
    CacheMisses,
    BranchInstructions,
    BranchMisses,
    BusCycles,
    StalledCyclesFront,
    StalledCyclesBack,
    RefCpuCycles,
}

/// Software event IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwEvent {
    CpuClock,
    TaskClock,
    PageFaults,
    ContextSwitches,
    CpuMigrations,
    MinorFaults,
    MajorFaults,
    AlignmentFaults,
    EmulationFaults,
}

/// Sample type flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleType(pub u64);

impl SampleType {
    pub const IP: Self = Self(1 << 0);
    pub const TID: Self = Self(1 << 1);
    pub const TIME: Self = Self(1 << 2);
    pub const ADDR: Self = Self(1 << 3);
    pub const CALLCHAIN: Self = Self(1 << 5);
    pub const CPU: Self = Self(1 << 7);
    pub const PERIOD: Self = Self(1 << 8);
    pub const WEIGHT: Self = Self(1 << 14);
    pub const DATA_SRC: Self = Self(1 << 15);
    pub const BRANCH_STACK: Self = Self(1 << 16);

    pub fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn combine(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Event attribute / configuration
#[derive(Debug, Clone)]
pub struct PerfEventAttr {
    pub event_type: PerfEventType,
    pub config: u64,
    pub sample_period: u64,
    pub sample_type: SampleType,
    pub exclude_user: bool,
    pub exclude_kernel: bool,
    pub exclude_hv: bool,
    pub inherit: bool,
    pub pinned: bool,
    pub exclusive: bool,
    pub enable_on_exec: bool,
    pub watermark: u32,
}

impl PerfEventAttr {
    pub fn hw(event: HwEvent) -> Self {
        Self {
            event_type: PerfEventType::Hardware,
            config: event as u64,
            sample_period: 100_000,
            sample_type: SampleType::IP.combine(SampleType::TID).combine(SampleType::TIME),
            exclude_user: false,
            exclude_kernel: false,
            exclude_hv: true,
            inherit: false,
            pinned: false,
            exclusive: false,
            enable_on_exec: false,
            watermark: 0,
        }
    }

    pub fn sw(event: SwEvent) -> Self {
        Self {
            event_type: PerfEventType::Software,
            config: event as u64,
            sample_period: 1,
            sample_type: SampleType::IP.combine(SampleType::TID),
            exclude_user: false,
            exclude_kernel: false,
            exclude_hv: true,
            inherit: false,
            pinned: false,
            exclusive: false,
            enable_on_exec: false,
            watermark: 0,
        }
    }
}

/// A perf sample record
#[derive(Debug, Clone)]
pub struct PerfSample {
    pub ip: u64,
    pub pid: u64,
    pub tid: u64,
    pub timestamp_ns: u64,
    pub cpu: u32,
    pub period: u64,
    pub weight: u32,
    pub addr: u64,
}

/// An opened perf event
#[derive(Debug)]
pub struct PerfEvent {
    pub fd: i32,
    pub pid: u64,
    pub cpu: i32,
    pub attr: PerfEventAttr,
    pub enabled: bool,
    pub counter_value: u64,
    pub time_enabled_ns: u64,
    pub time_running_ns: u64,
    samples: Vec<PerfSample>,
    max_samples: usize,
    overflow_count: u64,
}

impl PerfEvent {
    pub fn new(fd: i32, pid: u64, cpu: i32, attr: PerfEventAttr) -> Self {
        Self {
            fd,
            pid,
            cpu,
            attr,
            enabled: false,
            counter_value: 0,
            time_enabled_ns: 0,
            time_running_ns: 0,
            samples: Vec::new(),
            max_samples: 8192,
            overflow_count: 0,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn reset(&mut self) {
        self.counter_value = 0;
        self.time_enabled_ns = 0;
        self.time_running_ns = 0;
        self.samples.clear();
    }

    pub fn record_sample(&mut self, sample: PerfSample) {
        if self.samples.len() >= self.max_samples {
            self.overflow_count += 1;
            self.samples.remove(0);
        }
        self.samples.push(sample);
    }

    pub fn read_samples(&mut self, max: usize) -> Vec<PerfSample> {
        let count = max.min(self.samples.len());
        self.samples.drain(..count).collect()
    }

    pub fn multiplexing_ratio(&self) -> f64 {
        if self.time_enabled_ns == 0 { return 0.0; }
        self.time_running_ns as f64 / self.time_enabled_ns as f64
    }

    pub fn pending_samples(&self) -> usize {
        self.samples.len()
    }
}

/// Per-CPU PMU state
#[derive(Debug)]
pub struct CpuPmuState {
    pub cpu_id: u32,
    pub hw_counters_total: u32,
    pub hw_counters_used: u32,
    pub events_active: Vec<i32>,
    multiplexing_needed: bool,
}

impl CpuPmuState {
    pub fn new(cpu_id: u32, hw_counters: u32) -> Self {
        Self {
            cpu_id,
            hw_counters_total: hw_counters,
            hw_counters_used: 0,
            events_active: Vec::new(),
            multiplexing_needed: false,
        }
    }

    pub fn can_schedule(&self) -> bool {
        self.hw_counters_used < self.hw_counters_total
    }

    pub fn needs_multiplexing(&self) -> bool {
        self.multiplexing_needed
    }

    pub fn utilization(&self) -> f64 {
        if self.hw_counters_total == 0 { return 0.0; }
        self.hw_counters_used as f64 / self.hw_counters_total as f64
    }
}

/// Perf bridge stats
#[derive(Debug, Clone)]
pub struct PerfBridgeStats {
    pub events_opened: u64,
    pub events_closed: u64,
    pub samples_collected: u64,
    pub overflows: u64,
    pub multiplexing_events: u64,
    pub pmu_saturated_count: u64,
}

/// Main perf bridge manager
pub struct BridgePerf {
    events: BTreeMap<i32, PerfEvent>,
    cpu_pmu: BTreeMap<u32, CpuPmuState>,
    next_fd: i32,
    max_events_per_process: u32,
    stats: PerfBridgeStats,
}

impl BridgePerf {
    pub fn new() -> Self {
        Self {
            events: BTreeMap::new(),
            cpu_pmu: BTreeMap::new(),
            next_fd: 200,
            max_events_per_process: 1024,
            stats: PerfBridgeStats {
                events_opened: 0,
                events_closed: 0,
                samples_collected: 0,
                overflows: 0,
                multiplexing_events: 0,
                pmu_saturated_count: 0,
            },
        }
    }

    pub fn init_cpu(&mut self, cpu_id: u32, hw_counters: u32) {
        self.cpu_pmu.insert(cpu_id, CpuPmuState::new(cpu_id, hw_counters));
    }

    pub fn open_event(&mut self, pid: u64, cpu: i32, attr: PerfEventAttr) -> Option<i32> {
        // Check per-process limit
        let per_proc = self.events.values().filter(|e| e.pid == pid).count() as u32;
        if per_proc >= self.max_events_per_process {
            return None;
        }
        let fd = self.next_fd;
        self.next_fd += 1;

        // Try to allocate on CPU PMU
        if cpu >= 0 {
            if let Some(pmu) = self.cpu_pmu.get_mut(&(cpu as u32)) {
                if attr.event_type == PerfEventType::Hardware {
                    if pmu.can_schedule() {
                        pmu.hw_counters_used += 1;
                        pmu.events_active.push(fd);
                    } else {
                        pmu.multiplexing_needed = true;
                        pmu.events_active.push(fd);
                        self.stats.multiplexing_events += 1;
                    }
                }
            }
        }

        self.events.insert(fd, PerfEvent::new(fd, pid, cpu, attr));
        self.stats.events_opened += 1;
        Some(fd)
    }

    pub fn close_event(&mut self, fd: i32) -> bool {
        if let Some(event) = self.events.remove(&fd) {
            if event.cpu >= 0 {
                if let Some(pmu) = self.cpu_pmu.get_mut(&(event.cpu as u32)) {
                    pmu.events_active.retain(|&f| f != fd);
                    if event.attr.event_type == PerfEventType::Hardware && pmu.hw_counters_used > 0 {
                        pmu.hw_counters_used -= 1;
                    }
                }
            }
            self.stats.events_closed += 1;
            true
        } else {
            false
        }
    }

    pub fn enable_event(&mut self, fd: i32) -> bool {
        if let Some(event) = self.events.get_mut(&fd) {
            event.enable();
            true
        } else {
            false
        }
    }

    pub fn disable_event(&mut self, fd: i32) -> bool {
        if let Some(event) = self.events.get_mut(&fd) {
            event.disable();
            true
        } else {
            false
        }
    }

    pub fn record_sample(&mut self, fd: i32, sample: PerfSample) -> bool {
        if let Some(event) = self.events.get_mut(&fd) {
            if !event.enabled {
                return false;
            }
            event.record_sample(sample);
            self.stats.samples_collected += 1;
            true
        } else {
            false
        }
    }

    pub fn read_counter(&self, fd: i32) -> Option<(u64, u64, u64)> {
        self.events.get(&fd).map(|e| {
            (e.counter_value, e.time_enabled_ns, e.time_running_ns)
        })
    }

    pub fn read_samples(&mut self, fd: i32, max: usize) -> Vec<PerfSample> {
        if let Some(event) = self.events.get_mut(&fd) {
            event.read_samples(max)
        } else {
            Vec::new()
        }
    }

    pub fn stats(&self) -> &PerfBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from perf_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfV2EventType {
    Hardware, Software, Tracepoint, HwCache, RawPmu, Breakpoint,
}

/// Sample type flags
#[derive(Debug, Clone, Copy)]
pub struct PerfV2SampleType(pub u64);

impl PerfV2SampleType {
    pub const IP: u64 = 1 << 0;
    pub const TID: u64 = 1 << 1;
    pub const TIME: u64 = 1 << 2;
    pub const ADDR: u64 = 1 << 3;
    pub const READ: u64 = 1 << 4;
    pub const CALLCHAIN: u64 = 1 << 5;
    pub const ID: u64 = 1 << 6;
    pub const CPU: u64 = 1 << 7;
    pub const PERIOD: u64 = 1 << 8;
    pub const STREAM_ID: u64 = 1 << 9;
    pub const RAW: u64 = 1 << 10;
    pub const BRANCH_STACK: u64 = 1 << 11;
    pub const REGS_USER: u64 = 1 << 12;
    pub const STACK_USER: u64 = 1 << 13;
    pub const WEIGHT: u64 = 1 << 14;
    pub const DATA_SRC: u64 = 1 << 15;

    pub fn has(&self, f: u64) -> bool { self.0 & f != 0 }
}

/// Perf v2 event attribute
#[derive(Debug, Clone)]
pub struct PerfV2Attr {
    pub event_type: PerfV2EventType,
    pub config: u64,
    pub sample_period: u64,
    pub sample_type: PerfV2SampleType,
    pub disabled: bool,
    pub inherit: bool,
    pub exclusive: bool,
    pub exclude_user: bool,
    pub exclude_kernel: bool,
    pub exclude_hv: bool,
}

/// Perf v2 event instance
#[derive(Debug)]
pub struct PerfV2Event {
    pub id: u64,
    pub attr: PerfV2Attr,
    pub cpu: i32,
    pub pid: i64,
    pub count: u64,
    pub time_enabled: u64,
    pub time_running: u64,
    pub ring_buffer_pages: u32,
    pub lost_events: u64,
}

impl PerfV2Event {
    pub fn new(id: u64, attr: PerfV2Attr) -> Self {
        Self { id, attr, cpu: -1, pid: -1, count: 0, time_enabled: 0, time_running: 0, ring_buffer_pages: 16, lost_events: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PerfV2BridgeStats {
    pub total_events: u32,
    pub hw_events: u32,
    pub sw_events: u32,
    pub tracepoints: u32,
    pub total_samples: u64,
    pub lost_events: u64,
}

/// Main perf v2 bridge
pub struct BridgePerfV2 {
    events: BTreeMap<u64, PerfV2Event>,
    next_id: u64,
}

impl BridgePerfV2 {
    pub fn new() -> Self { Self { events: BTreeMap::new(), next_id: 1 } }

    pub fn open_event(&mut self, attr: PerfV2Attr) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.events.insert(id, PerfV2Event::new(id, attr));
        id
    }

    pub fn close_event(&mut self, id: u64) { self.events.remove(&id); }

    pub fn read(&self, id: u64) -> Option<u64> { self.events.get(&id).map(|e| e.count) }

    pub fn stats(&self) -> PerfV2BridgeStats {
        let hw = self.events.values().filter(|e| e.attr.event_type == PerfV2EventType::Hardware).count() as u32;
        let sw = self.events.values().filter(|e| e.attr.event_type == PerfV2EventType::Software).count() as u32;
        let tp = self.events.values().filter(|e| e.attr.event_type == PerfV2EventType::Tracepoint).count() as u32;
        let samples: u64 = self.events.values().map(|e| e.count).sum();
        let lost: u64 = self.events.values().map(|e| e.lost_events).sum();
        PerfV2BridgeStats { total_events: self.events.len() as u32, hw_events: hw, sw_events: sw, tracepoints: tp, total_samples: samples, lost_events: lost }
    }
}

// ============================================================================
// Merged from perf_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfV3EventType {
    Hardware,
    Software,
    Tracepoint,
    HwCache,
    Raw,
    Breakpoint,
}

/// Perf v3 hardware event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfV3HwEvent {
    CpuCycles,
    Instructions,
    CacheReferences,
    CacheMisses,
    BranchInstructions,
    BranchMisses,
    BusCycles,
    StalledFrontend,
    StalledBackend,
    RefCycles,
}

/// Perf v3 counter
#[derive(Debug)]
pub struct PerfV3Counter {
    pub id: u64,
    pub event_type: PerfV3EventType,
    pub hw_event: Option<PerfV3HwEvent>,
    pub cpu: i32,
    pub pid: i64,
    pub count: u64,
    pub time_enabled: u64,
    pub time_running: u64,
    pub sample_period: u64,
    pub overflow_count: u64,
}

impl PerfV3Counter {
    pub fn new(id: u64, etype: PerfV3EventType) -> Self {
        Self { id, event_type: etype, hw_event: None, cpu: -1, pid: -1, count: 0, time_enabled: 0, time_running: 0, sample_period: 0, overflow_count: 0 }
    }

    pub fn read(&self) -> u64 {
        if self.time_enabled == 0 { return self.count; }
        if self.time_running == 0 { return 0; }
        self.count * self.time_enabled / self.time_running
    }

    pub fn increment(&mut self, delta: u64) { self.count += delta; }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PerfV3BridgeStats {
    pub total_counters: u32,
    pub hw_counters: u32,
    pub sw_counters: u32,
    pub total_overflows: u64,
}

/// Main bridge perf v3
pub struct BridgePerfV3 {
    counters: BTreeMap<u64, PerfV3Counter>,
    next_id: u64,
}

impl BridgePerfV3 {
    pub fn new() -> Self { Self { counters: BTreeMap::new(), next_id: 1 } }

    pub fn open(&mut self, etype: PerfV3EventType, cpu: i32, pid: i64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut c = PerfV3Counter::new(id, etype);
        c.cpu = cpu;
        c.pid = pid;
        self.counters.insert(id, c);
        id
    }

    pub fn read(&self, id: u64) -> u64 {
        if let Some(c) = self.counters.get(&id) { c.read() } else { 0 }
    }

    pub fn increment(&mut self, id: u64, delta: u64) {
        if let Some(c) = self.counters.get_mut(&id) { c.increment(delta); }
    }

    pub fn close(&mut self, id: u64) { self.counters.remove(&id); }

    pub fn stats(&self) -> PerfV3BridgeStats {
        let hw = self.counters.values().filter(|c| c.event_type == PerfV3EventType::Hardware).count() as u32;
        let sw = self.counters.values().filter(|c| c.event_type == PerfV3EventType::Software).count() as u32;
        let overflows: u64 = self.counters.values().map(|c| c.overflow_count).sum();
        PerfV3BridgeStats { total_counters: self.counters.len() as u32, hw_counters: hw, sw_counters: sw, total_overflows: overflows }
    }
}
