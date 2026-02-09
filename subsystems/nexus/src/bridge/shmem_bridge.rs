// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Shmem (shared memory bridge)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Shared memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeShmemType { SysV, Posix, Memfd, Anonymous }

/// Shared memory region
#[derive(Debug, Clone)]
pub struct BridgeShmemRegion {
    pub id: u64,
    pub shm_type: BridgeShmemType,
    pub size: u64,
    pub attach_count: u32,
    pub creator_pid: u64,
    pub permissions: u32,
}

/// Shmem stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeShmemStats {
    pub total_created: u64,
    pub total_attached: u64,
    pub total_detached: u64,
    pub active_segments: u64,
    pub total_bytes: u64,
}

/// Manager for shmem bridge
#[repr(align(64))]
pub struct BridgeShmemManager {
    segments: BTreeMap<u64, BridgeShmemRegion>,
    next_id: u64,
    stats: BridgeShmemStats,
}

impl BridgeShmemManager {
    pub fn new() -> Self {
        Self {
            segments: BTreeMap::new(),
            next_id: 1,
            stats: BridgeShmemStats { total_created: 0, total_attached: 0, total_detached: 0, active_segments: 0, total_bytes: 0 },
        }
    }

    #[inline]
    pub fn create(&mut self, size: u64, shm_type: BridgeShmemType, creator: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let region = BridgeShmemRegion { id, shm_type, size, attach_count: 0, creator_pid: creator, permissions: 0o666 };
        self.segments.insert(id, region);
        self.stats.total_created += 1; self.stats.active_segments += 1; self.stats.total_bytes += size;
        id
    }

    #[inline(always)]
    pub fn attach(&mut self, id: u64) -> bool {
        if let Some(r) = self.segments.get_mut(&id) { r.attach_count += 1; self.stats.total_attached += 1; true } else { false }
    }

    #[inline(always)]
    pub fn detach(&mut self, id: u64) -> bool {
        if let Some(r) = self.segments.get_mut(&id) { r.attach_count = r.attach_count.saturating_sub(1); self.stats.total_detached += 1; true } else { false }
    }

    #[inline(always)]
    pub fn destroy(&mut self, id: u64) -> bool {
        if self.segments.remove(&id).is_some() { self.stats.active_segments -= 1; true } else { false }
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeShmemStats { &self.stats }
}
