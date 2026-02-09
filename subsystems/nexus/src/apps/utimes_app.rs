// SPDX-License-Identifier: GPL-2.0
//! App utimes — timestamp modification tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Utimes variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtimesCall {
    Utime,
    Utimes,
    Futimesat,
    Utimensat,
    Futimens,
}

/// Utimes result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtimesResult {
    Success,
    NotFound,
    PermissionDenied,
    ReadOnly,
    Fault,
    Error,
}

/// Utimes timestamp flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtimesFlag {
    SetBoth,
    SetAtime,
    SetMtime,
    OmitAtime,
    OmitMtime,
    Now,
}

/// Utimes record
#[derive(Debug, Clone)]
pub struct UtimesRecord {
    pub call: UtimesCall,
    pub result: UtimesResult,
    pub path_hash: u64,
    pub flag: UtimesFlag,
    pub atime_ns: u64,
    pub mtime_ns: u64,
}

impl UtimesRecord {
    pub fn new(call: UtimesCall, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            call,
            result: UtimesResult::Success,
            path_hash: h,
            flag: UtimesFlag::SetBoth,
            atime_ns: 0,
            mtime_ns: 0,
        }
    }

    #[inline(always)]
    pub fn is_ns_precision(&self) -> bool {
        matches!(self.call, UtimesCall::Utimensat | UtimesCall::Futimens)
    }
}

/// Utimes app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct UtimesAppStats {
    pub total_calls: u64,
    pub ns_precision_calls: u64,
    pub now_calls: u64,
    pub errors: u64,
}

/// Main app utimes
#[derive(Debug)]
pub struct AppUtimes {
    pub stats: UtimesAppStats,
}

impl AppUtimes {
    pub fn new() -> Self {
        Self {
            stats: UtimesAppStats {
                total_calls: 0,
                ns_precision_calls: 0,
                now_calls: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &UtimesRecord) {
        self.stats.total_calls += 1;
        if rec.is_ns_precision() {
            self.stats.ns_precision_calls += 1;
        }
        if rec.flag == UtimesFlag::Now {
            self.stats.now_calls += 1;
        }
        if rec.result != UtimesResult::Success {
            self.stats.errors += 1;
        }
    }
}
