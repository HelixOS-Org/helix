// SPDX-License-Identifier: GPL-2.0
//! Bridge RAID — md RAID management bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RAID bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidBridgeOp {
    Create,
    Assemble,
    Stop,
    AddDisk,
    RemoveDisk,
    FailDisk,
    Rebuild,
    CheckArray,
    Scrub,
    Grow,
}

/// RAID bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidBridgeResult {
    Success,
    Busy,
    DiskFailed,
    ArrayDegraded,
    NotSupported,
    Error,
}

/// RAID bridge record
#[derive(Debug, Clone)]
pub struct RaidBridgeRecord {
    pub op: RaidBridgeOp,
    pub result: RaidBridgeResult,
    pub array_id: u64,
    pub disk_id: u64,
    pub raid_level: u8,
    pub nr_disks: u32,
    pub duration_ns: u64,
}

impl RaidBridgeRecord {
    pub fn new(op: RaidBridgeOp, array_id: u64) -> Self {
        Self { op, result: RaidBridgeResult::Success, array_id, disk_id: 0, raid_level: 0, nr_disks: 0, duration_ns: 0 }
    }
}

/// RAID bridge stats
#[derive(Debug, Clone)]
pub struct RaidBridgeStats {
    pub total_ops: u64,
    pub creates: u64,
    pub disk_failures: u64,
    pub rebuilds: u64,
    pub scrubs: u64,
    pub errors: u64,
}

/// Main bridge RAID
#[derive(Debug)]
pub struct BridgeRaid {
    pub stats: RaidBridgeStats,
}

impl BridgeRaid {
    pub fn new() -> Self {
        Self { stats: RaidBridgeStats { total_ops: 0, creates: 0, disk_failures: 0, rebuilds: 0, scrubs: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &RaidBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            RaidBridgeOp::Create | RaidBridgeOp::Assemble => self.stats.creates += 1,
            RaidBridgeOp::FailDisk => self.stats.disk_failures += 1,
            RaidBridgeOp::Rebuild => self.stats.rebuilds += 1,
            RaidBridgeOp::Scrub | RaidBridgeOp::CheckArray => self.stats.scrubs += 1,
            _ => {}
        }
        if rec.result != RaidBridgeResult::Success { self.stats.errors += 1; }
    }
}
