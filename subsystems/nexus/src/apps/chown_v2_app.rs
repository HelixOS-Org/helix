// SPDX-License-Identifier: GPL-2.0
//! App chown v2 — ownership change tracking with user namespace awareness

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Chown v2 variant
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
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { call, result: ChownV2Result::Success, path_hash: h, old_uid: 0, new_uid: 0, old_gid: 0, new_gid: 0, ns_id: 0 }
    }

    pub fn uid_changed(&self) -> bool { self.old_uid != self.new_uid }
    pub fn gid_changed(&self) -> bool { self.old_gid != self.new_gid }
    pub fn is_root_transfer(&self) -> bool { self.new_uid == 0 || self.old_uid == 0 }
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
        Self { stats: ChownV2AppStats { total_calls: 0, uid_changes: 0, gid_changes: 0, root_transfers: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &ChownV2Record) {
        self.stats.total_calls += 1;
        if rec.uid_changed() { self.stats.uid_changes += 1; }
        if rec.gid_changed() { self.stats.gid_changes += 1; }
        if rec.is_root_transfer() { self.stats.root_transfers += 1; }
        if rec.result != ChownV2Result::Success { self.stats.errors += 1; }
    }
}
