//! # Holistic Hugepage Manager
//!
//! Hugepage allocation and management with holistic awareness:
//! - Transparent huge pages (THP) state tracking
//! - Explicit hugetlb pool management (2MB, 1GB)
//! - Compaction and defragmentation for hugepage availability
//! - Per-NUMA node hugepage accounting
//! - Reservation tracking (surplus/overcommit)
//! - Khugepaged activity monitoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Hugepage size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageSize {
    Size2MB,
    Size1GB,
    Size16KB,
    Size64KB,
    Custom(u64),
}

impl HugePageSize {
    pub fn bytes(&self) -> u64 {
        match self {
            Self::Size2MB => 2 * 1024 * 1024,
            Self::Size1GB => 1024 * 1024 * 1024,
            Self::Size16KB => 16 * 1024,
            Self::Size64KB => 64 * 1024,
            Self::Custom(s) => *s,
        }
    }
}

/// THP state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThpMode {
    Always,
    Madvise,
    Never,
}

/// THP defrag policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThpDefrag {
    Always,
    Defer,
    DeferMadvise,
    Madvise,
    Never,
}

/// Per-NUMA node hugepage pool
#[derive(Debug, Clone)]
pub struct NodeHugePool {
    pub node: u32,
    pub size: HugePageSize,
    pub total: u64,
    pub free: u64,
    pub reserved: u64,
    pub surplus: u64,
    pub allocated: u64,
}

impl NodeHugePool {
    pub fn new(node: u32, size: HugePageSize, total: u64) -> Self {
        Self { node, size, total, free: total, reserved: 0, surplus: 0, allocated: 0 }
    }

    pub fn allocate(&mut self) -> bool {
        if self.free > 0 { self.free -= 1; self.allocated += 1; true } else { false }
    }

    pub fn free_page(&mut self) {
        if self.allocated > 0 { self.allocated -= 1; self.free += 1; }
    }

    pub fn reserve(&mut self) -> bool {
        if self.free > self.reserved { self.reserved += 1; true } else { false }
    }

    pub fn unreserve(&mut self) { self.reserved = self.reserved.saturating_sub(1); }
    pub fn usage(&self) -> f64 { if self.total == 0 { 0.0 } else { self.allocated as f64 / self.total as f64 } }
}

/// THP statistics
#[derive(Debug, Clone, Default)]
pub struct ThpStats {
    pub anon_thp: u64,
    pub file_thp: u64,
    pub split_count: u64,
    pub collapse_count: u64,
    pub fault_alloc: u64,
    pub fault_fallback: u64,
    pub compaction_success: u64,
    pub compaction_fail: u64,
    pub khugepaged_scans: u64,
    pub khugepaged_collapsed: u64,
    pub zero_page_alloc: u64,
}

/// Compaction request
#[derive(Debug, Clone)]
pub struct CompactionRequest {
    pub id: u64,
    pub zone_id: u64,
    pub node: u32,
    pub order: u32,
    pub ts: u64,
    pub success: bool,
    pub pages_moved: u64,
    pub latency_ns: u64,
}

/// Hugepage allocation record
#[derive(Debug, Clone)]
pub struct HugeAllocRecord {
    pub pid: u64,
    pub vaddr: u64,
    pub size: HugePageSize,
    pub node: u32,
    pub ts: u64,
    pub is_thp: bool,
}

/// Hugepage manager stats
#[derive(Debug, Clone, Default)]
pub struct HugePageStats {
    pub total_2mb: u64,
    pub free_2mb: u64,
    pub total_1gb: u64,
    pub free_1gb: u64,
    pub thp_active: u64,
    pub reservations: u64,
    pub surplus_total: u64,
    pub compactions: u64,
    pub alloc_failures: u64,
}

/// Holistic hugepage manager
pub struct HolisticHugepageMgr {
    pools: BTreeMap<u64, NodeHugePool>,
    thp_mode: ThpMode,
    thp_defrag: ThpDefrag,
    thp_stats: ThpStats,
    compactions: Vec<CompactionRequest>,
    alloc_history: Vec<HugeAllocRecord>,
    stats: HugePageStats,
    next_pool_id: u64,
    next_compact_id: u64,
}

