//! # Apps IO Profiler
//!
//! Per-application I/O behavior profiling:
//! - I/O pattern classification (sequential, random, mixed)
//! - Read/write ratio analysis
//! - I/O size distribution
//! - Latency percentile tracking
//! - Device-level attribution
//! - I/O merging effectiveness

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// I/O direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDirection {
    Read,
    Write,
    Discard,
    Flush,
}

/// I/O pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPattern {
    Sequential,
    Random,
    Strided,
    Mixed,
}

/// I/O priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPrioClass {
    Idle,
    BestEffort,
    RealTime,
    None,
}

/// Single I/O record
#[derive(Debug, Clone)]
pub struct IoRecord {
    pub ts: u64,
    pub direction: IoDirection,
    pub offset: u64,
    pub size: u32,
    pub latency_ns: u64,
    pub device_id: u64,
    pub merged: bool,
}

/// Size distribution bucket
#[derive(Debug, Clone, Default)]
pub struct SizeBucket {
    pub le_512: u64,
    pub le_4k: u64,
    pub le_16k: u64,
    pub le_64k: u64,
    pub le_256k: u64,
    pub le_1m: u64,
    pub gt_1m: u64,
}

impl SizeBucket {
    #[inline]
    pub fn record(&mut self, size: u32) {
        match size {
            0..=512 => self.le_512 += 1,
            513..=4096 => self.le_4k += 1,
            4097..=16384 => self.le_16k += 1,
            16385..=65536 => self.le_64k += 1,
            65537..=262144 => self.le_256k += 1,
            262145..=1048576 => self.le_1m += 1,
            _ => self.gt_1m += 1,
        }
    }

    #[inline(always)]
    pub fn total(&self) -> u64 { self.le_512 + self.le_4k + self.le_16k + self.le_64k + self.le_256k + self.le_1m + self.gt_1m }
}

/// Latency percentile tracker (streaming approximation)
#[derive(Debug, Clone)]
pub struct LatencyTracker {
    pub samples: Vec<u64>,
    pub sorted: bool,
    pub count: u64,
    pub sum: u64,
    pub min: u64,
    pub max: u64,
    max_samples: usize,
}

impl LatencyTracker {
    pub fn new(max: usize) -> Self {
        Self { samples: Vec::new(), sorted: false, count: 0, sum: 0, min: u64::MAX, max: 0, max_samples: max }
    }

    #[inline]
    pub fn record(&mut self, ns: u64) {
        self.count += 1; self.sum += ns;
        if ns < self.min { self.min = ns; }
        if ns > self.max { self.max = ns; }
        if self.samples.len() < self.max_samples { self.samples.push(ns); self.sorted = false; }
    }

    fn ensure_sorted(&mut self) { if !self.sorted { self.samples.sort(); self.sorted = true; } }

    #[inline(always)]
    pub fn avg(&self) -> u64 { if self.count == 0 { 0 } else { self.sum / self.count } }

    #[inline]
    pub fn percentile(&mut self, p: f64) -> u64 {
        if self.samples.is_empty() { return 0; }
        self.ensure_sorted();
        let idx = ((p / 100.0) * (self.samples.len() - 1) as f64) as usize;
        self.samples[idx.min(self.samples.len() - 1)]
    }
}

/// Per-process I/O profile
#[derive(Debug, Clone)]
pub struct ProcessIoProfile {
    pub pid: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ios: u64,
    pub write_ios: u64,
    pub pattern: IoPattern,
    pub size_dist: SizeBucket,
    pub latency: LatencyTracker,
    pub last_offset: u64,
    pub sequential_count: u64,
    pub random_count: u64,
    pub prio: IoPrioClass,
    pub merges: u64,
}

