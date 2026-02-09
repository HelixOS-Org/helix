// SPDX-License-Identifier: GPL-2.0
//! App statx — statx(2) extended attributes tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Statx mask bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatxMask {
    Type,
    Mode,
    Nlink,
    Uid,
    Gid,
    Atime,
    Mtime,
    Ctime,
    Btime,
    Size,
    Blocks,
    MntId,
    All,
}

/// Statx result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatxResult {
    Success,
    NotFound,
    PermissionDenied,
    NotSupported,
    Fault,
    Error,
}

/// Statx attribute flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatxAttr {
    Compressed,
    Immutable,
    Append,
    NoDump,
    Encrypted,
    Verity,
    DaxAccess,
}

/// Statx record
#[derive(Debug, Clone)]
pub struct StatxRecord {
    pub path_hash: u64,
    pub result: StatxResult,
    pub mask: u32,
    pub stx_attributes: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_mnt_id: u64,
    pub latency_ns: u64,
}

impl StatxRecord {
    pub fn new(path: &[u8], mask: u32) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            path_hash: h,
            result: StatxResult::Success,
            mask,
            stx_attributes: 0,
            stx_size: 0,
            stx_blocks: 0,
            stx_mnt_id: 0,
            latency_ns: 0,
        }
    }

    #[inline(always)]
    pub fn has_btime(&self) -> bool {
        self.mask & 0x800 != 0
    }
    #[inline(always)]
    pub fn has_mnt_id(&self) -> bool {
        self.mask & 0x1000 != 0
    }
}

/// Statx app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StatxAppStats {
    pub total_calls: u64,
    pub btime_requests: u64,
    pub mnt_id_requests: u64,
    pub errors: u64,
}

/// Main app statx
#[derive(Debug)]
pub struct AppStatx {
    pub stats: StatxAppStats,
}

impl AppStatx {
    pub fn new() -> Self {
        Self {
            stats: StatxAppStats {
                total_calls: 0,
                btime_requests: 0,
                mnt_id_requests: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &StatxRecord) {
        self.stats.total_calls += 1;
        if rec.has_btime() {
            self.stats.btime_requests += 1;
        }
        if rec.has_mnt_id() {
            self.stats.mnt_id_requests += 1;
        }
        if rec.result != StatxResult::Success {
            self.stats.errors += 1;
        }
    }
}
