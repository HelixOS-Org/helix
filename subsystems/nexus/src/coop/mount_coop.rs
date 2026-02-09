// SPDX-License-Identifier: GPL-2.0
//! Coop mount — cooperative mount namespace sharing

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop mount propagation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMountProp {
    Private,
    Shared,
    Slave,
    Unbindable,
}

/// Coop mount state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMountState {
    Active,
    Shared,
    Propagating,
    Unmounting,
    Expired,
}

/// Shared mount point
#[derive(Debug, Clone)]
pub struct SharedMountPoint {
    pub mount_id: u64,
    pub source_hash: u64,
    pub target_hash: u64,
    pub propagation: CoopMountProp,
    pub state: CoopMountState,
    pub subscribers: u32,
    pub events: u64,
}

impl SharedMountPoint {
    pub fn new(mount_id: u64, source: &[u8], target: &[u8]) -> Self {
        let hash = |d: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in d { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        Self { mount_id, source_hash: hash(source), target_hash: hash(target), propagation: CoopMountProp::Shared, state: CoopMountState::Active, subscribers: 0, events: 0 }
    }

    #[inline(always)]
    pub fn subscribe(&mut self) { self.subscribers += 1; }
    #[inline(always)]
    pub fn unsubscribe(&mut self) { if self.subscribers > 0 { self.subscribers -= 1; } }
    #[inline(always)]
    pub fn propagate(&mut self) { self.events += 1; self.state = CoopMountState::Propagating; }
}

/// Coop mount stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopMountStats {
    pub total_mounts: u64,
    pub shared_mounts: u64,
    pub propagation_events: u64,
    pub unmounts: u64,
}

/// Main coop mount
#[derive(Debug)]
pub struct CoopMount {
    pub mounts: BTreeMap<u64, SharedMountPoint>,
    pub stats: CoopMountStats,
}

impl CoopMount {
    pub fn new() -> Self {
        Self { mounts: BTreeMap::new(), stats: CoopMountStats { total_mounts: 0, shared_mounts: 0, propagation_events: 0, unmounts: 0 } }
    }

    #[inline]
    pub fn mount(&mut self, id: u64, source: &[u8], target: &[u8]) {
        self.stats.total_mounts += 1;
        let mp = SharedMountPoint::new(id, source, target);
        self.stats.shared_mounts += 1;
        self.mounts.insert(id, mp);
    }

    #[inline(always)]
    pub fn umount(&mut self, id: u64) {
        if self.mounts.remove(&id).is_some() { self.stats.unmounts += 1; }
    }
}

// ============================================================================
// Merged from mount_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMountV2Op {
    Mount,
    Unmount,
    Remount,
    BindMount,
    MoveMount,
    Pivot,
}

/// Mount flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMountV2Flags {
    ReadOnly,
    NoExec,
    NoSuid,
    NoDev,
    Synchronous,
    MandLock,
    NoAtime,
    RelaTime,
}

/// Cooperative mount point entry
#[derive(Debug, Clone)]
pub struct CoopMountV2Entry {
    pub mount_id: u64,
    pub source: String,
    pub target: String,
    pub fs_type: String,
    pub flags: u32,
    pub parent_id: u64,
    pub ref_count: u32,
}

/// Stats for mount cooperation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopMountV2Stats {
    pub total_mounts: u64,
    pub active_mounts: u64,
    pub unmount_count: u64,
    pub remount_count: u64,
    pub mount_conflicts: u64,
}

/// Manager for mount cooperative operations
pub struct CoopMountV2Manager {
    mounts: BTreeMap<u64, CoopMountV2Entry>,
    path_index: LinearMap<u64, 64>,
    next_id: u64,
    stats: CoopMountV2Stats,
}

impl CoopMountV2Manager {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            path_index: LinearMap::new(),
            next_id: 1,
            stats: CoopMountV2Stats {
                total_mounts: 0,
                active_mounts: 0,
                unmount_count: 0,
                remount_count: 0,
                mount_conflicts: 0,
            },
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn mount(&mut self, source: &str, target: &str, fs_type: &str, flags: u32) -> Option<u64> {
        let hash = Self::hash_path(target);
        if self.path_index.contains_key(hash) {
            self.stats.mount_conflicts += 1;
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        let entry = CoopMountV2Entry {
            mount_id: id,
            source: String::from(source),
            target: String::from(target),
            fs_type: String::from(fs_type),
            flags,
            parent_id: 0,
            ref_count: 1,
        };
        self.mounts.insert(id, entry);
        self.path_index.insert(hash, id);
        self.stats.total_mounts += 1;
        self.stats.active_mounts += 1;
        Some(id)
    }

    #[inline]
    pub fn unmount(&mut self, mount_id: u64) -> bool {
        if let Some(entry) = self.mounts.remove(&mount_id) {
            let hash = Self::hash_path(&entry.target);
            self.path_index.remove(hash);
            self.stats.unmount_count += 1;
            self.stats.active_mounts = self.stats.active_mounts.saturating_sub(1);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn lookup_mount(&self, path: &str) -> Option<&CoopMountV2Entry> {
        let hash = Self::hash_path(path);
        self.path_index.get(hash).and_then(|id| self.mounts.get(id))
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopMountV2Stats {
        &self.stats
    }
}
