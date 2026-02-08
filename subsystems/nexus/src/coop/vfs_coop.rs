// SPDX-License-Identifier: GPL-2.0
//! Coop VFS — cooperative VFS layer with shared path resolution cache

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop VFS operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopVfsOp {
    Open,
    Close,
    Read,
    Write,
    Stat,
    Readdir,
    Mkdir,
    Rmdir,
    Unlink,
    Rename,
    Fsync,
    Mmap,
}

/// Coop VFS state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopVfsState {
    Idle,
    SharedRead,
    ExclusiveWrite,
    Resolving,
    Cached,
}

/// Shared path cache entry
#[derive(Debug, Clone)]
pub struct SharedPathEntry {
    pub path_hash: u64,
    pub inode: u64,
    pub hits: u64,
    pub shared_by: u32,
    pub state: CoopVfsState,
}

impl SharedPathEntry {
    pub fn new(path: &[u8], inode: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { path_hash: h, inode, hits: 0, shared_by: 1, state: CoopVfsState::Cached }
    }

    pub fn share(&mut self) { self.shared_by += 1; }
    pub fn hit(&mut self) { self.hits += 1; }
    pub fn unshare(&mut self) { if self.shared_by > 0 { self.shared_by -= 1; } }
}

/// Coop VFS stats
#[derive(Debug, Clone)]
pub struct CoopVfsStats {
    pub total_ops: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub shared_resolutions: u64,
    pub conflicts: u64,
}

/// Main coop VFS
#[derive(Debug)]
pub struct CoopVfs {
    pub cache: BTreeMap<u64, SharedPathEntry>,
    pub stats: CoopVfsStats,
}

impl CoopVfs {
    pub fn new() -> Self {
        Self { cache: BTreeMap::new(), stats: CoopVfsStats { total_ops: 0, cache_hits: 0, cache_misses: 0, shared_resolutions: 0, conflicts: 0 } }
    }

    pub fn resolve(&mut self, path: &[u8]) -> Option<u64> {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        self.stats.total_ops += 1;
        if let Some(entry) = self.cache.get_mut(&h) {
            entry.hit();
            self.stats.cache_hits += 1;
            Some(entry.inode)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    pub fn insert(&mut self, path: &[u8], inode: u64) {
        let entry = SharedPathEntry::new(path, inode);
        self.cache.insert(entry.path_hash, entry);
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.cache_hits + self.stats.cache_misses;
        if total == 0 { 0.0 } else { self.stats.cache_hits as f64 / total as f64 }
    }
}

// ============================================================================
// Merged from vfs_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopVfsV2Op {
    Lookup,
    Create,
    Remove,
    Rename,
    Link,
    Symlink,
    ReadLink,
    Permission,
    GetAttr,
    SetAttr,
}

/// VFS cooperation request
#[derive(Debug, Clone)]
pub struct CoopVfsV2Request {
    pub op: CoopVfsV2Op,
    pub path: String,
    pub inode: u64,
    pub requester_id: u64,
    pub priority: u8,
    pub timestamp: u64,
}

/// VFS cooperation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopVfsV2State {
    Idle,
    Processing,
    WaitingLock,
    Completed,
    Conflicted,
}

/// Stats for VFS cooperation
#[derive(Debug, Clone)]
pub struct CoopVfsV2Stats {
    pub total_ops: u64,
    pub conflicts_resolved: u64,
    pub lock_waits: u64,
    pub cache_coherency_events: u64,
    pub avg_resolution_us: u64,
}

/// Manager for VFS cooperative operations
pub struct CoopVfsV2Manager {
    pending_ops: BTreeMap<u64, CoopVfsV2Request>,
    state_map: BTreeMap<u64, CoopVfsV2State>,
    next_id: u64,
    stats: CoopVfsV2Stats,
}

impl CoopVfsV2Manager {
    pub fn new() -> Self {
        Self {
            pending_ops: BTreeMap::new(),
            state_map: BTreeMap::new(),
            next_id: 1,
            stats: CoopVfsV2Stats {
                total_ops: 0,
                conflicts_resolved: 0,
                lock_waits: 0,
                cache_coherency_events: 0,
                avg_resolution_us: 0,
            },
        }
    }

    pub fn submit_op(&mut self, op: CoopVfsV2Op, path: &str, inode: u64, requester: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let req = CoopVfsV2Request {
            op,
            path: String::from(path),
            inode,
            requester_id: requester,
            priority: 5,
            timestamp: id.wrapping_mul(31),
        };
        self.pending_ops.insert(id, req);
        self.state_map.insert(id, CoopVfsV2State::Processing);
        self.stats.total_ops += 1;
        id
    }

    pub fn resolve_conflict(&mut self, op_id: u64) -> bool {
        if let Some(state) = self.state_map.get_mut(&op_id) {
            if *state == CoopVfsV2State::Conflicted {
                *state = CoopVfsV2State::Completed;
                self.stats.conflicts_resolved += 1;
                return true;
            }
        }
        false
    }

    pub fn complete_op(&mut self, op_id: u64) -> bool {
        if self.pending_ops.remove(&op_id).is_some() {
            self.state_map.insert(op_id, CoopVfsV2State::Completed);
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> &CoopVfsV2Stats {
        &self.stats
    }
}
