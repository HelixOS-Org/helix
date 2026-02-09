// SPDX-License-Identifier: GPL-2.0
//! App seteuid — seteuid/setegid effective ID interface

extern crate alloc;

/// Effective ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffIdType {
    Euid,
    Egid,
}

/// Seteuid result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeteuidResult {
    Success,
    PermissionDenied,
    InvalidId,
    Error,
}

/// Seteuid record
#[derive(Debug, Clone)]
pub struct SeteuidRecord {
    pub id_type: EffIdType,
    pub result: SeteuidResult,
    pub old_id: u32,
    pub new_id: u32,
    pub pid: u32,
}

impl SeteuidRecord {
    pub fn new(id_type: EffIdType, new_id: u32) -> Self {
        Self {
            id_type,
            result: SeteuidResult::Success,
            old_id: 0,
            new_id,
            pid: 0,
        }
    }
}

/// Seteuid app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeteuidAppStats {
    pub total_ops: u64,
    pub euid_changes: u64,
    pub egid_changes: u64,
    pub denied: u64,
}

/// Main app seteuid
#[derive(Debug)]
pub struct AppSeteuid {
    pub stats: SeteuidAppStats,
}

impl AppSeteuid {
    pub fn new() -> Self {
        Self {
            stats: SeteuidAppStats {
                total_ops: 0,
                euid_changes: 0,
                egid_changes: 0,
                denied: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &SeteuidRecord) {
        self.stats.total_ops += 1;
        if rec.result == SeteuidResult::Success {
            match rec.id_type {
                EffIdType::Euid => self.stats.euid_changes += 1,
                EffIdType::Egid => self.stats.egid_changes += 1,
            }
        }
        if rec.result == SeteuidResult::PermissionDenied {
            self.stats.denied += 1;
        }
    }
}
