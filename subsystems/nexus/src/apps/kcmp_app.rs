// SPDX-License-Identifier: GPL-2.0
//! Apps kcmp_app — kcmp process comparison application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Kcmp resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcmpType {
    File,
    Vm,
    Files,
    Fs,
    Sighand,
    Io,
    Sysvsem,
    Epoll,
}

/// Kcmp result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcmpResult {
    Equal,
    Less,
    Greater,
    NotComparable,
}

/// Comparison record
#[derive(Debug)]
pub struct KcmpComparison {
    pub pid1: u64,
    pub pid2: u64,
    pub res_type: KcmpType,
    pub idx1: u64,
    pub idx2: u64,
    pub result: KcmpResult,
    pub timestamp: u64,
}

/// Process comparison tracker
#[derive(Debug)]
pub struct ProcessKcmpTracker {
    pub pid: u64,
    pub total_comparisons: u64,
    pub equal_count: u64,
    pub shared_files: u32,
    pub shared_vm: bool,
    pub shared_fs: bool,
}

impl ProcessKcmpTracker {
    pub fn new(pid: u64) -> Self {
        Self { pid, total_comparisons: 0, equal_count: 0, shared_files: 0, shared_vm: false, shared_fs: false }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KcmpAppStats {
    pub tracked_procs: u32,
    pub total_comparisons: u64,
    pub total_equal: u64,
}

/// Main app kcmp
pub struct AppKcmp {
    procs: BTreeMap<u64, ProcessKcmpTracker>,
}

impl AppKcmp {
    pub fn new() -> Self { Self { procs: BTreeMap::new() } }
    #[inline(always)]
    pub fn track(&mut self, pid: u64) { self.procs.insert(pid, ProcessKcmpTracker::new(pid)); }

    pub fn compare(&mut self, pid1: u64, pid2: u64, rtype: KcmpType, result: KcmpResult) {
        if let Some(p) = self.procs.get_mut(&pid1) {
            p.total_comparisons += 1;
            if result == KcmpResult::Equal { p.equal_count += 1; }
            match rtype {
                KcmpType::Vm if result == KcmpResult::Equal => p.shared_vm = true,
                KcmpType::Fs if result == KcmpResult::Equal => p.shared_fs = true,
                KcmpType::File if result == KcmpResult::Equal => p.shared_files += 1,
                _ => {}
            }
        }
        if let Some(p) = self.procs.get_mut(&pid2) {
            p.total_comparisons += 1;
            if result == KcmpResult::Equal { p.equal_count += 1; }
        }
    }

    #[inline(always)]
    pub fn untrack(&mut self, pid: u64) { self.procs.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> KcmpAppStats {
        let cmps: u64 = self.procs.values().map(|p| p.total_comparisons).sum();
        let eq: u64 = self.procs.values().map(|p| p.equal_count).sum();
        KcmpAppStats { tracked_procs: self.procs.len() as u32, total_comparisons: cmps, total_equal: eq }
    }
}
