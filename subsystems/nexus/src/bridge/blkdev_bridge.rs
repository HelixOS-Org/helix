// SPDX-License-Identifier: GPL-2.0
//! Bridge blkdev — block device ioctl and registration bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Blkdev bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlkdevBridgeOp {
    Open,
    Close,
    Ioctl,
    GetSize,
    GetSectorSize,
    Flush,
    Discard,
    RereadPartitions,
}

/// Blkdev bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlkdevBridgeResult {
    Success,
    Busy,
    NoDevice,
    PermissionDenied,
    NotSupported,
    Error,
}

/// Blkdev bridge record
#[derive(Debug, Clone)]
pub struct BlkdevBridgeRecord {
    pub op: BlkdevBridgeOp,
    pub result: BlkdevBridgeResult,
    pub dev_id: u64,
    pub major: u32,
    pub minor: u32,
    pub size_sectors: u64,
    pub latency_ns: u64,
}

impl BlkdevBridgeRecord {
    pub fn new(op: BlkdevBridgeOp, dev_id: u64) -> Self {
        Self { op, result: BlkdevBridgeResult::Success, dev_id, major: 0, minor: 0, size_sectors: 0, latency_ns: 0 }
    }
}

/// Blkdev bridge stats
#[derive(Debug, Clone)]
pub struct BlkdevBridgeStats {
    pub total_ops: u64,
    pub opens: u64,
    pub ioctls: u64,
    pub flushes: u64,
    pub errors: u64,
}

/// Main bridge blkdev
#[derive(Debug)]
pub struct BridgeBlkdev {
    pub stats: BlkdevBridgeStats,
}

impl BridgeBlkdev {
    pub fn new() -> Self {
        Self { stats: BlkdevBridgeStats { total_ops: 0, opens: 0, ioctls: 0, flushes: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &BlkdevBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            BlkdevBridgeOp::Open => self.stats.opens += 1,
            BlkdevBridgeOp::Ioctl => self.stats.ioctls += 1,
            BlkdevBridgeOp::Flush => self.stats.flushes += 1,
            _ => {}
        }
        if rec.result != BlkdevBridgeResult::Success { self.stats.errors += 1; }
    }
}
