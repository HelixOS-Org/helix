// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Chmod App (file permission change tracking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Permission bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodBits {
    OwnerRead,
    OwnerWrite,
    OwnerExec,
    GroupRead,
    GroupWrite,
    GroupExec,
    OtherRead,
    OtherWrite,
    OtherExec,
    SetUid,
    SetGid,
    Sticky,
}

/// Chmod result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodResult {
    Success,
    NotFound,
    PermissionDenied,
    ReadOnlyFs,
    IoError,
}

/// A chmod operation record
#[derive(Debug, Clone)]
pub struct ChmodRecord {
    pub path_hash: u64,
    pub old_mode: u32,
    pub new_mode: u32,
    pub pid: u64,
    pub uid: u32,
    pub result: ChmodResult,
    pub is_fchmod: bool,
    pub tick: u64,
}

/// Statistics for chmod app
#[derive(Debug, Clone)]
pub struct ChmodAppStats {
    pub total_calls: u64,
    pub chmod_calls: u64,
    pub fchmod_calls: u64,
    pub successful: u64,
    pub permission_denied: u64,
    pub setuid_changes: u64,
    pub setgid_changes: u64,
}

/// Main chmod app manager
#[derive(Debug)]
pub struct AppChmod {
    history: Vec<ChmodRecord>,
    max_history: usize,
    stats: ChmodAppStats,
}

impl AppChmod {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
            stats: ChmodAppStats {
                total_calls: 0, chmod_calls: 0, fchmod_calls: 0,
                successful: 0, permission_denied: 0,
                setuid_changes: 0, setgid_changes: 0,
            },
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn chmod(&mut self, path: &str, old_mode: u32, new_mode: u32, pid: u64, uid: u32, is_fchmod: bool, tick: u64) -> ChmodResult {
        self.stats.total_calls += 1;
        if is_fchmod { self.stats.fchmod_calls += 1; } else { self.stats.chmod_calls += 1; }
        self.stats.successful += 1;

        if (new_mode & 0o4000) != 0 && (old_mode & 0o4000) == 0 {
            self.stats.setuid_changes += 1;
        }
        if (new_mode & 0o2000) != 0 && (old_mode & 0o2000) == 0 {
            self.stats.setgid_changes += 1;
        }

        let record = ChmodRecord {
            path_hash: Self::hash_path(path),
            old_mode, new_mode, pid, uid,
            result: ChmodResult::Success,
            is_fchmod, tick,
        };
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(record);
        ChmodResult::Success
    }

    pub fn stats(&self) -> &ChmodAppStats {
        &self.stats
    }
}
