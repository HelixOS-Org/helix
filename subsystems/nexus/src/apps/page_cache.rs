//! # Application Page Cache Profiler
//!
//! Per-process page cache behavior analysis:
//! - Working set estimation via access tracking
//! - Page fault pattern analysis (major/minor)
//! - Eviction tracking and thrashing detection
//! - Read-ahead effectiveness monitoring
//! - Dirty page ratio management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// PAGE CACHE TYPES
// ============================================================================

/// Page fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFaultType {
    /// Minor fault (page in memory but not mapped)
    Minor,
    /// Major fault (page on disk)
    Major,
    /// Copy-on-write
    CopyOnWrite,
    /// Protection fault
    Protection,
    /// Stack growth
    StackGrowth,
}

/// Page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    /// Clean cached
    Clean,
    /// Dirty (modified)
    Dirty,
    /// Under writeback
    Writeback,
    /// Being read in
    ReadAhead,
    /// Evicted
    Evicted,
}

/// Access pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    /// Sequential reads
    Sequential,
    /// Random access
    Random,
    /// Strided access
    Strided,
    /// Temporal locality
    Temporal,
    /// Unknown
    Unknown,
}

// ============================================================================
// PAGE TRACKING
// ============================================================================

/// Page entry in cache
#[derive(Debug, Clone)]
pub struct CachedPage {
    /// Page frame number
    pub pfn: u64,
    /// File inode (0 for anonymous)
    pub inode: u64,
    /// Offset in file (pages)
    pub offset: u64,
    /// State
    pub state: PageState,
    /// Access count
    pub access_count: u32,
    /// Last access timestamp
    pub last_access: u64,
    /// First access timestamp
    pub first_access: u64,
    /// Is read-ahead page
    pub is_readahead: bool,
    /// Was read-ahead useful (accessed before eviction)
    pub readahead_useful: bool,
}

impl CachedPage {
    pub fn new(pfn: u64, inode: u64, offset: u64, now: u64) -> Self {
        Self {
            pfn,
            inode,
            offset,
            state: PageState::Clean,
            access_count: 1,
            last_access: now,
            first_access: now,
            is_readahead: false,
            readahead_useful: false,
        }
    }

    /// Record access
    pub fn touch(&mut self, now: u64) {
        self.access_count += 1;
        self.last_access = now;
        if self.is_readahead && !self.readahead_useful {
            self.readahead_useful = true;
        }
    }

    /// Mark dirty
    pub fn mark_dirty(&mut self) {
        self.state = PageState::Dirty;
    }

    /// Age (ns)
    pub fn age_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.first_access)
    }
}

// ============================================================================
// FAULT HISTORY
// ============================================================================

/// Page fault record
#[derive(Debug, Clone)]
pub struct PageFaultRecord {
    /// Fault type
    pub fault_type: PageFaultType,
    /// Faulting address (page-aligned)
    pub address: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Resolution latency (ns)
    pub latency_ns: u64,
}

/// Fault histogram by latency bucket
#[derive(Debug, Clone)]
pub struct FaultLatencyHistogram {
    /// Bucket boundaries (ns)
    boundaries: Vec<u64>,
    /// Counts per bucket
    counts: Vec<u64>,
}

impl FaultLatencyHistogram {
    pub fn new() -> Self {
        // Buckets: 0-1us, 1-10us, 10-100us, 100us-1ms, 1-10ms, 10-100ms, >100ms
        Self {
            boundaries: alloc::vec![1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000],
            counts: alloc::vec![0; 7],
        }
    }

    /// Record a latency sample
    pub fn record(&mut self, latency_ns: u64) {
        let idx = self
            .boundaries
            .iter()
            .position(|&b| latency_ns < b)
            .unwrap_or(self.counts.len() - 1);
        self.counts[idx] += 1;
    }

    /// Total samples
    pub fn total(&self) -> u64 {
        self.counts.iter().sum()
    }

    /// P50 bucket index
    pub fn p50_bucket(&self) -> usize {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        let target = total / 2;
        let mut acc = 0u64;
        for (i, &c) in self.counts.iter().enumerate() {
            acc += c;
            if acc >= target {
                return i;
            }
        }
        self.counts.len() - 1
    }
}

// ============================================================================
// WORKING SET ESTIMATION
// ============================================================================

