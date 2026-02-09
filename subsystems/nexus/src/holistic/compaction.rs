//! # Holistic Compaction Engine
//!
//! System-wide memory compaction and defragmentation:
//! - Physical memory compaction scheduling
//! - Page migration for NUMA balancing
//! - Huge page promotion/demotion
//! - Compaction pressure metrics
//! - Anti-fragmentation zone management

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// COMPACTION TYPES
// ============================================================================

/// Compaction urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompactionUrgency {
    /// Low (background)
    Low,
    /// Normal (proactive)
    Normal,
    /// High (allocation failing)
    High,
    /// Critical (OOM imminent)
    Critical,
}

/// Page order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOrder {
    /// 4KB base page
    Base,
    /// 2MB huge page
    Huge2M,
    /// 1GB huge page
    Huge1G,
}

/// Zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactZone {
    /// DMA zone
    Dma,
    /// Normal zone
    Normal,
    /// Highmem zone
    Highmem,
    /// Movable zone
    Movable,
}

/// Compaction action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactAction {
    /// Migrate movable pages
    Migrate,
    /// Promote to huge page
    Promote,
    /// Demote from huge page
    Demote,
    /// Defragment zone
    Defrag,
    /// No action needed
    None,
}

// ============================================================================
// ZONE STATE
// ============================================================================

/// Per-zone compaction state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ZoneCompactState {
    /// Zone type
    pub zone: CompactZone,
    /// Total pages
    pub total_pages: u64,
    /// Free pages
    pub free_pages: u64,
    /// Movable pages
    pub movable_pages: u64,
    /// Unmovable pages
    pub unmovable_pages: u64,
    /// Free page blocks by order
    pub free_blocks: ArrayMap<u64, 32>,
    /// Compaction scanning offset
    pub scan_offset: u64,
    /// Migrations completed
    pub migrations: u64,
    /// Migrations failed
    pub failed_migrations: u64,
}

impl ZoneCompactState {
    pub fn new(zone: CompactZone, total_pages: u64) -> Self {
        Self {
            zone,
            total_pages,
            free_pages: total_pages,
            movable_pages: 0,
            unmovable_pages: 0,
            free_blocks: ArrayMap::new(0),
            scan_offset: 0,
            migrations: 0,
            failed_migrations: 0,
        }
    }

    /// Fragmentation index (0 = no frag, 1 = fully fragmented)
    pub fn fragmentation_index(&self, target_order: u32) -> f64 {
        if self.free_pages == 0 {
            return 1.0;
        }
        // Count pages available at target_order or higher
        let mut available_at_order = 0u64;
        for (&order, &count) in &self.free_blocks {
            if order >= target_order {
                available_at_order += count * (1u64 << order);
            }
        }
        1.0 - (available_at_order as f64 / self.free_pages as f64)
    }

    /// Compaction worthwhile? (enough movable pages near free pages)
    #[inline(always)]
    pub fn compaction_suitable(&self) -> bool {
        self.movable_pages > 0 && self.free_pages > self.total_pages / 10
    }

    /// Record migration result
    #[inline]
    pub fn record_migration(&mut self, success: bool) {
        if success {
            self.migrations += 1;
        } else {
            self.failed_migrations += 1;
        }
    }

    /// Migration success rate
    #[inline]
    pub fn migration_success_rate(&self) -> f64 {
        let total = self.migrations + self.failed_migrations;
        if total == 0 {
            return 1.0;
        }
        self.migrations as f64 / total as f64
    }
}

// ============================================================================
// HUGE PAGE TRACKER
// ============================================================================

/// Huge page pool stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HugePagePool {
    /// Total huge pages
    pub total: u64,
    /// Free huge pages
    pub free: u64,
    /// Reserved huge pages
    pub reserved: u64,
    /// Surplus huge pages
    pub surplus: u64,
    /// Promotions (base -> huge)
    pub promotions: u64,
    /// Demotions (huge -> base)
    pub demotions: u64,
    /// Promotion failures
    pub promotion_failures: u64,
}

