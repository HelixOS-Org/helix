//! # Apps Memory Allocator Profiler
//!
//! Heap/memory allocator behavior profiling:
//! - Allocation size distribution
//! - Allocation lifetime tracking
//! - Fragmentation analysis
//! - Slab/arena detection
//! - Memory pressure correlation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::fast::math::{F64Ext};

/// Allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocType {
    /// Small (<= 64 bytes)
    Small,
    /// Medium (64-4096)
    Medium,
    /// Large (4096-1MB)
    Large,
    /// Huge (>1MB)
    Huge,
}

/// Allocator behavior class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocBehavior {
    /// Few large, long-lived
    PoolStyle,
    /// Many small, short-lived
    Transient,
    /// Arena-like (bulk alloc/free)
    Arena,
    /// Mixed
    Mixed,
    /// Memory-hungry (growing)
    Growing,
}

/// Allocation record
#[derive(Debug, Clone)]
pub struct AllocRecord {
    /// Allocation address
    pub address: u64,
    /// Size
    pub size: u64,
    /// Timestamp (ns)
    pub alloc_ns: u64,
    /// Free timestamp (0 = still live)
    pub free_ns: u64,
    /// Callsite hash
    pub callsite_hash: u64,
}

impl AllocRecord {
    pub fn new(address: u64, size: u64, now_ns: u64, callsite: u64) -> Self {
        Self {
            address,
            size,
            alloc_ns: now_ns,
            free_ns: 0,
            callsite_hash: callsite,
        }
    }

    /// Lifetime (ns, 0 = still live)
    #[inline]
    pub fn lifetime_ns(&self) -> u64 {
        if self.free_ns > 0 {
            self.free_ns.saturating_sub(self.alloc_ns)
        } else {
            0
        }
    }

    /// Classify by size
    #[inline]
    pub fn alloc_type(&self) -> AllocType {
        match self.size {
            0..=64 => AllocType::Small,
            65..=4096 => AllocType::Medium,
            4097..=1048576 => AllocType::Large,
            _ => AllocType::Huge,
        }
    }
}

/// Callsite statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CallsiteAllocStats {
    /// Callsite hash
    pub callsite_hash: u64,
    /// Allocation count
    pub alloc_count: u64,
    /// Free count
    pub free_count: u64,
    /// Total bytes allocated
    pub total_bytes: u64,
    /// Average size
    pub avg_size: f64,
    /// Average lifetime (ns)
    pub avg_lifetime_ns: f64,
    /// Max live at once
    pub peak_live: u64,
    /// Current live
    pub current_live: u64,
}

impl CallsiteAllocStats {
    pub fn new(callsite_hash: u64) -> Self {
        Self {
            callsite_hash,
            alloc_count: 0,
            free_count: 0,
            total_bytes: 0,
            avg_size: 0.0,
            avg_lifetime_ns: 0.0,
            peak_live: 0,
            current_live: 0,
        }
    }

    #[inline]
    pub fn record_alloc(&mut self, size: u64) {
        self.alloc_count += 1;
        self.total_bytes += size;
        self.avg_size = self.total_bytes as f64 / self.alloc_count as f64;
        self.current_live += 1;
        if self.current_live > self.peak_live {
            self.peak_live = self.current_live;
        }
    }

    #[inline]
    pub fn record_free(&mut self, lifetime_ns: u64) {
        self.free_count += 1;
        self.current_live = self.current_live.saturating_sub(1);
        self.avg_lifetime_ns = 0.9 * self.avg_lifetime_ns + 0.1 * lifetime_ns as f64;
    }

    /// Leak ratio
    #[inline(always)]
    pub fn leak_ratio(&self) -> f64 {
        if self.alloc_count == 0 { return 0.0; }
        1.0 - (self.free_count as f64 / self.alloc_count as f64)
    }
}

/// Process allocator profile
#[derive(Debug)]
pub struct ProcessAllocProfile {
    /// PID
    pub pid: u64,
    /// Live allocations
    live: BTreeMap<u64, AllocRecord>,
    /// Per-callsite stats
    callsites: BTreeMap<u64, CallsiteAllocStats>,
    /// Size histogram (log2 bucket)
    size_hist: BTreeMap<u8, u64>,
    /// Total allocs
    pub total_allocs: u64,
    /// Total frees
    pub total_frees: u64,
    /// Total bytes allocated
    pub total_bytes_alloc: u64,
    /// Current live bytes
    pub current_live_bytes: u64,
    /// Peak live bytes
    pub peak_live_bytes: u64,
    /// Detected behavior
    pub behavior: AllocBehavior,
}

