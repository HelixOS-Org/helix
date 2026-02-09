//! # Apps IO Scheduler Bridge
//!
//! Application I/O scheduling and prioritization:
//! - Per-app I/O bandwidth tracking
//! - I/O priority class assignment (RT/BE/Idle)
//! - Read-ahead tuning per workload pattern
//! - I/O merge and coalesce tracking
//! - Deadline and fairness enforcement
//! - Device queue depth management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// I/O priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppIoClass {
    Idle,
    BestEffort,
    Realtime,
}

/// I/O direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDirection {
    Read,
    Write,
    Discard,
    Flush,
}

/// I/O request entry
#[derive(Debug, Clone)]
pub struct AppIoRequest {
    pub id: u64,
    pub process_id: u64,
    pub direction: IoDirection,
    pub offset: u64,
    pub size: u64,
    pub io_class: AppIoClass,
    pub io_prio: u8,
    pub submit_ns: u64,
    pub complete_ns: u64,
    pub merged: bool,
    pub device_id: u32,
}

impl AppIoRequest {
    pub fn new(id: u64, pid: u64, dir: IoDirection, offset: u64, size: u64, ts: u64) -> Self {
        Self {
            id, process_id: pid, direction: dir, offset, size,
            io_class: AppIoClass::BestEffort, io_prio: 4,
            submit_ns: ts, complete_ns: 0, merged: false, device_id: 0,
        }
    }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 {
        if self.complete_ns > self.submit_ns { self.complete_ns - self.submit_ns } else { 0 }
    }

    #[inline]
    pub fn is_sequential_after(&self, prev: &AppIoRequest) -> bool {
        self.direction == prev.direction
            && self.device_id == prev.device_id
            && self.offset == prev.offset + prev.size
    }
}

/// Per-app I/O bandwidth tracker
#[derive(Debug, Clone)]
pub struct AppIoBandwidth {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub read_merged: u64,
    pub write_merged: u64,
    pub window_start_ns: u64,
    pub window_duration_ns: u64,
}

impl AppIoBandwidth {
    pub fn new(ts: u64, window: u64) -> Self {
        Self {
            read_bytes: 0, write_bytes: 0, read_ops: 0, write_ops: 0,
            read_merged: 0, write_merged: 0,
            window_start_ns: ts, window_duration_ns: window,
        }
    }

    #[inline(always)]
    pub fn read_bw_bps(&self) -> f64 {
        if self.window_duration_ns == 0 { return 0.0; }
        self.read_bytes as f64 / (self.window_duration_ns as f64 / 1_000_000_000.0)
    }

    #[inline(always)]
    pub fn write_bw_bps(&self) -> f64 {
        if self.window_duration_ns == 0 { return 0.0; }
        self.write_bytes as f64 / (self.window_duration_ns as f64 / 1_000_000_000.0)
    }

    #[inline(always)]
    pub fn total_iops(&self) -> f64 {
        if self.window_duration_ns == 0 { return 0.0; }
        (self.read_ops + self.write_ops) as f64 / (self.window_duration_ns as f64 / 1_000_000_000.0)
    }

    #[inline]
    pub fn merge_ratio(&self) -> f64 {
        let total = self.read_ops + self.write_ops;
        if total == 0 { return 0.0; }
        (self.read_merged + self.write_merged) as f64 / total as f64
    }
}

/// Read-ahead configuration
#[derive(Debug, Clone)]
pub struct ReadAheadConfig {
    pub pages: u32,
    pub async_pages: u32,
    pub adaptive: bool,
    pub sequential_threshold: u32,
    pub hit_rate: f64,
    pub miss_count: u64,
    pub hit_count: u64,
}

impl ReadAheadConfig {
    pub fn new(pages: u32) -> Self {
        Self {
            pages, async_pages: pages / 4,
            adaptive: true, sequential_threshold: 4,
            hit_rate: 0.0, miss_count: 0, hit_count: 0,
        }
    }

    #[inline(always)]
    pub fn record_hit(&mut self) {
        self.hit_count += 1;
        self.update_rate();
    }

    #[inline(always)]
    pub fn record_miss(&mut self) {
        self.miss_count += 1;
        self.update_rate();
    }

    fn update_rate(&mut self) {
        let total = self.hit_count + self.miss_count;
        if total > 0 { self.hit_rate = self.hit_count as f64 / total as f64; }
    }

