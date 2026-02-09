//! # Application Page Fault Profiler
//!
//! Per-process page fault analysis:
//! - Minor/major fault frequency tracking
//! - Fault hotspot detection (virtual address)
//! - COW fault tracking
//! - Demand paging analysis
//! - THP fault correlation
//! - Working set estimation from faults

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFaultType {
    Minor,
    Major,
    CopyOnWrite,
    DemandZero,
    SwapIn,
    HugePageSplit,
    ProtectionFault,
}

/// Page fault access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultAccess {
    Read,
    Write,
    Execute,
}

/// Single fault event
#[derive(Debug, Clone)]
pub struct PageFaultEvent {
    pub fault_type: PageFaultType,
    pub access: FaultAccess,
    pub virtual_addr: u64,
    pub timestamp: u64,
    pub thread_id: u64,
    pub resolution_ns: u64,
    pub was_in_cache: bool,
}

impl PageFaultEvent {
    #[inline(always)]
    pub fn is_expensive(&self) -> bool {
        matches!(self.fault_type, PageFaultType::Major | PageFaultType::SwapIn)
            || self.resolution_ns > 100_000
    }
}

/// Fault hotspot (page-aligned region with high fault rate)
#[derive(Debug, Clone)]
pub struct FaultHotspot {
    pub page_addr: u64,
    pub fault_count: u64,
    pub last_fault_ts: u64,
    pub dominant_type: PageFaultType,
    pub dominant_access: FaultAccess,
    pub total_resolution_ns: u64,
}

impl FaultHotspot {
    pub fn new(page_addr: u64, fault_type: PageFaultType, access: FaultAccess) -> Self {
        Self {
            page_addr,
            fault_count: 1,
            last_fault_ts: 0,
            dominant_type: fault_type,
            dominant_access: access,
            total_resolution_ns: 0,
        }
    }

    #[inline(always)]
    pub fn avg_resolution_ns(&self) -> u64 {
        if self.fault_count == 0 { return 0; }
        self.total_resolution_ns / self.fault_count
    }
}

/// Per-type fault counter
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FaultTypeCounter {
    pub minor: u64,
    pub major: u64,
    pub cow: u64,
    pub demand_zero: u64,
    pub swap_in: u64,
    pub huge_split: u64,
    pub protection: u64,
}

impl FaultTypeCounter {
    #[inline(always)]
    pub fn total(&self) -> u64 {
        self.minor + self.major + self.cow + self.demand_zero
            + self.swap_in + self.huge_split + self.protection
    }

    #[inline]
    pub fn record(&mut self, ft: PageFaultType) {
        match ft {
            PageFaultType::Minor => self.minor += 1,
            PageFaultType::Major => self.major += 1,
            PageFaultType::CopyOnWrite => self.cow += 1,
            PageFaultType::DemandZero => self.demand_zero += 1,
            PageFaultType::SwapIn => self.swap_in += 1,
            PageFaultType::HugePageSplit => self.huge_split += 1,
            PageFaultType::ProtectionFault => self.protection += 1,
        }
    }

    #[inline]
    pub fn major_ratio(&self) -> f64 {
        let total = self.total();
        if total == 0 { return 0.0; }
        self.major as f64 / total as f64
    }
}

/// Per-process page fault profile
#[derive(Debug, Clone)]
pub struct ProcessFaultProfile {
    pub pid: u64,
    pub counters: FaultTypeCounter,
    pub hotspots: BTreeMap<u64, FaultHotspot>,
    pub total_resolution_ns: u64,
    pub fault_rate_per_sec: f64,
    pub last_window_faults: u64,
    pub window_start_ts: u64,
    pub working_set_pages: u64,
    pub max_hotspots: usize,
}

impl ProcessFaultProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            counters: FaultTypeCounter::default(),
            hotspots: BTreeMap::new(),
            total_resolution_ns: 0,
            fault_rate_per_sec: 0.0,
            last_window_faults: 0,
            window_start_ts: 0,
            working_set_pages: 0,
            max_hotspots: 256,
        }
    }

    pub fn record_fault(&mut self, event: &PageFaultEvent) {
        self.counters.record(event.fault_type);
        self.total_resolution_ns += event.resolution_ns;

        let page = event.virtual_addr & !0xFFF;
        if let Some(hs) = self.hotspots.get_mut(&page) {
            hs.fault_count += 1;
            hs.last_fault_ts = event.timestamp;
            hs.total_resolution_ns += event.resolution_ns;
        } else {
            if self.hotspots.len() < self.max_hotspots {
                let mut hs = FaultHotspot::new(page, event.fault_type, event.access);
                hs.last_fault_ts = event.timestamp;
                hs.total_resolution_ns = event.resolution_ns;
                self.hotspots.insert(page, hs);
            }
        }

        self.last_window_faults += 1;
    }

    #[inline]
    pub fn update_rate(&mut self, now: u64) {
        let elapsed = now.saturating_sub(self.window_start_ts);
        if elapsed > 1_000_000_000 { // 1 second window
            self.fault_rate_per_sec = self.last_window_faults as f64
                / (elapsed as f64 / 1_000_000_000.0);
            self.last_window_faults = 0;
            self.window_start_ts = now;
        }
    }

    #[inline]
    pub fn avg_resolution_ns(&self) -> u64 {
        let total = self.counters.total();
        if total == 0 { return 0; }
        self.total_resolution_ns / total
    }

    #[inline]
    pub fn top_hotspots(&self, n: usize) -> Vec<&FaultHotspot> {
        let mut spots: Vec<_> = self.hotspots.values().collect();
        spots.sort_by(|a, b| b.fault_count.cmp(&a.fault_count));
        spots.truncate(n);
        spots
    }
}

/// App page fault profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppPageFaultProfilerStats {
    pub total_processes: usize,
    pub total_faults: u64,
    pub total_major: u64,
    pub total_minor: u64,
    pub total_cow: u64,
    pub avg_fault_rate: f64,
    pub hotspot_count: usize,
}

/// Application Page Fault Profiler
pub struct AppPageFaultProfiler {
    profiles: BTreeMap<u64, ProcessFaultProfile>,
    stats: AppPageFaultProfilerStats,
}

impl AppPageFaultProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppPageFaultProfilerStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessFaultProfile::new(pid));
    }

    #[inline]
    pub fn record_fault(&mut self, pid: u64, event: &PageFaultEvent) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_fault(event);
        }
    }

    #[inline]
    pub fn tick(&mut self, now: u64) {
        for profile in self.profiles.values_mut() {
            profile.update_rate(now);
        }
        self.recompute();
    }

    #[inline]
    pub fn high_fault_processes(&self, threshold: f64) -> Vec<u64> {
        self.profiles.values()
            .filter(|p| p.fault_rate_per_sec > threshold)
            .map(|p| p.pid)
            .collect()
    }

    fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_faults = self.profiles.values().map(|p| p.counters.total()).sum();
        self.stats.total_major = self.profiles.values().map(|p| p.counters.major).sum();
        self.stats.total_minor = self.profiles.values().map(|p| p.counters.minor).sum();
        self.stats.total_cow = self.profiles.values().map(|p| p.counters.cow).sum();
        self.stats.hotspot_count = self.profiles.values().map(|p| p.hotspots.len()).sum();

        let count = self.profiles.len();
        self.stats.avg_fault_rate = if count > 0 {
            self.profiles.values().map(|p| p.fault_rate_per_sec).sum::<f64>() / count as f64
        } else { 0.0 };
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessFaultProfile> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppPageFaultProfilerStats {
        &self.stats
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
