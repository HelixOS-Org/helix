// SPDX-License-Identifier: GPL-2.0
//! Apps statfs_app — filesystem statistics.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Filesystem type ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsTypeId {
    Ext4,
    Btrfs,
    Xfs,
    Tmpfs,
    Proc,
    Sysfs,
    DevTmpfs,
    Nfs,
    Cifs,
    Fuse,
    Other(u64),
}

/// Statfs result
#[derive(Debug)]
pub struct StatfsResult {
    pub fs_type: FsTypeId,
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub avail_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub fs_id: u64,
    pub name_max: u64,
    pub flags: u64,
}

impl StatfsResult {
    pub fn new(fs_type: FsTypeId, bsize: u64, total: u64, free: u64) -> Self {
        Self { fs_type, block_size: bsize, total_blocks: total, free_blocks: free, avail_blocks: free, total_inodes: 0, free_inodes: 0, fs_id: 0, name_max: 255, flags: 0 }
    }

    #[inline(always)]
    pub fn total_bytes(&self) -> u64 { self.total_blocks * self.block_size }
    #[inline(always)]
    pub fn free_bytes(&self) -> u64 { self.free_blocks * self.block_size }
    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 { if self.total_blocks == 0 { 0.0 } else { 1.0 - self.free_blocks as f64 / self.total_blocks as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StatfsAppStats {
    pub total_queries: u64,
    pub unique_filesystems: u32,
    pub total_capacity_bytes: u64,
    pub total_free_bytes: u64,
}

/// Main statfs app
pub struct AppStatfs {
    filesystems: BTreeMap<u64, StatfsResult>,
    queries: u64,
}

impl AppStatfs {
    pub fn new() -> Self { Self { filesystems: BTreeMap::new(), queries: 0 } }

    #[inline(always)]
    pub fn register(&mut self, fs_id: u64, result: StatfsResult) { self.filesystems.insert(fs_id, result); }

    #[inline(always)]
    pub fn query(&mut self, fs_id: u64) -> Option<&StatfsResult> {
        self.queries += 1;
        self.filesystems.get(&fs_id)
    }

    #[inline]
    pub fn stats(&self) -> StatfsAppStats {
        let cap: u64 = self.filesystems.values().map(|f| f.total_bytes()).sum();
        let free: u64 = self.filesystems.values().map(|f| f.free_bytes()).sum();
        StatfsAppStats { total_queries: self.queries, unique_filesystems: self.filesystems.len() as u32, total_capacity_bytes: cap, total_free_bytes: free }
    }
}
