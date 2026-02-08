//! # Apps Memory Advisor
//!
//! Application memory behavior analysis and advisory:
//! - Working set size estimation
//! - Memory access pattern detection
//! - NUMA locality scoring
//! - Page reclaim recommendations
//! - Memory pressure response guidance
//! - Transparent huge page advisory

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Memory access pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemPattern {
    Sequential,
    Random,
    Strided,
    Temporal,
    Streaming,
    Mixed,
}

/// Memory advisory action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemAdvice {
    Normal,
    WillNeed,
    DontNeed,
    Free,
    Mergeable,
    Unmergeable,
    HugePage,
    NoHugePage,
    Cold,
    PageOut,
}

/// NUMA preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaPref {
    Local,
    Interleaved,
    Preferred(u32),
    Bind(u32),
    NoPreference,
}

/// Per-region memory stats
#[derive(Debug, Clone)]
pub struct MemRegionStats {
    pub start_addr: u64,
    pub size_bytes: u64,
    pub resident_bytes: u64,
    pub swapped_bytes: u64,
    pub shared_bytes: u64,
    pub pattern: MemPattern,
    pub access_count: u64,
    pub fault_count: u64,
    pub numa_node: u32,
    pub numa_misses: u64,
    pub thp_eligible: bool,
    pub last_access_ts: u64,
    pub hot_score: f64,
}

impl MemRegionStats {
    pub fn new(start: u64, size: u64) -> Self {
        Self {
            start_addr: start, size_bytes: size, resident_bytes: 0,
            swapped_bytes: 0, shared_bytes: 0, pattern: MemPattern::Mixed,
            access_count: 0, fault_count: 0, numa_node: 0, numa_misses: 0,
            thp_eligible: false, last_access_ts: 0, hot_score: 0.0,
        }
    }

    pub fn residency_pct(&self) -> f64 { if self.size_bytes == 0 { 0.0 } else { self.resident_bytes as f64 / self.size_bytes as f64 * 100.0 } }
    pub fn numa_local_pct(&self) -> f64 { if self.access_count == 0 { 100.0 } else { (self.access_count - self.numa_misses) as f64 / self.access_count as f64 * 100.0 } }

    pub fn update_hot_score(&mut self, now: u64) {
        let age = now.saturating_sub(self.last_access_ts);
        let freq = if age == 0 { self.access_count as f64 } else { self.access_count as f64 / (age as f64 / 1_000_000.0) };
        self.hot_score = freq;
    }
}

/// Per-process memory profile
#[derive(Debug, Clone)]
pub struct ProcessMemProfile {
    pub pid: u64,
    pub rss_bytes: u64,
    pub vss_bytes: u64,
    pub shared_bytes: u64,
    pub swap_bytes: u64,
    pub regions: Vec<MemRegionStats>,
    pub working_set_bytes: u64,
    pub wss_window_us: u64,
    pub numa_pref: NumaPref,
    pub major_faults: u64,
    pub minor_faults: u64,
    pub oom_score_adj: i16,
}

impl ProcessMemProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, rss_bytes: 0, vss_bytes: 0, shared_bytes: 0, swap_bytes: 0,
            regions: Vec::new(), working_set_bytes: 0, wss_window_us: 1_000_000,
            numa_pref: NumaPref::Local, major_faults: 0, minor_faults: 0, oom_score_adj: 0,
        }
    }

    pub fn estimate_wss(&mut self, now: u64) {
        let threshold = now.saturating_sub(self.wss_window_us);
        self.working_set_bytes = self.regions.iter().filter(|r| r.last_access_ts >= threshold).map(|r| r.resident_bytes).sum();
    }

    pub fn rss_to_wss_ratio(&self) -> f64 { if self.working_set_bytes == 0 { 0.0 } else { self.rss_bytes as f64 / self.working_set_bytes as f64 } }
    pub fn fault_rate(&self) -> f64 { if self.rss_bytes == 0 { 0.0 } else { (self.major_faults + self.minor_faults) as f64 } }
}

