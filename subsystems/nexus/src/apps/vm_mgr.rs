//! # Apps Virtual Memory Manager
//!
//! Per-application virtual memory tracking and optimization:
//! - Page fault classification (major/minor/CoW/zero)
//! - Working set estimation per app
//! - madvise hint processing
//! - Transparent huge page promotion tracking
//! - Memory region lifecycle management
//! - NUMA-aware memory placement hints

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFaultType {
    Minor,
    Major,
    CopyOnWrite,
    ZeroPage,
    SwapIn,
    FileBacked,
    ProtectionViolation,
    DeviceFault,
}

/// Madvise hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadviseHint {
    Normal,
    Random,
    Sequential,
    WillNeed,
    DontNeed,
    Free,
    Remove,
    HugePage,
    NoHugePage,
    Mergeable,
    Unmergeable,
    DontFork,
    DoFork,
    Cold,
    PageOut,
}

/// Page fault record
#[derive(Debug, Clone)]
pub struct PageFaultRecord {
    pub address: u64,
    pub fault_type: PageFaultType,
    pub latency_ns: u64,
    pub timestamp_ns: u64,
}

/// Memory region descriptor
#[derive(Debug, Clone)]
pub struct AppMemRegion {
    pub start: u64,
    pub end: u64,
    pub resident_pages: u64,
    pub fault_count: u64,
    pub hint: MadviseHint,
    pub huge_page_eligible: bool,
    pub promoted_huge: u64,
    pub numa_node: Option<u32>,
    pub access_frequency: f64,
}

impl AppMemRegion {
    pub fn new(start: u64, end: u64) -> Self {
        Self {
            start, end,
            resident_pages: 0,
            fault_count: 0,
            hint: MadviseHint::Normal,
            huge_page_eligible: false,
            promoted_huge: 0,
            numa_node: None,
            access_frequency: 0.0,
        }
    }

    pub fn size(&self) -> u64 { self.end - self.start }
    pub fn page_count(&self) -> u64 { self.size() / 4096 }
    pub fn contains(&self, addr: u64) -> bool { addr >= self.start && addr < self.end }

    pub fn residency(&self) -> f64 {
        let pages = self.page_count();
        if pages == 0 { return 0.0; }
        self.resident_pages as f64 / pages as f64
    }
}

/// Working set estimation
#[derive(Debug, Clone)]
pub struct WorkingSetEstimate {
    pub sample_ns: u64,
    pub pages_accessed: u64,
    pub pages_resident: u64,
    pub estimated_wss_pages: u64,
    pub growth_rate: f64,
}

/// Per-app VM state
#[derive(Debug, Clone)]
pub struct AppVmState {
    pub process_id: u64,
    pub regions: Vec<AppMemRegion>,
    pub total_mapped_pages: u64,
    pub total_resident_pages: u64,
    pub fault_history: Vec<PageFaultRecord>,
    pub max_fault_history: usize,
    pub minor_faults: u64,
    pub major_faults: u64,
    pub cow_faults: u64,
    pub wss_estimates: Vec<WorkingSetEstimate>,
    pub peak_rss_pages: u64,
}

impl AppVmState {
    pub fn new(pid: u64, max_history: usize) -> Self {
        Self {
            process_id: pid,
            regions: Vec::new(),
            total_mapped_pages: 0,
            total_resident_pages: 0,
            fault_history: Vec::new(),
            max_fault_history: max_history,
            minor_faults: 0,
            major_faults: 0,
            cow_faults: 0,
            wss_estimates: Vec::new(),
            peak_rss_pages: 0,
        }
    }

    pub fn add_region(&mut self, region: AppMemRegion) {
        self.total_mapped_pages += region.page_count();
        self.regions.push(region);
    }

    pub fn record_fault(&mut self, addr: u64, fault_type: PageFaultType, latency: u64, ts: u64) {
        match fault_type {
            PageFaultType::Minor | PageFaultType::ZeroPage => self.minor_faults += 1,
            PageFaultType::Major | PageFaultType::SwapIn | PageFaultType::FileBacked => self.major_faults += 1,
            PageFaultType::CopyOnWrite => self.cow_faults += 1,
            _ => {}
        }

        if let Some(region) = self.regions.iter_mut().find(|r| r.contains(addr)) {
            region.fault_count += 1;
            region.resident_pages += 1;
        }

        self.fault_history.push(PageFaultRecord {
            address: addr, fault_type, latency_ns: latency, timestamp_ns: ts,
        });
        while self.fault_history.len() > self.max_fault_history {
            self.fault_history.remove(0);
        }
    }

