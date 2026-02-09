// SPDX-License-Identifier: GPL-2.0
//! App setreuid — setreuid/setregid real+effective ID interface

extern crate alloc;

/// Setreuid type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetreuidType {
    Setreuid,
    Setregid,
}

/// Setreuid result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetreuidResult {
    Success,
    PermissionDenied,
    InvalidId,
    Error,
}

/// Setreuid record
#[derive(Debug, Clone)]
pub struct SetreuidRecord {
    pub id_type: SetreuidType,
    pub result: SetreuidResult,
    pub real_id: u32,
    pub effective_id: u32,
    pub pid: u32,
}

impl SetreuidRecord {
    pub fn new(id_type: SetreuidType, real: u32, effective: u32) -> Self {
        Self {
            id_type,
            result: SetreuidResult::Success,
            real_id: real,
            effective_id: effective,
            pid: 0,
        }
    }
}

/// Setreuid app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SetreuidAppStats {
    pub total_ops: u64,
    pub uid_changes: u64,
    pub gid_changes: u64,
    pub denied: u64,
}

/// Main app setreuid
#[derive(Debug)]
pub struct AppSetreuid {
    pub stats: SetreuidAppStats,
}

impl AppSetreuid {
    pub fn new() -> Self {
        Self {
            stats: SetreuidAppStats {
                total_ops: 0,
                uid_changes: 0,
                gid_changes: 0,
                denied: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &SetreuidRecord) {
        self.stats.total_ops += 1;
        if rec.result == SetreuidResult::Success {
            match rec.id_type {
                SetreuidType::Setreuid => self.stats.uid_changes += 1,
                SetreuidType::Setregid => self.stats.gid_changes += 1,
            }
        }
        if rec.result == SetreuidResult::PermissionDenied {
            self.stats.denied += 1;
        }
    }
}
