// SPDX-License-Identifier: GPL-2.0
//! App statvfs — filesystem statistics tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Statvfs variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatvfsCall {
    Statfs,
    Fstatfs,
    Statvfs,
    Fstatvfs,
}

/// Statvfs result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatvfsResult {
    Success,
    NotFound,
    PermissionDenied,
    Fault,
    Error,
}

/// Statvfs record
#[derive(Debug, Clone)]
pub struct StatvfsRecord {
    pub call: StatvfsCall,
    pub result: StatvfsResult,
    pub path_hash: u64,
    pub fs_type: u64,
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub avail_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
}

impl StatvfsRecord {
    pub fn new(call: StatvfsCall, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            call,
            result: StatvfsResult::Success,
            path_hash: h,
            fs_type: 0,
            block_size: 4096,
            total_blocks: 0,
            free_blocks: 0,
            avail_blocks: 0,
            total_inodes: 0,
            free_inodes: 0,
        }
    }

    pub fn usage_pct(&self) -> f64 {
        if self.total_blocks == 0 {
            0.0
        } else {
            (self.total_blocks - self.free_blocks) as f64 / self.total_blocks as f64
        }
    }

    pub fn inode_usage_pct(&self) -> f64 {
        if self.total_inodes == 0 {
            0.0
        } else {
            (self.total_inodes - self.free_inodes) as f64 / self.total_inodes as f64
        }
    }
}

/// Statvfs app stats
#[derive(Debug, Clone)]
pub struct StatvfsAppStats {
    pub total_calls: u64,
    pub errors: u64,
    pub high_usage_count: u64,
}

/// Main app statvfs
#[derive(Debug)]
pub struct AppStatvfs {
    pub stats: StatvfsAppStats,
}

impl AppStatvfs {
    pub fn new() -> Self {
        Self {
            stats: StatvfsAppStats {
                total_calls: 0,
                errors: 0,
                high_usage_count: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &StatvfsRecord) {
        self.stats.total_calls += 1;
        if rec.result != StatvfsResult::Success {
            self.stats.errors += 1;
        }
        if rec.usage_pct() > 0.9 {
            self.stats.high_usage_count += 1;
        }
    }
}
