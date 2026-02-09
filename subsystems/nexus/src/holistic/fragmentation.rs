//! # Holistic Fragmentation Analyzer
//!
//! System-wide memory fragmentation analysis and mitigation:
//! - External fragmentation tracking
//! - Internal fragmentation analysis
//! - Compaction recommendations
//! - Slab utilization monitoring
//! - Huge page availability tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// FRAGMENTATION TYPES
// ============================================================================

/// Memory zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryZone {
    /// DMA zone (0-16MB)
    Dma,
    /// DMA32 zone (0-4GB)
    Dma32,
    /// Normal zone
    Normal,
    /// High memory
    HighMem,
    /// Device memory
    Device,
}

/// Fragmentation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragType {
    /// External (free space between allocations)
    External,
    /// Internal (wasted space within allocations)
    Internal,
    /// Slab fragmentation
    Slab,
}

/// Fragmentation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FragSeverity {
    /// Minimal (<10%)
    Minimal,
    /// Low (10-25%)
    Low,
    /// Moderate (25-50%)
    Moderate,
    /// High (50-75%)
    High,
    /// Critical (>75%)
    Critical,
}

// ============================================================================
// BUDDY SYSTEM ANALYSIS
// ============================================================================

/// Buddy allocator order statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BuddyOrderStats {
    /// Free blocks per order (order 0-10 typically)
    pub free_blocks: [u64; 11],
    /// Total blocks per order
    pub total_blocks: [u64; 11],
}

impl BuddyOrderStats {
    pub fn new() -> Self {
        Self {
            free_blocks: [0; 11],
            total_blocks: [0; 11],
        }
    }

    /// Free pages at order n = free_blocks[n] * 2^n
    #[inline]
    pub fn free_pages_at_order(&self, order: usize) -> u64 {
        if order >= 11 {
            return 0;
        }
        self.free_blocks[order] * (1u64 << order)
    }

    /// Total free pages
    #[inline]
    pub fn total_free_pages(&self) -> u64 {
        let mut total = 0u64;
        for order in 0..11 {
            total += self.free_pages_at_order(order);
        }
        total
    }

    /// External fragmentation index
    /// 1.0 means all free memory is in smallest blocks
    /// 0.0 means all free memory is in largest blocks
    pub fn fragmentation_index(&self) -> f64 {
        let total_free = self.total_free_pages();
        if total_free == 0 {
            return 0.0;
        }
        // Ideal: all free pages in max-order blocks
        let max_order_blocks = total_free / (1u64 << 10);
        let actual_max = self.free_blocks[10];
        if max_order_blocks == 0 {
            return 1.0;
        }
        1.0 - (actual_max as f64 / max_order_blocks as f64)
    }

    /// Can allocate contiguous pages of given order?
    #[inline]
    pub fn can_allocate(&self, order: usize) -> bool {
        for o in order..11 {
            if self.free_blocks[o] > 0 {
                return true;
            }
        }
        false
    }

    /// Largest contiguous allocation possible
    #[inline]
    pub fn max_contiguous_order(&self) -> usize {
        for order in (0..11).rev() {
            if self.free_blocks[order] > 0 {
                return order;
            }
        }
        0
    }

    /// Severity
    pub fn severity(&self) -> FragSeverity {
        let idx = self.fragmentation_index();
        if idx < 0.1 {
            FragSeverity::Minimal
        } else if idx < 0.25 {
            FragSeverity::Low
        } else if idx < 0.5 {
            FragSeverity::Moderate
        } else if idx < 0.75 {
            FragSeverity::High
        } else {
            FragSeverity::Critical
        }
    }
}

// ============================================================================
// SLAB ANALYSIS
// ============================================================================

/// Slab cache info
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SlabCacheInfo {
    /// Cache name hash
    pub name_hash: u64,
    /// Object size
    pub object_size: u32,
    /// Objects per slab
    pub objects_per_slab: u32,
    /// Active objects
    pub active_objects: u64,
    /// Total objects (allocated slots)
    pub total_objects: u64,
    /// Active slabs
    pub active_slabs: u64,
    /// Total slabs
    pub total_slabs: u64,
}

impl SlabCacheInfo {
    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total_objects == 0 {
            return 0.0;
        }
        self.active_objects as f64 / self.total_objects as f64
    }

    /// Internal fragmentation (wasted within allocated slabs)
    #[inline]
    pub fn internal_fragmentation(&self) -> f64 {
        if self.total_objects == 0 {
            return 0.0;
        }
        1.0 - self.utilization()
    }

    /// Wasted memory bytes
    #[inline(always)]
    pub fn wasted_bytes(&self) -> u64 {
        let unused = self.total_objects.saturating_sub(self.active_objects);
        unused * self.object_size as u64
    }

    /// Is this cache a fragmentation concern?
    #[inline(always)]
    pub fn is_fragmented(&self) -> bool {
        self.internal_fragmentation() > 0.5 && self.wasted_bytes() > 4096
    }
}

// ============================================================================
// ZONE STATS
// ============================================================================

/// Per-zone fragmentation data
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ZoneFragStats {
    /// Zone
    pub zone: MemoryZone,
    /// Buddy order stats
    pub buddy: BuddyOrderStats,
    /// Total pages
    pub total_pages: u64,
    /// Used pages
    pub used_pages: u64,
    /// Compaction attempts
    pub compaction_attempts: u64,
    /// Successful compactions
    pub compaction_successes: u64,
}

