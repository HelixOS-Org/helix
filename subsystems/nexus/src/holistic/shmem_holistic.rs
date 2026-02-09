// SPDX-License-Identifier: MIT
//! # Holistic Shared Memory Optimization
//!
//! System-wide shared memory analysis and optimization:
//! - Global shared memory utilization dashboard
//! - Cross-segment deduplication detection
//! - NUMA topology-aware placement optimization
//! - Orphan segment garbage collection
//! - Shared memory bandwidth monitoring

extern crate alloc;
use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct GlobalShmSegment {
    pub segment_id: u64,
    pub size: u64,
    pub attach_count: u32,
    pub numa_node: u32,
    pub bandwidth_bps: u64,
    pub last_access: u64,
    pub content_hash: u64,
}

impl GlobalShmSegment {
    #[inline(always)]
    pub fn is_orphan(&self, now: u64, threshold: u64) -> bool {
        self.attach_count == 0 && now.saturating_sub(self.last_access) > threshold
    }
    #[inline(always)]
    pub fn is_hot(&self, bw_threshold: u64) -> bool {
        self.bandwidth_bps > bw_threshold
    }
}

#[derive(Debug, Clone)]
pub struct NumaPlacement {
    pub segment_id: u64,
    pub current_node: u32,
    pub optimal_node: u32,
    pub access_from_nodes: ArrayMap<u64, 32>, // node → access_count
    pub migration_cost: u64,
}

impl NumaPlacement {
    #[inline(always)]
    pub fn should_migrate(&self) -> bool {
        self.current_node != self.optimal_node
            && self.migration_cost < self.total_remote_accesses() * 200 // remote access cost
    }
    #[inline]
    pub fn total_remote_accesses(&self) -> u64 {
        self.access_from_nodes.iter()
            .filter(|(&n, _)| n != self.current_node)
            .map(|(_, &c)| c)
            .sum()
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ShmHolisticStats {
    pub total_segments: u64,
    pub total_bytes: u64,
    pub orphan_segments: u64,
    pub dedup_opportunities: u64,
    pub numa_misplacements: u64,
    pub total_bandwidth: u64,
    pub gc_reclaimed_bytes: u64,
}

pub struct ShmHolisticManager {
    segments: BTreeMap<u64, GlobalShmSegment>,
    placements: BTreeMap<u64, NumaPlacement>,
    /// content_hash → segment_ids (for dedup detection)
    content_index: BTreeMap<u64, Vec<u64>>,
    orphan_threshold: u64,
    stats: ShmHolisticStats,
}

impl ShmHolisticManager {
    pub fn new(orphan_threshold: u64) -> Self {
        Self {
            segments: BTreeMap::new(),
            placements: BTreeMap::new(),
            content_index: BTreeMap::new(),
            orphan_threshold,
            stats: ShmHolisticStats::default(),
        }
    }

    #[inline]
    pub fn register_segment(&mut self, seg: GlobalShmSegment) {
        let hash = seg.content_hash;
        let id = seg.segment_id;
        self.stats.total_bytes += seg.size;
        self.stats.total_segments += 1;
        self.content_index.entry(hash).or_insert_with(Vec::new).push(id);
        self.segments.insert(id, seg);
    }

    /// Find segments with identical content
    #[inline]
    pub fn find_duplicates(&self) -> Vec<(u64, Vec<u64>)> {
        self.content_index.iter()
            .filter(|(_, ids)| ids.len() > 1)
            .map(|(&hash, ids)| (hash, ids.clone()))
            .collect()
    }

    /// Run orphan garbage collection
    pub fn gc_orphans(&mut self, now: u64) -> u64 {
        let orphans: Vec<u64> = self.segments.iter()
            .filter(|(_, s)| s.is_orphan(now, self.orphan_threshold))
            .map(|(id, _)| *id)
            .collect();

        let mut reclaimed = 0;
        for id in orphans {
            if let Some(seg) = self.segments.remove(&id) {
                reclaimed += seg.size;
                self.stats.orphan_segments += 1;
            }
        }
        self.stats.gc_reclaimed_bytes += reclaimed;
        reclaimed
    }

    /// Analyze NUMA placement for all segments
    #[inline]
    pub fn analyze_numa(&mut self) -> Vec<u64> {
        let mut misplaced = Vec::new();
        for (id, placement) in &self.placements {
            if placement.should_migrate() {
                misplaced.push(*id);
                self.stats.numa_misplacements += 1;
            }
        }
        misplaced
    }

    /// Update bandwidth measurement for a segment
    #[inline]
    pub fn update_bandwidth(&mut self, segment_id: u64, bps: u64) {
        if let Some(seg) = self.segments.get_mut(&segment_id) {
            seg.bandwidth_bps = seg.bandwidth_bps / 2 + bps / 2; // EMA
            self.stats.total_bandwidth = self.segments.values()
                .map(|s| s.bandwidth_bps).sum();
        }
    }

    /// Update NUMA access tracking
    pub fn record_numa_access(&mut self, segment_id: u64, from_node: u32) {
        let placement = self.placements.entry(segment_id).or_insert(NumaPlacement {
            segment_id, current_node: 0, optimal_node: 0,
            access_from_nodes: ArrayMap::new(0), migration_cost: 0,
        });
        *placement.access_from_nodes.entry(from_node).or_insert(0) += 1;
        // Recompute optimal: node with most accesses
        if let Some((&best, _)) = placement.access_from_nodes.iter()
            .max_by_key(|(_, &c)| c)
        {
            placement.optimal_node = best;
        }
    }

    #[inline(always)]
    pub fn segment(&self, id: u64) -> Option<&GlobalShmSegment> { self.segments.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &ShmHolisticStats { &self.stats }
}
