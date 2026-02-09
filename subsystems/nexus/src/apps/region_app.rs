// SPDX-License-Identifier: MIT
//! # Application Memory Region Manager
//!
//! Per-application VMA (Virtual Memory Area) region tracking:
//! - Region split/merge/resize lifecycle
//! - Interval tree for efficient range queries
//! - Region overlap detection and resolution
//! - Access permission gap analysis
//! - Memory map compaction scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Region type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    Code,
    Data,
    Heap,
    Stack,
    Mmap,
    SharedLib,
    Vdso,
    Guard,
    Anonymous,
}

/// A virtual memory region
#[derive(Debug, Clone)]
pub struct VmRegion {
    pub start: u64,
    pub end: u64,
    pub region_type: RegionType,
    pub prot: u32,
    pub flags: u32,
    pub mapped_file: Option<u64>,
    pub file_offset: u64,
    pub access_count: u64,
    pub fault_count: u64,
    pub last_access: u64,
}

impl VmRegion {
    #[inline(always)]
    pub fn size(&self) -> u64 { self.end.saturating_sub(self.start) }

    #[inline(always)]
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }

    #[inline(always)]
    pub fn overlaps(&self, other: &VmRegion) -> bool {
        self.start < other.end && other.start < self.end
    }

    #[inline(always)]
    pub fn is_adjacent(&self, other: &VmRegion) -> bool {
        self.end == other.start || other.end == self.start
    }

    #[inline]
    pub fn can_merge_with(&self, other: &VmRegion) -> bool {
        self.is_adjacent(other)
            && self.prot == other.prot
            && self.flags == other.flags
            && self.region_type == other.region_type
    }

    #[inline(always)]
    pub fn pages(&self) -> u64 { (self.size() + 4095) / 4096 }
}

/// Gap between two regions
#[derive(Debug, Clone)]
pub struct RegionGap {
    pub start: u64,
    pub end: u64,
    pub prev_region_type: Option<RegionType>,
    pub next_region_type: Option<RegionType>,
}

impl RegionGap {
    #[inline(always)]
    pub fn size(&self) -> u64 { self.end.saturating_sub(self.start) }
}

/// Region manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RegionAppStats {
    pub total_regions: u64,
    pub total_mapped_bytes: u64,
    pub splits: u64,
    pub merges: u64,
    pub resizes: u64,
    pub overlaps_resolved: u64,
}

/// Per-application VMA region manager
pub struct RegionAppManager {
    /// app_id → sorted list of regions (by start address)
    app_regions: BTreeMap<u64, Vec<VmRegion>>,
    stats: RegionAppStats,
}

impl RegionAppManager {
    pub fn new() -> Self {
        Self {
            app_regions: BTreeMap::new(),
            stats: RegionAppStats::default(),
        }
    }

    /// Add a region for an application, maintaining sorted order
    #[inline]
    pub fn add_region(&mut self, app_id: u64, region: VmRegion) {
        let regions = self.app_regions.entry(app_id).or_insert_with(Vec::new);

        // Insert in sorted order by start address
        let pos = regions.partition_point(|r| r.start < region.start);
        regions.insert(pos, region);
        self.stats.total_regions += 1;
    }

    /// Find the region containing a given address
    pub fn find_region(&self, app_id: u64, addr: u64) -> Option<&VmRegion> {
        let regions = self.app_regions.get(&app_id)?;
        // Binary search for the region containing addr
        let idx = regions.partition_point(|r| r.start <= addr);
        if idx > 0 {
            let candidate = &regions[idx - 1];
            if candidate.contains(addr) {
                return Some(candidate);
            }
        }
        None
    }

    /// Split a region at the given address
    pub fn split_region(&mut self, app_id: u64, split_addr: u64) -> bool {
        let regions = match self.app_regions.get_mut(&app_id) {
            Some(r) => r,
            None => return false,
        };

        let idx = regions.partition_point(|r| r.start <= split_addr);
        if idx == 0 { return false; }
        let idx = idx - 1;

        if !regions[idx].contains(split_addr) || split_addr == regions[idx].start {
            return false;
        }

        let original = regions[idx].clone();
        regions[idx].end = split_addr;

        let new_region = VmRegion {
            start: split_addr,
            end: original.end,
            region_type: original.region_type,
            prot: original.prot,
            flags: original.flags,
            mapped_file: original.mapped_file,
            file_offset: original.file_offset + (split_addr - original.start),
            access_count: 0,
            fault_count: 0,
            last_access: 0,
        };

        regions.insert(idx + 1, new_region);
        self.stats.splits += 1;
        self.stats.total_regions += 1;
        true
    }

    /// Merge adjacent compatible regions to reduce fragmentation
    pub fn merge_adjacent(&mut self, app_id: u64) -> u64 {
        let regions = match self.app_regions.get_mut(&app_id) {
            Some(r) => r,
            None => return 0,
        };

        let mut merged = 0u64;
        let mut i = 0;
        while i + 1 < regions.len() {
            if regions[i].can_merge_with(&regions[i + 1]) {
                regions[i].end = regions[i + 1].end;
                regions[i].access_count += regions[i + 1].access_count;
                regions.remove(i + 1);
                merged += 1;
                self.stats.merges += 1;
            } else {
                i += 1;
            }
        }
        merged
    }

    /// Find all gaps (unmapped ranges) in the address space
    pub fn find_gaps(&self, app_id: u64) -> Vec<RegionGap> {
        let regions = match self.app_regions.get(&app_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut gaps = Vec::new();
        for i in 0..regions.len().saturating_sub(1) {
            let gap_start = regions[i].end;
            let gap_end = regions[i + 1].start;
            if gap_end > gap_start {
                gaps.push(RegionGap {
                    start: gap_start,
                    end: gap_end,
                    prev_region_type: Some(regions[i].region_type),
                    next_region_type: Some(regions[i + 1].region_type),
                });
            }
        }
        gaps
    }

    /// Compute a compaction score (0.0 = compact, 1.0 = fragmented)
    pub fn compaction_score(&self, app_id: u64) -> f64 {
        let regions = match self.app_regions.get(&app_id) {
            Some(r) => r,
            None => return 0.0,
        };
        if regions.is_empty() { return 0.0; }

        let first = regions.first().map(|r| r.start).unwrap_or(0);
        let last = regions.last().map(|r| r.end).unwrap_or(0);
        let span = last.saturating_sub(first);
        if span == 0 { return 0.0; }

        let mapped: u64 = regions.iter().map(|r| r.size()).sum();
        1.0 - (mapped as f64 / span as f64)
    }

    #[inline(always)]
    pub fn region_count(&self, app_id: u64) -> usize {
        self.app_regions.get(&app_id).map(|r| r.len()).unwrap_or(0)
    }

    #[inline(always)]
    pub fn stats(&self) -> &RegionAppStats { &self.stats }
}
