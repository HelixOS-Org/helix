//! # Memory Cache
//!
//! Implements multi-level memory caching.
//! Supports write-through, write-back, and hybrid policies.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CACHE TYPES
// ============================================================================

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry<T: Clone> {
    /// Entry ID
    pub id: u64,
    /// Key
    pub key: String,
    /// Value
    pub value: T,
    /// Dirty (modified but not written back)
    pub dirty: bool,
    /// Created
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Access count
    pub access_count: u64,
    /// Size
    pub size: usize,
    /// TTL
    pub expires: Option<Timestamp>,
}

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLevel {
    L1,
    L2,
    L3,
}

/// Write policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritePolicy {
    WriteThrough,
    WriteBack,
    WriteAround,
}

/// Eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEvictionPolicy {
    LRU,
    LFU,
    FIFO,
    ARC,
    Clock,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Hits
    pub hits: u64,
    /// Misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Write backs
    pub write_backs: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Level
    pub level: CacheLevel,
    /// Capacity
    pub capacity: usize,
    /// Max size in bytes
    pub max_size: usize,
    /// Write policy
    pub write_policy: WritePolicy,
    /// Eviction policy
    pub eviction_policy: CacheEvictionPolicy,
    /// Default TTL
    pub default_ttl_ms: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            level: CacheLevel::L1,
            capacity: 1000,
            max_size: 1024 * 1024,
            write_policy: WritePolicy::WriteThrough,
            eviction_policy: CacheEvictionPolicy::LRU,
            default_ttl_ms: None,
        }
    }
}

// ============================================================================
// CACHE LAYER
// ============================================================================

/// Cache layer
pub struct CacheLayer<T: Clone> {
    /// Entries
    entries: BTreeMap<String, CacheEntry<T>>,
    /// Clock hand for Clock algorithm
    clock_hand: usize,
    /// Reference bits for Clock/ARC
    reference_bits: BTreeMap<String, bool>,
    /// Current size
    current_size: usize,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: CacheConfig,
    /// Statistics
    stats: CacheStats,
}

