//! # App Memory Leak Detector
//!
//! Memory leak detection and tracking:
//! - Allocation tracking with callsite hashing
//! - Growth rate analysis
//! - Unreachable allocation identification
//! - Leak severity scoring
//! - Allocation pattern classification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// LEAK TYPES
// ============================================================================

/// Leak severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakSeverity {
    /// Not leaking
    None,
    /// Possible leak (slow growth)
    Possible,
    /// Probable leak (steady growth)
    Probable,
    /// Confirmed leak (monotonic growth)
    Confirmed,
    /// Critical (affecting system)
    Critical,
}

/// Allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocType {
    /// Heap (malloc/new)
    Heap,
    /// Mmap
    Mmap,
    /// Slab
    Slab,
    /// Stack expansion
    Stack,
    /// DMA buffer
    Dma,
}

/// Allocation pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocPattern {
    /// Allocate and free regularly
    Regular,
    /// Allocate once, keep forever
    Permanent,
    /// Growing without bound
    Growing,
    /// Cyclic (allocate/free in batches)
    Cyclic,
    /// Pool-based
    Pool,
}

// ============================================================================
// ALLOCATION TRACKING
// ============================================================================

/// Allocation record
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    /// Address
    pub addr: u64,
    /// Size
    pub size: usize,
    /// Allocation type
    pub alloc_type: AllocType,
    /// Callsite hash
    pub callsite_hash: u64,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Is freed
    pub freed: bool,
}

/// Callsite aggregation
#[derive(Debug, Clone)]
pub struct CallsiteStats {
    /// Callsite hash
    pub callsite_hash: u64,
    /// Total allocations
    pub total_allocs: u64,
    /// Total frees
    pub total_frees: u64,
    /// Current live allocations
    pub live_allocs: u64,
    /// Total bytes allocated
    pub total_bytes: u64,
    /// Current live bytes
    pub live_bytes: u64,
    /// Peak live bytes
    pub peak_bytes: u64,
    /// Growth history (sampled live_bytes)
    growth_samples: Vec<u64>,
    /// Max samples
    max_samples: usize,
}

impl CallsiteStats {
    pub fn new(callsite_hash: u64) -> Self {
        Self {
            callsite_hash,
            total_allocs: 0,
            total_frees: 0,
            live_allocs: 0,
            total_bytes: 0,
            live_bytes: 0,
            peak_bytes: 0,
            growth_samples: Vec::new(),
            max_samples: 100,
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, size: usize) {
        self.total_allocs += 1;
        self.live_allocs += 1;
        self.total_bytes += size as u64;
        self.live_bytes += size as u64;
        if self.live_bytes > self.peak_bytes {
            self.peak_bytes = self.live_bytes;
        }
    }

    /// Record free
    pub fn record_free(&mut self, size: usize) {
        self.total_frees += 1;
        if self.live_allocs > 0 {
            self.live_allocs -= 1;
        }
        self.live_bytes = self.live_bytes.saturating_sub(size as u64);
    }

    /// Sample for growth tracking
    pub fn sample(&mut self) {
        if self.growth_samples.len() >= self.max_samples {
            self.growth_samples.remove(0);
        }
        self.growth_samples.push(self.live_bytes);
    }

    /// Detect leak severity
    pub fn leak_severity(&self) -> LeakSeverity {
        if self.live_allocs == 0 {
            return LeakSeverity::None;
        }
        if self.total_frees == 0 && self.total_allocs > 10 {
            return LeakSeverity::Confirmed;
        }

        let leak_ratio = if self.total_allocs > 0 {
            1.0 - (self.total_frees as f64 / self.total_allocs as f64)
        } else {
            0.0
        };

        // Check growth trend
        let is_growing = self.is_monotonically_growing();

        if is_growing && leak_ratio > 0.5 {
            LeakSeverity::Critical
        } else if is_growing && leak_ratio > 0.2 {
            LeakSeverity::Confirmed
        } else if leak_ratio > 0.3 {
            LeakSeverity::Probable
        } else if leak_ratio > 0.1 {
            LeakSeverity::Possible
        } else {
            LeakSeverity::None
        }
    }

    /// Check monotonic growth
    fn is_monotonically_growing(&self) -> bool {
        if self.growth_samples.len() < 5 {
            return false;
        }
        let n = self.growth_samples.len();
        let recent = &self.growth_samples[n.saturating_sub(10)..];
        let mut growing = 0;
        for i in 1..recent.len() {
            if recent[i] > recent[i - 1] {
                growing += 1;
            }
        }
        growing as f64 / (recent.len() - 1) as f64 > 0.7
    }

    /// Allocation pattern
    pub fn pattern(&self) -> AllocPattern {
        if self.total_frees == 0 && self.total_allocs > 0 {
            return AllocPattern::Permanent;
        }
        if self.is_monotonically_growing() {
            return AllocPattern::Growing;
        }
        let ratio = if self.total_allocs > 0 {
            self.total_frees as f64 / self.total_allocs as f64
        } else {
            0.0
        };
        if ratio > 0.9 {
            AllocPattern::Regular
        } else {
            AllocPattern::Cyclic
        }
    }
}

// ============================================================================
// LEAK REPORT
// ============================================================================

/// Leak report entry
#[derive(Debug, Clone)]
pub struct LeakReport {
    /// Callsite hash
    pub callsite_hash: u64,
    /// Severity
    pub severity: LeakSeverity,
    /// Live bytes
    pub live_bytes: u64,
    /// Live allocations
    pub live_allocs: u64,
    /// Estimated leak rate (bytes/sec)
    pub leak_rate_bps: f64,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Leak detector stats
#[derive(Debug, Clone, Default)]
pub struct AppLeakDetectorStats {
    /// Tracked callsites
    pub tracked_callsites: usize,
    /// Suspected leaks
    pub suspected_leaks: usize,
    /// Confirmed leaks
    pub confirmed_leaks: usize,
    /// Total live bytes (suspected leaking)
    pub leak_bytes: u64,
}

/// Per-process leak detector
#[derive(Debug)]
pub struct ProcessLeakDetector {
    /// PID
    pub pid: u64,
    /// Callsite stats
    callsites: BTreeMap<u64, CallsiteStats>,
    /// Active allocations: addr -> (size, callsite_hash)
    active: BTreeMap<u64, (usize, u64)>,
}

impl ProcessLeakDetector {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            callsites: BTreeMap::new(),
            active: BTreeMap::new(),
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, addr: u64, size: usize, callsite_hash: u64) {
        let cs = self
            .callsites
            .entry(callsite_hash)
            .or_insert_with(|| CallsiteStats::new(callsite_hash));
        cs.record_alloc(size);
        self.active.insert(addr, (size, callsite_hash));
    }

