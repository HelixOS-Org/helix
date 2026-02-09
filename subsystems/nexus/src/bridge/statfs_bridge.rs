// SPDX-License-Identifier: GPL-2.0
//! Bridge statfs — filesystem stat operation bridging

extern crate alloc;

/// Statfs bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatfsBridgeEvent { Statfs, Fstatfs, Statvfs, Fstatvfs }

/// Statfs bridge record
#[derive(Debug, Clone)]
pub struct StatfsBridgeRecord {
    pub event: StatfsBridgeEvent,
    pub fs_type: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
}

impl StatfsBridgeRecord {
    pub fn new(event: StatfsBridgeEvent) -> Self { Self { event, fs_type: 0, total_blocks: 0, free_blocks: 0, total_inodes: 0, free_inodes: 0 } }
}

/// Statfs bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StatfsBridgeStats { pub total_ops: u64, pub path_stats: u64, pub fd_stats: u64 }

/// Main bridge statfs
#[derive(Debug)]
pub struct BridgeStatfs { pub stats: StatfsBridgeStats }

impl BridgeStatfs {
    pub fn new() -> Self { Self { stats: StatfsBridgeStats { total_ops: 0, path_stats: 0, fd_stats: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &StatfsBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.event {
            StatfsBridgeEvent::Statfs | StatfsBridgeEvent::Statvfs => self.stats.path_stats += 1,
            StatfsBridgeEvent::Fstatfs | StatfsBridgeEvent::Fstatvfs => self.stats.fd_stats += 1,
        }
    }
}
