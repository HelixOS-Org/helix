// SPDX-License-Identifier: GPL-2.0
//! Holistic hugepage_v2 — huge page management v2.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Huge page size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageSize {
    Size2M,
    Size1G,
}

impl HugePageSize {
    pub fn bytes(self) -> u64 { match self { Self::Size2M => 2 * 1024 * 1024, Self::Size1G => 1024 * 1024 * 1024 } }
}

/// THP policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThpPolicy {
    Always,
    Madvise,
    Never,
    Defer,
    DeferMadvise,
}

/// Huge page pool
#[derive(Debug)]
pub struct HugePagePool {
    pub size: HugePageSize,
    pub total: u64,
    pub free: u64,
    pub reserved: u64,
    pub surplus: u64,
    pub alloc_count: u64,
    pub free_count: u64,
    pub fail_count: u64,
}

impl HugePagePool {
    pub fn new(size: HugePageSize, total: u64) -> Self {
        Self { size, total, free: total, reserved: 0, surplus: 0, alloc_count: 0, free_count: 0, fail_count: 0 }
    }

    pub fn alloc(&mut self) -> Option<u64> {
        if self.free > 0 { self.free -= 1; self.alloc_count += 1; Some(self.alloc_count) }
        else { self.fail_count += 1; None }
    }

    pub fn free(&mut self) { self.free += 1; self.free_count += 1; }
    pub fn utilization(&self) -> f64 { if self.total == 0 { 0.0 } else { (self.total - self.free) as f64 / self.total as f64 } }
}

/// THP collapse event
#[derive(Debug, Clone)]
pub struct ThpCollapseEvent {
    pub pid: u64,
    pub addr: u64,
    pub success: bool,
    pub timestamp: u64,
    pub scan_cost_ns: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct HugePageV2Stats {
    pub pools: u32,
    pub total_2m: u64,
    pub free_2m: u64,
    pub total_1g: u64,
    pub free_1g: u64,
    pub thp_collapses: u64,
    pub thp_failures: u64,
    pub thp_policy: u8,
}

/// Main huge page v2 manager
pub struct HolisticHugePageV2 {
    pools: BTreeMap<u32, HugePagePool>,
    thp_policy: ThpPolicy,
    thp_events: Vec<ThpCollapseEvent>,
    next_pool_id: u32,
}

impl HolisticHugePageV2 {
    pub fn new() -> Self { Self { pools: BTreeMap::new(), thp_policy: ThpPolicy::Madvise, thp_events: Vec::new(), next_pool_id: 1 } }

    pub fn add_pool(&mut self, size: HugePageSize, count: u64) -> u32 {
        let id = self.next_pool_id; self.next_pool_id += 1;
        self.pools.insert(id, HugePagePool::new(size, count));
        id
    }

    pub fn set_thp_policy(&mut self, policy: ThpPolicy) { self.thp_policy = policy; }

    pub fn alloc(&mut self, pool: u32) -> Option<u64> {
        self.pools.get_mut(&pool)?.alloc()
    }

    pub fn record_thp(&mut self, event: ThpCollapseEvent) {
        if self.thp_events.len() >= 4096 { self.thp_events.drain(..2048); }
        self.thp_events.push(event);
    }

    pub fn stats(&self) -> HugePageV2Stats {
        let total_2m: u64 = self.pools.values().filter(|p| p.size == HugePageSize::Size2M).map(|p| p.total).sum();
        let free_2m: u64 = self.pools.values().filter(|p| p.size == HugePageSize::Size2M).map(|p| p.free).sum();
        let total_1g: u64 = self.pools.values().filter(|p| p.size == HugePageSize::Size1G).map(|p| p.total).sum();
        let free_1g: u64 = self.pools.values().filter(|p| p.size == HugePageSize::Size1G).map(|p| p.free).sum();
        let collapses = self.thp_events.iter().filter(|e| e.success).count() as u64;
        let failures = self.thp_events.iter().filter(|e| !e.success).count() as u64;
        HugePageV2Stats { pools: self.pools.len() as u32, total_2m, free_2m, total_1g, free_1g, thp_collapses: collapses, thp_failures: failures, thp_policy: self.thp_policy as u8 }
    }
}

// ============================================================================
// Merged from hugepage_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HugePageV3Size {
    Page2M,
    Page1G,
    Page16G,
    PageCustom(u64),
}

impl HugePageV3Size {
    pub fn bytes(&self) -> u64 {
        match self {
            Self::Page2M => 2 * 1024 * 1024,
            Self::Page1G => 1024 * 1024 * 1024,
            Self::Page16G => 16 * 1024 * 1024 * 1024,
            Self::PageCustom(s) => *s,
        }
    }
}

/// State of a CMA region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmaRegionState {
    Free,
    Allocated,
    Migrating,
    Compacting,
    Reserved,
    Faulty,
}

/// CMA region descriptor.
#[derive(Debug, Clone)]
pub struct CmaRegion {
    pub base_pfn: u64,
    pub page_count: u64,
    pub state: CmaRegionState,
    pub numa_node: u32,
    pub alignment_order: u32,
    pub owner_pid: Option<u64>,
    pub alloc_timestamp: u64,
    pub migration_cost: u64,
}