    /// Record free
    pub fn record_free(&mut self, addr: u64) {
        if let Some((size, callsite_hash)) = self.active.remove(&addr) {
            if let Some(cs) = self.callsites.get_mut(&callsite_hash) {
                cs.record_free(size);
            }
        }
    }

    /// Sample all callsites
    pub fn sample_all(&mut self) {
        for cs in self.callsites.values_mut() {
            cs.sample();
        }
    }

    /// Generate leak report
    pub fn report(&self) -> Vec<LeakReport> {
        self.callsites
            .values()
            .filter(|cs| cs.leak_severity() != LeakSeverity::None)
            .map(|cs| LeakReport {
                callsite_hash: cs.callsite_hash,
                severity: cs.leak_severity(),
                live_bytes: cs.live_bytes,
                live_allocs: cs.live_allocs,
                leak_rate_bps: 0.0, // would need timestamps
            })
            .collect()
    }
}

/// App leak detector
pub struct AppLeakDetector {
    /// Per-process detectors
    processes: BTreeMap<u64, ProcessLeakDetector>,
    /// Stats
    stats: AppLeakDetectorStats,
}

impl AppLeakDetector {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppLeakDetectorStats::default(),
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, pid: u64, addr: u64, size: usize, callsite_hash: u64) {
        let detector = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessLeakDetector::new(pid));
        detector.record_alloc(addr, size, callsite_hash);
    }

    /// Record free
    pub fn record_free(&mut self, pid: u64, addr: u64) {
        if let Some(detector) = self.processes.get_mut(&pid) {
            detector.record_free(addr);
        }
    }

    /// Periodic scan
    pub fn scan(&mut self) {
        for detector in self.processes.values_mut() {
            detector.sample_all();
        }
        self.update_stats();
    }

    /// Get leaks for process
    pub fn process_leaks(&self, pid: u64) -> Vec<LeakReport> {
        self.processes
            .get(&pid)
            .map(|d| d.report())
            .unwrap_or_default()
    }

    fn update_stats(&mut self) {
        self.stats.tracked_callsites = self.processes.values().map(|d| d.callsites.len()).sum();
        let mut suspected = 0;
        let mut confirmed = 0;
        let mut bytes = 0u64;
        for detector in self.processes.values() {
            for cs in detector.callsites.values() {
                match cs.leak_severity() {
                    LeakSeverity::Possible | LeakSeverity::Probable => {
                        suspected += 1;
                        bytes += cs.live_bytes;
                    },
                    LeakSeverity::Confirmed | LeakSeverity::Critical => {
                        confirmed += 1;
                        bytes += cs.live_bytes;
                    },
                    _ => {},
                }
            }
        }
        self.stats.suspected_leaks = suspected;
        self.stats.confirmed_leaks = confirmed;
        self.stats.leak_bytes = bytes;
    }

    /// Stats
    pub fn stats(&self) -> &AppLeakDetectorStats {
        &self.stats
    }
}
