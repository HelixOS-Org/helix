//! Slab Cache Information
//!
//! This module provides slab cache metadata and statistics structures.

use alloc::string::String;

use super::{CacheState, SlabAllocatorType, SlabCacheId, SlabFlags};

/// Slab cache information
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SlabCacheInfo {
    /// Cache ID
    pub id: SlabCacheId,
    /// Cache name
    pub name: String,
    /// Object size (bytes)
    pub object_size: usize,
    /// Aligned object size
    pub aligned_size: usize,
    /// Objects per slab
    pub objects_per_slab: u32,
    /// Current slab count
    pub slab_count: u64,
    /// Active objects
    pub active_objects: u64,
    /// Total objects (capacity)
    pub total_objects: u64,
    /// Cache flags
    pub flags: SlabFlags,
    /// Allocator type
    pub allocator_type: SlabAllocatorType,
    /// Current state
    pub state: CacheState,
    /// Creation timestamp
    pub created_at: u64,
    /// Constructor function name
    pub constructor: Option<String>,
    /// Order (pages per slab)
    pub order: u32,
}

impl SlabCacheInfo {
    /// Create new slab cache info
    pub fn new(id: SlabCacheId, name: String, object_size: usize) -> Self {
        Self {
            id,
            name,
            object_size,
            aligned_size: object_size,
            objects_per_slab: 1,
            slab_count: 0,
            active_objects: 0,
            total_objects: 0,
            flags: SlabFlags::NONE,
            allocator_type: SlabAllocatorType::Slub,
            state: CacheState::Active,
            created_at: 0,
            constructor: None,
            order: 0,
        }
    }

    /// Calculate utilization
    #[inline]
    pub fn utilization(&self) -> f32 {
        if self.total_objects == 0 {
            return 0.0;
        }
        self.active_objects as f32 / self.total_objects as f32
    }

    /// Calculate memory usage
    #[inline(always)]
    pub fn memory_usage(&self) -> u64 {
        self.total_objects * self.aligned_size as u64
    }

    /// Calculate wasted memory
    #[inline(always)]
    pub fn wasted_memory(&self) -> u64 {
        (self.total_objects - self.active_objects) * self.aligned_size as u64
    }

    /// Check if cache is mostly empty
    #[inline(always)]
    pub fn is_underutilized(&self, threshold: f32) -> bool {
        self.utilization() < threshold
    }
}

/// Slab allocation statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SlabStats {
    /// Total allocations
    pub alloc_count: u64,
    /// Total frees
    pub free_count: u64,
    /// Allocation failures
    pub alloc_failures: u64,
    /// Allocations from cpu cache
    pub cpu_cache_hits: u64,
    /// Allocations from partial slabs
    pub partial_hits: u64,
    /// Allocations requiring new slab
    pub new_slab_allocs: u64,
    /// Total bytes allocated
    pub bytes_allocated: u64,
    /// Total bytes freed
    pub bytes_freed: u64,
    /// Slabs created
    pub slabs_created: u64,
    /// Slabs destroyed
    pub slabs_destroyed: u64,
    /// Objects moved between CPUs
    pub objects_moved: u64,
    /// NUMA remote allocations
    pub numa_remote_allocs: u64,
}

impl SlabStats {
    /// Calculate allocation rate (allocs/s)
    #[inline]
    pub fn allocation_rate(&self, duration_ns: u64) -> f32 {
        if duration_ns == 0 {
            return 0.0;
        }
        self.alloc_count as f32 / (duration_ns as f32 / 1_000_000_000.0)
    }

    /// Calculate CPU cache hit rate
    #[inline]
    pub fn cpu_cache_hit_rate(&self) -> f32 {
        if self.alloc_count == 0 {
            return 0.0;
        }
        self.cpu_cache_hits as f32 / self.alloc_count as f32
    }

    /// Calculate failure rate
    #[inline]
    pub fn failure_rate(&self) -> f32 {
        let total = self.alloc_count + self.alloc_failures;
        if total == 0 {
            return 0.0;
        }
        self.alloc_failures as f32 / total as f32
    }

    /// Calculate NUMA locality
    #[inline]
    pub fn numa_locality(&self) -> f32 {
        if self.alloc_count == 0 {
            return 1.0;
        }
        1.0 - (self.numa_remote_allocs as f32 / self.alloc_count as f32)
    }
}
