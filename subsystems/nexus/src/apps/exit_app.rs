// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Exit (process exit application interface)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Exit reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppExitReason {
    Normal,
    Signal,
    Abort,
    Segfault,
    OutOfMemory,
    GroupExit,
}

/// Exit record
#[derive(Debug, Clone)]
pub struct AppExitRecord {
    pub pid: u64,
    pub exit_code: i32,
    pub reason: AppExitReason,
    pub fds_closed: u32,
    pub memory_freed_kb: u64,
    pub timestamp: u64,
}

/// Stats for exit operations
#[derive(Debug, Clone)]
pub struct AppExitStats {
    pub total_exits: u64,
    pub normal_exits: u64,
    pub signal_exits: u64,
    pub oom_kills: u64,
    pub avg_cleanup_us: u64,
    pub total_memory_freed_kb: u64,
}

/// Manager for exit application operations
pub struct AppExitManager {
    exit_log: Vec<AppExitRecord>,
    pending_cleanup: BTreeMap<u64, u32>,
    stats: AppExitStats,
}

impl AppExitManager {
    pub fn new() -> Self {
        Self {
            exit_log: Vec::new(),
            pending_cleanup: BTreeMap::new(),
            stats: AppExitStats {
                total_exits: 0,
                normal_exits: 0,
                signal_exits: 0,
                oom_kills: 0,
                avg_cleanup_us: 0,
                total_memory_freed_kb: 0,
            },
        }
    }

    pub fn exit_process(&mut self, pid: u64, exit_code: i32, reason: AppExitReason, fds: u32, mem_kb: u64) {
        let record = AppExitRecord {
            pid,
            exit_code,
            reason,
            fds_closed: fds,
            memory_freed_kb: mem_kb,
            timestamp: self.stats.total_exits.wrapping_mul(41),
        };
        self.exit_log.push(record);
        self.stats.total_exits += 1;
        self.stats.total_memory_freed_kb += mem_kb;
        match reason {
            AppExitReason::Normal => self.stats.normal_exits += 1,
            AppExitReason::Signal | AppExitReason::Abort | AppExitReason::Segfault => self.stats.signal_exits += 1,
            AppExitReason::OutOfMemory => self.stats.oom_kills += 1,
            _ => {}
        }
    }

    pub fn schedule_cleanup(&mut self, pid: u64, steps: u32) {
        self.pending_cleanup.insert(pid, steps);
    }

    pub fn process_cleanup(&mut self, pid: u64) -> u32 {
        self.pending_cleanup.remove(&pid).unwrap_or(0)
    }

    pub fn stats(&self) -> &AppExitStats {
        &self.stats
    }
}
