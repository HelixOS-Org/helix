//! # Application Cache Profiling
//!
//! Per-application cache behavior analysis:
//! - Cache hit/miss tracking
//! - Cache pressure analysis
//! - Working set estimation
//! - Cache partitioning
//! - Cache pollution detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CACHE LEVEL
// ============================================================================

/// CPU cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLevel {
    /// L1 instruction cache
    L1I,
    /// L1 data cache
    L1D,
    /// L2 unified cache
    L2,
    /// L3 / last-level cache
    L3,
    /// TLB (translation lookaside buffer)
    Tlb,
    /// Branch prediction cache
    BranchPredict,
}

impl CacheLevel {
    /// Typical latency (ns)
    pub fn typical_latency_ns(&self) -> u64 {
        match self {
            Self::L1I | Self::L1D => 1,
            Self::L2 => 4,
            Self::L3 => 12,
            Self::Tlb => 2,
            Self::BranchPredict => 1,
        }
    }

    /// Main memory latency for miss
    pub fn miss_penalty_ns(&self) -> u64 {
        match self {
            Self::L1I | Self::L1D => 4,   // hits L2
            Self::L2 => 12,               // hits L3
            Self::L3 => 80,               // hits RAM
            Self::Tlb => 100,             // page walk
            Self::BranchPredict => 15,    // misprediction
        }
    }
}

/// Cache access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheAccessType {
    /// Read
    Read,
    /// Write
    Write,
    /// Prefetch
    Prefetch,
    /// Invalidate
    Invalidate,
}

// ============================================================================
// CACHE COUNTERS
// ============================================================================

/// Per-level cache counters
#[derive(Debug, Clone, Default)]
pub struct CacheLevelCounters {
    /// Total accesses
    pub accesses: u64,
    /// Hits
    pub hits: u64,
    /// Misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Prefetch hits
    pub prefetch_hits: u64,
    /// Writebacks
    pub writebacks: u64,
}

impl CacheLevelCounters {
    /// Hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.accesses == 0 {
            return 0.0;
        }
        self.hits as f64 / self.accesses as f64
    }

    /// Miss rate
    pub fn miss_rate(&self) -> f64 {
        if self.accesses == 0 {
            return 0.0;
        }
        self.misses as f64 / self.accesses as f64
    }

    /// Record hit
    pub fn record_hit(&mut self) {
        self.accesses += 1;
        self.hits += 1;
    }

    /// Record miss
    pub fn record_miss(&mut self) {
        self.accesses += 1;
        self.misses += 1;
    }
}

// ============================================================================
// WORKING SET
// ============================================================================

/// Working set estimation
#[derive(Debug, Clone)]
pub struct WorkingSetEstimate {
    /// Estimated pages
    pub pages: u64,
    /// Estimated bytes
    pub bytes: u64,
    /// Sample window (ns)
    pub window_ns: u64,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
    /// Growing/shrinking
    pub trend: WorkingSetTrend,
}

/// Working set trend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkingSetTrend {
    /// Growing
    Growing,
    /// Stable
    Stable,
    /// Shrinking
    Shrinking,
}

/// Working set tracker (page access frequency)
#[derive(Debug, Clone)]
pub struct WorkingSetTracker {
    /// Page access counts (page_id -> count)
    page_counts: BTreeMap<u64, u32>,
    /// Sampling window
    window_ns: u64,
    /// Last reset
    last_reset: u64,
    /// Previous size
    prev_size: u64,
    /// History
    history: Vec<u64>,
    /// Max history
    max_history: usize,
}

impl WorkingSetTracker {
    pub fn new(window_ns: u64) -> Self {
        Self {
            page_counts: BTreeMap::new(),
            window_ns,
            last_reset: 0,
            prev_size: 0,
            history: Vec::new(),
            max_history: 64,
        }
    }

    /// Record page access
    pub fn record_access(&mut self, page_id: u64) {
        *self.page_counts.entry(page_id).or_insert(0) += 1;
    }

    /// Estimate working set
    pub fn estimate(&mut self, now: u64) -> WorkingSetEstimate {
        let pages = self.page_counts.len() as u64;
        let bytes = pages * 4096; // assume 4K pages

        let trend = if pages > self.prev_size + self.prev_size / 10 {
            WorkingSetTrend::Growing
        } else if pages + pages / 10 < self.prev_size {
            WorkingSetTrend::Shrinking
        } else {
            WorkingSetTrend::Stable
        };

        // Confidence from sample size
        let confidence = if pages > 100 {
            1.0
        } else if pages > 10 {
            0.7
        } else {
            0.3
        };

        let elapsed = now.saturating_sub(self.last_reset);
        if elapsed >= self.window_ns {
            self.history.push(pages);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
            self.prev_size = pages;
            self.page_counts.clear();
            self.last_reset = now;
        }

        WorkingSetEstimate {
            pages,
            bytes,
            window_ns: elapsed,
            confidence,
            trend,
        }
    }

    /// Average working set size (pages)
    pub fn average_size(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<u64>() as f64 / self.history.len() as f64
    }
}

// ============================================================================
// CACHE PARTITION
// ============================================================================

/// Cache partitioning mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePartitionMode {
    /// No partitioning
    Shared,
    /// Way-based partitioning
    WayPartition,
    /// Capacity-based soft limit
    CapacitySoftLimit,
    /// Capacity-based hard limit
    CapacityHardLimit,
}

