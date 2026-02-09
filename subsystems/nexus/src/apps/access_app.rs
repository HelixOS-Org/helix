// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps access — File permission checking and faccessat2 tracking
//!
//! Tracks access/faccessat/faccessat2 calls with AT_EMPTY_PATH and
//! AT_SYMLINK_NOFOLLOW support, capability-based permission overrides.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Access check mode bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    Exists,
    Read,
    Write,
    Execute,
    ReadWrite,
    ReadExecute,
    WriteExecute,
    All,
}

/// Access check flags (faccessat2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessFlag {
    AtEmptyPath,
    AtSymlinkNofollow,
    AtEaccess,
}

/// Access check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessResult {
    Allowed,
    Denied,
    NotFound,
    PermissionDenied,
    Loop,
    NameTooLong,
    NotDirectory,
    IoError,
}

/// An access check record.
#[derive(Debug, Clone)]
pub struct AccessRecord {
    pub record_id: u64,
    pub pid: u64,
    pub path: String,
    pub mode: AccessMode,
    pub flags: Vec<AccessFlag>,
    pub result: AccessResult,
    pub uid: u32,
    pub gid: u32,
    pub used_capability: bool,
    pub timestamp: u64,
}

impl AccessRecord {
    pub fn new(record_id: u64, pid: u64, path: String, mode: AccessMode) -> Self {
        Self {
            record_id,
            pid,
            path,
            mode,
            flags: Vec::new(),
            result: AccessResult::Allowed,
            uid: 0,
            gid: 0,
            used_capability: false,
            timestamp: 0,
        }
    }
}

/// Per-process access pattern tracking.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessAccessState {
    pub pid: u64,
    pub checks_performed: u64,
    pub denied_count: u64,
    pub capability_overrides: u64,
    pub unique_paths: u64,
    pub path_cache: BTreeMap<u64, AccessResult>, // path hash → last result
}

impl ProcessAccessState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            checks_performed: 0,
            denied_count: 0,
            capability_overrides: 0,
            unique_paths: 0,
            path_cache: BTreeMap::new(),
        }
    }

    pub fn record_check(&mut self, path_hash: u64, result: AccessResult, cap_used: bool) {
        self.checks_performed += 1;
        if result == AccessResult::Denied || result == AccessResult::PermissionDenied {
            self.denied_count += 1;
        }
        if cap_used {
            self.capability_overrides += 1;
        }
        if !self.path_cache.contains_key(&path_hash) {
            self.unique_paths += 1;
        }
        self.path_cache.insert(path_hash, result);
    }

    #[inline]
    pub fn denial_rate(&self) -> f64 {
        if self.checks_performed == 0 {
            return 0.0;
        }
        self.denied_count as f64 / self.checks_performed as f64
    }
}

/// Statistics for access app.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AccessAppStats {
    pub total_checks: u64,
    pub total_allowed: u64,
    pub total_denied: u64,
    pub faccessat2_calls: u64,
    pub eaccess_checks: u64,
    pub capability_overrides: u64,
}

/// Main apps access manager.
pub struct AppAccess {
    pub processes: BTreeMap<u64, ProcessAccessState>,
    pub recent_records: Vec<AccessRecord>,
    pub next_record_id: u64,
    pub stats: AccessAppStats,
}

impl AppAccess {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            recent_records: Vec::new(),
            next_record_id: 1,
            stats: AccessAppStats {
                total_checks: 0,
                total_allowed: 0,
                total_denied: 0,
                faccessat2_calls: 0,
                eaccess_checks: 0,
                capability_overrides: 0,
            },
        }
    }

    pub fn record_check(
        &mut self,
        pid: u64,
        path: String,
        mode: AccessMode,
        result: AccessResult,
        cap_used: bool,
    ) -> u64 {
        let id = self.next_record_id;
        self.next_record_id += 1;
        // FNV-1a hash of path
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let state = self.processes.entry(pid).or_insert_with(|| ProcessAccessState::new(pid));
        state.record_check(h, result, cap_used);
        let mut rec = AccessRecord::new(id, pid, path, mode);
        rec.result = result;
        rec.used_capability = cap_used;
        self.stats.total_checks += 1;
        match result {
            AccessResult::Allowed => self.stats.total_allowed += 1,
            AccessResult::Denied | AccessResult::PermissionDenied => self.stats.total_denied += 1,
            _ => {}
        }
        if cap_used {
            self.stats.capability_overrides += 1;
        }
        self.recent_records.push(rec);
        id
    }

    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

// ============================================================================
// Merged from access_v2_app
// ============================================================================

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

    #[inline(always)]
    pub fn used_effective_id(&self) -> bool {
        self.flag == AccessV2Flag::AtEaccess
    }

    #[inline(always)]
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

    #[inline]
    pub fn record(&mut self, result: AccessV2Result, mode: AccessV2Mode) {
        self.check_count += 1;
        self.last_result = result;
        self.modes_checked |= 1u32 << (mode as u32);
        if result == AccessV2Result::PermissionDenied {
            self.denied_count += 1;
        }
    }

    #[inline(always)]
    pub fn deny_rate(&self) -> f64 {
        if self.check_count == 0 { 0.0 } else { self.denied_count as f64 / self.check_count as f64 }
    }
}

/// Access v2 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
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

    #[inline(always)]
    pub fn denial_rate(&self) -> f64 {
        if self.stats.total_checks == 0 { 0.0 }
        else { self.stats.denied as f64 / self.stats.total_checks as f64 }
    }
}

// ============================================================================
// Merged from access_v3_app
// ============================================================================

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
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            mode,
            flag: AccessV3Flag::None,
            result: AccessV3Result::Permitted,
            path_hash: h,
            dirfd: -100,
        }
    }
}

/// Access v3 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
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
        Self {
            stats: AccessV3AppStats {
                total_ops: 0,
                permitted: 0,
                denied: 0,
                not_found: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &AccessV3Record) {
        self.stats.total_ops += 1;
        match rec.result {
            AccessV3Result::Permitted => self.stats.permitted += 1,
            AccessV3Result::Denied => self.stats.denied += 1,
            AccessV3Result::NotFound => self.stats.not_found += 1,
            _ => {},
        }
    }
}
