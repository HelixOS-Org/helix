//! # Holistic Memory Tiering
//!
//! Multi-tier memory management (DRAM, PMEM, CXL):
//! - Tier promotion/demotion policies
//! - Access frequency tracking per page
//! - Migration cost estimation
//! - Bandwidth-aware placement
//! - NUMA-aware tier selection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TIER TYPES
// ============================================================================

/// Memory tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryTier {
    /// Fastest: CPU-local DRAM
    LocalDram,
    /// Remote DRAM (cross-NUMA)
    RemoteDram,
    /// Persistent memory (Optane etc)
    Pmem,
    /// CXL-attached memory
    Cxl,
    /// Compressed in-memory
    Compressed,
    /// Swap / disk-backed
    Swap,
}

/// Migration direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierMigrationDir {
    Promote,
    Demote,
    Lateral,
}

/// Page hotness
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageHotness {
    /// >100 accesses/sec
    Blazing,
    /// 10-100 accesses/sec
    Hot,
    /// 1-10 accesses/sec
    Warm,
    /// 0.1-1 accesses/sec
    Cool,
    /// <0.1 accesses/sec
    Cold,
    /// No access in monitoring window
    Frozen,
}

// ============================================================================
// TIER INFO
// ============================================================================

/// Tier characteristics
#[derive(Debug, Clone)]
pub struct TierInfo {
    /// Tier type
    pub tier: MemoryTier,
    /// Total capacity (pages)
    pub capacity_pages: u64,
    /// Used pages
    pub used_pages: u64,
    /// Read latency (ns)
    pub read_latency_ns: u64,
    /// Write latency (ns)
    pub write_latency_ns: u64,
    /// Bandwidth (MB/s)
    pub bandwidth_mbps: u64,
    /// Cost factor (relative, 1.0 = baseline DRAM)
    pub cost_factor: f64,
    /// Migration bandwidth (pages/sec)
    pub migration_bw: u64,
}

impl TierInfo {
    pub fn new(tier: MemoryTier, capacity: u64) -> Self {
        let (read_lat, write_lat, bw, cost, mig_bw) = match tier {
            MemoryTier::LocalDram => (80, 100, 50000, 1.0, 1000000),
            MemoryTier::RemoteDram => (200, 250, 30000, 1.0, 500000),
            MemoryTier::Pmem => (300, 1000, 10000, 0.3, 200000),
            MemoryTier::Cxl => (250, 400, 20000, 0.5, 300000),
            MemoryTier::Compressed => (500, 800, 5000, 0.1, 100000),
            MemoryTier::Swap => (10000, 50000, 500, 0.01, 10000),
        };
        Self {
            tier,
            capacity_pages: capacity,
            used_pages: 0,
            read_latency_ns: read_lat,
            write_latency_ns: write_lat,
            bandwidth_mbps: bw,
            cost_factor: cost,
            migration_bw: mig_bw,
        }
    }

    /// Available pages
    pub fn available(&self) -> u64 {
        self.capacity_pages.saturating_sub(self.used_pages)
    }

    /// Utilization
    pub fn utilization(&self) -> f64 {
        if self.capacity_pages == 0 {
            return 0.0;
        }
        self.used_pages as f64 / self.capacity_pages as f64
    }

    /// Migration cost (ns) to move n pages here
    pub fn migration_cost_ns(&self, pages: u64) -> u64 {
        if self.migration_bw == 0 {
            return u64::MAX;
        }
        (pages * 1_000_000_000) / self.migration_bw
    }
}

// ============================================================================
// PAGE TRACKING
// ============================================================================

/// Per-page tracking
#[derive(Debug, Clone)]
pub struct PageTierInfo {
    /// Page frame number
    pub pfn: u64,
    /// Current tier
    pub current_tier: MemoryTier,
    /// Access count in current window
    pub access_count: u64,
    /// Access rate EMA
    pub access_rate_ema: f64,
    /// Last access time (ns)
    pub last_access_ns: u64,
    /// Migration count
    pub migrations: u32,
    /// Owner PID
    pub owner_pid: u64,
}

impl PageTierInfo {
    pub fn new(pfn: u64, tier: MemoryTier, owner: u64) -> Self {
        Self {
            pfn,
            current_tier: tier,
            access_count: 0,
            access_rate_ema: 0.0,
            last_access_ns: 0,
            migrations: 0,
            owner_pid: owner,
        }
    }

