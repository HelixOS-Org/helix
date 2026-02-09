// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Munmap (memory unmap bridge)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;

/// Munmap result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMunmapResult { Success, NotMapped, PartialUnmap, InvalidRange }

/// Munmap stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeMunmapStats { pub total_ops: u64, pub successful: u64, pub partial: u64, pub failed: u64, pub total_unmapped_bytes: u64 }

/// Manager for munmap bridge
#[repr(align(64))]
pub struct BridgeMunmapManager {
    mapped_regions: LinearMap<u64, 64>,
    stats: BridgeMunmapStats,
}

impl BridgeMunmapManager {
    pub fn new() -> Self {
        Self { mapped_regions: LinearMap::new(), stats: BridgeMunmapStats { total_ops: 0, successful: 0, partial: 0, failed: 0, total_unmapped_bytes: 0 } }
    }

    #[inline(always)]
    pub fn track_map(&mut self, addr: u64, length: u64) { self.mapped_regions.insert(addr, length); }

    #[inline]
    pub fn munmap(&mut self, addr: u64, length: u64) -> BridgeMunmapResult {
        self.stats.total_ops += 1;
        if let Some(&mapped_len) = self.mapped_regions.get(addr) {
            if length >= mapped_len { self.mapped_regions.remove(addr); self.stats.successful += 1; }
            else { self.stats.partial += 1; }
            self.stats.total_unmapped_bytes += length;
            BridgeMunmapResult::Success
        } else {
            self.stats.failed += 1; BridgeMunmapResult::NotMapped
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeMunmapStats { &self.stats }
}
