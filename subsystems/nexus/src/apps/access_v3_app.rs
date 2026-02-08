// SPDX-License-Identifier: GPL-2.0
//! App access v3 — faccessat2 with AT_EACCESS and empty path

extern crate alloc;

/// Access v3 mode flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessV3Mode {
    Exists,
    Read,
    Write,
    Execute,
    ReadWrite,
    ReadExecute,
    All,
}

/// Access v3 flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessV3Flag {
    None,
    AtEaccess,
    AtSymlinkNofollow,
    AtEmptyPath,
}

/// Access v3 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessV3Result {
    Permitted,
    Denied,
    NotFound,
    Fault,
    Error,
}

/// Access v3 record
#[derive(Debug, Clone)]
pub struct AccessV3Record {
    pub mode: AccessV3Mode,
    pub flag: AccessV3Flag,
    pub result: AccessV3Result,
    pub path_hash: u64,
    pub dirfd: i32,
}

impl AccessV3Record {
    pub fn new(mode: AccessV3Mode, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { mode, flag: AccessV3Flag::None, result: AccessV3Result::Permitted, path_hash: h, dirfd: -100 }
    }
}

/// Access v3 app stats
#[derive(Debug, Clone)]
pub struct AccessV3AppStats {
    pub total_ops: u64,
    pub permitted: u64,
    pub denied: u64,
    pub not_found: u64,
}

/// Main app access v3
#[derive(Debug)]
pub struct AppAccessV3 {
    pub stats: AccessV3AppStats,
}

impl AppAccessV3 {
    pub fn new() -> Self {
        Self { stats: AccessV3AppStats { total_ops: 0, permitted: 0, denied: 0, not_found: 0 } }
    }

    pub fn record(&mut self, rec: &AccessV3Record) {
        self.stats.total_ops += 1;
        match rec.result {
            AccessV3Result::Permitted => self.stats.permitted += 1,
            AccessV3Result::Denied => self.stats.denied += 1,
            AccessV3Result::NotFound => self.stats.not_found += 1,
            _ => {}
        }
    }
}
