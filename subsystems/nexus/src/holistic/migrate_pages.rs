//! # Holistic Page Migration Manager
//!
//! Page migration across NUMA nodes and memory tiers:
//! - Access frequency-based migration decisions
//! - NUMA node affinity tracking
//! - Hot/cold page classification
//! - Batch migration scheduling
//! - Migration cost modeling
//! - Demotion to slower memory tiers

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Memory tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryTier {
    FastDram,    // HBM or close NUMA
    LocalDram,   // Local NUMA DRAM
    RemoteDram,  // Remote NUMA DRAM
    Pmem,        // Persistent memory (Intel Optane-like)
    SwapBacked,  // Swap-backed tier
}

impl MemoryTier {
    #[inline]
    pub fn latency_ns(&self) -> u64 {
        match self {
            MemoryTier::FastDram => 50,
            MemoryTier::LocalDram => 100,
            MemoryTier::RemoteDram => 300,
            MemoryTier::Pmem => 500,
            MemoryTier::SwapBacked => 10_000,
        }
    }

    #[inline]
    pub fn bandwidth_gbps(&self) -> f64 {
        match self {
            MemoryTier::FastDram => 200.0,
            MemoryTier::LocalDram => 50.0,
            MemoryTier::RemoteDram => 20.0,
            MemoryTier::Pmem => 8.0,
            MemoryTier::SwapBacked => 0.5,
        }
    }
}

/// Page hotness classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageHotness {
    Hot,
    Warm,
    Cold,
    Frozen,
}

/// Tracked page for migration
#[derive(Debug, Clone)]
pub struct TrackedPage {
    pub pfn: u64,
    pub current_node: u32,
    pub current_tier: MemoryTier,
    pub access_count: u64,
    pub last_access_ts: u64,
    pub owner_pid: u32,
    pub hotness: PageHotness,
    pub migration_count: u32,
    pub last_migration_ts: u64,
}

impl TrackedPage {
    pub fn new(pfn: u64, node: u32, tier: MemoryTier, pid: u32, ts: u64) -> Self {
        Self {
            pfn, current_node: node, current_tier: tier,
            access_count: 0, last_access_ts: ts, owner_pid: pid,
            hotness: PageHotness::Cold, migration_count: 0, last_migration_ts: 0,
        }
    }

    #[inline(always)]
    pub fn record_access(&mut self, ts: u64) {
        self.access_count += 1;
        self.last_access_ts = ts;
    }

    pub fn classify(&mut self, now: u64, hot_threshold: u64, warm_threshold: u64) {
        let age = now.saturating_sub(self.last_access_ts);
        if age < 1_000_000_000 && self.access_count > hot_threshold {
            self.hotness = PageHotness::Hot;
        } else if age < 5_000_000_000 && self.access_count > warm_threshold {
            self.hotness = PageHotness::Warm;
        } else if age < 30_000_000_000 {
            self.hotness = PageHotness::Cold;
        } else {
            self.hotness = PageHotness::Frozen;
        }
    }

    #[inline]
    pub fn migration_cost(&self, target_tier: MemoryTier) -> f64 {
        let bw = target_tier.bandwidth_gbps();
        if bw <= 0.0 { return f64::MAX; }
        // Cost in ns for 4KB page
        (4096.0 / (bw * 1e9 / 8.0)) * 1e9
    }
}

/// Migration request
#[derive(Debug, Clone)]
pub struct MigrationRequest {
    pub request_id: u64,
    pub pfn: u64,
    pub from_node: u32,
    pub to_node: u32,
    pub from_tier: MemoryTier,
    pub to_tier: MemoryTier,
    pub reason: MigrationReason,
    pub submitted_ts: u64,
    pub completed_ts: Option<u64>,
    pub success: bool,
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    NumaRebalance,
    HotPromotion,
    ColdDemotion,
    CapacityPressure,
    AffinityChange,
    Compaction,
}

/// Per-node migration state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NodeMigrationState {
    pub node_id: u32,
    pub tier: MemoryTier,
    pub total_pages: u64,
    pub hot_pages: u64,
    pub cold_pages: u64,
    pub pages_migrated_in: u64,
    pub pages_migrated_out: u64,
    pub capacity_pages: u64,
}

impl NodeMigrationState {
    pub fn new(id: u32, tier: MemoryTier, capacity: u64) -> Self {
        Self {
            node_id: id, tier, total_pages: 0, hot_pages: 0,
            cold_pages: 0, pages_migrated_in: 0, pages_migrated_out: 0,
            capacity_pages: capacity,
        }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.capacity_pages == 0 { return 0.0; }
        self.total_pages as f64 / self.capacity_pages as f64
    }

    #[inline(always)]
    pub fn hot_ratio(&self) -> f64 {
        if self.total_pages == 0 { return 0.0; }
        self.hot_pages as f64 / self.total_pages as f64
    }
}

/// Migrate pages stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MigratePageStats {
    pub tracked_pages: usize,
    pub total_migrations: u64,
    pub successful_migrations: u64,
    pub failed_migrations: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub total_nodes: usize,
    pub avg_migration_latency_ns: f64,
    pub hot_pages: usize,
    pub cold_pages: usize,
}

/// Holistic page migration manager
pub struct HolisticMigratePages {
    pages: BTreeMap<u64, TrackedPage>,
    nodes: BTreeMap<u32, NodeMigrationState>,
    requests: Vec<MigrationRequest>,
    next_request_id: u64,
    hot_threshold: u64,
    warm_threshold: u64,
    migration_rate_limit: u64,
    stats: MigratePageStats,
}

