//! Single cache level management.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::entry::CacheEntry;
use super::eviction::EvictionOptimizer;
use super::pattern::AccessPatternTracker;
use super::stats::CacheStats;
use super::types::{CacheId, CacheKey, CacheLevel, EvictionPolicy};

// ============================================================================
// CACHE MANAGER
// ============================================================================

/// Manages a single cache
#[repr(align(64))]
pub struct CacheManager {
    /// Cache ID
    id: CacheId,
    /// Cache level
    level: CacheLevel,
    /// Entries
    entries: BTreeMap<CacheKey, CacheEntry>,
    /// Eviction optimizer
    eviction: EvictionOptimizer,
    /// Statistics
    stats: CacheStats,
    /// Access pattern tracker
    pattern: AccessPatternTracker,
    /// Maximum size
    max_size: u64,
    /// Current size
    current_size: u64,
}

impl CacheManager {
    /// Create new cache manager
    pub fn new(id: CacheId, level: CacheLevel, max_size: u64) -> Self {
        Self {
            id,
            level,
            entries: BTreeMap::new(),
            eviction: EvictionOptimizer::default(),
            stats: CacheStats::new(max_size),
            pattern: AccessPatternTracker::default(),
            max_size,
            current_size: 0,
        }
    }

    /// Access cache
    pub fn access(&mut self, key: CacheKey) -> bool {
        self.pattern.record(key);

        if let Some(entry) = self.entries.get_mut(&key) {
            entry.access();
            self.stats.record_hit(entry.size as u64);
            true
        } else {
            self.stats.record_miss();
            self.eviction.record_regret(key);
            false
        }
    }

    /// Insert entry
    pub fn insert(&mut self, key: CacheKey, size: u32) {
        // Evict if necessary
        while self.current_size + size as u64 > self.max_size {
            if !self.evict_one() {
                break;
            }
        }

        let entry = CacheEntry::new(key, size);
        self.current_size += size as u64;
        self.entries.insert(key, entry);
        self.stats.record_insertion(size as u64);
    }

    /// Evict one entry
    fn evict_one(&mut self) -> bool {
        let entries: Vec<_> = self.entries.values().cloned().collect();
        if let Some(victim) = self.eviction.select_victim(&entries) {
            let key = victim.key;
            let size = victim.size;
            self.eviction.record_eviction(victim);
            self.entries.remove(&key);
            self.current_size -= size as u64;
            self.stats.record_eviction(size as u64);
            true
        } else {
            false
        }
    }

    /// Remove entry
    #[inline]
    pub fn remove(&mut self, key: CacheKey) -> bool {
        if let Some(entry) = self.entries.remove(&key) {
            self.current_size -= entry.size as u64;
            true
        } else {
            false
        }
    }

    /// Contains key
    #[inline(always)]
    pub fn contains(&self, key: CacheKey) -> bool {
        self.entries.contains_key(&key)
    }

    /// Get entry
    #[inline(always)]
    pub fn get(&self, key: CacheKey) -> Option<&CacheEntry> {
        self.entries.get(&key)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get pattern tracker
    #[inline(always)]
    pub fn pattern(&self) -> &AccessPatternTracker {
        &self.pattern
    }

    /// Get prefetch suggestions
    #[inline]
    pub fn prefetch_suggestions(&self, count: usize) -> Vec<CacheKey> {
        self.pattern
            .prefetch_suggestions(count)
            .into_iter()
            .filter(|k| !self.entries.contains_key(k))
            .collect()
    }

    /// Get fill ratio
    #[inline]
    pub fn fill_ratio(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            self.current_size as f64 / self.max_size as f64
        }
    }

    /// Get entry count
    #[inline(always)]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Set eviction policy
    #[inline(always)]
    pub fn set_eviction_policy(&mut self, policy: EvictionPolicy) {
        self.eviction.set_policy(policy);
    }

    /// Clear cache
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
        self.pattern.clear();
    }
}
