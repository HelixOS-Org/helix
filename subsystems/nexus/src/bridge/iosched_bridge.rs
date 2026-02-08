// SPDX-License-Identifier: GPL-2.0
//! Bridge IO scheduler — IO priority and scheduling bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IO sched bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedBridgeOp {
    IoprioSet,
    IoprioGet,
    Ionice,
    SetScheduler,
    GetScheduler,
    QueueDepth,
    NrRequests,
}

/// IO sched bridge priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedBridgePrio {
    RealTime,
    BestEffort,
    Idle,
    None,
}

/// IO sched bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedBridgeResult {
    Success,
    PermissionDenied,
    InvalidArg,
    NotSupported,
    Error,
}

/// IO sched bridge record
#[derive(Debug, Clone)]
pub struct IoSchedBridgeRecord {
    pub op: IoSchedBridgeOp,
    pub result: IoSchedBridgeResult,
    pub pid: u32,
    pub prio_class: IoSchedBridgePrio,
    pub prio_level: u32,
    pub scheduler_hash: u64,
}

impl IoSchedBridgeRecord {
    pub fn new(op: IoSchedBridgeOp, pid: u32) -> Self {
        Self { op, result: IoSchedBridgeResult::Success, pid, prio_class: IoSchedBridgePrio::BestEffort, prio_level: 4, scheduler_hash: 0 }
    }
}

/// IO sched bridge stats
#[derive(Debug, Clone)]
pub struct IoSchedBridgeStats {
    pub total_ops: u64,
    pub prio_sets: u64,
    pub prio_gets: u64,
    pub scheduler_changes: u64,
    pub errors: u64,
}

/// Main bridge IO scheduler
#[derive(Debug)]
pub struct BridgeIoSched {
    pub stats: IoSchedBridgeStats,
}

impl BridgeIoSched {
    pub fn new() -> Self {
        Self { stats: IoSchedBridgeStats { total_ops: 0, prio_sets: 0, prio_gets: 0, scheduler_changes: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &IoSchedBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            IoSchedBridgeOp::IoprioSet | IoSchedBridgeOp::Ionice => self.stats.prio_sets += 1,
            IoSchedBridgeOp::IoprioGet => self.stats.prio_gets += 1,
            IoSchedBridgeOp::SetScheduler => self.stats.scheduler_changes += 1,
            _ => {}
        }
        if rec.result != IoSchedBridgeResult::Success { self.stats.errors += 1; }
    }
}
