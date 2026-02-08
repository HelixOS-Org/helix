// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps chdir — Working directory change tracking and validation
//!
//! Tracks chdir/fchdir operations with path resolution caching,
//! per-process CWD history, and namespace-aware path lookup.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Chdir variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChdirVariant {
    Chdir,
    Fchdir,
}

/// Chdir result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChdirResult {
    Success,
    NotFound,
    NotDirectory,
    Permission,
    NameTooLong,
    BadFd,
    IoError,
    Loop,
}

/// A chdir operation record.
#[derive(Debug, Clone)]
pub struct ChdirRecord {
    pub record_id: u64,
    pub pid: u64,
    pub variant: ChdirVariant,
    pub target: String,
    pub old_cwd: String,
    pub result: ChdirResult,
    pub timestamp: u64,
}

impl ChdirRecord {
    pub fn new(record_id: u64, pid: u64, variant: ChdirVariant, target: String) -> Self {
        Self {
            record_id,
            pid,
            variant,
            target,
            old_cwd: String::new(),
            result: ChdirResult::Success,
            timestamp: 0,
        }
    }
}

/// Per-process CWD state.
#[derive(Debug, Clone)]
pub struct ProcessCwdState {
    pub pid: u64,
    pub current_cwd: String,
    pub cwd_history: Vec<String>,
    pub max_history: usize,
    pub chdir_count: u64,
    pub fchdir_count: u64,
    pub failed_count: u64,
}

impl ProcessCwdState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            current_cwd: String::from("/"),
            cwd_history: Vec::new(),
            max_history: 32,
            chdir_count: 0,
            fchdir_count: 0,
            failed_count: 0,
        }
    }

    pub fn change_dir(&mut self, new_cwd: String, variant: ChdirVariant) {
        if self.cwd_history.len() >= self.max_history {
            self.cwd_history.remove(0);
        }
        self.cwd_history.push(self.current_cwd.clone());
        self.current_cwd = new_cwd;
        match variant {
            ChdirVariant::Chdir => self.chdir_count += 1,
            ChdirVariant::Fchdir => self.fchdir_count += 1,
        }
    }

    pub fn record_failure(&mut self) {
        self.failed_count += 1;
    }

    pub fn total_changes(&self) -> u64 {
        self.chdir_count + self.fchdir_count
    }
}

/// Statistics for chdir app.
#[derive(Debug, Clone)]
pub struct ChdirAppStats {
    pub total_chdir: u64,
    pub total_fchdir: u64,
    pub total_failures: u64,
    pub unique_paths: u64,
    pub path_cache_hits: u64,
}

/// Main apps chdir manager.
pub struct AppChdir {
    pub processes: BTreeMap<u64, ProcessCwdState>,
    pub path_cache: BTreeMap<u64, String>, // path hash → resolved path
    pub recent_records: Vec<ChdirRecord>,
    pub next_record_id: u64,
    pub stats: ChdirAppStats,
}

impl AppChdir {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            path_cache: BTreeMap::new(),
            recent_records: Vec::new(),
            next_record_id: 1,
            stats: ChdirAppStats {
                total_chdir: 0,
                total_fchdir: 0,
                total_failures: 0,
                unique_paths: 0,
                path_cache_hits: 0,
            },
        }
    }

    pub fn record_chdir(
        &mut self,
        pid: u64,
        variant: ChdirVariant,
        target: String,
        result: ChdirResult,
    ) -> u64 {
        let id = self.next_record_id;
        self.next_record_id += 1;
        let state = self.processes.entry(pid).or_insert_with(|| ProcessCwdState::new(pid));
        let mut rec = ChdirRecord::new(id, pid, variant, target.clone());
        rec.old_cwd = state.current_cwd.clone();
        rec.result = result;
        if result == ChdirResult::Success {
            state.change_dir(target.clone(), variant);
            match variant {
                ChdirVariant::Chdir => self.stats.total_chdir += 1,
                ChdirVariant::Fchdir => self.stats.total_fchdir += 1,
            }
            // Cache the path
            let mut h: u64 = 0xcbf29ce484222325;
            for b in target.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            if !self.path_cache.contains_key(&h) {
                self.path_cache.insert(h, target);
                self.stats.unique_paths += 1;
            } else {
                self.stats.path_cache_hits += 1;
            }
        } else {
            state.record_failure();
            self.stats.total_failures += 1;
        }
        self.recent_records.push(rec);
        id
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}