impl HugePagePool {
    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.total - self.free) as f64 / self.total as f64
    }

    /// Should promote? (enough free space and low frag)
    #[inline(always)]
    pub fn should_promote(&self) -> bool {
        self.free > 0 || self.surplus > 0
    }

    /// Record promotion
    #[inline]
    pub fn record_promotion(&mut self, success: bool) {
        if success {
            self.promotions += 1;
            if self.free > 0 {
                self.free -= 1;
            }
        } else {
            self.promotion_failures += 1;
        }
    }

    /// Record demotion
    #[inline(always)]
    pub fn record_demotion(&mut self) {
        self.demotions += 1;
        self.free += 1;
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Compaction engine stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticCompactionStats {
    /// Tracked zones
    pub tracked_zones: usize,
    /// Total migrations
    pub total_migrations: u64,
    /// Average fragmentation
    pub avg_fragmentation: f64,
    /// Huge page utilization
    pub huge_utilization: f64,
    /// Total promotions
    pub total_promotions: u64,
}

/// Holistic compaction engine
pub struct HolisticCompactionEngine {
    /// Per-zone state
    zones: BTreeMap<u8, ZoneCompactState>,
    /// 2MB huge page pool
    pub huge_2m: HugePagePool,
    /// 1GB huge page pool
    pub huge_1g: HugePagePool,
    /// Current urgency
    pub urgency: CompactionUrgency,
    /// Stats
    stats: HolisticCompactionStats,
}

impl HolisticCompactionEngine {
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            huge_2m: HugePagePool::default(),
            huge_1g: HugePagePool::default(),
            urgency: CompactionUrgency::Low,
            stats: HolisticCompactionStats::default(),
        }
    }

    /// Register zone
    #[inline(always)]
    pub fn register_zone(&mut self, zone: CompactZone, total_pages: u64) {
        self.zones.insert(zone as u8, ZoneCompactState::new(zone, total_pages));
    }

    /// Update zone free pages
    #[inline]
    pub fn update_zone(&mut self, zone: CompactZone, free: u64, movable: u64, unmovable: u64) {
        if let Some(z) = self.zones.get_mut(&(zone as u8)) {
            z.free_pages = free;
            z.movable_pages = movable;
            z.unmovable_pages = unmovable;
        }
    }

    /// Decide compaction action
    pub fn decide(&self) -> Vec<(CompactZone, CompactAction)> {
        let mut actions = Vec::new();
        for z in self.zones.values() {
            let frag = z.fragmentation_index(9); // Order 9 = 2MB
            if frag > 0.8 && z.compaction_suitable() {
                actions.push((z.zone, CompactAction::Migrate));
            }
            if frag > 0.5 && self.huge_2m.free == 0 {
                actions.push((z.zone, CompactAction::Defrag));
            }
        }

        // Promotion/demotion
        if self.huge_2m.free > self.huge_2m.total / 4 {
            // Lots of free huge pages, no demotion needed
        } else if self.huge_2m.utilization() > 0.95 {
            // Might need more
            for z in self.zones.values() {
                if z.compaction_suitable() {
                    actions.push((z.zone, CompactAction::Promote));
                    break;
                }
            }
        }

        actions
    }

    /// Record migration
    #[inline]
    pub fn record_migration(&mut self, zone: CompactZone, success: bool) {
        if let Some(z) = self.zones.get_mut(&(zone as u8)) {
            z.record_migration(success);
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_zones = self.zones.len();
        self.stats.total_migrations = self.zones.values().map(|z| z.migrations).sum();
        if !self.zones.is_empty() {
            self.stats.avg_fragmentation = self.zones.values()
                .map(|z| z.fragmentation_index(9))
                .sum::<f64>() / self.zones.len() as f64;
        }
        self.stats.huge_utilization = self.huge_2m.utilization();
        self.stats.total_promotions = self.huge_2m.promotions + self.huge_1g.promotions;
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticCompactionStats {
        &self.stats
    }
}

// ============================================================================
// Merged from compaction_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionV2Mode {
    Sync,
    Async,
    Direct,
    Kcompactd,
}

/// Compaction v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionV2Result {
    Success,
    Skipped,
    Continue,
    NoSuitablePage,
    NotSuitable,
}

/// Compaction v2 zone
#[derive(Debug)]
pub struct CompactionV2Zone {
    pub zone_id: u32,
    pub migrate_pfn: u64,
    pub free_pfn: u64,
    pub pages_migrated: u64,
    pub pages_freed: u64,
    pub compact_count: u64,
    pub compact_fail: u64,
    pub fragmentation_score: u32,
}

impl CompactionV2Zone {
    pub fn new(id: u32) -> Self {
        Self { zone_id: id, migrate_pfn: 0, free_pfn: u64::MAX, pages_migrated: 0, pages_freed: 0, compact_count: 0, compact_fail: 0, fragmentation_score: 0 }
    }

    #[inline]
    pub fn compact(&mut self, migrated: u64, freed: u64) {
        self.pages_migrated += migrated;
        self.pages_freed += freed;
        self.compact_count += 1;
    }

    #[inline(always)]
    pub fn update_fragmentation(&mut self, score: u32) {
        self.fragmentation_score = score;
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CompactionV2Stats {
    pub total_zones: u32,
    pub total_compactions: u64,
    pub total_migrated: u64,
    pub total_freed: u64,
    pub avg_fragmentation: u32,
}

/// Main holistic compaction v2
pub struct HolisticCompactionV2 {
    zones: BTreeMap<u32, CompactionV2Zone>,
}

impl HolisticCompactionV2 {
    pub fn new() -> Self { Self { zones: BTreeMap::new() } }

    #[inline(always)]
    pub fn add_zone(&mut self, id: u32) { self.zones.insert(id, CompactionV2Zone::new(id)); }

    #[inline(always)]
    pub fn compact(&mut self, zone: u32, migrated: u64, freed: u64) {
        if let Some(z) = self.zones.get_mut(&zone) { z.compact(migrated, freed); }
    }

    #[inline(always)]
    pub fn update_fragmentation(&mut self, zone: u32, score: u32) {
        if let Some(z) = self.zones.get_mut(&zone) { z.update_fragmentation(score); }
    }

    #[inline]
    pub fn stats(&self) -> CompactionV2Stats {
        let compactions: u64 = self.zones.values().map(|z| z.compact_count).sum();
        let migrated: u64 = self.zones.values().map(|z| z.pages_migrated).sum();
        let freed: u64 = self.zones.values().map(|z| z.pages_freed).sum();
        let frag: u32 = if self.zones.is_empty() { 0 } else {
            self.zones.values().map(|z| z.fragmentation_score).sum::<u32>() / self.zones.len() as u32
        };
        CompactionV2Stats { total_zones: self.zones.len() as u32, total_compactions: compactions, total_migrated: migrated, total_freed: freed, avg_fragmentation: frag }
    }
}
