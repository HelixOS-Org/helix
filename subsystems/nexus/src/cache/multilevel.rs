//! Multi-level cache coordination.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::manager::CacheManager;
use super::stats::CacheStats;
use super::types::{CacheId, CacheKey, CacheLevel};

// ============================================================================
// MULTI-LEVEL CACHE
// ============================================================================

/// Multi-level cache coordinator
pub struct MultiLevelCache {
    /// Caches by level
    caches: BTreeMap<CacheLevel, CacheManager>,
    /// Inclusion policy
    inclusion: InclusionPolicy,
    /// Promotion threshold
    promotion_threshold: u32,
    /// Total accesses
    total_accesses: AtomicU64,
}

/// Cache inclusion policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InclusionPolicy {
    /// Inclusive (all data in lower levels also in higher)
    Inclusive,
    /// Exclusive (data in only one level)
    Exclusive,
    /// Non-inclusive (NINE)
    NonInclusive,
}

impl MultiLevelCache {
    /// Create new multi-level cache
    pub fn new() -> Self {
        Self {
            caches: BTreeMap::new(),
            inclusion: InclusionPolicy::Exclusive,
            promotion_threshold: 3,
            total_accesses: AtomicU64::new(0),
        }
    }

    /// Add cache level
    pub fn add_level(&mut self, id: CacheId, level: CacheLevel, size: u64) {
        self.caches
            .insert(level, CacheManager::new(id, level, size));
    }

    /// Access data
    pub fn access(&mut self, key: CacheKey) -> Option<CacheLevel> {
        self.total_accesses.fetch_add(1, Ordering::Relaxed);

        // Search from fastest to slowest
        for (&level, cache) in self.caches.iter_mut() {
            if cache.access(key) {
                // Consider promotion
                if let Some(entry) = cache.get(key) {
                    if entry.access_count >= self.promotion_threshold {
                        self.promote(key, level);
                    }
                }
                return Some(level);
            }
        }

        None
    }

    /// Promote entry to higher level
    fn promote(&mut self, key: CacheKey, current_level: CacheLevel) {
        // Find next higher level
        let higher_levels: Vec<_> = self
            .caches
            .keys()
            .filter(|&&l| l < current_level)
            .copied()
            .collect();

        if let Some(&target_level) = higher_levels.last() {
            // Get entry info
            let size = self
                .caches
                .get(&current_level)
                .and_then(|c| c.get(key))
                .map(|e| e.size)
                .unwrap_or(0);

            if size > 0 {
                // Insert in higher level
                if let Some(target_cache) = self.caches.get_mut(&target_level) {
                    target_cache.insert(key, size);
                }

                // Handle inclusion policy
                if self.inclusion == InclusionPolicy::Exclusive {
                    // Remove from current level
                    if let Some(current_cache) = self.caches.get_mut(&current_level) {
                        current_cache.remove(key);
                    }
                }
            }
        }
    }

    /// Insert data
    pub fn insert(&mut self, key: CacheKey, size: u32, level: CacheLevel) {
        if let Some(cache) = self.caches.get_mut(&level) {
            cache.insert(key, size);
        }

        // Handle inclusion policy
        if self.inclusion == InclusionPolicy::Inclusive {
            // Also insert in higher levels
            for (&l, cache) in self.caches.iter_mut() {
                if l > level && !cache.contains(key) {
                    cache.insert(key, size);
                }
            }
        }
    }

    /// Get aggregate statistics
    pub fn aggregate_stats(&self) -> CacheStats {
        let mut total = CacheStats::default();

        for cache in self.caches.values() {
            let stats = cache.stats();
            total.total_accesses += stats.total_accesses;
            total.hits += stats.hits;
            total.misses += stats.misses;
            total.evictions += stats.evictions;
            total.insertions += stats.insertions;
            total.current_size += stats.current_size;
            total.max_size += stats.max_size;
            total.current_entries += stats.current_entries;
        }

        total
    }

    /// Get cache at level
    pub fn get_level(&self, level: CacheLevel) -> Option<&CacheManager> {
        self.caches.get(&level)
    }

    /// Get mutable cache at level
    pub fn get_level_mut(&mut self, level: CacheLevel) -> Option<&mut CacheManager> {
        self.caches.get_mut(&level)
    }

    /// Set inclusion policy
    pub fn set_inclusion(&mut self, policy: InclusionPolicy) {
        self.inclusion = policy;
    }

    /// Get total accesses
    pub fn total_accesses(&self) -> u64 {
        self.total_accesses.load(Ordering::Relaxed)
    }
}

impl Default for MultiLevelCache {
    fn default() -> Self {
        Self::new()
    }
}