impl ZoneFragStats {
    pub fn new(zone: MemoryZone) -> Self {
        Self {
            zone,
            buddy: BuddyOrderStats::new(),
            total_pages: 0,
            used_pages: 0,
            compaction_attempts: 0,
            compaction_successes: 0,
        }
    }

    /// Free pages
    #[inline(always)]
    pub fn free_pages(&self) -> u64 {
        self.total_pages.saturating_sub(self.used_pages)
    }

    /// Usage ratio
    #[inline]
    pub fn usage_ratio(&self) -> f64 {
        if self.total_pages == 0 {
            return 0.0;
        }
        self.used_pages as f64 / self.total_pages as f64
    }

    /// Needs compaction?
    #[inline(always)]
    pub fn needs_compaction(&self) -> bool {
        self.buddy.severity() >= FragSeverity::High && self.free_pages() > 0
    }

    /// Compaction success rate
    #[inline]
    pub fn compaction_success_rate(&self) -> f64 {
        if self.compaction_attempts == 0 {
            return 0.0;
        }
        self.compaction_successes as f64 / self.compaction_attempts as f64
    }
}

// ============================================================================
// HUGE PAGE TRACKER
// ============================================================================

/// Huge page availability
#[derive(Debug, Clone)]
pub struct HugePageAvailability {
    /// 2MB huge pages free
    pub free_2mb: u64,
    /// 2MB huge pages total
    pub total_2mb: u64,
    /// 1GB huge pages free
    pub free_1gb: u64,
    /// 1GB huge pages total
    pub total_1gb: u64,
    /// Surplus huge pages
    pub surplus: u64,
}

impl HugePageAvailability {
    pub fn new() -> Self {
        Self {
            free_2mb: 0,
            total_2mb: 0,
            free_1gb: 0,
            total_1gb: 0,
            surplus: 0,
        }
    }

    /// 2MB utilization
    #[inline]
    pub fn utilization_2mb(&self) -> f64 {
        if self.total_2mb == 0 {
            return 0.0;
        }
        (self.total_2mb - self.free_2mb) as f64 / self.total_2mb as f64
    }

    /// 1GB utilization
    #[inline]
    pub fn utilization_1gb(&self) -> f64 {
        if self.total_1gb == 0 {
            return 0.0;
        }
        (self.total_1gb - self.free_1gb) as f64 / self.total_1gb as f64
    }
}

// ============================================================================
// FRAGMENTATION ENGINE
// ============================================================================

/// Fragmentation stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticFragmentationStats {
    /// Number of zones
    pub zone_count: usize,
    /// Slab caches tracked
    pub slab_caches: usize,
    /// Overall fragmentation index
    pub overall_fragmentation: f64,
    /// Compaction recommendations pending
    pub compaction_pending: u64,
}

/// Holistic fragmentation analyzer
pub struct HolisticFragmentationEngine {
    /// Zone stats
    zones: BTreeMap<u8, ZoneFragStats>,
    /// Slab caches
    slabs: BTreeMap<u64, SlabCacheInfo>,
    /// Huge page info
    pub huge_pages: HugePageAvailability,
    /// Stats
    stats: HolisticFragmentationStats,
}

impl HolisticFragmentationEngine {
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            slabs: BTreeMap::new(),
            huge_pages: HugePageAvailability::new(),
            stats: HolisticFragmentationStats::default(),
        }
    }

    /// Register zone
    #[inline]
    pub fn register_zone(&mut self, zone: MemoryZone) {
        let key = zone as u8;
        if !self.zones.contains_key(&key) {
            self.zones.insert(key, ZoneFragStats::new(zone));
        }
        self.update_stats();
    }

    /// Update zone buddy stats
    #[inline]
    pub fn update_zone_buddy(&mut self, zone: MemoryZone, order: usize, free: u64, total: u64) {
        let key = zone as u8;
        if let Some(zs) = self.zones.get_mut(&key) {
            if order < 11 {
                zs.buddy.free_blocks[order] = free;
                zs.buddy.total_blocks[order] = total;
            }
        }
        self.update_stats();
    }

    /// Register slab cache
    #[inline(always)]
    pub fn register_slab(&mut self, info: SlabCacheInfo) {
        self.slabs.insert(info.name_hash, info);
        self.update_stats();
    }

    /// Get fragmented slabs
    #[inline(always)]
    pub fn fragmented_slabs(&self) -> Vec<&SlabCacheInfo> {
        self.slabs.values().filter(|s| s.is_fragmented()).collect()
    }

    /// Zones needing compaction
    #[inline]
    pub fn zones_needing_compaction(&self) -> Vec<MemoryZone> {
        self.zones
            .values()
            .filter(|z| z.needs_compaction())
            .map(|z| z.zone)
            .collect()
    }

    /// Overall fragmentation
    #[inline]
    pub fn overall_fragmentation(&self) -> f64 {
        if self.zones.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.zones.values().map(|z| z.buddy.fragmentation_index()).sum();
        sum / self.zones.len() as f64
    }

    fn update_stats(&mut self) {
        self.stats.zone_count = self.zones.len();
        self.stats.slab_caches = self.slabs.len();
        self.stats.overall_fragmentation = self.overall_fragmentation();
        self.stats.compaction_pending = self.zones_needing_compaction().len() as u64;
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticFragmentationStats {
        &self.stats
    }
}