    #[inline]
    pub fn adapt(&mut self) {
        if !self.adaptive { return; }
        if self.hit_rate > 0.8 && self.pages < 256 {
            self.pages *= 2;
        } else if self.hit_rate < 0.3 && self.pages > 4 {
            self.pages /= 2;
        }
        self.async_pages = self.pages / 4;
    }
}

/// Per-app I/O scheduling state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppIoSchedState {
    pub process_id: u64,
    pub io_class: AppIoClass,
    pub io_nice: u8,
    pub bandwidth: AppIoBandwidth,
    pub readahead: ReadAheadConfig,
    pub pending_requests: Vec<AppIoRequest>,
    pub latency_sum_ns: u64,
    pub latency_count: u64,
    pub max_latency_ns: u64,
    pub sequential_ratio: f64,
}

impl AppIoSchedState {
    pub fn new(pid: u64, ts: u64) -> Self {
        Self {
            process_id: pid,
            io_class: AppIoClass::BestEffort,
            io_nice: 4,
            bandwidth: AppIoBandwidth::new(ts, 1_000_000_000),
            readahead: ReadAheadConfig::new(32),
            pending_requests: Vec::new(),
            latency_sum_ns: 0,
            latency_count: 0,
            max_latency_ns: 0,
            sequential_ratio: 0.0,
        }
    }

    #[inline]
    pub fn submit_io(&mut self, req: AppIoRequest) {
        match req.direction {
            IoDirection::Read => { self.bandwidth.read_ops += 1; self.bandwidth.read_bytes += req.size; }
            IoDirection::Write => { self.bandwidth.write_ops += 1; self.bandwidth.write_bytes += req.size; }
            _ => {}
        }
        self.pending_requests.push(req);
    }

    #[inline]
    pub fn complete_io(&mut self, id: u64, ts: u64) {
        if let Some(idx) = self.pending_requests.iter().position(|r| r.id == id) {
            let mut req = self.pending_requests.remove(idx);
            req.complete_ns = ts;
            let lat = req.latency_ns();
            self.latency_sum_ns += lat;
            self.latency_count += 1;
            if lat > self.max_latency_ns { self.max_latency_ns = lat; }
        }
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> u64 {
        if self.latency_count == 0 { 0 } else { self.latency_sum_ns / self.latency_count }
    }
}

/// Apps I/O scheduler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppsIoSchedStats {
    pub total_processes: usize,
    pub total_pending: usize,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
    pub avg_latency_ns: u64,
}

/// Apps IO Scheduler Bridge
pub struct AppsIoSchedBridge {
    states: BTreeMap<u64, AppIoSchedState>,
    stats: AppsIoSchedStats,
    next_io_id: u64,
}

impl AppsIoSchedBridge {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            stats: AppsIoSchedStats::default(),
            next_io_id: 1,
        }
    }

    #[inline(always)]
    pub fn register(&mut self, pid: u64, ts: u64) {
        self.states.entry(pid).or_insert_with(|| AppIoSchedState::new(pid, ts));
    }

    #[inline]
    pub fn set_io_priority(&mut self, pid: u64, class: AppIoClass, nice: u8) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.io_class = class;
            state.io_nice = nice;
        }
    }

    #[inline]
    pub fn submit(&mut self, pid: u64, dir: IoDirection, offset: u64, size: u64, ts: u64) -> u64 {
        let id = self.next_io_id;
        self.next_io_id += 1;
        let req = AppIoRequest::new(id, pid, dir, offset, size, ts);
        if let Some(state) = self.states.get_mut(&pid) { state.submit_io(req); }
        id
    }

    #[inline(always)]
    pub fn complete(&mut self, pid: u64, io_id: u64, ts: u64) {
        if let Some(state) = self.states.get_mut(&pid) { state.complete_io(io_id, ts); }
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) { self.states.remove(&pid); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.states.len();
        self.stats.total_pending = self.states.values().map(|s| s.pending_requests.len()).sum();
        self.stats.total_read_bytes = self.states.values().map(|s| s.bandwidth.read_bytes).sum();
        self.stats.total_write_bytes = self.states.values().map(|s| s.bandwidth.write_bytes).sum();
        let total_lat: u64 = self.states.values().map(|s| s.latency_sum_ns).sum();
        let total_count: u64 = self.states.values().map(|s| s.latency_count).sum();
        self.stats.avg_latency_ns = if total_count > 0 { total_lat / total_count } else { 0 };
    }

    #[inline(always)]
    pub fn app_state(&self, pid: u64) -> Option<&AppIoSchedState> { self.states.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &AppsIoSchedStats { &self.stats }
}