impl<T: Clone> CacheLayer<T> {
    /// Create new cache layer
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: BTreeMap::new(),
            clock_hand: 0,
            reference_bits: BTreeMap::new(),
            current_size: 0,
            next_id: AtomicU64::new(1),
            config,
            stats: CacheStats::default(),
        }
    }

    /// Get entry
    pub fn get(&mut self, key: &str) -> Option<&T> {
        if let Some(entry) = self.entries.get_mut(key) {
            // Check expiration
            if let Some(expires) = entry.expires {
                if Timestamp::now().0 > expires.0 {
                    self.remove(key);
                    self.stats.misses += 1;
                    return None;
                }
            }

            entry.last_accessed = Timestamp::now();
            entry.access_count += 1;
            self.reference_bits.insert(key.into(), true);
            self.stats.hits += 1;
            Some(&entry.value)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Put entry
    pub fn put(&mut self, key: &str, value: T, size: usize) {
        self.put_with_ttl(key, value, size, self.config.default_ttl_ms)
    }

    /// Put with TTL
    pub fn put_with_ttl(&mut self, key: &str, value: T, size: usize, ttl_ms: Option<u64>) {
        // Evict if needed
        while self.entries.len() >= self.config.capacity
            || self.current_size + size > self.config.max_size
        {
            if !self.evict() {
                break;
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let expires = ttl_ms.map(|ttl| Timestamp(now.0 + ttl));

        // Check if updating existing
        if let Some(existing) = self.entries.get_mut(key) {
            self.current_size -= existing.size;
            existing.value = value;
            existing.size = size;
            existing.dirty = true;
            existing.last_accessed = now;
            existing.expires = expires;
            self.current_size += size;
            return;
        }

        let entry = CacheEntry {
            id,
            key: key.into(),
            value,
            dirty: false,
            created: now,
            last_accessed: now,
            access_count: 0,
            size,
            expires,
        };

        self.entries.insert(key.into(), entry);
        self.reference_bits.insert(key.into(), true);
        self.current_size += size;
    }

    /// Remove entry
    pub fn remove(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.entries.remove(key) {
            self.current_size -= entry.size;
            self.reference_bits.remove(key);
            Some(entry.value)
        } else {
            None
        }
    }

    /// Evict one entry
    fn evict(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        let key_to_evict = match self.config.eviction_policy {
            CacheEvictionPolicy::LRU => self.find_lru(),
            CacheEvictionPolicy::LFU => self.find_lfu(),
            CacheEvictionPolicy::FIFO => self.find_fifo(),
            CacheEvictionPolicy::Clock => self.find_clock(),
            CacheEvictionPolicy::ARC => self.find_arc(),
        };

        if let Some(key) = key_to_evict {
            // Check if needs write back
            if let Some(entry) = self.entries.get(&key) {
                if entry.dirty && self.config.write_policy == WritePolicy::WriteBack {
                    self.stats.write_backs += 1;
                }
            }

            self.remove(&key);
            self.stats.evictions += 1;
            true
        } else {
            false
        }
    }

    fn find_lru(&self) -> Option<String> {
        self.entries
            .values()
            .min_by_key(|e| e.last_accessed.0)
            .map(|e| e.key.clone())
    }

    fn find_lfu(&self) -> Option<String> {
        self.entries
            .values()
            .min_by_key(|e| e.access_count)
            .map(|e| e.key.clone())
    }

    fn find_fifo(&self) -> Option<String> {
        self.entries
            .values()
            .min_by_key(|e| e.created.0)
            .map(|e| e.key.clone())
    }

    fn find_clock(&mut self) -> Option<String> {
        let keys: Vec<_> = self.entries.keys().cloned().collect();
        if keys.is_empty() {
            return None;
        }

        for _ in 0..keys.len() * 2 {
            let idx = self.clock_hand % keys.len();
            let key = &keys[idx];

            if let Some(&ref_bit) = self.reference_bits.get(key) {
                if !ref_bit {
                    self.clock_hand = (self.clock_hand + 1) % keys.len();
                    return Some(key.clone());
                } else {
                    self.reference_bits.insert(key.clone(), false);
                }
            }

            self.clock_hand = (self.clock_hand + 1) % keys.len();
        }

        // Fall back to first
        Some(keys[0].clone())
    }

    fn find_arc(&self) -> Option<String> {
        // Simplified ARC: combine LRU and LFU
        let lru_candidate = self.entries.values().min_by_key(|e| e.last_accessed.0);

        let lfu_candidate = self.entries.values().min_by_key(|e| e.access_count);

        match (lru_candidate, lfu_candidate) {
            (Some(lru), Some(lfu)) => {
                // Choose the one with lower score
                let lru_score =
                    lru.access_count as f64 / (Timestamp::now().0 - lru.last_accessed.0 + 1) as f64;
                let lfu_score =
                    lfu.access_count as f64 / (Timestamp::now().0 - lfu.last_accessed.0 + 1) as f64;

                if lru_score < lfu_score {
                    Some(lru.key.clone())
                } else {
                    Some(lfu.key.clone())
                }
            },
            (Some(lru), None) => Some(lru.key.clone()),
            (None, Some(lfu)) => Some(lfu.key.clone()),
            (None, None) => None,
        }
    }

    /// Mark dirty
    pub fn mark_dirty(&mut self, key: &str) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.dirty = true;
        }
    }

    /// Flush dirty entries (for write-back)
    pub fn flush(&mut self) -> Vec<(String, T)> {
        let mut flushed = Vec::new();

        for entry in self.entries.values_mut() {
            if entry.dirty {
                flushed.push((entry.key.clone(), entry.value.clone()));
                entry.dirty = false;
                self.stats.write_backs += 1;
            }
        }

        flushed
    }

    /// Contains key
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Current entry count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.reference_bits.clear();
        self.current_size = 0;
    }

    /// Get statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
}

// ============================================================================
// MULTI-LEVEL CACHE
// ============================================================================

/// Multi-level cache
pub struct MultiLevelCache<T: Clone> {
    /// L1 cache
    l1: CacheLayer<T>,
    /// L2 cache
    l2: CacheLayer<T>,
    /// L3 cache
    l3: Option<CacheLayer<T>>,
}

impl<T: Clone> MultiLevelCache<T> {
    /// Create new multi-level cache
    pub fn new(
        l1_config: CacheConfig,
        l2_config: CacheConfig,
        l3_config: Option<CacheConfig>,
    ) -> Self {
        Self {
            l1: CacheLayer::new(l1_config),
            l2: CacheLayer::new(l2_config),
            l3: l3_config.map(CacheLayer::new),
        }
    }

    /// Get from cache (tries L1, then L2, then L3)
    pub fn get(&mut self, key: &str) -> Option<T> {
        // Try L1
        if let Some(value) = self.l1.get(key) {
            return Some(value.clone());
        }

        // Try L2
        if let Some(value) = self.l2.get(key) {
            let value = value.clone();
            // Promote to L1
            self.l1.put(key, value.clone(), 0);
            return Some(value);
        }

        // Try L3
        if let Some(l3) = &mut self.l3 {
            if let Some(value) = l3.get(key) {
                let value = value.clone();
                // Promote to L2 and L1
                self.l2.put(key, value.clone(), 0);
                self.l1.put(key, value.clone(), 0);
                return Some(value);
            }
        }

        None
    }

