//! # Holistic Slab Allocator Optimizer
//!
//! System-wide slab cache optimization with holistic awareness:
//! - Per-cache fragmentation analysis
//! - Object reclaim and cache shrinking
//! - NUMA-aware slab allocation
//! - Slab merging for similar-sized objects
//! - Per-CPU partial slab management
//! - Memory accounting per cache

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Slab allocator backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabAllocator {
    Slub,
    Slab,
    Slob,
}

/// Cache shrink urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShrinkUrgency {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Per-slab page state
#[derive(Debug, Clone)]
pub struct SlabPage {
    pub page_frame: u64,
    pub objects_total: u32,
    pub objects_used: u32,
    pub frozen: bool,
    pub numa_node: u32,
}

impl SlabPage {
    pub fn new(pf: u64, total: u32, numa: u32) -> Self {
        Self {
            page_frame: pf,
            objects_total: total,
            objects_used: 0,
            frozen: false,
            numa_node: numa,
        }
    }

    #[inline(always)]
    pub fn free_objects(&self) -> u32 {
        self.objects_total.saturating_sub(self.objects_used)
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.objects_total == 0 { return 0.0; }
        self.objects_used as f64 / self.objects_total as f64
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.objects_used == 0 }
    #[inline(always)]
    pub fn is_full(&self) -> bool { self.objects_used >= self.objects_total }
}

/// Slab cache descriptor
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SlabCache {
    pub name: String,
    pub name_hash: u64,
    pub object_size: u32,
    pub align: u32,
    pub slab_order: u8,
    pub objects_per_slab: u32,
    pub total_slabs: u64,
    pub partial_slabs: u64,
    pub full_slabs: u64,
    pub free_slabs: u64,
    pub total_objects: u64,
    pub active_objects: u64,
    pub alloc_count: u64,
    pub free_count: u64,
    pub refill_count: u64,
    pub ctor_calls: u64,
    pub numa_local_allocs: u64,
    pub numa_remote_allocs: u64,
    pub mergeable: bool,
    pub reclaimable: bool,
}

impl SlabCache {
    pub fn new(name: String, object_size: u32, align: u32) -> Self {
        let name_hash = {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in name.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        };
        Self {
            name,
            name_hash,
            object_size,
            align,
            slab_order: 0,
            objects_per_slab: 0,
            total_slabs: 0,
            partial_slabs: 0,
            full_slabs: 0,
            free_slabs: 0,
            total_objects: 0,
            active_objects: 0,
            alloc_count: 0,
            free_count: 0,
            refill_count: 0,
            ctor_calls: 0,
            numa_local_allocs: 0,
            numa_remote_allocs: 0,
            mergeable: true,
            reclaimable: true,
        }
    }

    #[inline]
    pub fn fragmentation(&self) -> f64 {
        if self.total_objects == 0 { return 0.0; }
        let wasted = self.total_objects - self.active_objects;
        wasted as f64 / self.total_objects as f64
    }

    #[inline(always)]
    pub fn memory_used_bytes(&self) -> u64 {
        self.active_objects * self.object_size as u64
    }

    #[inline(always)]
    pub fn memory_wasted_bytes(&self) -> u64 {
        let total = self.total_objects * self.object_size as u64;
        total.saturating_sub(self.memory_used_bytes())
    }

    #[inline]
    pub fn numa_locality_ratio(&self) -> f64 {
        let total = self.numa_local_allocs + self.numa_remote_allocs;
        if total == 0 { return 1.0; }
        self.numa_local_allocs as f64 / total as f64
    }

    #[inline(always)]
    pub fn reclaimable_pages(&self) -> u64 {
        if !self.reclaimable { return 0; }
        self.free_slabs
    }
}

/// Merge candidate (two caches with similar object sizes)
#[derive(Debug, Clone)]
pub struct MergeCandidate {
    pub cache_a_hash: u64,
    pub cache_b_hash: u64,
    pub size_a: u32,
    pub size_b: u32,
    pub savings_pages: u64,
}

/// Holistic Slab Optimizer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticSlabStats {
    pub total_caches: usize,
    pub total_slab_pages: u64,
    pub total_fragmentation_pct: f64,
    pub total_wasted_bytes: u64,
    pub reclaimable_pages: u64,
    pub merge_candidates: usize,
}

/// Holistic Slab Allocator Optimizer
pub struct HolisticSlabOptimizer {
    caches: BTreeMap<u64, SlabCache>, // key: name_hash
    merge_candidates: Vec<MergeCandidate>,
    stats: HolisticSlabStats,
}

impl HolisticSlabOptimizer {
    pub fn new() -> Self {
        Self {
            caches: BTreeMap::new(),
            merge_candidates: Vec::new(),
            stats: HolisticSlabStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_cache(&mut self, cache: SlabCache) {
        self.caches.insert(cache.name_hash, cache);
    }

    #[inline]
    pub fn update_cache(&mut self, name_hash: u64, active: u64, total: u64, slabs: u64) {
        if let Some(cache) = self.caches.get_mut(&name_hash) {
            cache.active_objects = active;
            cache.total_objects = total;
            cache.total_slabs = slabs;
        }
    }

    /// Find merge candidates
    pub fn find_merges(&mut self) {
        self.merge_candidates.clear();
        let caches: Vec<(u64, u32, bool)> = self.caches.iter()
            .map(|(&hash, c)| (hash, c.object_size, c.mergeable))
            .collect();

        for i in 0..caches.len() {
            if !caches[i].2 { continue; }
            for j in (i + 1)..caches.len() {
                if !caches[j].2 { continue; }
                let (size_a, size_b) = (caches[i].1, caches[j].1);
                let diff = if size_a > size_b { size_a - size_b } else { size_b - size_a };
                let max_size = size_a.max(size_b);
                if max_size > 0 && diff as f64 / max_size as f64 <= 0.125 {
                    // Within 12.5% size — merge candidate
                    let smaller = self.caches.get(&caches[i].0)
                        .map(|c| c.free_slabs).unwrap_or(0);
                    self.merge_candidates.push(MergeCandidate {
                        cache_a_hash: caches[i].0,
                        cache_b_hash: caches[j].0,
                        size_a,
                        size_b,
                        savings_pages: smaller,
                    });
                }
            }
        }
    }

    /// Get caches by fragmentation (worst first)
    #[inline]
    pub fn fragmented_caches(&self, min_frag: f64) -> Vec<u64> {
        let mut entries: Vec<(u64, f64)> = self.caches.iter()
            .filter(|(_, c)| c.fragmentation() >= min_frag)
            .map(|(&hash, c)| (hash, c.fragmentation()))
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        entries.iter().map(|(h, _)| *h).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_caches = self.caches.len();
        self.stats.total_slab_pages = self.caches.values().map(|c| c.total_slabs).sum();
        let total_obj: u64 = self.caches.values().map(|c| c.total_objects).sum();
        let active_obj: u64 = self.caches.values().map(|c| c.active_objects).sum();
        self.stats.total_fragmentation_pct = if total_obj > 0 {
            (total_obj - active_obj) as f64 / total_obj as f64 * 100.0
        } else { 0.0 };
        self.stats.total_wasted_bytes = self.caches.values().map(|c| c.memory_wasted_bytes()).sum();
        self.stats.reclaimable_pages = self.caches.values().map(|c| c.reclaimable_pages()).sum();
        self.stats.merge_candidates = self.merge_candidates.len();
    }

    #[inline(always)]
    pub fn cache(&self, hash: u64) -> Option<&SlabCache> { self.caches.get(&hash) }
    #[inline(always)]
    pub fn stats(&self) -> &HolisticSlabStats { &self.stats }
}
