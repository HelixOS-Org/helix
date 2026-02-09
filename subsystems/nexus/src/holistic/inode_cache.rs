// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic inode cache — VFS inode cache with writeback state
//!
//! Models the inode cache with per-superblock partitioning, dirty inode
//! writeback tracking, inode number allocation, and generation tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Inode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeCacheState {
    Clean,
    Dirty,
    Writeback,
    Freeing,
    New,
    Locked,
}

/// Inode type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeCacheType {
    Regular,
    Directory,
    Symlink,
    BlockDev,
    CharDev,
    Fifo,
    Socket,
}

/// A cached inode.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InodeCacheEntry {
    pub ino: u64,
    pub super_block_id: u32,
    pub state: InodeCacheState,
    pub itype: InodeCacheType,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub nlink: u32,
    pub generation: u64,
    pub ref_count: u32,
    pub dirty_pages: u64,
    pub access_time: u64,
    pub modify_time: u64,
    pub change_time: u64,
}

impl InodeCacheEntry {
    pub fn new(ino: u64, super_block_id: u32, itype: InodeCacheType) -> Self {
        Self {
            ino,
            super_block_id,
            state: InodeCacheState::New,
            itype,
            mode: 0o644,
            uid: 0,
            gid: 0,
            size: 0,
            nlink: 1,
            generation: 0,
            ref_count: 1,
            dirty_pages: 0,
            access_time: 0,
            modify_time: 0,
            change_time: 0,
        }
    }

    #[inline(always)]
    pub fn mark_dirty(&mut self) {
        self.state = InodeCacheState::Dirty;
    }

    #[inline(always)]
    pub fn mark_clean(&mut self) {
        self.state = InodeCacheState::Clean;
        self.dirty_pages = 0;
    }

    #[inline(always)]
    pub fn is_dirty(&self) -> bool {
        self.state == InodeCacheState::Dirty
    }
}

/// Per-superblock inode partition.
#[derive(Debug, Clone)]
pub struct InodeSuperBlockPartition {
    pub sb_id: u32,
    pub inode_count: u64,
    pub dirty_count: u64,
    pub next_ino: u64,
}

impl InodeSuperBlockPartition {
    pub fn new(sb_id: u32) -> Self {
        Self {
            sb_id,
            inode_count: 0,
            dirty_count: 0,
            next_ino: 1,
        }
    }
}

/// Statistics for inode cache.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InodeCacheStats {
    pub total_inodes: u64,
    pub dirty_inodes: u64,
    pub lookups: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub writebacks: u64,
}

/// Main holistic inode cache manager.
#[repr(align(64))]
pub struct HolisticInodeCache {
    pub inodes: BTreeMap<u64, InodeCacheEntry>,
    pub partitions: BTreeMap<u32, InodeSuperBlockPartition>,
    pub max_inodes: usize,
    pub stats: InodeCacheStats,
}

impl HolisticInodeCache {
    pub fn new(max_inodes: usize) -> Self {
        Self {
            inodes: BTreeMap::new(),
            partitions: BTreeMap::new(),
            max_inodes,
            stats: InodeCacheStats {
                total_inodes: 0,
                dirty_inodes: 0,
                lookups: 0,
                cache_hits: 0,
                cache_misses: 0,
                evictions: 0,
                writebacks: 0,
            },
        }
    }

    #[inline]
    pub fn insert(&mut self, sb_id: u32, itype: InodeCacheType) -> u64 {
        let part = self.partitions.entry(sb_id).or_insert_with(|| InodeSuperBlockPartition::new(sb_id));
        let ino = part.next_ino;
        part.next_ino += 1;
        part.inode_count += 1;
        let entry = InodeCacheEntry::new(ino, sb_id, itype);
        self.inodes.insert(ino, entry);
        self.stats.total_inodes += 1;
        ino
    }

    #[inline]
    pub fn lookup(&mut self, ino: u64) -> Option<&InodeCacheEntry> {
        self.stats.lookups += 1;
        if self.inodes.contains_key(&ino) {
            self.stats.cache_hits += 1;
            self.inodes.get(&ino)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    #[inline]
    pub fn mark_dirty(&mut self, ino: u64) {
        if let Some(entry) = self.inodes.get_mut(&ino) {
            if !entry.is_dirty() {
                entry.mark_dirty();
                self.stats.dirty_inodes += 1;
            }
        }
    }

    #[inline(always)]
    pub fn inode_count(&self) -> usize {
        self.inodes.len()
    }
}
