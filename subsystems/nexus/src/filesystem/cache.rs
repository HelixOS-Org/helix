//! Page cache analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::Inode;
use crate::core::NexusTimestamp;
use crate::math;

// ============================================================================
// CACHED FILE INFO
// ============================================================================

/// Cached file information
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CachedFileInfo {
    /// Inode
    pub inode: Inode,
    /// Cached pages
    pub cached_pages: u32,
    /// Total pages
    pub total_pages: u32,
    /// Hit count
    pub hits: u64,
    /// Last access
    pub last_access: NexusTimestamp,
    /// Cache priority
    pub priority: f64,
}

// ============================================================================
// PAGE CACHE ANALYZER
// ============================================================================

/// Analyzes page cache usage
#[repr(align(64))]
pub struct PageCacheAnalyzer {
    /// Cached files
    cached_files: BTreeMap<Inode, CachedFileInfo>,
    /// Total cache size
    total_cache_size: u64,
    /// Cache capacity
    cache_capacity: u64,
    /// Hit count
    hits: AtomicU64,
    /// Miss count
    misses: AtomicU64,
}

impl PageCacheAnalyzer {
    /// Create new analyzer
    pub fn new(cache_capacity: u64) -> Self {
        Self {
            cached_files: BTreeMap::new(),
            total_cache_size: 0,
            cache_capacity,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Record cache hit
    #[inline]
    pub fn record_hit(&mut self, inode: Inode) {
        self.hits.fetch_add(1, Ordering::Relaxed);

        if let Some(info) = self.cached_files.get_mut(&inode) {
            info.hits += 1;
            info.last_access = NexusTimestamp::now();

            // Update priority
            info.priority = Self::calculate_priority_static(info);
        }
    }

    /// Record cache miss
    #[inline(always)]
    pub fn record_miss(&mut self, inode: Inode) {
        let _ = inode; // Used for tracking in more advanced implementations
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Add to cache
    pub fn add_to_cache(&mut self, inode: Inode, pages: u32, total_pages: u32) {
        let now = NexusTimestamp::now();
        let info = CachedFileInfo {
            inode,
            cached_pages: pages,
            total_pages,
            hits: 0,
            last_access: now,
            priority: 0.5,
        };

        self.total_cache_size += pages as u64 * 4096;
        self.cached_files.insert(inode, info);
    }

    /// Remove from cache
    #[inline]
    pub fn remove_from_cache(&mut self, inode: Inode) {
        if let Some(info) = self.cached_files.remove(&inode) {
            self.total_cache_size -= info.cached_pages as u64 * 4096;
        }
    }

    /// Calculate cache priority (static version)
    fn calculate_priority_static(info: &CachedFileInfo) -> f64 {
        let age = NexusTimestamp::now().duration_since(info.last_access) as f64;
        let recency = 1.0 / (age / 1_000_000_000.0 + 1.0);
        let frequency = math::ln(info.hits as f64 + 1.0) / 10.0;
        let coverage = info.cached_pages as f64 / info.total_pages.max(1) as f64;

        (recency * 0.4 + frequency * 0.4 + coverage * 0.2).min(1.0)
    }

    /// Calculate cache priority
    #[allow(dead_code)]
    fn calculate_priority(&self, info: &CachedFileInfo) -> f64 {
        Self::calculate_priority_static(info)
    }

    /// Get eviction candidates
    #[inline]
    pub fn eviction_candidates(&self, count: usize) -> Vec<Inode> {
        let mut files: Vec<_> = self.cached_files.iter().collect();
        files.sort_by(|a, b| a.1.priority.partial_cmp(&b.1.priority).unwrap());

        files.iter().take(count).map(|&(&inode, _)| inode).collect()
    }

    /// Get cache hit rate
    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Get cache fill ratio
    #[inline]
    pub fn fill_ratio(&self) -> f64 {
        if self.cache_capacity == 0 {
            0.0
        } else {
            self.total_cache_size as f64 / self.cache_capacity as f64
        }
    }

    /// Get hottest files in cache
    #[inline]
    pub fn hottest_files(&self, n: usize) -> Vec<&CachedFileInfo> {
        let mut files: Vec<_> = self.cached_files.values().collect();
        files.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
        files.truncate(n);
        files
    }
}

impl Default for PageCacheAnalyzer {
    fn default() -> Self {
        Self::new(1024 * 1024 * 1024) // 1GB default
    }
}
