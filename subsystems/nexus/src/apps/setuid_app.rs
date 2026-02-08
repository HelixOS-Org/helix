// SPDX-License-Identifier: GPL-2.0
//! App setuid — setuid/setresuid syscall interface

extern crate alloc;

/// Setuid variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetuidVariant {
    Setuid,
    Setresuid,
    Setfsuid,
}

/// Setuid result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetuidResult {
    Success,
    PermissionDenied,
    InvalidUid,
    Error,
}

/// Setuid record
#[derive(Debug, Clone)]
pub struct SetuidRecord {
    pub variant: SetuidVariant,
    pub result: SetuidResult,
    pub uid: u32,
    pub euid: u32,
    pub suid: u32,
    pub pid: u32,
}

impl SetuidRecord {
    pub fn new(variant: SetuidVariant, uid: u32) -> Self {
        Self {
            variant,
            result: SetuidResult::Success,
            uid,
            euid: uid,
            suid: uid,
            pid: 0,
        }
    }
}

/// Setuid app stats
#[derive(Debug, Clone)]
pub struct SetuidAppStats {
    pub total_ops: u64,
    pub successful: u64,
    pub denied: u64,
}

/// Main app setuid
#[derive(Debug)]
pub struct AppSetuid {
    pub stats: SetuidAppStats,
}

impl AppSetuid {
    pub fn new() -> Self {
        Self {
            stats: SetuidAppStats {
                total_ops: 0,
                successful: 0,
                denied: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &SetuidRecord) {
        self.stats.total_ops += 1;
        match rec.result {
            SetuidResult::Success => self.stats.successful += 1,
            SetuidResult::PermissionDenied => self.stats.denied += 1,
            _ => {},
        }
    }
}
