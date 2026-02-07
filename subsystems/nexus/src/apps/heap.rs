//! # Application Heap Analysis
//!
//! Track heap allocation patterns per application:
//! - Allocation size distribution
//! - Fragmentation analysis
//! - Allocation hotspot detection
//! - Leak detection heuristics
//! - Arena/pool recommendations

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ALLOCATION TYPES
// ============================================================================

/// Allocation size class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocSizeClass {
    /// Tiny: 0-64 bytes
    Tiny,
    /// Small: 65-256 bytes
    Small,
    /// Medium: 257-4096 bytes
    Medium,
    /// Large: 4097-65536 bytes
    Large,
    /// Huge: 65537+ bytes
    Huge,
}

impl AllocSizeClass {
    pub fn from_size(size: usize) -> Self {
        match size {
            0..=64 => Self::Tiny,
            65..=256 => Self::Small,
            257..=4096 => Self::Medium,
            4097..=65536 => Self::Large,
            _ => Self::Huge,
        }
    }
}

/// Allocation event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocEventType {
    /// Malloc/alloc
    Alloc,
    /// Free/dealloc
    Free,
    /// Realloc
    Realloc,
    /// Calloc
    Calloc,
}

/// Single allocation record
#[derive(Debug, Clone)]
pub struct AllocRecord {
    /// Allocation address
    pub address: u64,
    /// Size
    pub size: usize,
    /// Alignment
    pub alignment: usize,
    /// Timestamp
    pub timestamp: u64,
    /// Callsite hash (instruction pointer hash)
    pub callsite: u64,
    /// Still live?
    pub live: bool,
}

// ============================================================================
// SIZE DISTRIBUTION
// ============================================================================

/// Allocation size histogram
#[derive(Debug, Clone)]
pub struct AllocHistogram {
    /// Bucket counts (powers of 2: 0-8, 8-16, 16-32, ..., 1M+)
    buckets: [u64; 24],
    /// Total count
    total: u64,
}

impl AllocHistogram {
    pub fn new() -> Self {
        Self {
            buckets: [0; 24],
            total: 0,
        }
    }

    /// Add size
    pub fn record(&mut self, size: usize) {
        let bucket = if size == 0 {
            0
        } else {
            let mut bits = 0u32;
            let mut v = size;
            while v > 1 {
                v >>= 1;
                bits += 1;
            }
            (bits as usize).min(23)
        };
        self.buckets[bucket] += 1;
        self.total += 1;
    }

    /// Peak bucket (most common size range)
    pub fn peak_bucket(&self) -> usize {
        let mut max_idx = 0;
        let mut max_val = 0;
        for (i, &v) in self.buckets.iter().enumerate() {
            if v > max_val {
                max_val = v;
                max_idx = i;
            }
        }
        max_idx
    }

    /// Percentage in bucket
    pub fn bucket_pct(&self, bucket: usize) -> f64 {
        if self.total == 0 || bucket >= 24 {
            return 0.0;
        }
        self.buckets[bucket] as f64 / self.total as f64 * 100.0
    }
}

// ============================================================================
// FRAGMENTATION ANALYSIS
// ============================================================================

/// Fragmentation metric
#[derive(Debug, Clone)]
pub struct FragmentationInfo {
    /// Total allocated bytes
    pub allocated: u64,
    /// Total free bytes
    pub free: u64,
    /// Largest free block
    pub largest_free: u64,
    /// Free block count
    pub free_blocks: u64,
    /// Internal fragmentation ratio
    pub internal_frag: f64,
    /// External fragmentation ratio (1 - largest_free / total_free)
    pub external_frag: f64,
}

impl FragmentationInfo {
    pub fn compute(
        allocated: u64,
        free: u64,
        largest_free: u64,
        free_blocks: u64,
        wasted_internal: u64,
    ) -> Self {
        let internal_frag = if allocated > 0 {
            wasted_internal as f64 / allocated as f64
        } else {
            0.0
        };
        let external_frag = if free > 0 {
            1.0 - (largest_free as f64 / free as f64)
        } else {
            0.0
        };
        Self {
            allocated,
            free,
            largest_free,
            free_blocks,
            internal_frag,
            external_frag,
        }
    }