impl CmaRegion {
    pub fn new(base_pfn: u64, page_count: u64, numa_node: u32) -> Self {
        Self {
            base_pfn,
            page_count,
            state: CmaRegionState::Free,
            numa_node,
            alignment_order: 0,
            owner_pid: None,
            alloc_timestamp: 0,
            migration_cost: 0,
        }
    }

    pub fn size_bytes(&self) -> u64 {
        self.page_count * 4096
    }

    pub fn is_available(&self) -> bool {
        self.state == CmaRegionState::Free
    }
}

/// Per-NUMA-node huge page pool.
#[derive(Debug, Clone)]
pub struct NumaHugePagePool {
    pub node_id: u32,
    pub page_size: HugePageV3Size,
    pub total_pages: u64,
    pub free_pages: u64,
    pub reserved_pages: u64,
    pub surplus_pages: u64,
    pub alloc_failures: u64,
    pub compaction_count: u64,
}

impl NumaHugePagePool {
    pub fn new(node_id: u32, page_size: HugePageV3Size) -> Self {
        Self {
            node_id,
            page_size,
            total_pages: 0,
            free_pages: 0,
            reserved_pages: 0,
            surplus_pages: 0,
            alloc_failures: 0,
            compaction_count: 0,
        }
    }

    pub fn try_alloc(&mut self) -> bool {
        if self.free_pages > 0 {
            self.free_pages -= 1;
            true
        } else if self.surplus_pages > 0 {
            self.surplus_pages -= 1;
            true
        } else {
            self.alloc_failures += 1;
            false
        }
    }

    pub fn free_page(&mut self) {
        self.free_pages += 1;
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.total_pages == 0 {
            return 0.0;
        }
        let used = self.total_pages - self.free_pages;
        (used as f64 / self.total_pages as f64) * 100.0
    }
}

/// Migration candidate for compaction.
#[derive(Debug, Clone)]
pub struct MigrationCandidate {
    pub source_pfn: u64,
    pub dest_pfn: u64,
    pub page_count: u64,
    pub estimated_cost: u64,
    pub priority: u32,
    pub numa_node: u32,
}

/// Statistics for huge page V3 manager.
#[derive(Debug, Clone)]
pub struct HugePageV3Stats {
    pub total_pools: u64,
    pub total_cma_regions: u64,
    pub alloc_success: u64,
    pub alloc_failures: u64,
    pub compactions_run: u64,
    pub pages_migrated: u64,
    pub fragmentation_score: f64,
    pub total_memory_managed: u64,
}

/// Main holistic huge page V3 manager.
pub struct HolisticHugePageV3 {
    pub pools: BTreeMap<u64, NumaHugePagePool>,
    pub cma_regions: BTreeMap<u64, CmaRegion>,
    pub migration_queue: Vec<MigrationCandidate>,
    pub next_region_id: u64,
    pub stats: HugePageV3Stats,
}

impl HolisticHugePageV3 {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            cma_regions: BTreeMap::new(),
            migration_queue: Vec::new(),
            next_region_id: 1,
            stats: HugePageV3Stats {
                total_pools: 0,
                total_cma_regions: 0,
                alloc_success: 0,
                alloc_failures: 0,
                compactions_run: 0,
                pages_migrated: 0,
                fragmentation_score: 0.0,
                total_memory_managed: 0,
            },
        }
    }

    pub fn create_pool(&mut self, node_id: u32, page_size: HugePageV3Size, count: u64) -> u64 {
        let key = (node_id as u64) << 32 | (page_size.bytes() & 0xFFFFFFFF);
        let mut pool = NumaHugePagePool::new(node_id, page_size);
        pool.total_pages = count;
        pool.free_pages = count;
        self.pools.insert(key, pool);
        self.stats.total_pools += 1;
        self.stats.total_memory_managed += count * page_size.bytes();
        key
    }

    pub fn register_cma_region(&mut self, base_pfn: u64, page_count: u64, numa_node: u32) -> u64 {
        let id = self.next_region_id;
        self.next_region_id += 1;
        let region = CmaRegion::new(base_pfn, page_count, numa_node);
        self.cma_regions.insert(id, region);
        self.stats.total_cma_regions += 1;
        id
    }

    pub fn enqueue_migration(&mut self, candidate: MigrationCandidate) {
        self.migration_queue.push(candidate);
        self.migration_queue.sort_by(|a, b| a.estimated_cost.cmp(&b.estimated_cost));
    }

    pub fn run_compaction(&mut self) -> u64 {
        let mut migrated = 0u64;
        let queue = core::mem::take(&mut self.migration_queue);
        for candidate in &queue {
            if let Some(region) = self.cma_regions.get_mut(&candidate.source_pfn) {
                if region.state == CmaRegionState::Free {
                    region.state = CmaRegionState::Migrating;
                    migrated += candidate.page_count;
                }
            }
        }
        self.stats.compactions_run += 1;
        self.stats.pages_migrated += migrated;
        migrated
    }

    pub fn compute_fragmentation(&self) -> f64 {
        let total = self.cma_regions.len() as f64;
        if total == 0.0 {
            return 0.0;
        }
        let free = self
            .cma_regions
            .values()
            .filter(|r| r.state == CmaRegionState::Free)
            .count() as f64;
        1.0 - (free / total)
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    pub fn region_count(&self) -> usize {
        self.cma_regions.len()
    }
}
