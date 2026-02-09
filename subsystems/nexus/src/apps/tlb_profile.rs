//! # Apps TLB Profile
//!
//! Translation Lookaside Buffer behavior profiling:
//! - TLB miss tracking per process
//! - Working set size estimation from TLB behavior
//! - Huge page eligibility detection
//! - PCID (Process Context ID) utilization
//! - ASID pressure monitoring

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TLB level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbLevel {
    /// Instruction TLB
    ITlb,
    /// Data TLB L1
    DTlb,
    /// Unified L2 TLB (STLB)
    Stlb,
}

/// Page size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbPageSize {
    /// 4KB standard
    Page4K,
    /// 2MB huge page
    Page2M,
    /// 1GB giant page
    Page1G,
}

/// TLB event
#[derive(Debug, Clone)]
pub struct TlbEvent {
    pub level: TlbLevel,
    pub page_size: TlbPageSize,
    pub is_miss: bool,
    pub address: u64,
    pub timestamp_ns: u64,
}

/// Per-process TLB profile
#[derive(Debug)]
pub struct ProcessTlbProfile {
    pub pid: u64,
    /// Misses per level
    itlb_misses: u64,
    dtlb_misses: u64,
    stlb_misses: u64,
    /// Hits per level
    itlb_hits: u64,
    dtlb_hits: u64,
    stlb_hits: u64,
    /// Miss rate EMA
    pub itlb_miss_rate: f64,
    pub dtlb_miss_rate: f64,
    pub stlb_miss_rate: f64,
    /// Distinct pages accessed (for WSS estimation)
    distinct_pages: LinearMap<u32, 64>,
    /// Huge page hits
    pub huge_page_hits: u64,
    /// Total accesses
    pub total_accesses: u64,
    /// PCID
    pub pcid: u16,
    /// PCID reuses
    pub pcid_reuses: u64,
}

impl ProcessTlbProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            itlb_misses: 0,
            dtlb_misses: 0,
            stlb_misses: 0,
            itlb_hits: 0,
            dtlb_hits: 0,
            stlb_hits: 0,
            itlb_miss_rate: 0.0,
            dtlb_miss_rate: 0.0,
            stlb_miss_rate: 0.0,
            distinct_pages: LinearMap::new(),
            huge_page_hits: 0,
            total_accesses: 0,
            pcid: 0,
            pcid_reuses: 0,
        }
    }

    /// Record TLB event
    pub fn record_event(&mut self, event: &TlbEvent) {
        self.total_accesses += 1;

        match (event.level, event.is_miss) {
            (TlbLevel::ITlb, true) => self.itlb_misses += 1,
            (TlbLevel::ITlb, false) => self.itlb_hits += 1,
            (TlbLevel::DTlb, true) => self.dtlb_misses += 1,
            (TlbLevel::DTlb, false) => self.dtlb_hits += 1,
            (TlbLevel::Stlb, true) => self.stlb_misses += 1,
            (TlbLevel::Stlb, false) => self.stlb_hits += 1,
        }

        if matches!(event.page_size, TlbPageSize::Page2M | TlbPageSize::Page1G) {
            self.huge_page_hits += 1;
        }

        // Track distinct pages for WSS
        let page_addr = event.address >> 12; // 4K-aligned
        self.distinct_pages.add(page_addr, 1);

        self.update_miss_rates();
    }

    fn update_miss_rates(&mut self) {
        let itlb_total = self.itlb_hits + self.itlb_misses;
        if itlb_total > 0 {
            let rate = self.itlb_misses as f64 / itlb_total as f64;
            self.itlb_miss_rate = 0.9 * self.itlb_miss_rate + 0.1 * rate;
        }
        let dtlb_total = self.dtlb_hits + self.dtlb_misses;
        if dtlb_total > 0 {
            let rate = self.dtlb_misses as f64 / dtlb_total as f64;
            self.dtlb_miss_rate = 0.9 * self.dtlb_miss_rate + 0.1 * rate;
        }
        let stlb_total = self.stlb_hits + self.stlb_misses;
        if stlb_total > 0 {
            let rate = self.stlb_misses as f64 / stlb_total as f64;
            self.stlb_miss_rate = 0.9 * self.stlb_miss_rate + 0.1 * rate;
        }
    }

    /// Working set size estimate (pages)
    #[inline(always)]
    pub fn wss_pages(&self) -> usize {
        self.distinct_pages.len()
    }

    /// Working set size estimate (bytes, assuming 4K pages)
    #[inline(always)]
    pub fn wss_bytes(&self) -> u64 {
        self.distinct_pages.len() as u64 * 4096
    }

    /// Should use huge pages?
    #[inline(always)]
    pub fn should_use_huge_pages(&self) -> bool {
        // High DTLB miss rate + large WSS
        self.dtlb_miss_rate > 0.05 && self.wss_bytes() > 4 * 1024 * 1024
    }

    /// Overall TLB pressure
    #[inline(always)]
    pub fn tlb_pressure(&self) -> f64 {
        (self.dtlb_miss_rate + self.itlb_miss_rate + self.stlb_miss_rate) / 3.0
    }

    /// Hot pages (most accessed)
    #[inline]
    pub fn hot_pages(&self, n: usize) -> Vec<(u64, u32)> {
        let mut pages: Vec<(u64, u32)> = self.distinct_pages.iter()
            .map(|(&addr, &count)| (addr, count))
            .collect();
        pages.sort_by(|a, b| b.1.cmp(&a.1));
        pages.truncate(n);
        pages
    }
}

/// TLB profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppTlbProfilerStats {
    pub tracked_processes: usize,
    pub total_tlb_misses: u64,
    pub avg_dtlb_miss_rate: f64,
    pub huge_page_candidates: usize,
    pub avg_wss_pages: f64,
}

/// App TLB profiler
pub struct AppTlbProfiler {
    processes: BTreeMap<u64, ProcessTlbProfile>,
    stats: AppTlbProfilerStats,
}

impl AppTlbProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppTlbProfilerStats::default(),
        }
    }

    #[inline]
    pub fn record_event(&mut self, pid: u64, event: &TlbEvent) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessTlbProfile::new(pid))
            .record_event(event);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_tlb_misses = self.processes.values()
            .map(|p| p.itlb_misses + p.dtlb_misses + p.stlb_misses)
            .sum();
        if !self.processes.is_empty() {
            self.stats.avg_dtlb_miss_rate = self.processes.values()
                .map(|p| p.dtlb_miss_rate)
                .sum::<f64>() / self.processes.len() as f64;
            self.stats.avg_wss_pages = self.processes.values()
                .map(|p| p.wss_pages() as f64)
                .sum::<f64>() / self.processes.len() as f64;
        }
        self.stats.huge_page_candidates = self.processes.values()
            .filter(|p| p.should_use_huge_pages())
            .count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppTlbProfilerStats {
        &self.stats
    }
}
