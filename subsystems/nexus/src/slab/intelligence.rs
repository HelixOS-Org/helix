//! Slab Intelligence
//!
//! This module provides comprehensive slab allocator analysis and optimization.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    SlabCacheId, CpuId,
    SlabCacheInfo, SlabStats,
    UtilizationSample, CacheUtilizationAnalyzer,
    FragmentationLevel, FragmentationSample, FragmentationAnalyzer,
    LifetimeStats, ObjectLifetimePredictor,
    CpuCacheOptimizer,
    MemoryPressureLevel, ShrinkAction, MemoryPressureHandler,
};

/// Slab analysis result
#[derive(Debug, Clone)]
pub struct SlabAnalysis {
    /// Cache ID
    pub cache_id: SlabCacheId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Fragmentation level
    pub fragmentation: FragmentationLevel,
    /// Utilization
    pub utilization: f32,
    /// Detected issues
    pub issues: Vec<SlabIssue>,
    /// Recommendations
    pub recommendations: Vec<SlabRecommendation>,
}

/// Slab issue
#[derive(Debug, Clone)]
pub struct SlabIssue {
    /// Issue type
    pub issue_type: SlabIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Slab issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabIssueType {
    /// Low utilization
    LowUtilization,
    /// High fragmentation
    HighFragmentation,
    /// Memory pressure
    MemoryPressure,
    /// CPU cache inefficiency
    CpuCacheInefficiency,
    /// NUMA imbalance
    NumaImbalance,
    /// High allocation failure rate
    HighFailureRate,
}

/// Slab recommendation
#[derive(Debug, Clone)]
pub struct SlabRecommendation {
    /// Action type
    pub action: SlabAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Slab action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabAction {
    /// Shrink cache
    Shrink,
    /// Grow cache
    Grow,
    /// Defragment
    Defragment,
    /// Merge with another cache
    Merge,
    /// Adjust CPU cache size
    AdjustCpuCache,
    /// Enable NUMA awareness
    EnableNuma,
    /// Change object order
    ChangeOrder,
}

/// Slab Intelligence - comprehensive slab allocator analysis and optimization
pub struct SlabIntelligence {
    /// Slab caches
    caches: BTreeMap<SlabCacheId, SlabCacheInfo>,
    /// Per-cache statistics
    stats: BTreeMap<SlabCacheId, SlabStats>,
    /// Utilization analyzers
    utilization_analyzers: BTreeMap<SlabCacheId, CacheUtilizationAnalyzer>,
    /// Fragmentation analyzers
    fragmentation_analyzers: BTreeMap<SlabCacheId, FragmentationAnalyzer>,
    /// Lifetime predictors
    lifetime_predictors: BTreeMap<SlabCacheId, ObjectLifetimePredictor>,
    /// CPU cache optimizers
    pub(crate) cpu_cache_optimizers: BTreeMap<SlabCacheId, CpuCacheOptimizer>,
    /// Memory pressure handler
    pressure_handler: MemoryPressureHandler,
    /// Total allocations
    total_allocations: AtomicU64,
    /// Total frees
    total_frees: AtomicU64,
    /// Analysis interval
    analysis_interval_ns: u64,
    /// Last analysis timestamp
    last_analysis_ns: u64,
}

impl SlabIntelligence {
    /// Create new slab intelligence
    pub fn new() -> Self {
        Self {
            caches: BTreeMap::new(),
            stats: BTreeMap::new(),
            utilization_analyzers: BTreeMap::new(),
            fragmentation_analyzers: BTreeMap::new(),
            lifetime_predictors: BTreeMap::new(),
            cpu_cache_optimizers: BTreeMap::new(),
            pressure_handler: MemoryPressureHandler::new(),
            total_allocations: AtomicU64::new(0),
            total_frees: AtomicU64::new(0),
            analysis_interval_ns: 1_000_000_000, // 1 second
            last_analysis_ns: 0,
        }
    }

    /// Register slab cache
    pub fn register_cache(&mut self, id: SlabCacheId, name: String, object_size: usize) {
        let info = SlabCacheInfo::new(id, name, object_size);
        self.caches.insert(id, info);
        self.stats.insert(id, SlabStats::default());
        self.utilization_analyzers.insert(id, CacheUtilizationAnalyzer::new(id));
        self.fragmentation_analyzers.insert(id, FragmentationAnalyzer::new(id));
        self.lifetime_predictors.insert(id, ObjectLifetimePredictor::new(id));
        self.cpu_cache_optimizers.insert(id, CpuCacheOptimizer::new(id, 32));
    }