    /// Put to all levels
    pub fn put(&mut self, key: &str, value: T, size: usize) {
        self.l1.put(key, value.clone(), size);
        self.l2.put(key, value.clone(), size);
        if let Some(l3) = &mut self.l3 {
            l3.put(key, value, size);
        }
    }

    /// Invalidate key across all levels
    pub fn invalidate(&mut self, key: &str) {
        self.l1.remove(key);
        self.l2.remove(key);
        if let Some(l3) = &mut self.l3 {
            l3.remove(key);
        }
    }

    /// Clear all levels
    pub fn clear(&mut self) {
        self.l1.clear();
        self.l2.clear();
        if let Some(l3) = &mut self.l3 {
            l3.clear();
        }
    }

    /// Get combined stats
    pub fn total_stats(&self) -> CacheStats {
        let mut total = CacheStats::default();

        total.hits = self.l1.stats.hits + self.l2.stats.hits;
        total.misses = self.l1.stats.misses;
        total.evictions = self.l1.stats.evictions + self.l2.stats.evictions;
        total.write_backs = self.l1.stats.write_backs + self.l2.stats.write_backs;

        if let Some(l3) = &self.l3 {
            total.hits += l3.stats.hits;
            total.evictions += l3.stats.evictions;
            total.write_backs += l3.stats.write_backs;
        }

        total
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get() {
        let config = CacheConfig::default();
        let mut cache: CacheLayer<i32> = CacheLayer::new(config);

        cache.put("key1", 42, 4);
        assert_eq!(cache.get("key1"), Some(&42));
    }

    #[test]
    fn test_lru_eviction() {
        let config = CacheConfig {
            capacity: 3,
            eviction_policy: CacheEvictionPolicy::LRU,
            ..Default::default()
        };

        let mut cache: CacheLayer<i32> = CacheLayer::new(config);

        cache.put("a", 1, 4);
        cache.put("b", 2, 4);
        cache.put("c", 3, 4);

        // Access a and c
        cache.get("a");
        cache.get("c");

        // Add d, should evict b (LRU)
        cache.put("d", 4, 4);

        assert!(cache.contains("a"));
        assert!(!cache.contains("b"));
        assert!(cache.contains("c"));
        assert!(cache.contains("d"));
    }

    #[test]
    fn test_clock_eviction() {
        let config = CacheConfig {
            capacity: 3,
            eviction_policy: CacheEvictionPolicy::Clock,
            ..Default::default()
        };

        let mut cache: CacheLayer<i32> = CacheLayer::new(config);

        cache.put("a", 1, 4);
        cache.put("b", 2, 4);
        cache.put("c", 3, 4);

        cache.put("d", 4, 4);

        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_hit_rate() {
        let config = CacheConfig::default();
        let mut cache: CacheLayer<i32> = CacheLayer::new(config);

        cache.put("a", 1, 4);

        cache.get("a"); // Hit
        cache.get("a"); // Hit
        cache.get("b"); // Miss

        assert!((cache.stats.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_ttl() {
        let config = CacheConfig {
            default_ttl_ms: Some(0), // Expire immediately
            ..Default::default()
        };

        let mut cache: CacheLayer<i32> = CacheLayer::new(config);

        cache.put("a", 1, 4);

        // Should be expired
        assert!(cache.get("a").is_none());
    }

    #[test]
    fn test_dirty_flush() {
        let config = CacheConfig {
            write_policy: WritePolicy::WriteBack,
            ..Default::default()
        };

        let mut cache: CacheLayer<i32> = CacheLayer::new(config);

        cache.put("a", 1, 4);
        cache.mark_dirty("a");

        let flushed = cache.flush();
        assert_eq!(flushed.len(), 1);
    }

    #[test]
    fn test_multi_level() {
        let l1 = CacheConfig {
            level: CacheLevel::L1,
            capacity: 2,
            ..Default::default()
        };

        let l2 = CacheConfig {
            level: CacheLevel::L2,
            capacity: 10,
            ..Default::default()
        };

        let mut cache: MultiLevelCache<i32> = MultiLevelCache::new(l1, l2, None);

        cache.put("a", 1, 4);

        assert_eq!(cache.get("a"), Some(1));
    }
}
