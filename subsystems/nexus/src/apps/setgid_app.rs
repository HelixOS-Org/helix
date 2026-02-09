// SPDX-License-Identifier: GPL-2.0
//! App setgid — setgid/setresgid syscall interface

extern crate alloc;

/// Setgid variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetgidVariant {
    Setgid,
    Setresgid,
    Setfsgid,
}

/// Setgid result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetgidResult {
    Success,
    PermissionDenied,
    InvalidGid,
    Error,
}

/// Setgid record
#[derive(Debug, Clone)]
pub struct SetgidRecord {
    pub variant: SetgidVariant,
    pub result: SetgidResult,
    pub gid: u32,
    pub egid: u32,
    pub sgid: u32,
    pub pid: u32,
}

impl SetgidRecord {
    pub fn new(variant: SetgidVariant, gid: u32) -> Self {
        Self {
            variant,
            result: SetgidResult::Success,
            gid,
            egid: gid,
            sgid: gid,
            pid: 0,
        }
    }
}

/// Setgid app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SetgidAppStats {
    pub total_ops: u64,
    pub successful: u64,
    pub denied: u64,
}

/// Main app setgid
#[derive(Debug)]
pub struct AppSetgid {
    pub stats: SetgidAppStats,
}

impl AppSetgid {
    pub fn new() -> Self {
        Self {
            stats: SetgidAppStats {
                total_ops: 0,
                successful: 0,
                denied: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &SetgidRecord) {
        self.stats.total_ops += 1;
        match rec.result {
            SetgidResult::Success => self.stats.successful += 1,
            SetgidResult::PermissionDenied => self.stats.denied += 1,
            _ => {},
        }
    }
}
