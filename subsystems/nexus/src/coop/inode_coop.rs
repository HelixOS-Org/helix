// SPDX-License-Identifier: GPL-2.0
//! Coop inode — cooperative inode sharing with reference tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop inode state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopInodeState {
    Clean,
    Dirty,
    Shared,
    Writeback,
    Evicting,
}

/// Shared inode
#[derive(Debug, Clone)]
pub struct CoopInodeEntry {
    pub inode: u64,
    pub state: CoopInodeState,
    pub size: u64,
    pub nlink: u32,
    pub shared_count: u32,
    pub dirty_pages: u64,
    pub read_count: u64,
    pub write_count: u64,
}

impl CoopInodeEntry {
    pub fn new(inode: u64) -> Self {
        Self { inode, state: CoopInodeState::Clean, size: 0, nlink: 1, shared_count: 1, dirty_pages: 0, read_count: 0, write_count: 0 }
    }

    pub fn share(&mut self) { self.shared_count += 1; self.state = CoopInodeState::Shared; }
    pub fn unshare(&mut self) { if self.shared_count > 0 { self.shared_count -= 1; } }
    pub fn mark_dirty(&mut self, pages: u64) { self.dirty_pages += pages; self.state = CoopInodeState::Dirty; }
    pub fn writeback(&mut self, pages: u64) { self.dirty_pages = self.dirty_pages.saturating_sub(pages); if self.dirty_pages == 0 && self.shared_count > 1 { self.state = CoopInodeState::Shared; } else if self.dirty_pages == 0 { self.state = CoopInodeState::Clean; } }
}

/// Coop inode stats
#[derive(Debug, Clone)]
pub struct CoopInodeStats {
    pub total_inodes: u64,
    pub shared_inodes: u64,
    pub dirty_inodes: u64,
    pub writebacks: u64,
    pub evictions: u64,
}

/// Main coop inode
#[derive(Debug)]
pub struct CoopInode {
    pub inodes: BTreeMap<u64, CoopInodeEntry>,
    pub stats: CoopInodeStats,
}

impl CoopInode {
    pub fn new() -> Self {
        Self { inodes: BTreeMap::new(), stats: CoopInodeStats { total_inodes: 0, shared_inodes: 0, dirty_inodes: 0, writebacks: 0, evictions: 0 } }
    }

    pub fn create(&mut self, inode: u64) {
        self.stats.total_inodes += 1;
        self.inodes.insert(inode, CoopInodeEntry::new(inode));
    }

    pub fn share(&mut self, inode: u64) {
        if let Some(e) = self.inodes.get_mut(&inode) { e.share(); self.stats.shared_inodes += 1; }
    }

    pub fn evict(&mut self, inode: u64) {
        if self.inodes.remove(&inode).is_some() { self.stats.evictions += 1; }
    }
}

// ============================================================================
// Merged from inode_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopInodeV2Op {
    Allocate,
    Free,
    Read,
    Write,
    Truncate,
    SetAttr,
    GetAttr,
    Link,
    Unlink,
}

/// Cooperative inode entry
#[derive(Debug, Clone)]
pub struct CoopInodeV2Entry {
    pub inode_number: u64,
    pub size: u64,
    pub blocks: u64,
    pub link_count: u32,
    pub uid: u32,
    pub gid: u32,
    pub permissions: u32,
    pub generation: u64,
    pub dirty: bool,
}

/// Stats for inode cooperation
#[derive(Debug, Clone)]
pub struct CoopInodeV2Stats {
    pub total_ops: u64,
    pub allocations: u64,
    pub frees: u64,
    pub conflicts: u64,
    pub dirty_inodes: u64,
    pub writeback_count: u64,
}

/// Manager for inode cooperative operations
pub struct CoopInodeV2Manager {
    inodes: BTreeMap<u64, CoopInodeV2Entry>,
    dirty_set: BTreeMap<u64, bool>,
    next_inode: u64,
    stats: CoopInodeV2Stats,
}

impl CoopInodeV2Manager {
    pub fn new() -> Self {
        Self {
            inodes: BTreeMap::new(),
            dirty_set: BTreeMap::new(),
            next_inode: 2,
            stats: CoopInodeV2Stats {
                total_ops: 0,
                allocations: 0,
                frees: 0,
                conflicts: 0,
                dirty_inodes: 0,
                writeback_count: 0,
            },
        }
    }

    pub fn allocate(&mut self) -> u64 {
        let ino = self.next_inode;
        self.next_inode += 1;
        let entry = CoopInodeV2Entry {
            inode_number: ino,
            size: 0,
            blocks: 0,
            link_count: 1,
            uid: 0,
            gid: 0,
            permissions: 0o644,
            generation: ino.wrapping_mul(37),
            dirty: true,
        };
        self.inodes.insert(ino, entry);
        self.dirty_set.insert(ino, true);
        self.stats.allocations += 1;
        self.stats.total_ops += 1;
        self.stats.dirty_inodes += 1;
        ino
    }

    pub fn free(&mut self, inode: u64) -> bool {
        self.stats.total_ops += 1;
        if self.inodes.remove(&inode).is_some() {
            self.dirty_set.remove(&inode);
            self.stats.frees += 1;
            true
        } else {
            false
        }
    }

    pub fn mark_dirty(&mut self, inode: u64) {
        if let Some(entry) = self.inodes.get_mut(&inode) {
            if !entry.dirty {
                entry.dirty = true;
                self.dirty_set.insert(inode, true);
                self.stats.dirty_inodes += 1;
            }
        }
    }

    pub fn writeback(&mut self) -> usize {
        let dirty: alloc::vec::Vec<u64> = self.dirty_set.keys().cloned().collect();
        let count = dirty.len();
        for ino in dirty {
            if let Some(entry) = self.inodes.get_mut(&ino) {
                entry.dirty = false;
            }
            self.dirty_set.remove(&ino);
        }
        self.stats.writeback_count += count as u64;
        self.stats.dirty_inodes = 0;
        count
    }

    pub fn get(&self, inode: u64) -> Option<&CoopInodeV2Entry> {
        self.inodes.get(&inode)
    }

    pub fn stats(&self) -> &CoopInodeV2Stats {
        &self.stats
    }
}