/// Advisory recommendation
#[derive(Debug, Clone)]
pub struct MemAdvisory {
    pub pid: u64,
    pub region_start: u64,
    pub advice: MemAdvice,
    pub reason: AdvisoryReason,
    pub priority: u8,
    pub ts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryReason {
    Cold,
    Hot,
    HighSwap,
    NumaMismatch,
    ThpCandidate,
    MemoryPressure,
    LargeWss,
    Unused,
}

/// Memory advisor stats
#[derive(Debug, Clone, Default)]
pub struct MemAdvisorStats {
    pub tracked_processes: usize,
    pub total_rss: u64,
    pub total_wss: u64,
    pub total_advisories: usize,
    pub avg_numa_local_pct: f64,
    pub processes_swapping: usize,
}

/// Apps memory advisor
pub struct AppsMemAdvisor {
    profiles: BTreeMap<u64, ProcessMemProfile>,
    advisories: Vec<MemAdvisory>,
    stats: MemAdvisorStats,
}

impl AppsMemAdvisor {
    pub fn new() -> Self { Self { profiles: BTreeMap::new(), advisories: Vec::new(), stats: MemAdvisorStats::default() } }

    pub fn track(&mut self, pid: u64) { self.profiles.entry(pid).or_insert_with(|| ProcessMemProfile::new(pid)); }
    pub fn untrack(&mut self, pid: u64) { self.profiles.remove(&pid); }

    pub fn update_rss(&mut self, pid: u64, rss: u64, swap: u64) {
        if let Some(p) = self.profiles.get_mut(&pid) { p.rss_bytes = rss; p.swap_bytes = swap; }
    }

    pub fn record_fault(&mut self, pid: u64, major: bool) {
        if let Some(p) = self.profiles.get_mut(&pid) { if major { p.major_faults += 1; } else { p.minor_faults += 1; } }
    }

    pub fn add_region(&mut self, pid: u64, start: u64, size: u64) {
        if let Some(p) = self.profiles.get_mut(&pid) { p.regions.push(MemRegionStats::new(start, size)); }
    }

    pub fn generate_advisories(&mut self, now: u64) {
        self.advisories.clear();
        let pids: Vec<u64> = self.profiles.keys().copied().collect();
        for pid in pids {
            if let Some(p) = self.profiles.get_mut(&pid) {
                p.estimate_wss(now);
                for r in &p.regions {
                    if r.hot_score < 0.1 && r.resident_bytes > 4096 {
                        self.advisories.push(MemAdvisory { pid, region_start: r.start_addr, advice: MemAdvice::Cold, reason: AdvisoryReason::Cold, priority: 3, ts: now });
                    }
                    if r.numa_local_pct() < 50.0 {
                        self.advisories.push(MemAdvisory { pid, region_start: r.start_addr, advice: MemAdvice::Normal, reason: AdvisoryReason::NumaMismatch, priority: 5, ts: now });
                    }
                    if r.thp_eligible && r.size_bytes >= 2 * 1024 * 1024 {
                        self.advisories.push(MemAdvisory { pid, region_start: r.start_addr, advice: MemAdvice::HugePage, reason: AdvisoryReason::ThpCandidate, priority: 2, ts: now });
                    }
                }
                if p.swap_bytes > p.rss_bytes / 4 {
                    self.advisories.push(MemAdvisory { pid, region_start: 0, advice: MemAdvice::WillNeed, reason: AdvisoryReason::HighSwap, priority: 7, ts: now });
                }
            }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.profiles.len();
        self.stats.total_rss = self.profiles.values().map(|p| p.rss_bytes).sum();
        self.stats.total_wss = self.profiles.values().map(|p| p.working_set_bytes).sum();
        self.stats.total_advisories = self.advisories.len();
        self.stats.processes_swapping = self.profiles.values().filter(|p| p.swap_bytes > 0).count();
    }

    pub fn profile(&self, pid: u64) -> Option<&ProcessMemProfile> { self.profiles.get(&pid) }
    pub fn advisories(&self) -> &[MemAdvisory] { &self.advisories }
    pub fn stats(&self) -> &MemAdvisorStats { &self.stats }
}