    /// Record access
    pub fn record_access(&mut self, now_ns: u64) {
        self.access_count += 1;
        if self.last_access_ns > 0 {
            let interval = now_ns.saturating_sub(self.last_access_ns) as f64 / 1_000_000_000.0;
            if interval > 0.0 {
                let instant_rate = 1.0 / interval;
                self.access_rate_ema = 0.8 * self.access_rate_ema + 0.2 * instant_rate;
            }
        }
        self.last_access_ns = now_ns;
    }

    /// Hotness classification
    pub fn hotness(&self) -> PageHotness {
        let rate = self.access_rate_ema;
        if rate > 100.0 {
            PageHotness::Blazing
        } else if rate > 10.0 {
            PageHotness::Hot
        } else if rate > 1.0 {
            PageHotness::Warm
        } else if rate > 0.1 {
            PageHotness::Cool
        } else if rate > 0.001 {
            PageHotness::Cold
        } else {
            PageHotness::Frozen
        }
    }

    /// Should promote?
    pub fn should_promote(&self) -> bool {
        matches!(self.hotness(), PageHotness::Blazing | PageHotness::Hot)
            && !matches!(self.current_tier, MemoryTier::LocalDram)
    }

    /// Should demote?
    pub fn should_demote(&self) -> bool {
        matches!(self.hotness(), PageHotness::Cold | PageHotness::Frozen)
            && matches!(self.current_tier, MemoryTier::LocalDram | MemoryTier::RemoteDram)
    }
}

// ============================================================================
// MIGRATION DECISION
// ============================================================================

/// Migration decision
#[derive(Debug, Clone)]
pub struct TierMigrationDecision {
    /// Page PFN
    pub pfn: u64,
    /// Source tier
    pub from: MemoryTier,
    /// Target tier
    pub to: MemoryTier,
    /// Direction
    pub direction: TierMigrationDir,
    /// Estimated benefit (lower = better latency saved)
    pub benefit_score: f64,
    /// Migration cost (ns)
    pub cost_ns: u64,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Tiering stats
#[derive(Debug, Clone, Default)]
pub struct HolisticMemoryTieringStats {
    /// Active tiers
    pub active_tiers: usize,
    /// Tracked pages
    pub tracked_pages: usize,
    /// Total promotions
    pub total_promotions: u64,
    /// Total demotions
    pub total_demotions: u64,
    /// Pending migrations
    pub pending_migrations: usize,
    /// Hottest tier utilization
    pub hot_tier_utilization: f64,
}

/// System-wide memory tiering engine
pub struct HolisticMemoryTiering {
    /// Tier info
    tiers: BTreeMap<u8, TierInfo>,
    /// Page tracking (pfn -> info)
    pages: BTreeMap<u64, PageTierInfo>,
    /// Pending migrations
    pending: Vec<TierMigrationDecision>,
    /// Total promotions
    promotions: u64,
    /// Total demotions
    demotions: u64,
    /// Stats
    stats: HolisticMemoryTieringStats,
}

impl HolisticMemoryTiering {
    pub fn new() -> Self {
        Self {
            tiers: BTreeMap::new(),
            pages: BTreeMap::new(),
            pending: Vec::new(),
            promotions: 0,
            demotions: 0,
            stats: HolisticMemoryTieringStats::default(),
        }
    }

    /// Add a tier
    pub fn add_tier(&mut self, tier: MemoryTier, capacity_pages: u64) {
        self.tiers.insert(tier as u8, TierInfo::new(tier, capacity_pages));
        self.update_stats();
    }

    /// Place page in tier
    pub fn place_page(&mut self, pfn: u64, tier: MemoryTier, owner: u64) {
        let page = PageTierInfo::new(pfn, tier, owner);
        self.pages.insert(pfn, page);
        if let Some(ti) = self.tiers.get_mut(&(tier as u8)) {
            ti.used_pages += 1;
        }
        self.update_stats();
    }

    /// Record page access
    pub fn record_access(&mut self, pfn: u64, now_ns: u64) {
        if let Some(page) = self.pages.get_mut(&pfn) {
            page.record_access(now_ns);
        }
    }

