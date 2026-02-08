// SPDX-License-Identifier: GPL-2.0
//! Bridge capability — POSIX capability management bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeCap {
    CapChown,
    CapDacOverride,
    CapDacReadSearch,
    CapFowner,
    CapFsetid,
    CapKill,
    CapSetgid,
    CapSetuid,
    CapSetpcap,
    CapLinuxImmutable,
    CapNetBindService,
    CapNetBroadcast,
    CapNetAdmin,
    CapNetRaw,
    CapSysAdmin,
    CapSysRawio,
}

/// Capability operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapOp {
    Check,
    Get,
    Set,
    Drop,
    Inherit,
    Ambient,
}

/// Capability result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapResult {
    Capable,
    NotCapable,
    Dropped,
    Set,
    Error,
}

/// Capability record
#[derive(Debug, Clone)]
pub struct CapRecord {
    pub op: CapOp,
    pub cap: BridgeCap,
    pub result: CapResult,
    pub pid: u32,
    pub uid: u32,
}

impl CapRecord {
    pub fn new(op: CapOp, cap: BridgeCap) -> Self {
        Self {
            op,
            cap,
            result: CapResult::Capable,
            pid: 0,
            uid: 0,
        }
    }
}

/// Capability bridge stats
#[derive(Debug, Clone)]
pub struct CapBridgeStats {
    pub total_ops: u64,
    pub checks: u64,
    pub capable: u64,
    pub not_capable: u64,
    pub drops: u64,
}

/// Main bridge capability
#[derive(Debug)]
pub struct BridgeCapability {
    pub stats: CapBridgeStats,
}

impl BridgeCapability {
    pub fn new() -> Self {
        Self {
            stats: CapBridgeStats {
                total_ops: 0,
                checks: 0,
                capable: 0,
                not_capable: 0,
                drops: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &CapRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            CapOp::Check => {
                self.stats.checks += 1;
                match rec.result {
                    CapResult::Capable => self.stats.capable += 1,
                    CapResult::NotCapable => self.stats.not_capable += 1,
                    _ => {},
                }
            },
            CapOp::Drop => self.stats.drops += 1,
            _ => {},
        }
    }
}