impl HolisticMigratePages {
    pub fn new() -> Self {
        Self {
            pages: BTreeMap::new(), nodes: BTreeMap::new(),
            requests: Vec::new(), next_request_id: 1,
            hot_threshold: 100, warm_threshold: 10,
            migration_rate_limit: 1000,
            stats: MigratePageStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_node(&mut self, id: u32, tier: MemoryTier, capacity: u64) {
        self.nodes.insert(id, NodeMigrationState::new(id, tier, capacity));
    }

    #[inline(always)]
    pub fn track_page(&mut self, pfn: u64, node: u32, tier: MemoryTier, pid: u32, ts: u64) {
        self.pages.insert(pfn, TrackedPage::new(pfn, node, tier, pid, ts));
        if let Some(n) = self.nodes.get_mut(&node) { n.total_pages += 1; }
    }

    #[inline(always)]
    pub fn record_access(&mut self, pfn: u64, ts: u64) {
        if let Some(page) = self.pages.get_mut(&pfn) { page.record_access(ts); }
    }

    pub fn classify_all(&mut self, now: u64) {
        for page in self.pages.values_mut() {
            page.classify(now, self.hot_threshold, self.warm_threshold);
        }
        // Update node counts
        for node in self.nodes.values_mut() { node.hot_pages = 0; node.cold_pages = 0; }
        for page in self.pages.values() {
            if let Some(n) = self.nodes.get_mut(&page.current_node) {
                match page.hotness {
                    PageHotness::Hot => n.hot_pages += 1,
                    PageHotness::Cold | PageHotness::Frozen => n.cold_pages += 1,
                    _ => {}
                }
            }
        }
    }

    pub fn suggest_migrations(&self) -> Vec<(u64, u32, MigrationReason)> {
        let mut suggestions = Vec::new();
        for page in self.pages.values() {
            match page.hotness {
                PageHotness::Hot if page.current_tier != MemoryTier::FastDram && page.current_tier != MemoryTier::LocalDram => {
                    // Suggest promotion
                    if let Some(target_node) = self.find_fast_node() {
                        suggestions.push((page.pfn, target_node, MigrationReason::HotPromotion));
                    }
                }
                PageHotness::Frozen if page.current_tier == MemoryTier::LocalDram || page.current_tier == MemoryTier::FastDram => {
                    // Suggest demotion
                    if let Some(target_node) = self.find_slow_node() {
                        suggestions.push((page.pfn, target_node, MigrationReason::ColdDemotion));
                    }
                }
                _ => {}
            }
        }
        suggestions
    }

    fn find_fast_node(&self) -> Option<u32> {
        self.nodes.values()
            .filter(|n| (n.tier == MemoryTier::FastDram || n.tier == MemoryTier::LocalDram) && n.utilization() < 0.9)
            .min_by(|a, b| a.utilization().partial_cmp(&b.utilization()).unwrap_or(core::cmp::Ordering::Equal))
            .map(|n| n.node_id)
    }

    fn find_slow_node(&self) -> Option<u32> {
        self.nodes.values()
            .filter(|n| (n.tier == MemoryTier::Pmem || n.tier == MemoryTier::RemoteDram) && n.utilization() < 0.9)
            .min_by(|a, b| a.utilization().partial_cmp(&b.utilization()).unwrap_or(core::cmp::Ordering::Equal))
            .map(|n| n.node_id)
    }

    pub fn execute_migration(&mut self, pfn: u64, to_node: u32, reason: MigrationReason, ts: u64) -> bool {
        let page = match self.pages.get_mut(&pfn) { Some(p) => p, None => return false };
        let from_node = page.current_node;
        let from_tier = page.current_tier;
        let to_tier = self.nodes.get(&to_node).map(|n| n.tier).unwrap_or(MemoryTier::LocalDram);

        page.current_node = to_node;
        page.current_tier = to_tier;
        page.migration_count += 1;
        page.last_migration_ts = ts;

        if let Some(n) = self.nodes.get_mut(&from_node) { n.pages_migrated_out += 1; n.total_pages = n.total_pages.saturating_sub(1); }
        if let Some(n) = self.nodes.get_mut(&to_node) { n.pages_migrated_in += 1; n.total_pages += 1; }

        let rid = self.next_request_id; self.next_request_id += 1;
        self.requests.push(MigrationRequest {
            request_id: rid, pfn, from_node, to_node, from_tier, to_tier,
            reason, submitted_ts: ts, completed_ts: Some(ts), success: true,
        });
        true
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.tracked_pages = self.pages.len();
        self.stats.total_migrations = self.requests.len() as u64;
        self.stats.successful_migrations = self.requests.iter().filter(|r| r.success).count() as u64;
        self.stats.failed_migrations = self.stats.total_migrations - self.stats.successful_migrations;
        self.stats.promotions = self.requests.iter().filter(|r| r.reason == MigrationReason::HotPromotion).count() as u64;
        self.stats.demotions = self.requests.iter().filter(|r| r.reason == MigrationReason::ColdDemotion).count() as u64;
        self.stats.total_nodes = self.nodes.len();
        self.stats.hot_pages = self.pages.values().filter(|p| p.hotness == PageHotness::Hot).count();
        self.stats.cold_pages = self.pages.values().filter(|p| matches!(p.hotness, PageHotness::Cold | PageHotness::Frozen)).count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &MigratePageStats { &self.stats }
}
