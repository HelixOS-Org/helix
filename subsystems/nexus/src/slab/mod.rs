//! Slab Allocator Intelligence
//!
//! This module provides slab allocator monitoring, analysis, and optimization.
//!
//! ## Modules
//!
//! - [`types`] - Core identifiers and flags
//! - [`cache`] - Slab cache info and statistics
//! - [`utilization`] - Utilization analysis
//! - [`fragmentation`] - Fragmentation detection
//! - [`lifetime`] - Object lifetime prediction
//! - [`cpu_cache`] - Per-CPU cache optimization
//! - [`pressure`] - Memory pressure handling
//! - [`intelligence`] - Comprehensive slab intelligence

#![no_std]

extern crate alloc;

pub mod types;
pub mod cache;
pub mod utilization;
pub mod fragmentation;
pub mod lifetime;
pub mod cpu_cache;
pub mod pressure;
pub mod intelligence;

// Re-export types
pub use types::{
    SlabCacheId, SlabId, NodeId, CpuId,
    SlabFlags, SlabAllocatorType, CacheState,
};

// Re-export cache
pub use cache::{SlabCacheInfo, SlabStats};

// Re-export utilization
pub use utilization::{
    UtilizationSample, UtilizationTrend, ResizeRecommendation,
    CacheUtilizationAnalyzer,
};

// Re-export fragmentation
pub use fragmentation::{
    FragmentationLevel, FragmentationSample,
    FragmentationAnalyzer,
};

// Re-export lifetime
pub use lifetime::{
    LifetimeBucket, LifetimeStats, PlacementStrategy,
    ObjectLifetimePredictor,
};

// Re-export cpu_cache
pub use cpu_cache::{CpuCacheStats, CpuCacheOptimizer};

// Re-export pressure
pub use pressure::{MemoryPressureLevel, ShrinkAction, MemoryPressureHandler};

// Re-export intelligence
pub use intelligence::{
    SlabAnalysis, SlabIssue, SlabIssueType,
    SlabRecommendation, SlabAction,
    SlabIntelligence,
};

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn test_slab_cache_id() {
        let id1 = SlabCacheId::new(1);
        let id2 = SlabCacheId::new(2);
        assert_eq!(id1.raw(), 1);
        assert!(id1 < id2);
    }

    #[test]
    fn test_slab_flags() {
        let mut flags = SlabFlags::NONE;
        assert!(!flags.contains(SlabFlags::HWCACHE_ALIGN));
        flags.add(SlabFlags::HWCACHE_ALIGN);
        assert!(flags.contains(SlabFlags::HWCACHE_ALIGN));
        flags.remove(SlabFlags::HWCACHE_ALIGN);
        assert!(!flags.contains(SlabFlags::HWCACHE_ALIGN));
    }

    #[test]
    fn test_cache_info() {
        let mut info = SlabCacheInfo::new(SlabCacheId::new(1), String::from("test"), 64);
        info.active_objects = 50;
        info.total_objects = 100;
        info.aligned_size = 64;

        assert_eq!(info.utilization(), 0.5);
        assert_eq!(info.memory_usage(), 6400);
        assert_eq!(info.wasted_memory(), 3200);
    }

    #[test]
    fn test_slab_stats() {
        let mut stats = SlabStats::default();
        stats.alloc_count = 1000;
        stats.cpu_cache_hits = 800;
        stats.alloc_failures = 10;
        stats.numa_remote_allocs = 50;

        assert!((stats.cpu_cache_hit_rate() - 0.8).abs() < 0.01);
        assert!((stats.numa_locality() - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_utilization_analyzer() {
        let mut analyzer = CacheUtilizationAnalyzer::new(SlabCacheId::new(1));

        for i in 0..20 {
            analyzer.record_sample(UtilizationSample {
                timestamp: i * 1_000_000_000,
                utilization: 0.5,
                active_objects: 50,
                total_objects: 100,
                memory_bytes: 6400,
            });
        }

        assert_eq!(analyzer.detect_trend(), UtilizationTrend::Stable);
    }

    #[test]
    fn test_fragmentation_analyzer() {
        let mut analyzer = FragmentationAnalyzer::new(SlabCacheId::new(1));

        let internal = analyzer.calculate_internal_fragmentation(60, 64);
        assert!(internal > 0.0);

        let external = analyzer.calculate_external_fragmentation(10, 5, 2, 80, 100);
        assert!(external > 0.0);
    }

    #[test]
    fn test_lifetime_predictor() {
        let mut predictor = ObjectLifetimePredictor::new(SlabCacheId::new(1));

        // Record short-lived objects
        for _ in 0..100 {
            predictor.record_lifetime(500_000); // 500µs
        }

        let stats = predictor.calculate_stats();
        assert!(stats.mean_ns > 0.0);
        assert_eq!(predictor.recommend_placement(), PlacementStrategy::CpuLocal);
    }

    #[test]
    fn test_cpu_cache_optimizer() {
        let mut optimizer = CpuCacheOptimizer::new(SlabCacheId::new(1), 32);
        let cpu = CpuId::new(0);

        optimizer.register_cpu(cpu);
        optimizer.update_stats(cpu, 20, 900, 100);

        let stats = optimizer.get_stats(cpu).unwrap();
        assert!((stats.hit_rate() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_memory_pressure_handler() {
        let mut handler = MemoryPressureHandler::new();

        handler.update_memory(1000, 900);
        assert_eq!(handler.current_level(), MemoryPressureLevel::None);

        handler.update_memory(1000, 100);
        assert!(handler.current_level() >= MemoryPressureLevel::Critical);
    }

    #[test]
    fn test_slab_intelligence() {
        let mut intel = SlabIntelligence::new();
        let cache_id = SlabCacheId::new(1);
        let cpu = CpuId::new(0);

        intel.register_cache(cache_id, String::from("test_cache"), 64);

        // Simulate allocations
        for _ in 0..100 {
            intel.record_allocation(cache_id, cpu, true);
        }

        // Simulate some frees
        for _ in 0..30 {
            intel.record_free(cache_id, 1_000_000);
        }

        let analysis = intel.analyze_cache(cache_id);
        assert!(analysis.is_some());
    }
}