    pub fn apply_madvise(&mut self, addr: u64, hint: MadviseHint) {
        if let Some(region) = self.regions.iter_mut().find(|r| r.contains(addr)) {
            region.hint = hint;
            match hint {
                MadviseHint::HugePage => region.huge_page_eligible = true,
                MadviseHint::NoHugePage => region.huge_page_eligible = false,
                MadviseHint::DontNeed => {
                    region.resident_pages = 0;
                }
                _ => {}
            }
        }
    }

    pub fn estimate_wss(&mut self, ts: u64) {
        // Simple WSS estimation: count recently faulted unique pages
        let window = 10_000_000_000u64; // 10 seconds
        let cutoff = ts.saturating_sub(window);
        let mut accessed = BTreeMap::new();
        for fault in self.fault_history.iter().rev() {
            if fault.timestamp_ns < cutoff { break; }
            let page = fault.address / 4096;
            *accessed.entry(page).or_insert(0u32) += 1;
        }
        let est = WorkingSetEstimate {
            sample_ns: ts,
            pages_accessed: accessed.len() as u64,
            pages_resident: self.total_resident_pages,
            estimated_wss_pages: accessed.len() as u64,
            growth_rate: 0.0,
        };
        self.wss_estimates.push(est);
        if self.wss_estimates.len() > 64 { self.wss_estimates.remove(0); }
    }

    pub fn update_totals(&mut self) {
        self.total_resident_pages = self.regions.iter().map(|r| r.resident_pages).sum();
        if self.total_resident_pages > self.peak_rss_pages {
            self.peak_rss_pages = self.total_resident_pages;
        }
    }

    pub fn fault_rate(&self, window_faults: u64, window_ns: u64) -> f64 {
        if window_ns == 0 { return 0.0; }
        window_faults as f64 / (window_ns as f64 / 1_000_000_000.0)
    }
}

/// Apps VM manager stats
#[derive(Debug, Clone, Default)]
pub struct AppsVmMgrStats {
    pub total_processes: usize,
    pub total_regions: usize,
    pub total_mapped_pages: u64,
    pub total_resident_pages: u64,
    pub total_minor_faults: u64,
    pub total_major_faults: u64,
}

/// Apps Virtual Memory Manager
pub struct AppsVmMgr {
    states: BTreeMap<u64, AppVmState>,
    stats: AppsVmMgrStats,
}

impl AppsVmMgr {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            stats: AppsVmMgrStats::default(),
        }
    }

    pub fn register(&mut self, pid: u64, max_history: usize) {
        self.states.entry(pid).or_insert_with(|| AppVmState::new(pid, max_history));
    }

    pub fn add_region(&mut self, pid: u64, region: AppMemRegion) {
        if let Some(state) = self.states.get_mut(&pid) { state.add_region(region); }
    }

    pub fn record_fault(&mut self, pid: u64, addr: u64, ftype: PageFaultType, latency: u64, ts: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.record_fault(addr, ftype, latency, ts);
        }
    }

    pub fn apply_madvise(&mut self, pid: u64, addr: u64, hint: MadviseHint) {
        if let Some(state) = self.states.get_mut(&pid) { state.apply_madvise(addr, hint); }
    }

    pub fn estimate_all_wss(&mut self, ts: u64) {
        for state in self.states.values_mut() { state.estimate_wss(ts); }
    }

    pub fn remove_process(&mut self, pid: u64) { self.states.remove(&pid); }

    pub fn recompute(&mut self) {
        self.stats.total_processes = self.states.len();
        self.stats.total_regions = self.states.values().map(|s| s.regions.len()).sum();
        self.stats.total_mapped_pages = self.states.values().map(|s| s.total_mapped_pages).sum();
        self.stats.total_resident_pages = self.states.values().map(|s| s.total_resident_pages).sum();
        self.stats.total_minor_faults = self.states.values().map(|s| s.minor_faults).sum();
        self.stats.total_major_faults = self.states.values().map(|s| s.major_faults).sum();
    }

    pub fn app_state(&self, pid: u64) -> Option<&AppVmState> { self.states.get(&pid) }
    pub fn stats(&self) -> &AppsVmMgrStats { &self.stats }
}