    /// Record allocation
    pub fn record_allocation(&mut self, cache_id: SlabCacheId, cpu_id: CpuId, from_cpu_cache: bool) {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);

        if let Some(stats) = self.stats.get_mut(&cache_id) {
            stats.alloc_count += 1;
            if from_cpu_cache {
                stats.cpu_cache_hits += 1;
            }
        }

        if let Some(info) = self.caches.get_mut(&cache_id) {
            info.active_objects += 1;
        }

        // Record CPU cache hit/miss
        if let Some(optimizer) = self.cpu_cache_optimizers.get_mut(&cache_id) {
            if from_cpu_cache {
                optimizer.record_hit(cpu_id);
            } else {
                optimizer.record_miss(cpu_id);
            }
        }
    }

    /// Record free
    pub fn record_free(&mut self, cache_id: SlabCacheId, lifetime_ns: u64) {
        self.total_frees.fetch_add(1, Ordering::Relaxed);

        if let Some(stats) = self.stats.get_mut(&cache_id) {
            stats.free_count += 1;
        }

        if let Some(info) = self.caches.get_mut(&cache_id) {
            info.active_objects = info.active_objects.saturating_sub(1);
        }

        // Record lifetime
        if let Some(predictor) = self.lifetime_predictors.get_mut(&cache_id) {
            predictor.record_lifetime(lifetime_ns);
        }
    }

    /// Record allocation failure
    pub fn record_failure(&mut self, cache_id: SlabCacheId) {
        if let Some(stats) = self.stats.get_mut(&cache_id) {
            stats.alloc_failures += 1;
        }
    }

    /// Update cache utilization
    pub fn update_utilization(&mut self, cache_id: SlabCacheId, timestamp: u64) {
        if let Some(info) = self.caches.get(&cache_id) {
            let sample = UtilizationSample {
                timestamp,
                utilization: info.utilization(),
                active_objects: info.active_objects,
                total_objects: info.total_objects,
                memory_bytes: info.memory_usage(),
            };

            if let Some(analyzer) = self.utilization_analyzers.get_mut(&cache_id) {
                analyzer.record_sample(sample);
            }
        }
    }

    /// Update fragmentation
    pub fn update_fragmentation(&mut self, cache_id: SlabCacheId, timestamp: u64, partial: u64, full: u64, empty: u64) {
        if let Some(info) = self.caches.get(&cache_id) {
            if let Some(analyzer) = self.fragmentation_analyzers.get_mut(&cache_id) {
                let internal = analyzer.calculate_internal_fragmentation(info.object_size, info.aligned_size);
                let external = analyzer.calculate_external_fragmentation(partial, full, empty, info.active_objects, info.total_objects);

                let sample = FragmentationSample {
                    timestamp,
                    internal_frag: internal,
                    external_frag: external,
                    partial_slabs: partial,
                    full_slabs: full,
                    empty_slabs: empty,
                };

                analyzer.record_sample(sample);
            }
        }
    }

