//! # Holistic Page Table Manager
//!
//! Multi-level page table management and optimization:
//! - 4-level page table tracking (PML4/PDPT/PD/PT)
//! - Large page (2MB/1GB) coalescing detection
//! - Page table memory accounting
//! - THP (Transparent Huge Page) promotion/demotion
//! - PCID (Process Context ID) management
//! - Page table sharing between processes

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page table level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PtLevel {
    Pml4,   // Level 4: 512GB per entry
    Pdpt,   // Level 3: 1GB per entry
    Pd,     // Level 2: 2MB per entry
    Pt,     // Level 1: 4KB per entry
}

impl PtLevel {
    #[inline]
    pub fn entry_coverage_bytes(&self) -> u64 {
        match self {
            PtLevel::Pml4 => 512 * 1024 * 1024 * 1024,
            PtLevel::Pdpt => 1024 * 1024 * 1024,
            PtLevel::Pd => 2 * 1024 * 1024,
            PtLevel::Pt => 4096,
        }
    }

    #[inline(always)]
    pub fn entries_per_table(&self) -> usize { 512 }
}

/// Page table entry flags
#[derive(Debug, Clone, Copy)]
pub struct PteFlags {
    pub present: bool,
    pub writable: bool,
    pub user: bool,
    pub write_through: bool,
    pub cache_disable: bool,
    pub accessed: bool,
    pub dirty: bool,
    pub huge: bool,
    pub global: bool,
    pub no_execute: bool,
}

impl PteFlags {
    #[inline]
    pub fn empty() -> Self {
        Self {
            present: false, writable: false, user: false,
            write_through: false, cache_disable: false,
            accessed: false, dirty: false, huge: false,
            global: false, no_execute: false,
        }
    }

    #[inline(always)]
    pub fn kernel_rw() -> Self {
        Self { present: true, writable: true, no_execute: true, ..Self::empty() }
    }

    #[inline(always)]
    pub fn user_ro() -> Self {
        Self { present: true, user: true, no_execute: true, ..Self::empty() }
    }

    #[inline(always)]
    pub fn user_rw() -> Self {
        Self { present: true, writable: true, user: true, no_execute: true, ..Self::empty() }
    }
}

/// Page table page tracking
#[derive(Debug, Clone)]
pub struct PageTablePage {
    pub phys_addr: u64,
    pub level: PtLevel,
    pub owner_pid: u32,
    pub entries_present: u16,
    pub entries_total: u16,
    pub shared: bool,
    pub share_count: u32,
}

impl PageTablePage {
    pub fn new(addr: u64, level: PtLevel, pid: u32) -> Self {
        Self {
            phys_addr: addr, level, owner_pid: pid,
            entries_present: 0, entries_total: 512,
            shared: false, share_count: 1,
        }
    }

    #[inline(always)]
    pub fn occupancy(&self) -> f64 {
        if self.entries_total == 0 { return 0.0; }
        self.entries_present as f64 / self.entries_total as f64
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.entries_present == 0 }
    #[inline(always)]
    pub fn is_full(&self) -> bool { self.entries_present >= self.entries_total }
}

/// THP (Transparent Huge Page) candidate
#[derive(Debug, Clone)]
pub struct ThpCandidate {
    pub vaddr: u64,
    pub pid: u32,
    pub small_pages_present: u32,
    pub total_possible: u32,
    pub all_same_prot: bool,
    pub promotion_score: f64,
}

impl ThpCandidate {
    #[inline(always)]
    pub fn coverage_ratio(&self) -> f64 {
        if self.total_possible == 0 { return 0.0; }
        self.small_pages_present as f64 / self.total_possible as f64
    }

    #[inline(always)]
    pub fn is_promotable(&self) -> bool {
        self.small_pages_present == self.total_possible && self.all_same_prot
    }
}

/// PCID assignment
#[derive(Debug, Clone)]
pub struct PcidEntry {
    pub pcid: u16,
    pub pid: u32,
    pub assigned_ts: u64,
    pub context_switches: u64,
    pub tlb_flushes_avoided: u64,
}

/// Per-process page table state
#[derive(Debug, Clone)]
pub struct ProcessPageTable {
    pub pid: u32,
    pub cr3_phys: u64,
    pub pcid: Option<u16>,
    pub total_pt_pages: u32,
    pub total_mapped_pages: u64,
    pub huge_pages_2mb: u32,
    pub huge_pages_1gb: u32,
    pub shared_pt_pages: u32,
    pub pt_memory_bytes: u64,
}

impl ProcessPageTable {
    pub fn new(pid: u32, cr3: u64) -> Self {
        Self {
            pid, cr3_phys: cr3, pcid: None, total_pt_pages: 0,
            total_mapped_pages: 0, huge_pages_2mb: 0, huge_pages_1gb: 0,
            shared_pt_pages: 0, pt_memory_bytes: 0,
        }
    }

