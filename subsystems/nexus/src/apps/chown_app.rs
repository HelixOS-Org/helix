// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Chown App (file ownership change tracking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Chown result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChownResult {
    Success,
    NotFound,
    PermissionDenied,
    InvalidUser,
    InvalidGroup,
    ReadOnlyFs,
    IoError,
}

/// Chown variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChownVariant {
    Chown,
    Lchown,
    Fchown,
    FchownAt,
}

/// A chown operation record
#[derive(Debug, Clone)]
pub struct ChownRecord {
    pub path_hash: u64,
    pub old_uid: u32,
    pub new_uid: u32,
    pub old_gid: u32,
    pub new_gid: u32,
    pub pid: u64,
    pub variant: ChownVariant,
    pub result: ChownResult,
    pub tick: u64,
}

/// Statistics for chown app
#[derive(Debug, Clone)]
pub struct ChownAppStats {
    pub total_calls: u64,
    pub chown_calls: u64,
    pub fchown_calls: u64,
    pub lchown_calls: u64,
    pub successful: u64,
    pub permission_denied: u64,
    pub uid_changes: u64,
    pub gid_changes: u64,
}

/// Main chown app manager
#[derive(Debug)]
pub struct AppChown {
    history: Vec<ChownRecord>,
    max_history: usize,
    stats: ChownAppStats,
}

impl AppChown {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
            stats: ChownAppStats {
                total_calls: 0, chown_calls: 0, fchown_calls: 0,
                lchown_calls: 0, successful: 0, permission_denied: 0,
                uid_changes: 0, gid_changes: 0,
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

    pub fn chown(&mut self, path: &str, old_uid: u32, new_uid: u32, old_gid: u32, new_gid: u32, pid: u64, variant: ChownVariant, tick: u64) -> ChownResult {
        self.stats.total_calls += 1;
        match variant {
            ChownVariant::Chown | ChownVariant::FchownAt => self.stats.chown_calls += 1,
            ChownVariant::Fchown => self.stats.fchown_calls += 1,
            ChownVariant::Lchown => self.stats.lchown_calls += 1,
        }
        self.stats.successful += 1;
        if old_uid != new_uid { self.stats.uid_changes += 1; }
        if old_gid != new_gid { self.stats.gid_changes += 1; }

        let record = ChownRecord {
            path_hash: Self::hash_path(path),
            old_uid, new_uid, old_gid, new_gid,
            pid, variant, result: ChownResult::Success, tick,
        };
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(record);
        ChownResult::Success
    }

    pub fn stats(&self) -> &ChownAppStats {
        &self.stats
    }
}

// ============================================================================
// Merged from chown_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChownV2Call {
    Chown,
    Fchown,
    Lchown,
    Fchownat,
}

/// Chown v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChownV2Result {
    Success,
    PermissionDenied,
    NotFound,
    ReadOnly,
    NotOwner,
    InvalidUid,
    InvalidGid,
    Error,
}

/// Chown v2 record
#[derive(Debug, Clone)]
pub struct ChownV2Record {
    pub call: ChownV2Call,
    pub result: ChownV2Result,
    pub path_hash: u64,
    pub old_uid: u32,
    pub new_uid: u32,
    pub old_gid: u32,
    pub new_gid: u32,
    pub ns_id: u64,
}

impl ChownV2Record {
    pub fn new(call: ChownV2Call, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            call,
            result: ChownV2Result::Success,
            path_hash: h,
            old_uid: 0,
            new_uid: 0,
            old_gid: 0,
            new_gid: 0,
            ns_id: 0,
        }
    }

    pub fn uid_changed(&self) -> bool {
        self.old_uid != self.new_uid
    }
    pub fn gid_changed(&self) -> bool {
        self.old_gid != self.new_gid
    }
    pub fn is_root_transfer(&self) -> bool {
        self.new_uid == 0 || self.old_uid == 0
    }
}

/// Chown v2 app stats
#[derive(Debug, Clone)]
pub struct ChownV2AppStats {
    pub total_calls: u64,
    pub uid_changes: u64,
    pub gid_changes: u64,
    pub root_transfers: u64,
    pub errors: u64,
}

/// Main app chown v2
#[derive(Debug)]
pub struct AppChownV2 {
    pub stats: ChownV2AppStats,
}

impl AppChownV2 {
    pub fn new() -> Self {
        Self {
            stats: ChownV2AppStats {
                total_calls: 0,
                uid_changes: 0,
                gid_changes: 0,
                root_transfers: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &ChownV2Record) {
        self.stats.total_calls += 1;
        if rec.uid_changed() {
            self.stats.uid_changes += 1;
        }
        if rec.gid_changed() {
            self.stats.gid_changes += 1;
        }
        if rec.is_root_transfer() {
            self.stats.root_transfers += 1;
        }
        if rec.result != ChownV2Result::Success {
            self.stats.errors += 1;
        }
    }
}