/// Cache partition
#[derive(Debug, Clone)]
pub struct CachePartition {
    /// Partition ID
    pub id: u32,
    /// Assigned process
    pub pid: u64,
    /// Cache level
    pub level: CacheLevel,
    /// Mode
    pub mode: CachePartitionMode,
    /// Allocated ways
    pub ways: u32,
    /// Capacity limit (bytes)
    pub capacity_limit: u64,
    /// Current usage (bytes)
    pub current_usage: u64,
}

impl CachePartition {
    pub fn new(id: u32, pid: u64, level: CacheLevel, mode: CachePartitionMode) -> Self {
        Self {
            id,
            pid,
            level,
            mode,
            ways: 0,
            capacity_limit: 0,
            current_usage: 0,
        }
    }

    /// Usage fraction
    pub fn usage_fraction(&self) -> f64 {
        if self.capacity_limit == 0 {
            return 0.0;
        }
        self.current_usage as f64 / self.capacity_limit as f64
    }
}

// ============================================================================
// POLLUTION DETECTOR
// ============================================================================

/// Cache pollution event
#[derive(Debug, Clone)]
pub struct PollutionEvent {
    /// Polluting PID
    pub pid: u64,
    /// Cache level
    pub level: CacheLevel,
    /// Estimated evictions caused
    pub evictions_caused: u64,
    /// Affected PIDs
    pub affected_pids: Vec<u64>,
    /// Timestamp
    pub timestamp: u64,
}

/// Cache pollution detector
#[derive(Debug, Clone)]
pub struct PollutionDetector {
    /// Eviction history per PID per level
    eviction_rates: BTreeMap<(u64, u8), Vec<u64>>,
    /// Pollution threshold
    threshold: u64,
    /// Max history
    max_history: usize,
}

impl PollutionDetector {
    pub fn new(threshold: u64) -> Self {
        Self {
            eviction_rates: BTreeMap::new(),
            threshold,
            max_history: 32,
        }
    }

    /// Record eviction rate
    pub fn record_evictions(&mut self, pid: u64, level: CacheLevel, evictions: u64) {
        let key = (pid, level as u8);
        let history = self.eviction_rates.entry(key).or_insert_with(Vec::new);
        history.push(evictions);
        if history.len() > self.max_history {
            history.remove(0);
        }
    }

    /// Detect polluters
    pub fn detect_polluters(&self, level: CacheLevel) -> Vec<u64> {
        let mut polluters = Vec::new();

        for (&(pid, lvl), history) in &self.eviction_rates {
            if lvl != level as u8 || history.is_empty() {
                continue;
            }
            let avg = history.iter().sum::<u64>() / history.len() as u64;
            if avg > self.threshold {
                polluters.push(pid);
            }
        }

        polluters
    }
}

// ============================================================================
// APP CACHE ANALYZER
// ============================================================================

/// App cache stats
#[derive(Debug, Clone, Default)]
pub struct AppCacheStats {
    /// Tracked processes
    pub process_count: usize,
    /// Total accesses
    pub total_accesses: u64,
    /// Total hits
    pub total_hits: u64,
    /// Pollution events
    pub pollution_events: u64,
    /// Active partitions
    pub partition_count: usize,
}

/// Application cache analyzer
pub struct AppCacheAnalyzer {
    /// Per-process cache counters
    counters: BTreeMap<(u64, u8), CacheLevelCounters>,
    /// Working set trackers
    working_sets: BTreeMap<u64, WorkingSetTracker>,
    /// Partitions
    partitions: BTreeMap<u32, CachePartition>,
    /// Pollution detector
    pollution_detector: PollutionDetector,
    /// Next partition ID
    next_partition_id: u32,
    /// Stats
    stats: AppCacheStats,
}

impl AppCacheAnalyzer {
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
            working_sets: BTreeMap::new(),
            partitions: BTreeMap::new(),
            pollution_detector: PollutionDetector::new(1000),
            next_partition_id: 1,
            stats: AppCacheStats::default(),
        }
    }

    /// Record cache access
    pub fn record_access(&mut self, pid: u64, level: CacheLevel, hit: bool) {
        let key = (pid, level as u8);
        let counters = self.counters.entry(key).or_insert_with(CacheLevelCounters::default);
        if hit {
            counters.record_hit();
            self.stats.total_hits += 1;
        } else {
            counters.record_miss();
        }
        self.stats.total_accesses += 1;
    }

    /// Record page access for working set
    pub fn record_page_access(&mut self, pid: u64, page_id: u64) {
        let tracker = self
            .working_sets
            .entry(pid)
            .or_insert_with(|| WorkingSetTracker::new(1_000_000_000));
        tracker.record_access(page_id);
        self.stats.process_count = self.working_sets.len();
    }

    /// Get working set estimate
    pub fn working_set(&mut self, pid: u64, now: u64) -> Option<WorkingSetEstimate> {
        self.working_sets.get_mut(&pid).map(|t| t.estimate(now))
    }

    /// Create partition
    pub fn create_partition(
        &mut self,
        pid: u64,
        level: CacheLevel,
        mode: CachePartitionMode,
    ) -> u32 {
        let id = self.next_partition_id;
        self.next_partition_id += 1;
        self.partitions
            .insert(id, CachePartition::new(id, pid, level, mode));
        self.stats.partition_count = self.partitions.len();
        id
    }

    /// Get counters for process/level
    pub fn counters(&self, pid: u64, level: CacheLevel) -> Option<&CacheLevelCounters> {
        self.counters.get(&(pid, level as u8))
    }

    /// Detect polluters at a cache level
    pub fn detect_polluters(&self, level: CacheLevel) -> Vec<u64> {
        self.pollution_detector.detect_polluters(level)
    }

    /// Stats
    pub fn stats(&self) -> &AppCacheStats {
        &self.stats
    }
}
