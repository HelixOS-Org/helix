//! AI-powered cache intelligence coordination.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::multilevel::MultiLevelCache;
use super::types::{CacheKey, CacheLevel};
use super::warmer::CacheWarmer;

// ============================================================================
// CACHE INTELLIGENCE
// ============================================================================

/// Central cache intelligence coordinator
#[repr(align(64))]
pub struct CacheIntelligence {
    /// Multi-level cache
    mlc: MultiLevelCache,
    /// Cache warmer
    warmer: CacheWarmer,
    /// Hit rate history
    hit_rate_history: VecDeque<f64>,
    /// Max history
    max_history: usize,
}

impl CacheIntelligence {
    /// Create new cache intelligence
    pub fn new() -> Self {
        let mut mlc = MultiLevelCache::new();

        // Add default cache levels
        mlc.add_level(1, CacheLevel::L1, 32 * 1024);
        mlc.add_level(2, CacheLevel::L2, 256 * 1024);
        mlc.add_level(3, CacheLevel::L3, 8 * 1024 * 1024);
        mlc.add_level(4, CacheLevel::Memory, 1024 * 1024 * 1024);

        Self {
            mlc,
            warmer: CacheWarmer::default(),
            hit_rate_history: VecDeque::new(),
            max_history: 1000,
        }
    }

    /// Access cache
    #[inline(always)]
    pub fn access(&mut self, key: CacheKey) -> Option<CacheLevel> {
        self.mlc.access(key)
    }

    /// Insert into cache
    #[inline(always)]
    pub fn insert(&mut self, key: CacheKey, size: u32) {
        // Default to lowest level
        self.mlc.insert(key, size, CacheLevel::Memory);
    }

    /// Get prefetch suggestions
    #[inline]
    pub fn prefetch_suggestions(&self, level: CacheLevel, count: usize) -> Vec<CacheKey> {
        self.mlc
            .get_level(level)
            .map(|c| c.prefetch_suggestions(count))
            .unwrap_or_default()
    }

    /// Sample hit rate
    #[inline]
    pub fn sample(&mut self) {
        let stats = self.mlc.aggregate_stats();
        self.hit_rate_history.push_back(stats.hit_rate());

        if self.hit_rate_history.len() > self.max_history {
            self.hit_rate_history.pop_front();
        }
    }

    /// Get hit rate trend
    #[inline]
    pub fn hit_rate_trend(&self) -> f64 {
        if self.hit_rate_history.len() < 10 {
            return 0.0;
        }

        let recent = &self.hit_rate_history[self.hit_rate_history.len() - 10..];
        let first = recent[0];
        let last = recent[9];

        last - first
    }

    /// Get current hit rate
    #[inline(always)]
    pub fn hit_rate(&self) -> f64 {
        self.mlc.aggregate_stats().hit_rate()
    }

    /// Get multi-level cache
    #[inline(always)]
    pub fn mlc(&self) -> &MultiLevelCache {
        &self.mlc
    }

    /// Get mutable multi-level cache
    #[inline(always)]
    pub fn mlc_mut(&mut self) -> &mut MultiLevelCache {
        &mut self.mlc
    }

    /// Get warmer
    #[inline(always)]
    pub fn warmer(&self) -> &CacheWarmer {
        &self.warmer
    }

    /// Get mutable warmer
    #[inline(always)]
    pub fn warmer_mut(&mut self) -> &mut CacheWarmer {
        &mut self.warmer
    }

    /// Warm cache from candidates
    #[inline]
    pub fn warm(&mut self, count: usize) {
        let candidates = self.warmer.next_candidates(count);
        for key in candidates {
            self.mlc.insert(key, 4096, CacheLevel::Memory);
        }
    }
}

impl Default for CacheIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
