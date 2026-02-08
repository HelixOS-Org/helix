// SPDX-License-Identifier: GPL-2.0
//! Bridge device mapper — DM ioctl and target management bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// DM bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmBridgeOp {
    Create,
    Remove,
    Suspend,
    Resume,
    LoadTable,
    StatusInfo,
    StatusTable,
    Wait,
    Message,
    ListDevices,
    ListVersions,
}

/// DM bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmBridgeResult {
    Success,
    DeviceExists,
    DeviceNotFound,
    Busy,
    InvalidTable,
    PermissionDenied,
    Error,
}

/// DM bridge record
#[derive(Debug, Clone)]
pub struct DmBridgeRecord {
    pub op: DmBridgeOp,
    pub result: DmBridgeResult,
    pub name_hash: u64,
    pub uuid_hash: u64,
    pub nr_targets: u32,
    pub flags: u32,
    pub latency_ns: u64,
}

impl DmBridgeRecord {
    pub fn new(op: DmBridgeOp, name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { op, result: DmBridgeResult::Success, name_hash: h, uuid_hash: 0, nr_targets: 0, flags: 0, latency_ns: 0 }
    }
}

/// DM bridge stats
#[derive(Debug, Clone)]
pub struct DmBridgeStats {
    pub total_ops: u64,
    pub creates: u64,
    pub removes: u64,
    pub suspends: u64,
    pub table_loads: u64,
    pub errors: u64,
}

/// Main bridge device mapper
#[derive(Debug)]
pub struct BridgeDevMapper {
    pub stats: DmBridgeStats,
}

impl BridgeDevMapper {
    pub fn new() -> Self {
        Self { stats: DmBridgeStats { total_ops: 0, creates: 0, removes: 0, suspends: 0, table_loads: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &DmBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            DmBridgeOp::Create => self.stats.creates += 1,
            DmBridgeOp::Remove => self.stats.removes += 1,
            DmBridgeOp::Suspend => self.stats.suspends += 1,
            DmBridgeOp::LoadTable => self.stats.table_loads += 1,
            _ => {}
        }
        if rec.result != DmBridgeResult::Success { self.stats.errors += 1; }
    }
}
