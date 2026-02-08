// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Close (file descriptor close operations)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Close operation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCloseResult {
    Success,
    InvalidFd,
    Busy,
    IoError,
    Interrupted,
}

/// Pending close entry with deferred operations
#[derive(Debug, Clone)]
pub struct AppCloseEntry {
    pub fd: u64,
    pub flush_pending: bool,
    pub sync_required: bool,
    pub lock_held: bool,
    pub close_timestamp: u64,
}

/// Statistics for close operations
#[derive(Debug, Clone)]
pub struct AppCloseStats {
    pub total_closes: u64,
    pub successful_closes: u64,
    pub failed_closes: u64,
    pub deferred_closes: u64,
    pub avg_close_latency_us: u64,
}

/// Manager for file close application operations
pub struct AppCloseManager {
    deferred_closes: BTreeMap<u64, AppCloseEntry>,
    close_history: Vec<(u64, AppCloseResult)>,
    stats: AppCloseStats,
    max_deferred: usize,
}

impl AppCloseManager {
    pub fn new() -> Self {
        Self {
            deferred_closes: BTreeMap::new(),
            close_history: Vec::new(),
            stats: AppCloseStats {
                total_closes: 0,
                successful_closes: 0,
                failed_closes: 0,
                deferred_closes: 0,
                avg_close_latency_us: 0,
            },
            max_deferred: 256,
        }
    }

    pub fn close_fd(&mut self, fd: u64, flush_pending: bool, sync_required: bool) -> AppCloseResult {
        self.stats.total_closes += 1;
        if flush_pending || sync_required {
            if self.deferred_closes.len() >= self.max_deferred {
                self.stats.failed_closes += 1;
                return AppCloseResult::Busy;
            }
            let entry = AppCloseEntry {
                fd,
                flush_pending,
                sync_required,
                lock_held: false,
                close_timestamp: self.stats.total_closes.wrapping_mul(23),
            };
            self.deferred_closes.insert(fd, entry);
            self.stats.deferred_closes += 1;
            AppCloseResult::Success
        } else {
            self.close_history.push((fd, AppCloseResult::Success));
            self.stats.successful_closes += 1;
            AppCloseResult::Success
        }
    }

    pub fn process_deferred(&mut self) -> usize {
        let fds: Vec<u64> = self.deferred_closes.keys().cloned().collect();
        let count = fds.len();
        for fd in fds {
            self.deferred_closes.remove(&fd);
            self.close_history.push((fd, AppCloseResult::Success));
            self.stats.successful_closes += 1;
        }
        count
    }

    pub fn deferred_count(&self) -> usize {
        self.deferred_closes.len()
    }

    pub fn stats(&self) -> &AppCloseStats {
        &self.stats
    }
}
