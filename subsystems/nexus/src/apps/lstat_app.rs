// SPDX-License-Identifier: GPL-2.0
//! App lstat — symlink-aware stat application tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lstat result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LstatResult {
    Success,
    NotFound,
    PermissionDenied,
    Loop,
    NameTooLong,
    NotDir,
    Error,
}

/// Lstat file type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LstatFileType {
    Regular,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Unknown,
}

/// Lstat record
#[derive(Debug, Clone)]
pub struct LstatRecord {
    pub path_hash: u64,
    pub result: LstatResult,
    pub file_type: LstatFileType,
    pub inode: u64,
    pub size: u64,
    pub nlink: u32,
    pub is_symlink: bool,
    pub latency_ns: u64,
}

impl LstatRecord {
    pub fn new(path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            path_hash: h,
            result: LstatResult::Success,
            file_type: LstatFileType::Regular,
            inode: 0,
            size: 0,
            nlink: 1,
            is_symlink: false,
            latency_ns: 0,
        }
    }
}

/// Lstat app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LstatAppStats {
    pub total_calls: u64,
    pub symlinks_found: u64,
    pub not_found: u64,
    pub errors: u64,
}

/// Main app lstat
#[derive(Debug)]
pub struct AppLstat {
    pub stats: LstatAppStats,
}

impl AppLstat {
    pub fn new() -> Self {
        Self {
            stats: LstatAppStats {
                total_calls: 0,
                symlinks_found: 0,
                not_found: 0,
                errors: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &LstatRecord) {
        self.stats.total_calls += 1;
        if rec.is_symlink || rec.file_type == LstatFileType::Symlink {
            self.stats.symlinks_found += 1;
        }
        match rec.result {
            LstatResult::NotFound => self.stats.not_found += 1,
            LstatResult::Success => {},
            _ => self.stats.errors += 1,
        }
    }

    #[inline]
    pub fn symlink_ratio(&self) -> f64 {
        if self.stats.total_calls == 0 {
            0.0
        } else {
            self.stats.symlinks_found as f64 / self.stats.total_calls as f64
        }
    }
}
