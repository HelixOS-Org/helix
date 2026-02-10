//! # Application Memory Analysis
//!
//! Deep memory behavior analysis per application:
//! - Working set estimation
//! - Access pattern classification (sequential, random, strided)
//! - Page hotness tracking
//! - Memory allocation pattern analysis
//! - Cache behavior prediction
//! - NUMA affinity detection

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::fast::linear_map::LinearMap;
use crate::fast::math::F64Ext;

// ============================================================================
// WORKING SET ESTIMATION
// ============================================================================

/// Working set estimator using page access tracking
#[derive(Debug)]
pub struct WorkingSetEstimator {
    /// Page access counts (page_number → access_count)
    page_accesses: LinearMap<u32, 64>,
    /// Window samples (timestamp → working_set_pages)
    samples: Vec<(u64, u64)>,
    /// Max samples
    max_samples: usize,
    /// Current working set estimate (pages)
    pub current_wss: u64,
    /// Peak working set (pages)
    pub peak_wss: u64,
    /// Page size (bytes)
    pub page_size: u64,
    /// Decay counter (for aging)
    decay_counter: u32,
    /// Decay interval
    decay_interval: u32,
}

impl WorkingSetEstimator {
    pub fn new(page_size: u64) -> Self {
        Self {
            page_accesses: LinearMap::new(),
            samples: Vec::new(),
            max_samples: 300,
            current_wss: 0,
            peak_wss: 0,
            page_size,
            decay_counter: 0,
            decay_interval: 100,
        }
    }

    /// Record a page access
    #[inline]
    pub fn record_access(&mut self, page_number: u64) {
        self.page_accesses.add(page_number, 1);

        self.decay_counter += 1;
        if self.decay_counter >= self.decay_interval {
            self.decay();
            self.decay_counter = 0;
        }
    }

    /// Decay old access counts
    fn decay(&mut self) {
        let mut to_remove = Vec::new();
        let keys: Vec<u64> = self.page_accesses.keys().collect();
        for page in keys {
            if let Some(count) = self.page_accesses.get(page) {
                let new_count = count / 2;
                if new_count == 0 {
                    to_remove.push(page);
                } else {
                    self.page_accesses.insert(page, new_count);
                }
            }
        }
        for page in to_remove {
            self.page_accesses.remove(page);
        }
    }

    /// Estimate current working set
    pub fn estimate(&mut self, timestamp: u64) -> u64 {
        // Pages with access count > 0 are in the working set
        self.current_wss = self.page_accesses.len() as u64;
        if self.current_wss > self.peak_wss {
            self.peak_wss = self.current_wss;
        }

        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push((timestamp, self.current_wss));

        self.current_wss
    }

    /// Working set in bytes
    #[inline(always)]
    pub fn wss_bytes(&self) -> u64 {
        self.current_wss * self.page_size
    }

    /// Working set trend
    pub fn trend(&self) -> f64 {
        if self.samples.len() < 10 {
            return 0.0;
        }
        let n = self.samples.len();
        let first: f64 = self.samples[..n / 2]
            .iter()
            .map(|(_, w)| *w as f64)
            .sum::<f64>()
            / (n / 2) as f64;
        let second: f64 = self.samples[n / 2..]
            .iter()
            .map(|(_, w)| *w as f64)
            .sum::<f64>()
            / (n - n / 2) as f64;
        if first < 1.0 {
            0.0
        } else {
            (second - first) / first
        }
    }

    /// Hot pages (most accessed)
    #[inline]
    pub fn hot_pages(&self, n: usize) -> Vec<(u64, u32)> {
        let mut pages: Vec<(u64, u32)> = self.page_accesses.iter().map(|(p, c)| (p, c)).collect();
        pages.sort_by(|a, b| b.1.cmp(&a.1));
        pages.truncate(n);
        pages
    }
}

// ============================================================================
// ACCESS PATTERN CLASSIFICATION
// ============================================================================

/// Memory access pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    /// Sequential (addresses increase linearly)
    Sequential,
    /// Reverse sequential
    ReverseSequential,
    /// Random access
    Random,
    /// Strided (fixed stride between accesses)
    Strided(u64),
    /// Clustered (accesses group in regions)
    Clustered,
    /// Ping-pong (alternating between two regions)
    PingPong,
    /// Unknown / insufficient data
    Unknown,
}

/// Access pattern detector
#[derive(Debug)]
pub struct AccessPatternDetector {
    /// Recent access addresses
    recent_addresses: Vec<u64>,
    /// Max addresses to track
    max_addresses: usize,
    /// Detected pattern
    pub pattern: AccessPattern,
    /// Pattern confidence
    pub confidence: f64,
    /// Detected stride (if strided)
    pub stride: Option<u64>,
}