impl HolisticHugepageMgr {
    pub fn new(thp_mode: ThpMode, defrag: ThpDefrag) -> Self {
        Self {
            pools: BTreeMap::new(), thp_mode, thp_defrag: defrag,
            thp_stats: ThpStats::default(), compactions: Vec::new(),
            alloc_history: Vec::new(), stats: HugePageStats::default(),
            next_pool_id: 1, next_compact_id: 1,
        }
    }

    pub fn add_pool(&mut self, node: u32, size: HugePageSize, total: u64) -> u64 {
        let id = self.next_pool_id; self.next_pool_id += 1;
        self.pools.insert(id, NodeHugePool::new(node, size, total));
        id
    }

    pub fn allocate(&mut self, pool_id: u64, pid: u64, vaddr: u64, ts: u64) -> bool {
        if let Some(p) = self.pools.get_mut(&pool_id) {
            if p.allocate() {
                self.alloc_history.push(HugeAllocRecord { pid, vaddr, size: p.size, node: p.node, ts, is_thp: false });
                return true;
            }
            self.stats.alloc_failures += 1;
        }
        false
    }

    pub fn free_page(&mut self, pool_id: u64) {
        if let Some(p) = self.pools.get_mut(&pool_id) { p.free_page(); }
    }

    pub fn thp_fault(&mut self, success: bool) {
        if success { self.thp_stats.fault_alloc += 1; } else { self.thp_stats.fault_fallback += 1; }
    }

    pub fn thp_collapse(&mut self) { self.thp_stats.collapse_count += 1; self.thp_stats.khugepaged_collapsed += 1; }
    pub fn thp_split(&mut self) { self.thp_stats.split_count += 1; }

    pub fn request_compaction(&mut self, zone: u64, node: u32, order: u32, ts: u64) -> u64 {
        let id = self.next_compact_id; self.next_compact_id += 1;
        self.compactions.push(CompactionRequest { id, zone_id: zone, node, order, ts, success: false, pages_moved: 0, latency_ns: 0 });
        self.stats.compactions += 1;
        id
    }

    pub fn complete_compaction(&mut self, id: u64, success: bool, pages: u64, latency: u64) {
        if let Some(c) = self.compactions.iter_mut().find(|c| c.id == id) {
            c.success = success;
            c.pages_moved = pages;
            c.latency_ns = latency;
        }
        if success { self.thp_stats.compaction_success += 1; } else { self.thp_stats.compaction_fail += 1; }
    }

    pub fn set_thp_mode(&mut self, mode: ThpMode) { self.thp_mode = mode; }
    pub fn set_defrag(&mut self, defrag: ThpDefrag) { self.thp_defrag = defrag; }

    pub fn recompute(&mut self) {
        self.stats.total_2mb = self.pools.values().filter(|p| matches!(p.size, HugePageSize::Size2MB)).map(|p| p.total).sum();
        self.stats.free_2mb = self.pools.values().filter(|p| matches!(p.size, HugePageSize::Size2MB)).map(|p| p.free).sum();
        self.stats.total_1gb = self.pools.values().filter(|p| matches!(p.size, HugePageSize::Size1GB)).map(|p| p.total).sum();
        self.stats.free_1gb = self.pools.values().filter(|p| matches!(p.size, HugePageSize::Size1GB)).map(|p| p.free).sum();
        self.stats.reservations = self.pools.values().map(|p| p.reserved).sum();
        self.stats.surplus_total = self.pools.values().map(|p| p.surplus).sum();
        self.stats.thp_active = self.thp_stats.fault_alloc.saturating_sub(self.thp_stats.split_count);
    }

    pub fn pool(&self, id: u64) -> Option<&NodeHugePool> { self.pools.get(&id) }
    pub fn thp_stats(&self) -> &ThpStats { &self.thp_stats }
    pub fn thp_mode(&self) -> ThpMode { self.thp_mode }
    pub fn stats(&self) -> &HugePageStats { &self.stats }
}