    /// Overall fragmentation score (0-1)
    pub fn score(&self) -> f64 {
        (self.internal_frag + self.external_frag) / 2.0
    }
}

// ============================================================================
// LEAK DETECTION
// ============================================================================

/// Potential leak
#[derive(Debug, Clone)]
pub struct PotentialLeak {
    /// Callsite hash
    pub callsite: u64,
    /// Outstanding allocations
    pub outstanding: u64,
    /// Outstanding bytes
    pub outstanding_bytes: u64,
    /// Growth rate (allocs/sec)
    pub growth_rate: f64,
    /// Confidence (0-1)
    pub confidence: f64,
}

/// Callsite allocation tracking
#[derive(Debug, Clone)]
pub struct CallsiteProfile {
    /// Callsite hash
    pub callsite: u64,
    /// Total allocs
    pub total_allocs: u64,
    /// Total frees
    pub total_frees: u64,
    /// Outstanding bytes
    pub outstanding_bytes: u64,
    /// Max outstanding
    pub max_outstanding: u64,
    /// First seen
    pub first_seen: u64,
    /// Last alloc
    pub last_alloc: u64,
}

impl CallsiteProfile {
    pub fn new(callsite: u64, now: u64) -> Self {
        Self {
            callsite,
            total_allocs: 0,
            total_frees: 0,
            outstanding_bytes: 0,
            max_outstanding: 0,
            first_seen: now,
            last_alloc: now,
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, size: usize, now: u64) {
        self.total_allocs += 1;
        self.outstanding_bytes += size as u64;
        if self.outstanding_bytes > self.max_outstanding {
            self.max_outstanding = self.outstanding_bytes;
        }
        self.last_alloc = now;
    }

    /// Record free
    pub fn record_free(&mut self, size: usize) {
        self.total_frees += 1;
        self.outstanding_bytes = self.outstanding_bytes.saturating_sub(size as u64);
    }

    /// Outstanding allocation count
    pub fn outstanding_count(&self) -> u64 {
        self.total_allocs.saturating_sub(self.total_frees)
    }

    /// Growth rate
    pub fn growth_rate(&self, now: u64) -> f64 {
        let elapsed_ns = now.saturating_sub(self.first_seen);
        if elapsed_ns == 0 {
            return 0.0;
        }
        let elapsed_secs = elapsed_ns as f64 / 1_000_000_000.0;
        self.outstanding_count() as f64 / elapsed_secs
    }

    /// Is likely leak?
    pub fn is_likely_leak(&self, now: u64) -> bool {
        let outstanding = self.outstanding_count();
        let elapsed = now.saturating_sub(self.first_seen);
        // Needs significant samples and time
        outstanding > 100 && elapsed > 10_000_000_000 && self.total_frees < self.total_allocs / 2
    }
}

// ============================================================================
// PROCESS HEAP PROFILE
// ============================================================================

/// Process heap profile
#[derive(Debug)]
pub struct ProcessHeapProfile {
    /// Process id
    pub pid: u64,
    /// Size histogram
    pub histogram: AllocHistogram,
    /// Callsite profiles
    pub callsites: BTreeMap<u64, CallsiteProfile>,
    /// Live allocation count
    pub live_allocs: u64,
    /// Live allocation bytes
    pub live_bytes: u64,
    /// Peak live bytes
    pub peak_bytes: u64,
    /// Total allocs ever
    pub total_allocs: u64,
    /// Total frees ever
    pub total_frees: u64,
}

impl ProcessHeapProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            histogram: AllocHistogram::new(),
            callsites: BTreeMap::new(),
            live_allocs: 0,
            live_bytes: 0,
            peak_bytes: 0,
            total_allocs: 0,
            total_frees: 0,
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, size: usize, callsite: u64, now: u64) {
        self.histogram.record(size);
        self.live_allocs += 1;
        self.live_bytes += size as u64;
        self.total_allocs += 1;
        if self.live_bytes > self.peak_bytes {
            self.peak_bytes = self.live_bytes;
        }
        let profile = self
            .callsites
            .entry(callsite)
            .or_insert_with(|| CallsiteProfile::new(callsite, now));
        profile.record_alloc(size, now);
    }

    /// Record free
    pub fn record_free(&mut self, size: usize, callsite: u64) {
        self.live_allocs = self.live_allocs.saturating_sub(1);
        self.live_bytes = self.live_bytes.saturating_sub(size as u64);
        self.total_frees += 1;
        if let Some(profile) = self.callsites.get_mut(&callsite) {
            profile.record_free(size);
        }
    }

    /// Detect potential leaks
    pub fn detect_leaks(&self, now: u64) -> Vec<PotentialLeak> {
        let mut leaks = Vec::new();
        for profile in self.callsites.values() {
            if profile.is_likely_leak(now) {
                let growth = profile.growth_rate(now);
                let outstanding = profile.outstanding_count();
                // Confidence based on outstanding count and free ratio
                let free_ratio = if profile.total_allocs > 0 {
                    profile.total_frees as f64 / profile.total_allocs as f64
                } else {
                    1.0
                };
                let confidence = if free_ratio < 0.1 {
                    0.9
                } else if free_ratio < 0.3 {
                    0.7
                } else {
                    0.5
                };
                leaks.push(PotentialLeak {
                    callsite: profile.callsite,
                    outstanding,
                    outstanding_bytes: profile.outstanding_bytes,
                    growth_rate: growth,
                    confidence,
                });
            }
        }
        leaks.sort_by(|a, b| {
            b.outstanding_bytes
                .partial_cmp(&a.outstanding_bytes)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        leaks
    }

    /// Top allocation callsites
    pub fn top_callsites(&self, limit: usize) -> Vec<(u64, u64)> {
        let mut sites: Vec<_> = self
            .callsites
            .values()
            .map(|c| (c.callsite, c.outstanding_bytes))
            .collect();
        sites.sort_by(|a, b| b.1.cmp(&a.1));
        sites.truncate(limit);
        sites
    }
}

// ============================================================================
// HEAP ANALYZER
// ============================================================================

/// Heap stats
#[derive(Debug, Clone, Default)]
pub struct AppHeapStats {
    /// Processes tracked
    pub processes: usize,
    /// Total live bytes
    pub total_live_bytes: u64,
    /// Total leaks detected
    pub leaks_detected: usize,
}

/// Application heap analyzer
pub struct AppHeapAnalyzer {
    /// Profiles
    profiles: BTreeMap<u64, ProcessHeapProfile>,
    /// Stats
    stats: AppHeapStats,
}

impl AppHeapAnalyzer {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppHeapStats::default(),
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, pid: u64, size: usize, callsite: u64, now: u64) {
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessHeapProfile::new(pid));
        profile.record_alloc(size, callsite, now);
        self.stats.processes = self.profiles.len();
        self.stats.total_live_bytes = self.profiles.values().map(|p| p.live_bytes).sum();
    }

    /// Record free
    pub fn record_free(&mut self, pid: u64, size: usize, callsite: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_free(size, callsite);
            self.stats.total_live_bytes = self.profiles.values().map(|p| p.live_bytes).sum();
        }
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessHeapProfile> {
        self.profiles.get(&pid)
    }

    /// Detect all leaks
    pub fn detect_leaks(&self, now: u64) -> Vec<(u64, Vec<PotentialLeak>)> {
        let mut result = Vec::new();
        for profile in self.profiles.values() {
            let leaks = profile.detect_leaks(now);
            if !leaks.is_empty() {
                result.push((profile.pid, leaks));
            }
        }
        result
    }

    /// Stats
    pub fn stats(&self) -> &AppHeapStats {
        &self.stats
    }
}
