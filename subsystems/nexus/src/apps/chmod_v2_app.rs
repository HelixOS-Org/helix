// SPDX-License-Identifier: GPL-2.0
//! App chmod v2 — permission change tracking with ACL awareness

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Chmod v2 variant
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
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { call, result: ChmodV2Result::Success, path_hash: h, old_mode: 0, new_mode, fd: -1 }
    }

    pub fn is_setuid(&self) -> bool { self.new_mode & 0o4000 != 0 }
    pub fn is_setgid(&self) -> bool { self.new_mode & 0o2000 != 0 }
    pub fn is_sticky(&self) -> bool { self.new_mode & 0o1000 != 0 }
    pub fn made_world_writable(&self) -> bool { self.new_mode & 0o002 != 0 && self.old_mode & 0o002 == 0 }
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
        Self { stats: ChmodV2AppStats { total_calls: 0, setuid_changes: 0, world_writable: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &ChmodV2Record) {
        self.stats.total_calls += 1;
        if rec.is_setuid() || rec.is_setgid() { self.stats.setuid_changes += 1; }
        if rec.made_world_writable() { self.stats.world_writable += 1; }
        if rec.result != ChmodV2Result::Success { self.stats.errors += 1; }
    }
}
