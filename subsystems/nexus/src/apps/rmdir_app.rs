// SPDX-License-Identifier: GPL-2.0
//! App rmdir — directory removal with empty-check and recursive support

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Rmdir result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RmdirResult {
    Success,
    NotEmpty,
    NotFound,
    PermissionDenied,
    Busy,
    ReadOnlyFs,
    NotDirectory,
    Error,
}

/// Rmdir mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RmdirMode {
    Single,
    Recursive,
    Force,
}

/// Rmdir record
#[derive(Debug, Clone)]
pub struct RmdirRecord {
    pub path_hash: u64,
    pub mode: RmdirMode,
    pub result: RmdirResult,
    pub inode: u64,
    pub dirs_removed: u32,
    pub files_removed: u32,
    pub bytes_freed: u64,
    pub duration_ns: u64,
}

impl RmdirRecord {
    pub fn new(path: &[u8], mode: RmdirMode) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            path_hash: h,
            mode,
            result: RmdirResult::Success,
            inode: 0,
            dirs_removed: 0,
            files_removed: 0,
            bytes_freed: 0,
            duration_ns: 0,
        }
    }

    pub fn total_entries_removed(&self) -> u32 {
        self.dirs_removed + self.files_removed
    }
}

/// Rmdir app stats
#[derive(Debug, Clone)]
pub struct RmdirAppStats {
    pub total_ops: u64,
    pub total_dirs_removed: u64,
    pub total_files_removed: u64,
    pub total_bytes_freed: u64,
    pub failures: u64,
    pub not_empty_errors: u64,
}

/// Main app rmdir
#[derive(Debug)]
pub struct AppRmdir {
    pub stats: RmdirAppStats,
}

impl AppRmdir {
    pub fn new() -> Self {
        Self {
            stats: RmdirAppStats {
                total_ops: 0,
                total_dirs_removed: 0,
                total_files_removed: 0,
                total_bytes_freed: 0,
                failures: 0,
                not_empty_errors: 0,
            },
        }
    }

    pub fn record(&mut self, record: &RmdirRecord) {
        self.stats.total_ops += 1;
        self.stats.total_dirs_removed += record.dirs_removed as u64;
        self.stats.total_files_removed += record.files_removed as u64;
        self.stats.total_bytes_freed += record.bytes_freed;
        match record.result {
            RmdirResult::Success => {}
            RmdirResult::NotEmpty => {
                self.stats.failures += 1;
                self.stats.not_empty_errors += 1;
            }
            _ => self.stats.failures += 1,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.stats.total_ops == 0 { 0.0 }
        else { (self.stats.total_ops - self.stats.failures) as f64 / self.stats.total_ops as f64 }
    }
}