impl ProcessAllocProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            live: BTreeMap::new(),
            callsites: BTreeMap::new(),
            size_hist: BTreeMap::new(),
            total_allocs: 0,
            total_frees: 0,
            total_bytes_alloc: 0,
            current_live_bytes: 0,
            peak_live_bytes: 0,
            behavior: AllocBehavior::Mixed,
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, address: u64, size: u64, callsite: u64, now_ns: u64) {
        let record = AllocRecord::new(address, size, now_ns, callsite);
        self.live.insert(address, record);
        self.total_allocs += 1;
        self.total_bytes_alloc += size;
        self.current_live_bytes += size;
        if self.current_live_bytes > self.peak_live_bytes {
            self.peak_live_bytes = self.current_live_bytes;
        }

        // Size histogram
        let bucket = if size == 0 { 0 } else { (size as f64).log2() as u8 };
        *self.size_hist.entry(bucket).or_insert(0) += 1;

        // Callsite
        self.callsites.entry(callsite)
            .or_insert_with(|| CallsiteAllocStats::new(callsite))
            .record_alloc(size);

        self.detect_behavior();
    }

    /// Record free
    #[inline]
    pub fn record_free(&mut self, address: u64, now_ns: u64) {
        if let Some(record) = self.live.remove(&address) {
            self.total_frees += 1;
            self.current_live_bytes = self.current_live_bytes.saturating_sub(record.size);
            let lifetime = now_ns.saturating_sub(record.alloc_ns);
            if let Some(cs) = self.callsites.get_mut(&record.callsite_hash) {
                cs.record_free(lifetime);
            }
        }
        self.detect_behavior();
    }

    fn detect_behavior(&mut self) {
        if self.total_allocs < 100 {
            return;
        }
        let free_ratio = self.total_frees as f64 / self.total_allocs as f64;
        let avg_size = self.total_bytes_alloc as f64 / self.total_allocs as f64;

        self.behavior = if free_ratio < 0.3 {
            AllocBehavior::Growing
        } else if avg_size > 4096.0 && free_ratio > 0.8 {
            AllocBehavior::PoolStyle
        } else if avg_size < 128.0 && free_ratio > 0.9 {
            AllocBehavior::Transient
        } else {
            AllocBehavior::Mixed
        };
    }

    /// Top leaking callsites
    #[inline]
    pub fn top_leakers(&self, n: usize) -> Vec<(u64, f64)> {
        let mut leakers: Vec<(u64, f64)> = self.callsites.iter()
            .filter(|(_, cs)| cs.alloc_count > 10)
            .map(|(&hash, cs)| (hash, cs.leak_ratio()))
            .collect();
        leakers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        leakers.truncate(n);
        leakers
    }
}

/// Allocator profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppAllocProfilerStats {
    pub tracked_processes: usize,
    pub total_live_allocs: usize,
    pub total_live_bytes: u64,
    pub peak_bytes: u64,
    pub growing_processes: usize,
}

/// App allocator profiler
pub struct AppAllocProfiler {
    processes: BTreeMap<u64, ProcessAllocProfile>,
    stats: AppAllocProfilerStats,
}

impl AppAllocProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppAllocProfilerStats::default(),
        }
    }

    #[inline]
    pub fn record_alloc(&mut self, pid: u64, address: u64, size: u64, callsite: u64, now_ns: u64) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessAllocProfile::new(pid))
            .record_alloc(address, size, callsite, now_ns);
        self.update_stats();
    }

    #[inline]
    pub fn record_free(&mut self, pid: u64, address: u64, now_ns: u64) {
        if let Some(proc_profile) = self.processes.get_mut(&pid) {
            proc_profile.record_free(address, now_ns);
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_live_allocs = self.processes.values().map(|p| p.live.len()).sum();
        self.stats.total_live_bytes = self.processes.values().map(|p| p.current_live_bytes).sum();
        self.stats.peak_bytes = self.processes.values().map(|p| p.peak_live_bytes).max().unwrap_or(0);
        self.stats.growing_processes = self.processes.values()
            .filter(|p| matches!(p.behavior, AllocBehavior::Growing))
            .count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppAllocProfilerStats {
        &self.stats
    }
}