    #[inline]
    pub fn overhead_ratio(&self) -> f64 {
        if self.total_mapped_pages == 0 { return 0.0; }
        let mapped_bytes = self.total_mapped_pages * 4096;
        self.pt_memory_bytes as f64 / mapped_bytes as f64
    }
}

/// Page table manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PgtableMgrStats {
    pub total_processes: usize,
    pub total_pt_pages: u64,
    pub total_pt_memory_bytes: u64,
    pub total_huge_2mb: u64,
    pub total_huge_1gb: u64,
    pub thp_candidates: usize,
    pub shared_pt_pages: usize,
    pub pcids_assigned: usize,
    pub avg_occupancy: f64,
    pub avg_pt_overhead: f64,
}

/// Holistic page table manager
pub struct HolisticPgtableMgr {
    processes: BTreeMap<u32, ProcessPageTable>,
    pt_pages: BTreeMap<u64, PageTablePage>,
    pcids: BTreeMap<u16, PcidEntry>,
    thp_candidates: Vec<ThpCandidate>,
    next_pcid: u16,
    max_pcid: u16,
    stats: PgtableMgrStats,
}

impl HolisticPgtableMgr {
    pub fn new(max_pcid: u16) -> Self {
        Self {
            processes: BTreeMap::new(), pt_pages: BTreeMap::new(),
            pcids: BTreeMap::new(), thp_candidates: Vec::new(),
            next_pcid: 1, max_pcid, stats: PgtableMgrStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u32, cr3: u64) {
        self.processes.insert(pid, ProcessPageTable::new(pid, cr3));
    }

    #[inline]
    pub fn add_pt_page(&mut self, addr: u64, level: PtLevel, pid: u32, present: u16) {
        let mut page = PageTablePage::new(addr, level, pid);
        page.entries_present = present;
        self.pt_pages.insert(addr, page);
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.total_pt_pages += 1;
            proc.pt_memory_bytes += 4096;
        }
    }

    #[inline]
    pub fn assign_pcid(&mut self, pid: u32, ts: u64) -> Option<u16> {
        if self.next_pcid > self.max_pcid { return None; }
        let pcid = self.next_pcid; self.next_pcid += 1;
        self.pcids.insert(pcid, PcidEntry {
            pcid, pid, assigned_ts: ts, context_switches: 0, tlb_flushes_avoided: 0,
        });
        if let Some(proc) = self.processes.get_mut(&pid) { proc.pcid = Some(pcid); }
        Some(pcid)
    }

    #[inline]
    pub fn mark_shared(&mut self, addr: u64, count: u32) {
        if let Some(page) = self.pt_pages.get_mut(&addr) {
            page.shared = true;
            page.share_count = count;
        }
    }

    pub fn scan_thp_candidates(&mut self) {
        self.thp_candidates.clear();
        // Simplified: look for PD entries with many present PT entries
        for (_, page) in &self.pt_pages {
            if page.level == PtLevel::Pd && page.entries_present >= 400 {
                self.thp_candidates.push(ThpCandidate {
                    vaddr: page.phys_addr, pid: page.owner_pid,
                    small_pages_present: page.entries_present as u32,
                    total_possible: 512, all_same_prot: true,
                    promotion_score: page.occupancy(),
                });
            }
        }
    }

    #[inline]
    pub fn record_huge_page(&mut self, pid: u32, is_1gb: bool) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            if is_1gb { proc.huge_pages_1gb += 1; } else { proc.huge_pages_2mb += 1; }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_processes = self.processes.len();
        self.stats.total_pt_pages = self.pt_pages.len() as u64;
        self.stats.total_pt_memory_bytes = self.stats.total_pt_pages * 4096;
        self.stats.total_huge_2mb = self.processes.values().map(|p| p.huge_pages_2mb as u64).sum();
        self.stats.total_huge_1gb = self.processes.values().map(|p| p.huge_pages_1gb as u64).sum();
        self.stats.thp_candidates = self.thp_candidates.len();
        self.stats.shared_pt_pages = self.pt_pages.values().filter(|p| p.shared).count();
        self.stats.pcids_assigned = self.pcids.len();
        if !self.pt_pages.is_empty() {
            self.stats.avg_occupancy = self.pt_pages.values().map(|p| p.occupancy()).sum::<f64>() / self.pt_pages.len() as f64;
        }
        if !self.processes.is_empty() {
            self.stats.avg_pt_overhead = self.processes.values().map(|p| p.overhead_ratio()).sum::<f64>() / self.processes.len() as f64;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &PgtableMgrStats { &self.stats }
}
