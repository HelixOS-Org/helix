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
use alloc::collections::VecDeque;
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
#[repr(align(64))]
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
    #[inline]
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
    #[inline]
    pub fn record_free(&mut self, size: usize) {
        self.total_frees += 1;
        if self.live_allocs > 0 {
            self.live_allocs -= 1;
        }
        self.live_bytes = self.live_bytes.saturating_sub(size as u64);
    }

    /// Sample for growth tracking
    #[inline]
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
#[repr(align(64))]
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
    #[inline]
    pub fn record_alloc(&mut self, addr: u64, size: usize, callsite_hash: u64) {
        let cs = self
            .callsites
            .entry(callsite_hash)
            .or_insert_with(|| CallsiteStats::new(callsite_hash));
        cs.record_alloc(size);
        self.active.insert(addr, (size, callsite_hash));
    }

    /// Record free
    #[inline]
    pub fn record_free(&mut self, addr: u64) {
        if let Some((size, callsite_hash)) = self.active.remove(&addr) {
            if let Some(cs) = self.callsites.get_mut(&callsite_hash) {
                cs.record_free(size);
            }
        }
    }

    /// Sample all callsites
    #[inline]
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
    #[inline]
    pub fn record_alloc(&mut self, pid: u64, addr: u64, size: usize, callsite_hash: u64) {
        let detector = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessLeakDetector::new(pid));
        detector.record_alloc(addr, size, callsite_hash);
    }

    /// Record free
    #[inline]
    pub fn record_free(&mut self, pid: u64, addr: u64) {
        if let Some(detector) = self.processes.get_mut(&pid) {
            detector.record_free(addr);
        }
    }

    /// Periodic scan
    #[inline]
    pub fn scan(&mut self) {
        for detector in self.processes.values_mut() {
            detector.sample_all();
        }
        self.update_stats();
    }

    /// Get leaks for process
    #[inline]
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
    #[inline(always)]
    pub fn stats(&self) -> &AppLeakDetectorStats {
        &self.stats
    }
}

// ============================================================================
// Merged from leak_detect_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakSeverityV2 {
    None,
    Suspected,
    Probable,
    Confirmed,
    Critical,
}

/// Allocation record
#[derive(Debug, Clone)]
pub struct AllocRecordV2 {
    pub addr: u64,
    pub size: u64,
    pub callsite_hash: u64,
    pub timestamp: u64,
    pub thread_id: u64,
    pub freed: bool,
    pub free_timestamp: u64,
}

impl AllocRecordV2 {
    pub fn new(addr: u64, size: u64, callsite: u64, thread_id: u64, ts: u64) -> Self {
        Self {
            addr, size, callsite_hash: callsite, timestamp: ts,
            thread_id, freed: false, free_timestamp: 0,
        }
    }

    #[inline(always)]
    pub fn lifetime_ns(&self) -> u64 {
        if self.freed { self.free_timestamp.saturating_sub(self.timestamp) } else { 0 }
    }
}

/// Per-callsite leak aggregation
#[derive(Debug, Clone)]
pub struct CallsiteLeakProfile {
    pub callsite_hash: u64,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub total_bytes_allocated: u64,
    pub total_bytes_freed: u64,
    pub live_count: u64,
    pub live_bytes: u64,
    pub peak_live_count: u64,
    pub peak_live_bytes: u64,
    pub growth_samples: Vec<(u64, u64)>,
}

impl CallsiteLeakProfile {
    pub fn new(callsite_hash: u64) -> Self {
        Self {
            callsite_hash,
            total_allocs: 0, total_frees: 0,
            total_bytes_allocated: 0, total_bytes_freed: 0,
            live_count: 0, live_bytes: 0,
            peak_live_count: 0, peak_live_bytes: 0,
            growth_samples: Vec::new(),
        }
    }

    pub fn record_alloc(&mut self, size: u64, ts: u64) {
        self.total_allocs += 1;
        self.total_bytes_allocated += size;
        self.live_count += 1;
        self.live_bytes += size;
        if self.live_count > self.peak_live_count { self.peak_live_count = self.live_count; }
        if self.live_bytes > self.peak_live_bytes { self.peak_live_bytes = self.live_bytes; }

        self.growth_samples.push((ts, self.live_bytes));
        if self.growth_samples.len() > 128 {
            let mut new_s = Vec::new();
            for (i, s) in self.growth_samples.iter().enumerate() {
                if i % 2 == 0 || i == self.growth_samples.len() - 1 {
                    new_s.push(s.clone());
                }
            }
            self.growth_samples = new_s;
        }
    }

    #[inline]
    pub fn record_free(&mut self, size: u64) {
        self.total_frees += 1;
        self.total_bytes_freed += size;
        self.live_count = self.live_count.saturating_sub(1);
        self.live_bytes = self.live_bytes.saturating_sub(size);
    }

    #[inline(always)]
    pub fn leak_ratio(&self) -> f64 {
        if self.total_allocs == 0 { return 0.0; }
        (self.total_allocs - self.total_frees) as f64 / self.total_allocs as f64
    }