impl AccessPatternDetector {
    pub fn new(window_size: usize) -> Self {
        Self {
            recent_addresses: Vec::new(),
            max_addresses: window_size,
            pattern: AccessPattern::Unknown,
            confidence: 0.0,
            stride: None,
        }
    }

    /// Record an access
    #[inline]
    pub fn record(&mut self, address: u64) {
        if self.recent_addresses.len() >= self.max_addresses {
            self.recent_addresses.remove(0);
        }
        self.recent_addresses.push(address);

        if self.recent_addresses.len() >= 10 {
            self.classify();
        }
    }

    fn classify(&mut self) {
        let addrs = &self.recent_addresses;
        let n = addrs.len();

        // Check for sequential access
        let mut sequential_count = 0;
        let mut reverse_count = 0;
        for i in 1..n {
            if addrs[i] > addrs[i - 1] && addrs[i] - addrs[i - 1] <= 4096 {
                sequential_count += 1;
            }
            if addrs[i] < addrs[i - 1] && addrs[i - 1] - addrs[i] <= 4096 {
                reverse_count += 1;
            }
        }

        let total = (n - 1) as f64;
        let seq_ratio = sequential_count as f64 / total;
        let rev_ratio = reverse_count as f64 / total;

        if seq_ratio > 0.8 {
            self.pattern = AccessPattern::Sequential;
            self.confidence = seq_ratio;
            return;
        }

        if rev_ratio > 0.8 {
            self.pattern = AccessPattern::ReverseSequential;
            self.confidence = rev_ratio;
            return;
        }

        // Check for strided access
        if n >= 4 {
            let strides: Vec<i64> = addrs
                .windows(2)
                .map(|w| w[1] as i64 - w[0] as i64)
                .collect();
            let first_stride = strides[0];
            let stride_match = strides.iter().filter(|&&s| s == first_stride).count();
            let stride_ratio = stride_match as f64 / strides.len() as f64;

            if stride_ratio > 0.7 && first_stride != 0 {
                let abs_stride = if first_stride >= 0 {
                    first_stride as u64
                } else {
                    (-first_stride) as u64
                };
                self.pattern = AccessPattern::Strided(abs_stride);
                self.stride = Some(abs_stride);
                self.confidence = stride_ratio;
                return;
            }
        }

        // Check for clustering
        if self.is_clustered(addrs) {
            self.pattern = AccessPattern::Clustered;
            self.confidence = 0.7;
            return;
        }

        // Default: random
        self.pattern = AccessPattern::Random;
        self.confidence = 1.0 - seq_ratio - rev_ratio;
    }

    fn is_clustered(&self, addrs: &[u64]) -> bool {
        // Check if accesses cluster in a few page ranges
        let mut page_set: LinearMap<u32, 64> = LinearMap::new();
        for &addr in addrs {
            *page_set.entry(addr / 4096).or_insert(0) += 1;
        }

        // If less than 20% of pages account for 80% of accesses → clustered
        let total_accesses = addrs.len() as u32;
        let mut counts: Vec<u32> = page_set.values().collect();
        counts.sort_by(|a, b| b.cmp(a));

        let top_20_pct = (page_set.len() as f64 * 0.2).ceil() as usize;
        let top_count: u32 = counts.iter().take(top_20_pct.max(1)).sum();

        top_count as f64 / total_accesses as f64 > 0.8
    }
}

// ============================================================================
// ALLOCATION PATTERN ANALYZER
// ============================================================================

/// Allocation size class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocSizeClass {
    /// Tiny (≤ 64 bytes)
    Tiny,
    /// Small (65 - 256 bytes)
    Small,
    /// Medium (257 - 4096 bytes)
    Medium,
    /// Large (4097 - 65536 bytes)
    Large,
    /// Huge (> 65536 bytes)
    Huge,
}

impl AllocSizeClass {
    #[inline]
    pub fn from_size(size: u64) -> Self {
        match size {
            0..=64 => AllocSizeClass::Tiny,
            65..=256 => AllocSizeClass::Small,
            257..=4096 => AllocSizeClass::Medium,
            4097..=65536 => AllocSizeClass::Large,
            _ => AllocSizeClass::Huge,
        }
    }
}

/// Allocation pattern analyzer
#[derive(Debug)]
pub struct AllocationAnalyzer {
    /// Allocations per size class
    alloc_counts: BTreeMap<u8, u64>,
    /// Total bytes allocated per size class
    alloc_bytes: BTreeMap<u8, u64>,
    /// Free counts per size class
    free_counts: BTreeMap<u8, u64>,
    /// Outstanding allocations (not freed)
    outstanding: u64,
    /// Peak outstanding
    peak_outstanding: u64,
    /// Total allocations
    pub total_allocs: u64,
    /// Total frees
    pub total_frees: u64,
    /// Total bytes allocated
    pub total_bytes: u64,
    /// Allocation rate (per second)
    alloc_rate_samples: Vec<f64>,
}

