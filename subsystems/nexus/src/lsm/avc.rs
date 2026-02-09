//! Access Vector Cache
//!
//! AVC for caching security decisions.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{AvcEntry, ObjectClass, SecurityContext};

/// Access Vector Cache
pub struct Avc {
    /// Entries (keyed by hash of source:target:class)
    entries: BTreeMap<u64, AvcEntry>,
    /// Max entries
    max_entries: usize,
    /// Total lookups
    lookups: AtomicU64,
    /// Cache hits
    hits: AtomicU64,
    /// Cache misses
    misses: AtomicU64,
}

impl Avc {
    /// Create new AVC
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries,
            lookups: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Compute cache key
    fn cache_key(source: &SecurityContext, target: &SecurityContext, class: ObjectClass) -> u64 {
        // Simple hash combining
        let mut hash = 0u64;
        for b in source.to_string().as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*b as u64);
        }
        for b in target.to_string().as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*b as u64);
        }
        hash = hash.wrapping_mul(31).wrapping_add(class as u64);
        hash
    }

    /// Lookup entry
    pub fn lookup(
        &self,
        source: &SecurityContext,
        target: &SecurityContext,
        class: ObjectClass,
    ) -> Option<&AvcEntry> {
        self.lookups.fetch_add(1, Ordering::Relaxed);
        let key = Self::cache_key(source, target, class);

        if let Some(entry) = self.entries.get(&key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert entry
    pub fn insert(&mut self, entry: AvcEntry) {
        let key = Self::cache_key(&entry.source, &entry.target, entry.class);

        // Evict if needed
        if self.entries.len() >= self.max_entries {
            // Remove oldest (first in BTreeMap)
            if let Some(&oldest_key) = self.entries.keys().next() {
                self.entries.remove(&oldest_key);
            }
        }

        self.entries.insert(key, entry);
    }

    /// Invalidate entry
    #[inline(always)]
    pub fn invalidate(
        &mut self,
        source: &SecurityContext,
        target: &SecurityContext,
        class: ObjectClass,
    ) {
        let key = Self::cache_key(source, target, class);
        self.entries.remove(&key);
    }

    /// Clear all entries
    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get hit rate
    #[inline]
    pub fn hit_rate(&self) -> f32 {
        let total = self.lookups.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        (self.hits.load(Ordering::Relaxed) as f32 / total as f32) * 100.0
    }

    /// Get entry count
    #[inline(always)]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for Avc {
    fn default() -> Self {
        Self::new(10000)
    }
}
