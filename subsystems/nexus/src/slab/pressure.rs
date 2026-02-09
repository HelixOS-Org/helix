//! Memory Pressure Handler
//!
//! This module provides memory pressure detection and cache shrinking for slab allocators.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use super::{SlabCacheId, SlabCacheInfo};

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPressureLevel {
    /// No pressure
    None = 0,
    /// Low pressure
    Low = 1,
    /// Medium pressure
    Medium = 2,
    /// High pressure
    High = 3,
    /// Critical pressure
    Critical = 4,
}

impl MemoryPressureLevel {
    /// Get level name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Shrink action
#[derive(Debug, Clone)]
pub struct ShrinkAction {
    /// Cache ID to shrink
    pub cache_id: SlabCacheId,
    /// Slabs to free
    pub slabs_to_free: u64,
    /// Objects to reclaim
    pub objects_to_reclaim: u64,
    /// Expected memory freed (bytes)
    pub memory_freed: u64,
    /// Priority (higher = shrink first)
    pub priority: u8,
}

/// Memory pressure handler
pub struct MemoryPressureHandler {
    /// Current pressure level
    current_level: MemoryPressureLevel,
    /// Shrinker priority per cache
    shrinker_priority: BTreeMap<SlabCacheId, u8>,
    /// Total memory under slab control
    total_memory: AtomicU64,
    /// Available memory
    available_memory: AtomicU64,
    /// Shrink callbacks count
    shrink_count: AtomicU64,
    /// Total objects reclaimed
    objects_reclaimed: AtomicU64,
    /// Pressure thresholds (% of available)
    thresholds: [f32; 4], // Low, Medium, High, Critical
}

impl MemoryPressureHandler {
    /// Create new pressure handler
    pub fn new() -> Self {
        Self {
            current_level: MemoryPressureLevel::None,
            shrinker_priority: BTreeMap::new(),
            total_memory: AtomicU64::new(0),
            available_memory: AtomicU64::new(u64::MAX),
            shrink_count: AtomicU64::new(0),
            objects_reclaimed: AtomicU64::new(0),
            thresholds: [0.8, 0.6, 0.4, 0.2], // 80%, 60%, 40%, 20%
        }
    }

    /// Update memory statistics
    pub fn update_memory(&mut self, total: u64, available: u64) {
        self.total_memory.store(total, Ordering::Relaxed);
        self.available_memory.store(available, Ordering::Relaxed);

        // Update pressure level
        let ratio = available as f32 / total.max(1) as f32;
        self.current_level = if ratio > self.thresholds[0] {
            MemoryPressureLevel::None
        } else if ratio > self.thresholds[1] {
            MemoryPressureLevel::Low
        } else if ratio > self.thresholds[2] {
            MemoryPressureLevel::Medium
        } else if ratio > self.thresholds[3] {
            MemoryPressureLevel::High
        } else {
            MemoryPressureLevel::Critical
        };
    }

    /// Set shrinker priority for cache
    #[inline(always)]
    pub fn set_priority(&mut self, cache_id: SlabCacheId, priority: u8) {
        self.shrinker_priority.insert(cache_id, priority);
    }

    /// Get current pressure level
    #[inline(always)]
    pub fn current_level(&self) -> MemoryPressureLevel {
        self.current_level
    }

    /// Calculate shrink targets based on pressure
    pub fn calculate_shrink_targets(
        &self,
        caches: &BTreeMap<SlabCacheId, SlabCacheInfo>,
    ) -> Vec<ShrinkAction> {
        let mut actions = Vec::new();

        if self.current_level == MemoryPressureLevel::None {
            return actions;
        }

        // Calculate how much memory we need to free
        let target_pct = match self.current_level {
            MemoryPressureLevel::Low => 0.1,
            MemoryPressureLevel::Medium => 0.2,
            MemoryPressureLevel::High => 0.35,
            MemoryPressureLevel::Critical => 0.5,
            MemoryPressureLevel::None => return actions,
        };

        let total = self.total_memory.load(Ordering::Relaxed);
        let target_bytes = (total as f64 * target_pct) as u64;

        // Sort caches by priority and utilization
        let mut cache_list: Vec<_> = caches.iter()
            .filter(|(_, info)| info.utilization() < 0.9) // Don't shrink nearly full caches
            .collect();

        cache_list.sort_by(|(id_a, info_a), (id_b, info_b)| {
            let prio_a = self.shrinker_priority.get(id_a).copied().unwrap_or(5);
            let prio_b = self.shrinker_priority.get(id_b).copied().unwrap_or(5);

            // Sort by priority (ascending), then by utilization (ascending)
            prio_a.cmp(&prio_b)
                .then_with(|| {
                    let util_a = (info_a.utilization() * 100.0) as u32;
                    let util_b = (info_b.utilization() * 100.0) as u32;
                    util_a.cmp(&util_b)
                })
        });

        let mut freed_so_far = 0u64;

        for (cache_id, info) in cache_list {
            if freed_so_far >= target_bytes {
                break;
            }

            let freeable = info.wasted_memory();
            if freeable == 0 {
                continue;
            }

            let to_free = freeable.min(target_bytes - freed_so_far);
            let slabs = to_free / (info.aligned_size as u64 * info.objects_per_slab as u64).max(1);
            let objects = to_free / info.aligned_size as u64;

            actions.push(ShrinkAction {
                cache_id: *cache_id,
                slabs_to_free: slabs,
                objects_to_reclaim: objects,
                memory_freed: to_free,
                priority: self.shrinker_priority.get(cache_id).copied().unwrap_or(5),
            });

            freed_so_far += to_free;
        }

        actions
    }

    /// Record shrink operation
    #[inline(always)]
    pub fn record_shrink(&self, objects: u64) {
        self.shrink_count.fetch_add(1, Ordering::Relaxed);
        self.objects_reclaimed.fetch_add(objects, Ordering::Relaxed);
    }

    /// Get shrink count
    #[inline(always)]
    pub fn shrink_count(&self) -> u64 {
        self.shrink_count.load(Ordering::Relaxed)
    }

    /// Get objects reclaimed
    #[inline(always)]
    pub fn objects_reclaimed(&self) -> u64 {
        self.objects_reclaimed.load(Ordering::Relaxed)
    }

    /// Set thresholds
    #[inline(always)]
    pub fn set_thresholds(&mut self, low: f32, medium: f32, high: f32, critical: f32) {
        self.thresholds = [low, medium, high, critical];
    }
}

impl Default for MemoryPressureHandler {
    fn default() -> Self {
        Self::new()
    }
}