    /// Evaluate migrations
    pub fn evaluate_migrations(&mut self, max_decisions: usize) {
        self.pending.clear();
        let mut decisions = Vec::new();

        for page in self.pages.values() {
            if decisions.len() >= max_decisions {
                break;
            }

            if page.should_promote() {
                // Find next better tier
                if let Some(target) = self.find_promotion_target(page.current_tier) {
                    let cost = self.tiers.get(&(target as u8))
                        .map(|t| t.migration_cost_ns(1))
                        .unwrap_or(u64::MAX);
                    decisions.push(TierMigrationDecision {
                        pfn: page.pfn,
                        from: page.current_tier,
                        to: target,
                        direction: TierMigrationDir::Promote,
                        benefit_score: page.access_rate_ema,
                        cost_ns: cost,
                    });
                }
            } else if page.should_demote() {
                if let Some(target) = self.find_demotion_target(page.current_tier) {
                    let cost = self.tiers.get(&(target as u8))
                        .map(|t| t.migration_cost_ns(1))
                        .unwrap_or(u64::MAX);
                    decisions.push(TierMigrationDecision {
                        pfn: page.pfn,
                        from: page.current_tier,
                        to: target,
                        direction: TierMigrationDir::Demote,
                        benefit_score: 1.0 / (page.access_rate_ema + 0.001),
                        cost_ns: cost,
                    });
                }
            }
        }

        // Sort by benefit (higher first for promotions)
        decisions.sort_by(|a, b| b.benefit_score.partial_cmp(&a.benefit_score).unwrap_or(core::cmp::Ordering::Equal));
        self.pending = decisions;
        self.update_stats();
    }

    /// Execute pending migrations
    pub fn execute_migrations(&mut self) -> u64 {
        let mut migrated = 0u64;
        let pending = core::mem::take(&mut self.pending);

        for decision in &pending {
            if let Some(page) = self.pages.get_mut(&decision.pfn) {
                // Update tier usage
                if let Some(src) = self.tiers.get_mut(&(decision.from as u8)) {
                    src.used_pages = src.used_pages.saturating_sub(1);
                }
                if let Some(dst) = self.tiers.get_mut(&(decision.to as u8)) {
                    if dst.available() > 0 {
                        dst.used_pages += 1;
                        page.current_tier = decision.to;
                        page.migrations += 1;
                        match decision.direction {
                            TierMigrationDir::Promote => self.promotions += 1,
                            TierMigrationDir::Demote => self.demotions += 1,
                            TierMigrationDir::Lateral => {}
                        }
                        migrated += 1;
                    } else {
                        // Restore source
                        if let Some(src) = self.tiers.get_mut(&(decision.from as u8)) {
                            src.used_pages += 1;
                        }
                    }
                }
            }
        }
        self.update_stats();
        migrated
    }

    fn find_promotion_target(&self, current: MemoryTier) -> Option<MemoryTier> {
        let order = [
            MemoryTier::LocalDram,
            MemoryTier::RemoteDram,
            MemoryTier::Cxl,
            MemoryTier::Pmem,
            MemoryTier::Compressed,
            MemoryTier::Swap,
        ];
        let current_idx = order.iter().position(|&t| t == current)?;
        for &tier in &order[..current_idx] {
            if let Some(info) = self.tiers.get(&(tier as u8)) {
                if info.available() > 0 {
                    return Some(tier);
                }
            }
        }
        None
    }

    fn find_demotion_target(&self, current: MemoryTier) -> Option<MemoryTier> {
        let order = [
            MemoryTier::LocalDram,
            MemoryTier::RemoteDram,
            MemoryTier::Cxl,
            MemoryTier::Pmem,
            MemoryTier::Compressed,
            MemoryTier::Swap,
        ];
        let current_idx = order.iter().position(|&t| t == current)?;
        for &tier in &order[current_idx + 1..] {
            if let Some(info) = self.tiers.get(&(tier as u8)) {
                if info.available() > 0 {
                    return Some(tier);
                }
            }
        }
        None
    }

    fn update_stats(&mut self) {
        self.stats.active_tiers = self.tiers.len();
        self.stats.tracked_pages = self.pages.len();
        self.stats.total_promotions = self.promotions;
        self.stats.total_demotions = self.demotions;
        self.stats.pending_migrations = self.pending.len();
        self.stats.hot_tier_utilization = self.tiers.get(&(MemoryTier::LocalDram as u8))
            .map(|t| t.utilization())
            .unwrap_or(0.0);
    }

    /// Stats
    pub fn stats(&self) -> &HolisticMemoryTieringStats {
        &self.stats
    }
}
