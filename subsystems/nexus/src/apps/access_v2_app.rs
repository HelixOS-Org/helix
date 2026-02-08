// SPDX-License-Identifier: GPL-2.0
//! App access v2 — permission checking with faccessat2 and effective-id support

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Access v2 check mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessV2Mode {
    Existence,
    Read,
    Write,
    Execute,
    ReadWrite,
    ReadExecute,
    WriteExecute,
    All,
}

/// Access v2 flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessV2Flag {
    None,
    AtEaccess,
    AtSymlinkNofollow,
    AtEmptyPath,
}

/// Access v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessV2Result {
    Allowed,
    PermissionDenied,
    NotFound,
    Loop,
    NameTooLong,
    ReadOnlyFs,
    Error,
}

/// Access check record
#[derive(Debug, Clone)]
pub struct AccessV2Record {
    pub path_hash: u64,
    pub mode: AccessV2Mode,
    pub flag: AccessV2Flag,
    pub result: AccessV2Result,
    pub uid: u32,
    pub gid: u32,
    pub effective_uid: u32,
    pub inode_mode: u16,
    pub duration_ns: u64,
}

impl AccessV2Record {
    pub fn new(path: &[u8], mode: AccessV2Mode, flag: AccessV2Flag) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            path_hash: h,
            mode,
            flag,
            result: AccessV2Result::Allowed,
            uid: 0,
            gid: 0,
            effective_uid: 0,
            inode_mode: 0,
            duration_ns: 0,
        }
    }

    pub fn used_effective_id(&self) -> bool {
        self.flag == AccessV2Flag::AtEaccess
    }

    pub fn is_privileged_check(&self) -> bool {
        self.uid == 0 || self.effective_uid == 0
    }
}

/// Path access pattern tracker
#[derive(Debug, Clone)]
pub struct PathAccessPattern {
    pub path_hash: u64,
    pub check_count: u64,
    pub denied_count: u64,
    pub last_result: AccessV2Result,
    pub modes_checked: u32,
}

impl PathAccessPattern {
    pub fn new(path_hash: u64) -> Self {
        Self {
            path_hash,
            check_count: 0,
            denied_count: 0,
            last_result: AccessV2Result::Allowed,
            modes_checked: 0,
        }
    }

    pub fn record(&mut self, result: AccessV2Result, mode: AccessV2Mode) {
        self.check_count += 1;
        self.last_result = result;
        self.modes_checked |= 1u32 << (mode as u32);
        if result == AccessV2Result::PermissionDenied {
            self.denied_count += 1;
        }
    }

    pub fn deny_rate(&self) -> f64 {
        if self.check_count == 0 { 0.0 } else { self.denied_count as f64 / self.check_count as f64 }
    }
}

/// Access v2 app stats
#[derive(Debug, Clone)]
pub struct AccessV2AppStats {
    pub total_checks: u64,
    pub allowed: u64,
    pub denied: u64,
    pub not_found: u64,
    pub eaccess_checks: u64,
}

/// Main app access v2
#[derive(Debug)]
pub struct AppAccessV2 {
    pub patterns: BTreeMap<u64, PathAccessPattern>,
    pub stats: AccessV2AppStats,
}

impl AppAccessV2 {
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            stats: AccessV2AppStats {
                total_checks: 0,
                allowed: 0,
                denied: 0,
                not_found: 0,
                eaccess_checks: 0,
            },
        }
    }

    pub fn record(&mut self, record: &AccessV2Record) {
        self.stats.total_checks += 1;
        match record.result {
            AccessV2Result::Allowed => self.stats.allowed += 1,
            AccessV2Result::PermissionDenied => self.stats.denied += 1,
            AccessV2Result::NotFound => self.stats.not_found += 1,
            _ => {}
        }
        if record.used_effective_id() {
            self.stats.eaccess_checks += 1;
        }
        let pattern = self.patterns.entry(record.path_hash)
            .or_insert_with(|| PathAccessPattern::new(record.path_hash));
        pattern.record(record.result, record.mode);
    }

    pub fn denial_rate(&self) -> f64 {
        if self.stats.total_checks == 0 { 0.0 }
        else { self.stats.denied as f64 / self.stats.total_checks as f64 }
    }
}
