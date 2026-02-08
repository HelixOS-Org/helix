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

    pub fn denial_rate(&self) -> f64 {
        if self.checks_performed == 0 {
            return 0.0;
        }
        self.denied_count as f64 / self.checks_performed as f64
    }
}

/// Statistics for access app.
#[derive(Debug, Clone)]
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

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}