/// Working set estimator using access tracking
#[derive(Debug)]
pub struct WorkingSetEstimator {
    /// Active pages (recently accessed)
    active_pages: u64,
    /// Inactive pages
    inactive_pages: u64,
    /// Recent window sizes (for trend)
    history: Vec<u64>,
    /// Max history
    max_history: usize,
    /// Estimated working set in pages
    pub estimated_pages: u64,
    /// EMA alpha
    alpha: f64,
}

impl WorkingSetEstimator {
    pub fn new() -> Self {
        Self {
            active_pages: 0,
            inactive_pages: 0,
            history: Vec::new(),
            max_history: 32,
            estimated_pages: 0,
            alpha: 0.3,
        }
    }

    /// Update working set estimate
    pub fn update(&mut self, active: u64, inactive: u64) {
        self.active_pages = active;
        self.inactive_pages = inactive;

        let new_estimate = active;
        if self.estimated_pages == 0 {
            self.estimated_pages = new_estimate;
        } else {
            let ema =
                self.alpha * new_estimate as f64 + (1.0 - self.alpha) * self.estimated_pages as f64;
            self.estimated_pages = ema as u64;
        }

        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(self.estimated_pages);
    }

    /// Is working set growing?
    pub fn is_growing(&self) -> bool {
        if self.history.len() < 4 {
            return false;
        }
        let recent = &self.history[self.history.len() - 4..];
        recent[3] > recent[0] && recent[2] > recent[1]
    }

    /// Estimated bytes (4K pages)
    pub fn estimated_bytes(&self) -> u64 {
        self.estimated_pages * 4096
    }
}

// ============================================================================
// THRASHING DETECTOR
// ============================================================================

/// Thrashing detector
#[derive(Debug)]
pub struct ThrashingDetector {
    /// Recent eviction then re-fault count
    refault_count: u64,
    /// Total faults in window
    window_faults: u64,
    /// Window start time
    window_start: u64,
    /// Window duration (ns)
    window_ns: u64,
    /// Thrashing threshold (refault ratio)
    threshold: f64,
    /// Is thrashing detected
    pub is_thrashing: bool,
    /// Recently evicted pages (pfn -> eviction time)
    evicted: BTreeMap<u64, u64>,
    /// Max tracked evictions
    max_evicted: usize,
}

impl ThrashingDetector {
    pub fn new(window_ns: u64) -> Self {
        Self {
            refault_count: 0,
            window_faults: 0,
            window_start: 0,
            window_ns,
            threshold: 0.25,
            is_thrashing: false,
            evicted: BTreeMap::new(),
            max_evicted: 4096,
        }
    }

    /// Record eviction
    pub fn on_eviction(&mut self, pfn: u64, now: u64) {
        if self.evicted.len() >= self.max_evicted {
            // Remove oldest
            if let Some(&oldest_key) = self.evicted.keys().next() {
                self.evicted.remove(&oldest_key);
            }
        }
        self.evicted.insert(pfn, now);
    }

    /// Record fault — check if it's a refault
    pub fn on_fault(&mut self, pfn: u64, now: u64) {
        self.maybe_reset_window(now);
        self.window_faults += 1;

        if self.evicted.remove(&pfn).is_some() {
            self.refault_count += 1;
        }

        self.update_thrashing();
    }

    fn maybe_reset_window(&mut self, now: u64) {
        if now.saturating_sub(self.window_start) > self.window_ns {
            self.refault_count = 0;
            self.window_faults = 0;
            self.window_start = now;
        }
    }

    fn update_thrashing(&mut self) {
        if self.window_faults < 10 {
            self.is_thrashing = false;
            return;
        }
        let ratio = self.refault_count as f64 / self.window_faults as f64;
        self.is_thrashing = ratio > self.threshold;
    }

