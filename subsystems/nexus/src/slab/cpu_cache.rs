//! CPU Cache Optimizer
//!
//! This module provides per-CPU cache optimization for slab allocators.

use alloc::collections::BTreeMap;
use super::{SlabCacheId, CpuId};

/// Per-CPU cache statistics
#[derive(Debug, Clone, Default)]
pub struct CpuCacheStats {
    /// CPU ID
    pub cpu_id: u32,
    /// Objects in cache
    pub cached_objects: u32,
    /// Cache capacity
    pub capacity: u32,
    /// Hits (allocations from cache)
    pub hits: u64,
    /// Misses (allocations not from cache)
    pub misses: u64,
    /// Refills from partial slabs
    pub refills: u64,
    /// Flushes to partial slabs
    pub flushes: u64,
}

impl CpuCacheStats {
    /// Create new CPU cache stats
    pub fn new(cpu_id: CpuId) -> Self {
        Self {
            cpu_id: cpu_id.raw(),
            ..Default::default()
        }
    }

    /// Calculate hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f32 / total as f32
    }

    /// Calculate fill level
    pub fn fill_level(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.cached_objects as f32 / self.capacity as f32
    }
}

/// CPU cache optimizer
pub struct CpuCacheOptimizer {
    /// Cache ID
    cache_id: SlabCacheId,
    /// Per-CPU statistics
    pub(crate) cpu_stats: BTreeMap<CpuId, CpuCacheStats>,
    /// Default cache size
    default_size: u32,
    /// Minimum cache size
    min_size: u32,
    /// Maximum cache size
    max_size: u32,
    /// Hit rate target
    hit_rate_target: f32,
    /// Size adjustments made
    adjustments: u64,
}

impl CpuCacheOptimizer {
    /// Create new CPU cache optimizer
    pub fn new(cache_id: SlabCacheId, default_size: u32) -> Self {
        Self {
            cache_id,
            cpu_stats: BTreeMap::new(),
            default_size,
            min_size: 8,
            max_size: 256,
            hit_rate_target: 0.90,
            adjustments: 0,
        }
    }

    /// Register CPU
    pub fn register_cpu(&mut self, cpu_id: CpuId) {
        let mut stats = CpuCacheStats::new(cpu_id);
        stats.capacity = self.default_size;
        self.cpu_stats.insert(cpu_id, stats);
    }

    /// Update CPU cache stats
    pub fn update_stats(&mut self, cpu_id: CpuId, cached: u32, hits: u64, misses: u64) {
        if let Some(stats) = self.cpu_stats.get_mut(&cpu_id) {
            stats.cached_objects = cached;
            stats.hits = hits;
            stats.misses = misses;
        }
    }

    /// Record hit
    pub fn record_hit(&mut self, cpu_id: CpuId) {
        if let Some(stats) = self.cpu_stats.get_mut(&cpu_id) {
            stats.hits += 1;
        }
    }

    /// Record miss
    pub fn record_miss(&mut self, cpu_id: CpuId) {
        if let Some(stats) = self.cpu_stats.get_mut(&cpu_id) {
            stats.misses += 1;
        }
    }

    /// Record refill
    pub fn record_refill(&mut self, cpu_id: CpuId, count: u64) {
        if let Some(stats) = self.cpu_stats.get_mut(&cpu_id) {
            stats.refills += count;
        }
    }

    /// Record flush
    pub fn record_flush(&mut self, cpu_id: CpuId, count: u64) {
        if let Some(stats) = self.cpu_stats.get_mut(&cpu_id) {
            stats.flushes += count;
        }
    }

    /// Optimize cache size for CPU
    pub fn optimize_cpu_cache(&mut self, cpu_id: CpuId) -> Option<u32> {
        let stats = self.cpu_stats.get(&cpu_id)?;

        let hit_rate = stats.hit_rate();
        let fill_level = stats.fill_level();

        let new_size = if hit_rate < self.hit_rate_target && fill_level > 0.8 {
            // Cache too small, increase
            (stats.capacity * 3 / 2).min(self.max_size)
        } else if hit_rate > self.hit_rate_target + 0.05 && fill_level < 0.3 {
            // Cache too large, decrease
            (stats.capacity * 2 / 3).max(self.min_size)
        } else {
            return None;
        };

        if new_size != stats.capacity {
            self.adjustments += 1;
            if let Some(stats) = self.cpu_stats.get_mut(&cpu_id) {
                stats.capacity = new_size;
            }
            Some(new_size)
        } else {
            None
        }
    }

    /// Get average hit rate across all CPUs
    pub fn average_hit_rate(&self) -> f32 {
        if self.cpu_stats.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.cpu_stats.values().map(|s| s.hit_rate()).sum();
        sum / self.cpu_stats.len() as f32
    }

    /// Get CPU stats
    pub fn get_stats(&self, cpu_id: CpuId) -> Option<&CpuCacheStats> {
        self.cpu_stats.get(&cpu_id)
    }

    /// Get cache ID
    pub fn cache_id(&self) -> SlabCacheId {
        self.cache_id
    }

    /// Get adjustment count
    pub fn adjustments(&self) -> u64 {
        self.adjustments
    }

    /// Set hit rate target
    pub fn set_hit_rate_target(&mut self, target: f32) {
        self.hit_rate_target = target;
    }
}