    /// Analyze cache
    pub fn analyze_cache(&self, cache_id: SlabCacheId) -> Option<SlabAnalysis> {
        let info = self.caches.get(&cache_id)?;
        let stats = self.stats.get(&cache_id)?;

        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check utilization
        let utilization = info.utilization();
        if utilization < 0.25 {
            health_score -= 20.0;
            issues.push(SlabIssue {
                issue_type: SlabIssueType::LowUtilization,
                severity: 5,
                description: String::from("Low cache utilization"),
            });
            recommendations.push(SlabRecommendation {
                action: SlabAction::Shrink,
                expected_improvement: 15.0,
                reason: String::from("Release unused slabs"),
            });
        }

        // Check fragmentation
        let frag_level = self.fragmentation_analyzers.get(&cache_id)
            .map(|a| a.current_level())
            .unwrap_or(FragmentationLevel::None);

        if frag_level >= FragmentationLevel::High {
            health_score -= 25.0;
            issues.push(SlabIssue {
                issue_type: SlabIssueType::HighFragmentation,
                severity: 7,
                description: String::from("High cache fragmentation"),
            });
            recommendations.push(SlabRecommendation {
                action: SlabAction::Defragment,
                expected_improvement: 20.0,
                reason: String::from("Reduce memory fragmentation"),
            });
        }

        // Check failure rate
        if stats.failure_rate() > 0.01 {
            health_score -= 30.0;
            issues.push(SlabIssue {
                issue_type: SlabIssueType::HighFailureRate,
                severity: 9,
                description: String::from("High allocation failure rate"),
            });
            recommendations.push(SlabRecommendation {
                action: SlabAction::Grow,
                expected_improvement: 25.0,
                reason: String::from("Increase cache capacity"),
            });
        }

        // Check CPU cache efficiency
        if let Some(optimizer) = self.cpu_cache_optimizers.get(&cache_id) {
            if optimizer.average_hit_rate() < 0.7 {
                health_score -= 15.0;
                issues.push(SlabIssue {
                    issue_type: SlabIssueType::CpuCacheInefficiency,
                    severity: 5,
                    description: String::from("Low CPU cache hit rate"),
                });
                recommendations.push(SlabRecommendation {
                    action: SlabAction::AdjustCpuCache,
                    expected_improvement: 10.0,
                    reason: String::from("Optimize per-CPU cache size"),
                });
            }
        }

        // Check NUMA locality
        if stats.numa_locality() < 0.8 {
            health_score -= 10.0;
            issues.push(SlabIssue {
                issue_type: SlabIssueType::NumaImbalance,
                severity: 4,
                description: String::from("Poor NUMA locality"),
            });
            recommendations.push(SlabRecommendation {
                action: SlabAction::EnableNuma,
                expected_improvement: 8.0,
                reason: String::from("Enable NUMA-aware allocation"),
            });
        }

        // Check memory pressure
        let pressure = self.pressure_handler.current_level();
        if pressure >= MemoryPressureLevel::High {
            health_score -= 20.0;
            issues.push(SlabIssue {
                issue_type: SlabIssueType::MemoryPressure,
                severity: 8,
                description: String::from("High memory pressure"),
            });
        }

        health_score = health_score.max(0.0);

        Some(SlabAnalysis {
            cache_id,
            health_score,
            fragmentation: frag_level,
            utilization,
            issues,
            recommendations,
        })
    }

    /// Get shrink targets based on memory pressure
    pub fn get_shrink_targets(&self) -> Vec<ShrinkAction> {
        self.pressure_handler.calculate_shrink_targets(&self.caches)
    }

    /// Update memory pressure
    pub fn update_memory_pressure(&mut self, total: u64, available: u64) {
        self.pressure_handler.update_memory(total, available);
    }

    /// Get cache info
    pub fn get_cache(&self, cache_id: SlabCacheId) -> Option<&SlabCacheInfo> {
        self.caches.get(&cache_id)
    }

    /// Get cache stats
    pub fn get_stats(&self, cache_id: SlabCacheId) -> Option<&SlabStats> {
        self.stats.get(&cache_id)
    }

    /// Get lifetime stats
    pub fn get_lifetime_stats(&self, cache_id: SlabCacheId) -> Option<LifetimeStats> {
        self.lifetime_predictors.get(&cache_id).map(|p| p.calculate_stats())
    }

    /// Get pressure handler
    pub fn pressure_handler(&self) -> &MemoryPressureHandler {
        &self.pressure_handler
    }

    /// Get pressure handler mutably
    pub fn pressure_handler_mut(&mut self) -> &mut MemoryPressureHandler {
        &mut self.pressure_handler
    }

    /// Get cache count
    pub fn cache_count(&self) -> usize {
        self.caches.len()
    }

    /// Get total allocations
    pub fn total_allocations(&self) -> u64 {
        self.total_allocations.load(Ordering::Relaxed)
    }

    /// Get total frees
    pub fn total_frees(&self) -> u64 {
        self.total_frees.load(Ordering::Relaxed)
    }

    /// Perform periodic maintenance
    pub fn periodic_maintenance(&mut self, current_time_ns: u64) {
        if current_time_ns - self.last_analysis_ns < self.analysis_interval_ns {
            return;
        }
        self.last_analysis_ns = current_time_ns;

        // Update utilization for all caches
        let cache_ids: Vec<_> = self.caches.keys().copied().collect();
        for cache_id in cache_ids {
            self.update_utilization(cache_id, current_time_ns);
        }

        // Optimize CPU caches
        for optimizer in self.cpu_cache_optimizers.values_mut() {
            let cpu_ids: Vec<_> = optimizer.cpu_stats.keys().copied().collect();
            for cpu_id in cpu_ids {
                let _ = optimizer.optimize_cpu_cache(cpu_id);
            }
        }
    }
}

impl Default for SlabIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
