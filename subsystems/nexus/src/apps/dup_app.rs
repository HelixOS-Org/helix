// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps dup — File descriptor duplication tracking
//!
//! Tracks dup/dup2/dup3 operations with FD inheritance chains,
//! CLOEXEC flag management, and FD leak detection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Dup variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DupVariant {
    Dup,
    Dup2,
    Dup3,
    Fcntl,
}

/// Dup result status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DupResult {
    Success,
    BadFd,
    Busy,
    InvalidArg,
    TooManyFds,
    Interrupted,
}

/// Dup flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DupFlag {
    CloseExec,
}

/// A tracked dup operation.
#[derive(Debug, Clone)]
pub struct DupRecord {
    pub record_id: u64,
    pub pid: u64,
    pub variant: DupVariant,
    pub old_fd: i32,
    pub new_fd: i32,
    pub result: DupResult,
    pub flags: Vec<DupFlag>,
    pub timestamp: u64,
}

impl DupRecord {
    pub fn new(record_id: u64, pid: u64, variant: DupVariant, old_fd: i32, new_fd: i32) -> Self {
        Self {
            record_id,
            pid,
            variant,
            old_fd,
            new_fd,
            result: DupResult::Success,
            flags: Vec::new(),
            timestamp: 0,
        }
    }
}

/// Per-process FD chain for leak detection.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessDupState {
    pub pid: u64,
    pub dup_chains: BTreeMap<i32, Vec<i32>>, // original fd → [dup'd fds]
    pub total_dups: u64,
    pub total_closes: u64,
    pub suspected_leaks: u64,
    pub max_fd_seen: i32,
}

impl ProcessDupState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            dup_chains: BTreeMap::new(),
            total_dups: 0,
            total_closes: 0,
            suspected_leaks: 0,
            max_fd_seen: -1,
        }
    }

    #[inline]
    pub fn record_dup(&mut self, old_fd: i32, new_fd: i32) {
        let chain = self.dup_chains.entry(old_fd).or_insert_with(Vec::new);
        chain.push(new_fd);
        self.total_dups += 1;
        if new_fd > self.max_fd_seen {
            self.max_fd_seen = new_fd;
        }
    }

    #[inline]
    pub fn record_close(&mut self, fd: i32) {
        self.total_closes += 1;
        // Remove from chains
        for chain in self.dup_chains.values_mut() {
            chain.retain(|&f| f != fd);
        }
        self.dup_chains.remove(&fd);
    }

    #[inline]
    pub fn leak_score(&self) -> f64 {
        if self.total_dups == 0 {
            return 0.0;
        }
        let unclosed = self.total_dups.saturating_sub(self.total_closes);
        unclosed as f64 / self.total_dups as f64
    }
}

/// Statistics for the dup app.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DupAppStats {
    pub total_dups: u64,
    pub total_dup2s: u64,
    pub total_dup3s: u64,
    pub total_failures: u64,
    pub cloexec_count: u64,
    pub leak_warnings: u64,
}

/// Main apps dup manager.
pub struct AppDup {
    pub processes: BTreeMap<u64, ProcessDupState>,
    pub recent_records: Vec<DupRecord>,
    pub next_record_id: u64,
    pub stats: DupAppStats,
}

impl AppDup {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            recent_records: Vec::new(),
            next_record_id: 1,
            stats: DupAppStats {
                total_dups: 0,
                total_dup2s: 0,
                total_dup3s: 0,
                total_failures: 0,
                cloexec_count: 0,
                leak_warnings: 0,
            },
        }
    }

    pub fn record_dup(
        &mut self,
        pid: u64,
        variant: DupVariant,
        old_fd: i32,
        new_fd: i32,
        cloexec: bool,
    ) -> u64 {
        let id = self.next_record_id;
        self.next_record_id += 1;
        let mut rec = DupRecord::new(id, pid, variant, old_fd, new_fd);
        if cloexec {
            rec.flags.push(DupFlag::CloseExec);
            self.stats.cloexec_count += 1;
        }
        let state = self.processes.entry(pid).or_insert_with(|| ProcessDupState::new(pid));
        state.record_dup(old_fd, new_fd);
        match variant {
            DupVariant::Dup => self.stats.total_dups += 1,
            DupVariant::Dup2 => self.stats.total_dup2s += 1,
            DupVariant::Dup3 => self.stats.total_dup3s += 1,
            DupVariant::Fcntl => self.stats.total_dups += 1,
        }
        self.recent_records.push(rec);
        id
    }

    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}
