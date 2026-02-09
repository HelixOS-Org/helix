// SPDX-License-Identifier: GPL-2.0
//! Coop superblock — cooperative superblock sharing for filesystem instances

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop superblock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopSbState {
    Active,
    Shared,
    Frozen,
    Syncing,
    Unmounting,
}

/// Shared superblock
#[derive(Debug, Clone)]
pub struct CoopSuperblockEntry {
    pub dev_id: u64,
    pub fs_type_hash: u64,
    pub state: CoopSbState,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub shared_count: u32,
    pub sync_count: u64,
}

impl CoopSuperblockEntry {
    pub fn new(dev_id: u64, fs_type: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in fs_type { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { dev_id, fs_type_hash: h, state: CoopSbState::Active, total_blocks: 0, free_blocks: 0, shared_count: 1, sync_count: 0 }
    }

    #[inline(always)]
    pub fn share(&mut self) { self.shared_count += 1; self.state = CoopSbState::Shared; }
    #[inline(always)]
    pub fn sync(&mut self) { self.sync_count += 1; self.state = CoopSbState::Syncing; }
    #[inline(always)]
    pub fn freeze(&mut self) { self.state = CoopSbState::Frozen; }
    #[inline(always)]
    pub fn thaw(&mut self) { self.state = if self.shared_count > 1 { CoopSbState::Shared } else { CoopSbState::Active }; }
    #[inline(always)]
    pub fn usage_pct(&self) -> f64 { if self.total_blocks == 0 { 0.0 } else { (self.total_blocks - self.free_blocks) as f64 / self.total_blocks as f64 } }
}

/// Coop superblock stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopSuperblockStats {
    pub total_superblocks: u64,
    pub shared: u64,
    pub syncs: u64,
    pub freezes: u64,
}

/// Main coop superblock
#[derive(Debug)]
pub struct CoopSuperblock {
    pub sbs: BTreeMap<u64, CoopSuperblockEntry>,
    pub stats: CoopSuperblockStats,
}

impl CoopSuperblock {
    pub fn new() -> Self {
        Self { sbs: BTreeMap::new(), stats: CoopSuperblockStats { total_superblocks: 0, shared: 0, syncs: 0, freezes: 0 } }
    }

    #[inline(always)]
    pub fn register(&mut self, dev_id: u64, fs_type: &[u8]) {
        self.stats.total_superblocks += 1;
        self.sbs.insert(dev_id, CoopSuperblockEntry::new(dev_id, fs_type));
    }

    #[inline(always)]
    pub fn share(&mut self, dev_id: u64) {
        if let Some(sb) = self.sbs.get_mut(&dev_id) { sb.share(); self.stats.shared += 1; }
    }

    #[inline(always)]
    pub fn sync(&mut self, dev_id: u64) {
        if let Some(sb) = self.sbs.get_mut(&dev_id) { sb.sync(); self.stats.syncs += 1; }
    }
}

// ============================================================================
// Merged from superblock_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopSuperblockV2State {
    Clean,
    Dirty,
    Syncing,
    Error,
    ReadOnly,
}

/// Cooperative superblock entry
#[derive(Debug, Clone)]
pub struct CoopSuperblockV2Entry {
    pub sb_id: u64,
    pub fs_type: String,
    pub block_size: u32,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub state: CoopSuperblockV2State,
    pub mount_count: u32,
}

/// Stats for superblock cooperation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopSuperblockV2Stats {
    pub total_syncs: u64,
    pub dirty_superblocks: u64,
    pub error_count: u64,
    pub state_transitions: u64,
}

/// Manager for superblock cooperative operations
pub struct CoopSuperblockV2Manager {
    superblocks: BTreeMap<u64, CoopSuperblockV2Entry>,
    next_id: u64,
    stats: CoopSuperblockV2Stats,
}

impl CoopSuperblockV2Manager {
    pub fn new() -> Self {
        Self {
            superblocks: BTreeMap::new(),
            next_id: 1,
            stats: CoopSuperblockV2Stats {
                total_syncs: 0,
                dirty_superblocks: 0,
                error_count: 0,
                state_transitions: 0,
            },
        }
    }

    pub fn register(&mut self, fs_type: &str, block_size: u32, total_blocks: u64, total_inodes: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = CoopSuperblockV2Entry {
            sb_id: id,
            fs_type: String::from(fs_type),
            block_size,
            total_blocks,
            free_blocks: total_blocks,
            total_inodes,
            free_inodes: total_inodes,
            state: CoopSuperblockV2State::Clean,
            mount_count: 0,
        };
        self.superblocks.insert(id, entry);
        id
    }

    #[inline]
    pub fn mark_dirty(&mut self, sb_id: u64) {
        if let Some(sb) = self.superblocks.get_mut(&sb_id) {
            if sb.state != CoopSuperblockV2State::Dirty {
                sb.state = CoopSuperblockV2State::Dirty;
                self.stats.dirty_superblocks += 1;
                self.stats.state_transitions += 1;
            }
        }
    }

    #[inline]
    pub fn sync(&mut self, sb_id: u64) -> bool {
        if let Some(sb) = self.superblocks.get_mut(&sb_id) {
            sb.state = CoopSuperblockV2State::Clean;
            self.stats.total_syncs += 1;
            self.stats.dirty_superblocks = self.stats.dirty_superblocks.saturating_sub(1);
            self.stats.state_transitions += 1;
            true
        } else {
            false
        }
    }

    pub fn allocate_blocks(&mut self, sb_id: u64, count: u64) -> bool {
        if let Some(sb) = self.superblocks.get_mut(&sb_id) {
            if sb.free_blocks >= count {
                sb.free_blocks -= count;
                sb.state = CoopSuperblockV2State::Dirty;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    #[inline]
    pub fn free_blocks(&mut self, sb_id: u64, count: u64) {
        if let Some(sb) = self.superblocks.get_mut(&sb_id) {
            sb.free_blocks = (sb.free_blocks + count).min(sb.total_blocks);
            sb.state = CoopSuperblockV2State::Dirty;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopSuperblockV2Stats {
        &self.stats
    }
}
