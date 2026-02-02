//! # Cache Intelligence Module
//!
//! AI-powered cache management and optimization.
//!
//! ## Key Features
//!
//! - **Cache Prediction**: Predict cache access patterns
//! - **Eviction Intelligence**: Smart cache eviction policies
//! - **Hit Rate Optimization**: Maximize cache hit rates
//! - **Multi-Level Caching**: Manage hierarchical caches
//! - **Workload Adaptation**: Adapt to changing workloads
//! - **Cache Warming**: Intelligent cache pre-warming

mod entry;
mod eviction;
mod intelligence;
mod manager;
mod multilevel;
mod pattern;
mod stats;
mod types;
mod warmer;

// Re-export types
// Re-export entry
pub use entry::CacheEntry;
// Re-export eviction
pub use eviction::EvictionOptimizer;
// Re-export intelligence
pub use intelligence::CacheIntelligence;
// Re-export manager
pub use manager::CacheManager;
// Re-export multilevel
pub use multilevel::{InclusionPolicy, MultiLevelCache};
// Re-export pattern
pub use pattern::{AccessPattern, AccessPatternTracker};
// Re-export stats
pub use stats::CacheStats;
pub use types::{CacheId, CacheKey, CacheLevel, CacheLineState, EvictionPolicy};
// Re-export warmer
pub use warmer::CacheWarmer;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry() {
        let mut entry = CacheEntry::new(1, 4096);
        assert_eq!(entry.access_count, 1);

        entry.access();
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_access_pattern_tracker() {
        let mut tracker = AccessPatternTracker::new(100);

        // Sequential access
        for i in 0..50 {
            tracker.record(i);
        }

        assert_eq!(tracker.pattern(), Some(AccessPattern::Sequential));
    }

    #[test]
    fn test_cache_manager() {
        let mut cache = CacheManager::new(1, CacheLevel::L1, 4096 * 10);

        cache.insert(1, 4096);
        assert!(cache.access(1));
        assert!(!cache.access(2));
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::new(1000);

        stats.record_hit(100);
        stats.record_miss();
        stats.record_hit(100);

        assert_eq!(stats.total_accesses, 3);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_multi_level_cache() {
        let mut mlc = MultiLevelCache::new();
        mlc.add_level(1, CacheLevel::L1, 4096);
        mlc.add_level(2, CacheLevel::Memory, 65536);

        mlc.insert(1, 1024, CacheLevel::Memory);
        assert_eq!(mlc.access(1), Some(CacheLevel::Memory));
    }
}
