// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Chmod App (file permission change tracking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
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
    history: VecDeque<ChmodRecord>,
    max_history: usize,
    stats: ChmodAppStats,
}

impl AppChmod {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: VecDeque::new(),
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
            self.history.pop_front();
        }
        self.history.push_back(record);
        ChmodResult::Success
    }

    pub fn stats(&self) -> &ChmodAppStats {
        &self.stats
    }
}

// ============================================================================
// Merged from chmod_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodV2Call {
    Chmod,
    Fchmod,
    Fchmodat,
    Fchmodat2,
}

/// Chmod v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodV2Result {
    Success,
    PermissionDenied,
    NotFound,
    ReadOnly,
    NotOwner,
    Error,
}

/// Chmod v2 record
#[derive(Debug, Clone)]
pub struct ChmodV2Record {
    pub call: ChmodV2Call,
    pub result: ChmodV2Result,
    pub path_hash: u64,
    pub old_mode: u32,
    pub new_mode: u32,
    pub fd: i32,
}

impl ChmodV2Record {
    pub fn new(call: ChmodV2Call, path: &[u8], new_mode: u32) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            call,
            result: ChmodV2Result::Success,
            path_hash: h,
            old_mode: 0,
            new_mode,
            fd: -1,
        }
    }

    pub fn is_setuid(&self) -> bool {
        self.new_mode & 0o4000 != 0
    }
    pub fn is_setgid(&self) -> bool {
        self.new_mode & 0o2000 != 0
    }
    pub fn is_sticky(&self) -> bool {
        self.new_mode & 0o1000 != 0
    }
    pub fn made_world_writable(&self) -> bool {
        self.new_mode & 0o002 != 0 && self.old_mode & 0o002 == 0
    }
}

/// Chmod v2 app stats
#[derive(Debug, Clone)]
pub struct ChmodV2AppStats {
    pub total_calls: u64,
    pub setuid_changes: u64,
    pub world_writable: u64,
    pub errors: u64,
}

/// Main app chmod v2
#[derive(Debug)]
pub struct AppChmodV2 {
    pub stats: ChmodV2AppStats,
}

impl AppChmodV2 {
    pub fn new() -> Self {
        Self {
            stats: ChmodV2AppStats {
                total_calls: 0,
                setuid_changes: 0,
                world_writable: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &ChmodV2Record) {
        self.stats.total_calls += 1;
        if rec.is_setuid() || rec.is_setgid() {
            self.stats.setuid_changes += 1;
        }
        if rec.made_world_writable() {
            self.stats.world_writable += 1;
        }
        if rec.result != ChmodV2Result::Success {
            self.stats.errors += 1;
        }
    }
}