    pub fn growth_rate_bps(&self) -> f64 {
        if self.growth_samples.len() < 2 { return 0.0; }
        let n = self.growth_samples.len() as f64;
        let sum_x: f64 = self.growth_samples.iter().map(|(t, _)| *t as f64).sum();
        let sum_y: f64 = self.growth_samples.iter().map(|(_, b)| *b as f64).sum();
        let sum_xy: f64 = self.growth_samples.iter().map(|(t, b)| *t as f64 * *b as f64).sum();
        let sum_x2: f64 = self.growth_samples.iter().map(|(t, _)| (*t as f64) * (*t as f64)).sum();
        let denom = n * sum_x2 - sum_x * sum_x;
        if libm::fabs(denom) < 1e-10 { return 0.0; }
        let slope = (n * sum_xy - sum_x * sum_y) / denom;
        slope * 1_000_000_000.0
    }

    #[inline]
    pub fn severity(&self) -> LeakSeverityV2 {
        let ratio = self.leak_ratio();
        let growth = self.growth_rate_bps();
        if ratio < 0.01 || self.live_count < 10 { return LeakSeverityV2::None; }
        if ratio < 0.1 && growth < 1024.0 { return LeakSeverityV2::Suspected; }
        if ratio < 0.3 && growth < 65536.0 { return LeakSeverityV2::Probable; }
        if self.live_bytes > 1024 * 1024 && growth > 65536.0 { return LeakSeverityV2::Critical; }
        LeakSeverityV2::Confirmed
    }
}

/// Per-process leak analysis
#[derive(Debug, Clone)]
pub struct ProcessLeakProfileV2 {
    pub pid: u64,
    pub callsites: BTreeMap<u64, CallsiteLeakProfile>,
    pub live_allocs: BTreeMap<u64, AllocRecordV2>,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub live_bytes: u64,
    pub peak_live_bytes: u64,
}

impl ProcessLeakProfileV2 {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            callsites: BTreeMap::new(),
            live_allocs: BTreeMap::new(),
            total_allocs: 0,
            total_frees: 0,
            live_bytes: 0,
            peak_live_bytes: 0,
        }
    }

    #[inline]
    pub fn record_alloc(&mut self, addr: u64, size: u64, callsite: u64, tid: u64, ts: u64) {
        let record = AllocRecordV2::new(addr, size, callsite, tid, ts);
        self.live_allocs.insert(addr, record);
        self.total_allocs += 1;
        self.live_bytes += size;
        if self.live_bytes > self.peak_live_bytes { self.peak_live_bytes = self.live_bytes; }
        self.callsites.entry(callsite)
            .or_insert_with(|| CallsiteLeakProfile::new(callsite))
            .record_alloc(size, ts);
    }

    #[inline]
    pub fn record_free(&mut self, addr: u64, ts: u64) {
        if let Some(mut rec) = self.live_allocs.remove(&addr) {
            rec.freed = true;
            rec.free_timestamp = ts;
            self.total_frees += 1;
            self.live_bytes = self.live_bytes.saturating_sub(rec.size);
            if let Some(cs) = self.callsites.get_mut(&rec.callsite_hash) {
                cs.record_free(rec.size);
            }
        }
    }

    pub fn worst_severity(&self) -> LeakSeverityV2 {
        self.callsites.values()
            .map(|cs| cs.severity())
            .max_by_key(|s| match s {
                LeakSeverityV2::None => 0,
                LeakSeverityV2::Suspected => 1,
                LeakSeverityV2::Probable => 2,
                LeakSeverityV2::Confirmed => 3,
                LeakSeverityV2::Critical => 4,
            })
            .unwrap_or(LeakSeverityV2::None)
    }

    #[inline]
    pub fn leak_sites(&self) -> Vec<&CallsiteLeakProfile> {
        let mut sites: Vec<_> = self.callsites.values()
            .filter(|cs| cs.severity() != LeakSeverityV2::None)
            .collect();
        sites.sort_by(|a, b| b.live_bytes.cmp(&a.live_bytes));
        sites
    }
}

/// App leak detector v2 stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppLeakDetectorV2Stats {
    pub total_processes: usize,
    pub total_live_allocs: usize,
    pub total_live_bytes: u64,
    pub suspected_leaks: usize,
    pub confirmed_leaks: usize,
    pub critical_leaks: usize,
}

/// Application Memory Leak Detector V2
pub struct AppLeakDetectorV2 {
    profiles: BTreeMap<u64, ProcessLeakProfileV2>,
    stats: AppLeakDetectorV2Stats,
}

impl AppLeakDetectorV2 {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppLeakDetectorV2Stats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessLeakProfileV2::new(pid));
    }

    #[inline]
    pub fn record_alloc(&mut self, pid: u64, addr: u64, size: u64, callsite: u64, tid: u64, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_alloc(addr, size, callsite, tid, ts);
        }
    }

    #[inline]
    pub fn record_free(&mut self, pid: u64, addr: u64, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_free(addr, ts);
        }
    }

    pub fn analyze(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_live_allocs = self.profiles.values().map(|p| p.live_allocs.len()).sum();
        self.stats.total_live_bytes = self.profiles.values().map(|p| p.live_bytes).sum();
        let mut suspected = 0usize;
        let mut confirmed = 0usize;
        let mut critical = 0usize;
        for prof in self.profiles.values() {
            for cs in prof.callsites.values() {
                match cs.severity() {
                    LeakSeverityV2::Suspected => suspected += 1,
                    LeakSeverityV2::Probable | LeakSeverityV2::Confirmed => confirmed += 1,
                    LeakSeverityV2::Critical => critical += 1,
                    _ => {}
                }
            }
        }
        self.stats.suspected_leaks = suspected;
        self.stats.confirmed_leaks = confirmed;
        self.stats.critical_leaks = critical;
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessLeakProfileV2> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppLeakDetectorV2Stats {
        &self.stats
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
    }
}
