// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Exit (process exit bridge)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Exit reason classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeExitReason {
    Normal,
    Signal,
    CoreDump,
    GroupExit,
    ExecFail,
    OutOfMemory,
}

/// Exit cleanup action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeExitCleanup {
    CloseFds,
    ReleaseMmap,
    FlushSignals,
    DetachTty,
    NotifyParent,
    ReleaseIpc,
}

/// Process exit record
#[derive(Debug, Clone)]
pub struct BridgeExitRecord {
    pub pid: u64,
    pub exit_code: i32,
    pub reason: BridgeExitReason,
    pub cleanup_actions: u32,
    pub timestamp: u64,
}

/// Stats for exit operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeExitStats {
    pub total_exits: u64,
    pub normal_exits: u64,
    pub signal_exits: u64,
    pub core_dumps: u64,
    pub oom_kills: u64,
    pub avg_cleanup_us: u64,
}

/// Manager for exit bridge operations
#[repr(align(64))]
pub struct BridgeExitManager {
    exit_records: Vec<BridgeExitRecord>,
    pending_cleanup: BTreeMap<u64, Vec<BridgeExitCleanup>>,
    stats: BridgeExitStats,
}

impl BridgeExitManager {
    pub fn new() -> Self {
        Self {
            exit_records: Vec::new(),
            pending_cleanup: BTreeMap::new(),
            stats: BridgeExitStats {
                total_exits: 0,
                normal_exits: 0,
                signal_exits: 0,
                core_dumps: 0,
                oom_kills: 0,
                avg_cleanup_us: 0,
            },
        }
    }

    pub fn initiate_exit(&mut self, pid: u64, exit_code: i32, reason: BridgeExitReason) {
        let record = BridgeExitRecord {
            pid,
            exit_code,
            reason,
            cleanup_actions: 0,
            timestamp: self.stats.total_exits.wrapping_mul(41),
        };
        self.exit_records.push(record);
        self.stats.total_exits += 1;
        match reason {
            BridgeExitReason::Normal => self.stats.normal_exits += 1,
            BridgeExitReason::Signal => self.stats.signal_exits += 1,
            BridgeExitReason::CoreDump => self.stats.core_dumps += 1,
            BridgeExitReason::OutOfMemory => self.stats.oom_kills += 1,
            _ => {}
        }
        let cleanups = alloc::vec![
            BridgeExitCleanup::CloseFds,
            BridgeExitCleanup::ReleaseMmap,
            BridgeExitCleanup::FlushSignals,
            BridgeExitCleanup::NotifyParent,
        ];
        self.pending_cleanup.insert(pid, cleanups);
    }

    #[inline]
    pub fn process_cleanup(&mut self, pid: u64) -> usize {
        if let Some(cleanups) = self.pending_cleanup.remove(&pid) {
            cleanups.len()
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn pending_cleanups(&self) -> usize {
        self.pending_cleanup.len()
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeExitStats {
        &self.stats
    }
}
