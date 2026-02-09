// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic btrfs COW — Copy-on-write extent tracking for btrfs
//!
//! Models btrfs COW semantics with extent reference counting, snapshot
//! tracking, shared extent deduplication, and space accounting.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Btrfs extent type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtrfsCowExtentType {
    Data,
    Metadata,
    System,
}

/// Btrfs extent state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtrfsCowExtentState {
    Allocated,
    Referenced,
    Shared,
    Cow,
    Freed,
}

/// A COW extent.
#[derive(Debug, Clone)]
pub struct BtrfsCowExtent {
    pub extent_id: u64,
    pub logical_addr: u64,
    pub physical_addr: u64,
    pub length: u64,
    pub etype: BtrfsCowExtentType,
    pub state: BtrfsCowExtentState,
    pub ref_count: u32,
    pub generation: u64,
    pub snapshot_ids: Vec<u64>,
}

impl BtrfsCowExtent {
    pub fn new(extent_id: u64, logical: u64, physical: u64, length: u64) -> Self {
        Self {
            extent_id,
            logical_addr: logical,
            physical_addr: physical,
            length,
            etype: BtrfsCowExtentType::Data,
            state: BtrfsCowExtentState::Allocated,
            ref_count: 1,
            generation: 0,
            snapshot_ids: Vec::new(),
        }
    }

    #[inline]
    pub fn add_ref(&mut self) {
        self.ref_count += 1;
        if self.ref_count > 1 {
            self.state = BtrfsCowExtentState::Shared;
        }
    }

    pub fn drop_ref(&mut self) -> bool {
        if self.ref_count > 0 {
            self.ref_count -= 1;
        }
        if self.ref_count == 0 {
            self.state = BtrfsCowExtentState::Freed;
            true
        } else {
            if self.ref_count == 1 {
                self.state = BtrfsCowExtentState::Referenced;
            }
            false
        }
    }

    #[inline(always)]
    pub fn is_shared(&self) -> bool {
        self.ref_count > 1
    }
}

/// A btrfs snapshot.
#[derive(Debug, Clone)]
pub struct BtrfsSnapshot {
    pub snapshot_id: u64,
    pub parent_subvol: u64,
    pub generation: u64,
    pub extent_count: u64,
    pub shared_bytes: u64,
    pub exclusive_bytes: u64,
    pub read_only: bool,
}

impl BtrfsSnapshot {
    pub fn new(snapshot_id: u64, parent: u64, gen: u64) -> Self {
        Self {
            snapshot_id,
            parent_subvol: parent,
            generation: gen,
            extent_count: 0,
            shared_bytes: 0,
            exclusive_bytes: 0,
            read_only: true,
        }
    }
}

/// Space accounting.
#[derive(Debug, Clone)]
pub struct BtrfsCowSpaceInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub shared_bytes: u64,
    pub cow_bytes: u64,
    pub freed_bytes: u64,
}

impl BtrfsCowSpaceInfo {
    pub fn new(total: u64) -> Self {
        Self {
            total_bytes: total,
            used_bytes: 0,
            shared_bytes: 0,
            cow_bytes: 0,
            freed_bytes: 0,
        }
    }
}

/// Statistics for btrfs COW.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BtrfsCowStats {
    pub total_extents: u64,
    pub shared_extents: u64,
    pub cow_operations: u64,
    pub snapshots: u64,
    pub dedup_savings: u64,
}

/// Main holistic btrfs COW manager.
pub struct HolisticBtrfsCow {
    pub extents: BTreeMap<u64, BtrfsCowExtent>,
    pub snapshots: BTreeMap<u64, BtrfsSnapshot>,
    pub space: BtrfsCowSpaceInfo,
    pub next_extent_id: u64,
    pub next_snapshot_id: u64,
    pub current_generation: u64,
    pub stats: BtrfsCowStats,
}

impl HolisticBtrfsCow {
    pub fn new(total_bytes: u64) -> Self {
        Self {
            extents: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            space: BtrfsCowSpaceInfo::new(total_bytes),
            next_extent_id: 1,
            next_snapshot_id: 1,
            current_generation: 1,
            stats: BtrfsCowStats {
                total_extents: 0,
                shared_extents: 0,
                cow_operations: 0,
                snapshots: 0,
                dedup_savings: 0,
            },
        }
    }

    #[inline]
    pub fn allocate_extent(&mut self, logical: u64, physical: u64, length: u64) -> u64 {
        let id = self.next_extent_id;
        self.next_extent_id += 1;
        let mut ext = BtrfsCowExtent::new(id, logical, physical, length);
        ext.generation = self.current_generation;
        self.space.used_bytes += length;
        self.extents.insert(id, ext);
        self.stats.total_extents += 1;
        id
    }

    pub fn cow_extent(&mut self, extent_id: u64, new_physical: u64) -> Option<u64> {
        if let Some(old) = self.extents.get_mut(&extent_id) {
            old.add_ref();
            let new_id = self.next_extent_id;
            self.next_extent_id += 1;
            let mut new_ext = BtrfsCowExtent::new(new_id, old.logical_addr, new_physical, old.length);
            new_ext.generation = self.current_generation;
            new_ext.state = BtrfsCowExtentState::Cow;
            self.space.cow_bytes += new_ext.length;
            self.extents.insert(new_id, new_ext);
            self.stats.cow_operations += 1;
            self.stats.total_extents += 1;
            Some(new_id)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn extent_count(&self) -> usize {
        self.extents.len()
    }
}
