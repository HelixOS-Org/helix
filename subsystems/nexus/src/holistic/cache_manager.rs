//! # Holistic Cache Manager
//!
//! System-wide cache management and optimization:
//! - LLC partitioning across processes
//! - Cache coloring for NUMA-aware placement
//! - Hot/cold page classification
//! - Cache pollution detection
//! - Prefetch scheduling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CACHE TYPES
// ============================================================================

/// Cache partition scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionScheme {
    /// Equal partitions
    Equal,
    /// Weighted by priority
    Weighted,
    /// Dynamic (based on miss rate)
    Dynamic,
    /// None (shared)
    Shared,
}

/// Page temperature
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageTemp {
    /// Very hot (accessed every tick)
    Hot,
    /// Warm (accessed recently)
    Warm,
    /// Cool (not accessed recently)
    Cool,
    /// Cold (candidate for eviction)
    Cold,
    /// Frozen (not accessed for long time)
    Frozen,
}

/// Prefetch hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchHint {
    /// Sequential read ahead
    Sequential,
    /// Stride pattern
    Stride,
    /// Random (no prefetch)
    Random,
    /// Temporal locality
    Temporal,
}

// ============================================================================
// CACHE PARTITION
// ============================================================================

/// LLC partition for a process/group
#[derive(Debug, Clone)]
pub struct CachePartitionEntry {
    /// Owner PID (0 = group)
    pub owner: u64,
    /// Partition ways (out of total)
    pub ways: u32,
    /// Total available ways
    pub total_ways: u32,
    /// Current occupancy (KB)
    pub occupancy_kb: u64,
    /// Hit rate
    pub hit_rate: f64,
    /// Miss rate
    pub miss_rate: f64,
    /// Accesses
    pub accesses: u64,
    /// Misses
    pub misses: u64,
}

impl CachePartitionEntry {
    pub fn new(owner: u64, ways: u32, total_ways: u32) -> Self {
        Self {
            owner,
            ways,
            total_ways,
            occupancy_kb: 0,
            hit_rate: 0.0,
            miss_rate: 0.0,
            accesses: 0,
            misses: 0,
        }
    }

    /// Record access
    pub fn record_access(&mut self, hit: bool) {
        self.accesses += 1;
        if !hit {
            self.misses += 1;
        }
        if self.accesses > 0 {
            self.miss_rate = self.misses as f64 / self.accesses as f64;
            self.hit_rate = 1.0 - self.miss_rate;
        }
    }

    /// Partition share
    pub fn share(&self) -> f64 {
        if self.total_ways == 0 {
            return 0.0;
        }
        self.ways as f64 / self.total_ways as f64
    }

    /// Utility: miss rate improvement per additional way
    pub fn marginal_utility(&self) -> f64 {
        // Rough model: miss_rate ~ C / ways^alpha
        // Marginal = alpha * miss_rate / ways
        if self.ways == 0 {
            return f64::MAX;
        }
        0.7 * self.miss_rate / self.ways as f64
    }
}

// ============================================================================
// PAGE CLASSIFIER
// ============================================================================

/// Page classification entry
#[derive(Debug, Clone)]
pub struct PageClassification {
    /// Page frame number
    pub pfn: u64,
    /// Temperature
    pub temperature: PageTemp,
    /// Access count
    pub access_count: u64,
    /// Last access (ns)
    pub last_access_ns: u64,
    /// Owner PID
    pub owner_pid: u64,
}

/// Hot/cold page classifier
#[derive(Debug)]
pub struct PageClassifier {
    /// Pages
    pages: BTreeMap<u64, PageClassification>,
    /// Temperature thresholds (accesses in window)
    pub hot_threshold: u64,
    pub warm_threshold: u64,
    pub cool_threshold: u64,
}

impl PageClassifier {
    pub fn new() -> Self {
        Self {
            pages: BTreeMap::new(),
            hot_threshold: 100,
            warm_threshold: 10,
            cool_threshold: 2,
        }
    }

    /// Record access
    pub fn record_access(&mut self, pfn: u64, owner: u64, now: u64) {
        let entry = self.pages.entry(pfn).or_insert(PageClassification {
            pfn,
            temperature: PageTemp::Cold,
            access_count: 0,
            last_access_ns: now,
            owner_pid: owner,
        });
        entry.access_count += 1;
        entry.last_access_ns = now;
    }

    /// Reclassify all pages
    pub fn reclassify(&mut self, now: u64, aging_window_ns: u64) {
        for page in self.pages.values_mut() {
            let age = now.saturating_sub(page.last_access_ns);
            page.temperature = if age > aging_window_ns * 4 {
                PageTemp::Frozen
            } else if age > aging_window_ns * 2 {
                PageTemp::Cold
            } else if page.access_count >= self.hot_threshold {
                PageTemp::Hot
            } else if page.access_count >= self.warm_threshold {
                PageTemp::Warm
            } else {
                PageTemp::Cool
            };
        }
    }

