// SPDX-License-Identifier: GPL-2.0
//! Apps execve_app — execve process execution application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Exec type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecType {
    Execve,
    Execveat,
    Fexecve,
}

/// Exec result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecResult {
    Success,
    NotFound,
    PermissionDenied,
    NotExecutable,
    TooManyArgs,
    BadInterpreter,
    OutOfMemory,
}

/// Exec entry
#[derive(Debug)]
pub struct ExecEntry {
    pub pid: u64,
    pub binary_hash: u64,
    pub exec_type: ExecType,
    pub result: ExecResult,
    pub argc: u32,
    pub envc: u32,
    pub timestamp: u64,
    pub load_time_ns: u64,
}

/// Process exec tracker
#[derive(Debug)]
pub struct ProcessExecTracker {
    pub pid: u64,
    pub total_execs: u64,
    pub success_count: u64,
    pub fail_count: u64,
    pub last_binary_hash: u64,
    pub avg_load_ns: u64,
}

impl ProcessExecTracker {
    pub fn new(pid: u64) -> Self {
        Self { pid, total_execs: 0, success_count: 0, fail_count: 0, last_binary_hash: 0, avg_load_ns: 0 }
    }

    #[inline]
    pub fn record(&mut self, entry: &ExecEntry) {
        self.total_execs += 1;
        if entry.result == ExecResult::Success {
            self.success_count += 1;
            self.last_binary_hash = entry.binary_hash;
            let n = self.success_count;
            self.avg_load_ns = self.avg_load_ns * (n - 1) / n + entry.load_time_ns / n;
        } else {
            self.fail_count += 1;
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ExecveAppStats {
    pub tracked_procs: u32,
    pub total_execs: u64,
    pub total_success: u64,
    pub total_failures: u64,
}

/// Main app execve
pub struct AppExecve {
    procs: BTreeMap<u64, ProcessExecTracker>,
}

impl AppExecve {
    pub fn new() -> Self { Self { procs: BTreeMap::new() } }

    #[inline(always)]
    pub fn track(&mut self, pid: u64) { self.procs.insert(pid, ProcessExecTracker::new(pid)); }

    #[inline(always)]
    pub fn exec(&mut self, entry: &ExecEntry) {
        if let Some(t) = self.procs.get_mut(&entry.pid) { t.record(entry); }
    }

    #[inline(always)]
    pub fn untrack(&mut self, pid: u64) { self.procs.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> ExecveAppStats {
        let execs: u64 = self.procs.values().map(|p| p.total_execs).sum();
        let succ: u64 = self.procs.values().map(|p| p.success_count).sum();
        let fail: u64 = self.procs.values().map(|p| p.fail_count).sum();
        ExecveAppStats { tracked_procs: self.procs.len() as u32, total_execs: execs, total_success: succ, total_failures: fail }
    }
}