impl ProcessIoProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, read_bytes: 0, write_bytes: 0, read_ios: 0, write_ios: 0,
            pattern: IoPattern::Mixed, size_dist: SizeBucket::default(),
            latency: LatencyTracker::new(1024), last_offset: 0,
            sequential_count: 0, random_count: 0,
            prio: IoPrioClass::BestEffort, merges: 0,
        }
    }

    pub fn record(&mut self, rec: &IoRecord) {
        match rec.direction {
            IoDirection::Read => { self.read_ios += 1; self.read_bytes += rec.size as u64; }
            IoDirection::Write => { self.write_ios += 1; self.write_bytes += rec.size as u64; }
            _ => {}
        }
        self.size_dist.record(rec.size);
        self.latency.record(rec.latency_ns);
        if rec.merged { self.merges += 1; }

        let diff = if rec.offset >= self.last_offset { rec.offset - self.last_offset } else { self.last_offset - rec.offset };
        if diff <= rec.size as u64 * 2 { self.sequential_count += 1; } else { self.random_count += 1; }
        self.last_offset = rec.offset + rec.size as u64;
        self.update_pattern();
    }

    fn update_pattern(&mut self) {
        let total = self.sequential_count + self.random_count;
        if total == 0 { return; }
        let seq_pct = self.sequential_count * 100 / total;
        self.pattern = match seq_pct {
            80..=100 => IoPattern::Sequential,
            0..=20 => IoPattern::Random,
            _ => IoPattern::Mixed,
        };
    }

    #[inline(always)]
    pub fn rw_ratio(&self) -> f64 { if self.write_ios == 0 { f64::MAX } else { self.read_ios as f64 / self.write_ios as f64 } }
    #[inline(always)]
    pub fn total_ios(&self) -> u64 { self.read_ios + self.write_ios }
    #[inline(always)]
    pub fn total_bytes(&self) -> u64 { self.read_bytes + self.write_bytes }
    #[inline(always)]
    pub fn avg_size(&self) -> u64 { let t = self.total_ios(); if t == 0 { 0 } else { self.total_bytes() / t } }
    #[inline(always)]
    pub fn merge_pct(&self) -> f64 { let t = self.total_ios(); if t == 0 { 0.0 } else { self.merges as f64 / t as f64 * 100.0 } }
}

/// IO profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct IoProfilerStats {
    pub tracked_processes: usize,
    pub total_ios: u64,
    pub total_bytes: u64,
    pub avg_latency_ns: u64,
    pub sequential_pct: f64,
    pub total_merges: u64,
}

/// Apps IO profiler
pub struct AppsIoProfiler {
    profiles: BTreeMap<u64, ProcessIoProfile>,
    stats: IoProfilerStats,
}

impl AppsIoProfiler {
    pub fn new() -> Self { Self { profiles: BTreeMap::new(), stats: IoProfilerStats::default() } }

    #[inline(always)]
    pub fn track(&mut self, pid: u64) { self.profiles.entry(pid).or_insert_with(|| ProcessIoProfile::new(pid)); }
    #[inline(always)]
    pub fn untrack(&mut self, pid: u64) { self.profiles.remove(&pid); }

    #[inline(always)]
    pub fn record(&mut self, pid: u64, rec: &IoRecord) {
        let p = self.profiles.entry(pid).or_insert_with(|| ProcessIoProfile::new(pid));
        p.record(rec);
    }

    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.profiles.len();
        self.stats.total_ios = self.profiles.values().map(|p| p.total_ios()).sum();
        self.stats.total_bytes = self.profiles.values().map(|p| p.total_bytes()).sum();
        self.stats.total_merges = self.profiles.values().map(|p| p.merges).sum();
        if !self.profiles.is_empty() {
            self.stats.avg_latency_ns = self.profiles.values().map(|p| p.latency.avg()).sum::<u64>() / self.profiles.len() as u64;
            let total_seq: u64 = self.profiles.values().map(|p| p.sequential_count).sum();
            let total_ops: u64 = self.profiles.values().map(|p| p.sequential_count + p.random_count).sum();
            self.stats.sequential_pct = if total_ops == 0 { 0.0 } else { total_seq as f64 / total_ops as f64 * 100.0 };
        }
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessIoProfile> { self.profiles.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &IoProfilerStats { &self.stats }
}