    /// Get cold pages (eviction candidates)
    pub fn cold_pages(&self, limit: usize) -> Vec<u64> {
        let mut cold: Vec<&PageClassification> = self.pages.values()
            .filter(|p| matches!(p.temperature, PageTemp::Cold | PageTemp::Frozen))
            .collect();
        cold.sort_by(|a, b| a.access_count.cmp(&b.access_count));
        cold.into_iter().take(limit).map(|p| p.pfn).collect()
    }

    /// Count by temperature
    pub fn temperature_distribution(&self) -> BTreeMap<u8, usize> {
        let mut dist = BTreeMap::new();
        for page in self.pages.values() {
            *dist.entry(page.temperature as u8).or_insert(0) += 1;
        }
        dist
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Cache manager stats
#[derive(Debug, Clone, Default)]
pub struct HolisticCacheManagerStats {
    /// Partitions
    pub partitions: usize,
    /// Tracked pages
    pub tracked_pages: usize,
    /// Average hit rate
    pub avg_hit_rate: f64,
    /// Hot pages
    pub hot_pages: usize,
    /// Cold pages
    pub cold_pages: usize,
}

/// Holistic cache manager
pub struct HolisticCacheManager {
    /// Partitions
    partitions: BTreeMap<u64, CachePartitionEntry>,
    /// Page classifier
    pub classifier: PageClassifier,
    /// Total LLC ways
    pub total_ways: u32,
    /// Partition scheme
    pub scheme: PartitionScheme,
    /// Stats
    stats: HolisticCacheManagerStats,
}

impl HolisticCacheManager {
    pub fn new(total_ways: u32) -> Self {
        Self {
            partitions: BTreeMap::new(),
            classifier: PageClassifier::new(),
            total_ways,
            scheme: PartitionScheme::Dynamic,
            stats: HolisticCacheManagerStats::default(),
        }
    }

    /// Create partition
    pub fn create_partition(&mut self, owner: u64, ways: u32) {
        self.partitions.insert(owner, CachePartitionEntry::new(owner, ways, self.total_ways));
        self.update_stats();
    }

    /// Record cache access
    pub fn record_access(&mut self, owner: u64, pfn: u64, hit: bool, now: u64) {
        if let Some(part) = self.partitions.get_mut(&owner) {
            part.record_access(hit);
        }
        self.classifier.record_access(pfn, owner, now);
    }

    /// Rebalance partitions (dynamic scheme)
    pub fn rebalance(&mut self) {
        if self.scheme != PartitionScheme::Dynamic || self.partitions.is_empty() {
            return;
        }

        // Allocate ways proportionally to marginal utility
        let total_utility: f64 = self.partitions.values().map(|p| p.marginal_utility()).sum();
        if total_utility <= 0.0 {
            return;
        }

        let mut remaining = self.total_ways;
        let n = self.partitions.len() as u32;
        let min_ways = 1u32;

        let keys: Vec<u64> = self.partitions.keys().cloned().collect();
        for key in &keys {
            if let Some(part) = self.partitions.get_mut(key) {
                let utility = part.marginal_utility();
                let share = (utility / total_utility * self.total_ways as f64) as u32;
                let ways = share.max(min_ways).min(remaining);
                part.ways = ways;
                remaining = remaining.saturating_sub(ways);
            }
        }

        // Distribute remaining ways
        if remaining > 0 {
            for key in &keys {
                if remaining == 0 {
                    break;
                }
                if let Some(part) = self.partitions.get_mut(key) {
                    part.ways += 1;
                    remaining -= 1;
                }
            }
        }
    }

    /// Remove partition
    pub fn remove_partition(&mut self, owner: u64) {
        self.partitions.remove(&owner);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.partitions = self.partitions.len();
        self.stats.tracked_pages = self.classifier.pages.len();
        if !self.partitions.is_empty() {
            self.stats.avg_hit_rate = self.partitions.values()
                .map(|p| p.hit_rate)
                .sum::<f64>() / self.partitions.len() as f64;
        }
        let dist = self.classifier.temperature_distribution();
        self.stats.hot_pages = dist.get(&(PageTemp::Hot as u8)).copied().unwrap_or(0);
        self.stats.cold_pages = dist.get(&(PageTemp::Cold as u8)).copied().unwrap_or(0)
            + dist.get(&(PageTemp::Frozen as u8)).copied().unwrap_or(0);
    }

    /// Stats
    pub fn stats(&self) -> &HolisticCacheManagerStats {
        &self.stats
    }
}