    /// Refault ratio
    pub fn refault_ratio(&self) -> f64 {
        if self.window_faults == 0 {
            return 0.0;
        }
        self.refault_count as f64 / self.window_faults as f64
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Per-process page cache stats
#[derive(Debug, Clone, Default)]
pub struct ProcessPageCacheStats {
    /// Total cached pages
    pub cached_pages: u64,
    /// Dirty pages
    pub dirty_pages: u64,
    /// Major faults
    pub major_faults: u64,
    /// Minor faults
    pub minor_faults: u64,
    /// Read-ahead hits
    pub readahead_hits: u64,
    /// Read-ahead misses
    pub readahead_misses: u64,
}

/// App page cache stats
#[derive(Debug, Clone, Default)]
pub struct AppPageCacheStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total cached pages
    pub total_cached: u64,
    /// Total dirty pages
    pub total_dirty: u64,
    /// Thrashing processes
    pub thrashing_count: usize,
}

/// App page cache profiler
pub struct AppPageCacheProfiler {
    /// Per-process cached pages: pid -> pages
    pages: BTreeMap<u64, Vec<CachedPage>>,
    /// Per-process fault histogram
    fault_histograms: BTreeMap<u64, FaultLatencyHistogram>,
    /// Per-process working set
    working_sets: BTreeMap<u64, WorkingSetEstimator>,
    /// Per-process thrashing
    thrashing: BTreeMap<u64, ThrashingDetector>,
    /// Stats
    stats: AppPageCacheStats,
}

impl AppPageCacheProfiler {
    pub fn new() -> Self {
        Self {
            pages: BTreeMap::new(),
            fault_histograms: BTreeMap::new(),
            working_sets: BTreeMap::new(),
            thrashing: BTreeMap::new(),
            stats: AppPageCacheStats::default(),
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64) {
        self.pages.insert(pid, Vec::new());
        self.fault_histograms
            .insert(pid, FaultLatencyHistogram::new());
        self.working_sets.insert(pid, WorkingSetEstimator::new());
        // 1 second window for thrashing detection
        self.thrashing
            .insert(pid, ThrashingDetector::new(1_000_000_000));
        self.update_stats();
    }

    /// Record page fault
    pub fn on_fault(&mut self, pid: u64, fault: PageFaultRecord) {
        if let Some(hist) = self.fault_histograms.get_mut(&pid) {
            hist.record(fault.latency_ns);
        }
        if let Some(thrash) = self.thrashing.get_mut(&pid) {
            thrash.on_fault(fault.address >> 12, fault.timestamp);
        }
        self.update_stats();
    }

    /// Record page eviction
    pub fn on_eviction(&mut self, pid: u64, pfn: u64, now: u64) {
        if let Some(thrash) = self.thrashing.get_mut(&pid) {
            thrash.on_eviction(pfn, now);
        }
        // Remove from cache
        if let Some(pages) = self.pages.get_mut(&pid) {
            pages.retain(|p| p.pfn != pfn);
        }
        self.update_stats();
    }

    /// Add cached page
    pub fn add_page(&mut self, pid: u64, page: CachedPage) {
        if let Some(pages) = self.pages.get_mut(&pid) {
            pages.push(page);
        }
        self.update_stats();
    }

    /// Update working set estimate
    pub fn update_working_set(&mut self, pid: u64, active: u64, inactive: u64) {
        if let Some(ws) = self.working_sets.get_mut(&pid) {
            ws.update(active, inactive);
        }
    }

    /// Remove process
    pub fn remove(&mut self, pid: u64) {
        self.pages.remove(&pid);
        self.fault_histograms.remove(&pid);
        self.working_sets.remove(&pid);
        self.thrashing.remove(&pid);
        self.update_stats();
    }

    /// Process stats
    pub fn process_stats(&self, pid: u64) -> Option<ProcessPageCacheStats> {
        let pages = self.pages.get(&pid)?;
        let dirty = pages.iter().filter(|p| p.state == PageState::Dirty).count() as u64;
        let ra_total = pages.iter().filter(|p| p.is_readahead).count() as u64;
        let ra_hits = pages
            .iter()
            .filter(|p| p.is_readahead && p.readahead_useful)
            .count() as u64;
        Some(ProcessPageCacheStats {
            cached_pages: pages.len() as u64,
            dirty_pages: dirty,
            major_faults: 0,
            minor_faults: 0,
            readahead_hits: ra_hits,
            readahead_misses: ra_total.saturating_sub(ra_hits),
        })
    }

    /// Is process thrashing?
    pub fn is_thrashing(&self, pid: u64) -> bool {
        self.thrashing.get(&pid).map_or(false, |t| t.is_thrashing)
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.pages.len();
        self.stats.total_cached = self.pages.values().map(|v| v.len() as u64).sum();
        self.stats.total_dirty = self
            .pages
            .values()
            .flat_map(|v| v.iter())
            .filter(|p| p.state == PageState::Dirty)
            .count() as u64;
        self.stats.thrashing_count = self.thrashing.values().filter(|t| t.is_thrashing).count();
    }

    /// Stats
    pub fn stats(&self) -> &AppPageCacheStats {
        &self.stats
    }
}