impl AllocationAnalyzer {
    pub fn new() -> Self {
        Self {
            alloc_counts: BTreeMap::new(),
            alloc_bytes: BTreeMap::new(),
            free_counts: BTreeMap::new(),
            outstanding: 0,
            peak_outstanding: 0,
            total_allocs: 0,
            total_frees: 0,
            total_bytes: 0,
            alloc_rate_samples: Vec::new(),
        }
    }

    /// Record an allocation
    #[inline]
    pub fn record_alloc(&mut self, size: u64) {
        let class = AllocSizeClass::from_size(size);
        *self.alloc_counts.entry(class as u8).or_insert(0) += 1;
        *self.alloc_bytes.entry(class as u8).or_insert(0) += size;
        self.total_allocs += 1;
        self.total_bytes += size;
        self.outstanding += 1;
        if self.outstanding > self.peak_outstanding {
            self.peak_outstanding = self.outstanding;
        }
    }

    /// Record a free
    #[inline]
    pub fn record_free(&mut self, size: u64) {
        let class = AllocSizeClass::from_size(size);
        *self.free_counts.entry(class as u8).or_insert(0) += 1;
        self.total_frees += 1;
        self.outstanding = self.outstanding.saturating_sub(1);
    }

    /// Outstanding allocation count
    #[inline(always)]
    pub fn outstanding(&self) -> u64 {
        self.outstanding
    }

    /// Possible memory leak (allocations >> frees)
    #[inline]
    pub fn possible_leak(&self) -> bool {
        if self.total_allocs < 100 {
            return false;
        }
        let free_ratio = self.total_frees as f64 / self.total_allocs as f64;
        free_ratio < 0.5 && self.outstanding > 1000
    }

    /// Dominant allocation size class
    pub fn dominant_class(&self) -> AllocSizeClass {
        self.alloc_counts
            .iter()
            .max_by_key(|(_, &v)| v)
            .map(|(&k, _)| match k {
                0 => AllocSizeClass::Tiny,
                1 => AllocSizeClass::Small,
                2 => AllocSizeClass::Medium,
                3 => AllocSizeClass::Large,
                _ => AllocSizeClass::Huge,
            })
            .unwrap_or(AllocSizeClass::Medium)
    }
}

// ============================================================================
// COMPOSITE MEMORY ANALYZER
// ============================================================================

/// Full memory analysis for a process
pub struct MemoryAnalyzer {
    /// Per-process working set estimators
    working_sets: BTreeMap<u64, WorkingSetEstimator>,
    /// Per-process access pattern detectors
    patterns: BTreeMap<u64, AccessPatternDetector>,
    /// Per-process allocation analyzers
    allocations: BTreeMap<u64, AllocationAnalyzer>,
    /// Page size
    page_size: u64,
    /// Max processes
    max_processes: usize,
}

impl MemoryAnalyzer {
    pub fn new(page_size: u64, max_processes: usize) -> Self {
        Self {
            working_sets: BTreeMap::new(),
            patterns: BTreeMap::new(),
            allocations: BTreeMap::new(),
            page_size,
            max_processes,
        }
    }

    /// Get or create working set estimator
    #[inline]
    pub fn working_set(&mut self, pid: u64) -> &mut WorkingSetEstimator {
        let ps = self.page_size;
        if !self.working_sets.contains_key(&pid) && self.working_sets.len() < self.max_processes {
            self.working_sets.insert(pid, WorkingSetEstimator::new(ps));
        }
        self.working_sets
            .entry(pid)
            .or_insert_with(|| WorkingSetEstimator::new(ps))
    }

    /// Get or create access pattern detector
    #[inline]
    pub fn access_pattern(&mut self, pid: u64) -> &mut AccessPatternDetector {
        if !self.patterns.contains_key(&pid) && self.patterns.len() < self.max_processes {
            self.patterns.insert(pid, AccessPatternDetector::new(256));
        }
        self.patterns
            .entry(pid)
            .or_insert_with(|| AccessPatternDetector::new(256))
    }

    /// Get or create allocation analyzer
    #[inline]
    pub fn allocations(&mut self, pid: u64) -> &mut AllocationAnalyzer {
        if !self.allocations.contains_key(&pid) && self.allocations.len() < self.max_processes {
            self.allocations.insert(pid, AllocationAnalyzer::new());
        }
        self.allocations
            .entry(pid)
            .or_insert_with(AllocationAnalyzer::new)
    }

    /// Remove process
    #[inline]
    pub fn remove_process(&mut self, pid: u64) {
        self.working_sets.remove(&pid);
        self.patterns.remove(&pid);
        self.allocations.remove(&pid);
    }

    /// Processes with possible memory leaks
    #[inline]
    pub fn leaking_processes(&self) -> Vec<u64> {
        self.allocations
            .iter()
            .filter(|(_, a)| a.possible_leak())
            .map(|(&pid, _)| pid)
            .collect()
    }
}
