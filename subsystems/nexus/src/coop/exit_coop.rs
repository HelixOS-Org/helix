// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Exit (cooperative process exit/cleanup)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Cooperative exit phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopExitPhase {
    SignalHandlers,
    FdCleanup,
    MmapCleanup,
    IpcCleanup,
    ParentNotify,
    ZombieState,
    Complete,
}

/// Exit cooperation record
#[derive(Debug, Clone)]
pub struct CoopExitRecord {
    pub pid: u64,
    pub exit_code: i32,
    pub phase: CoopExitPhase,
    pub fds_released: u32,
    pub pages_freed: u64,
    pub cleanup_us: u64,
}

/// Exit cooperation stats
#[derive(Debug, Clone)]
pub struct CoopExitStats {
    pub total_exits: u64,
    pub clean_exits: u64,
    pub forced_exits: u64,
    pub total_fds_released: u64,
    pub total_pages_freed: u64,
    pub avg_cleanup_us: u64,
}

/// Manager for cooperative exit operations
pub struct CoopExitManager {
    records: Vec<CoopExitRecord>,
    orphans: BTreeMap<u64, u64>,
    stats: CoopExitStats,
}

impl CoopExitManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            orphans: BTreeMap::new(),
            stats: CoopExitStats {
                total_exits: 0,
                clean_exits: 0,
                forced_exits: 0,
                total_fds_released: 0,
                total_pages_freed: 0,
                avg_cleanup_us: 0,
            },
        }
    }

    pub fn cooperative_exit(&mut self, pid: u64, code: i32, fds: u32, pages: u64, forced: bool) {
        let record = CoopExitRecord {
            pid,
            exit_code: code,
            phase: CoopExitPhase::Complete,
            fds_released: fds,
            pages_freed: pages,
            cleanup_us: if forced { 50 } else { 200 },
        };
        self.records.push(record);
        self.stats.total_exits += 1;
        self.stats.total_fds_released += fds as u64;
        self.stats.total_pages_freed += pages;
        if forced { self.stats.forced_exits += 1; } else { self.stats.clean_exits += 1; }
    }

    pub fn register_orphan(&mut self, child: u64, new_parent: u64) {
        self.orphans.insert(child, new_parent);
    }

    pub fn reparent_orphan(&mut self, child: u64) -> Option<u64> {
        self.orphans.remove(&child)
    }

    pub fn stats(&self) -> &CoopExitStats {
        &self.stats
    }
}
