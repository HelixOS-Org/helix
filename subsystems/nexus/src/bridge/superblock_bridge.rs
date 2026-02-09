// SPDX-License-Identifier: GPL-2.0
//! Bridge superblock — filesystem superblock syscall bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Superblock bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbBridgeOp {
    Statfs,
    SyncFs,
    Freeze,
    Thaw,
    Remount,
    QuotaSync,
    QuotaOn,
    QuotaOff,
}

/// Superblock bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbBridgeResult {
    Success,
    ReadOnly,
    Busy,
    NotSupported,
    Error,
}

/// Superblock bridge record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SbBridgeRecord {
    pub op: SbBridgeOp,
    pub result: SbBridgeResult,
    pub dev_id: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub latency_ns: u64,
}

impl SbBridgeRecord {
    pub fn new(op: SbBridgeOp, dev_id: u64) -> Self {
        Self { op, result: SbBridgeResult::Success, dev_id, total_blocks: 0, free_blocks: 0, total_inodes: 0, free_inodes: 0, latency_ns: 0 }
    }

    #[inline(always)]
    pub fn usage_pct(&self) -> f64 {
        if self.total_blocks == 0 { 0.0 } else { (self.total_blocks - self.free_blocks) as f64 / self.total_blocks as f64 }
    }
}

/// Superblock bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SbBridgeStats {
    pub total_ops: u64,
    pub statfs_calls: u64,
    pub syncs: u64,
    pub freezes: u64,
    pub errors: u64,
}

/// Main bridge superblock
#[derive(Debug)]
pub struct BridgeSuperblock {
    pub stats: SbBridgeStats,
}

impl BridgeSuperblock {
    pub fn new() -> Self {
        Self { stats: SbBridgeStats { total_ops: 0, statfs_calls: 0, syncs: 0, freezes: 0, errors: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &SbBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            SbBridgeOp::Statfs => self.stats.statfs_calls += 1,
            SbBridgeOp::SyncFs => self.stats.syncs += 1,
            SbBridgeOp::Freeze => self.stats.freezes += 1,
            _ => {}
        }
        if rec.result != SbBridgeResult::Success { self.stats.errors += 1; }
    }
}

// ============================================================================
// Merged from superblock_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuperblockV2Event { Sync, Freeze, Thaw, Remount, StatFs, QuotaSync }

/// Superblock v2 record
#[derive(Debug, Clone)]
pub struct SuperblockV2Record {
    pub event: SuperblockV2Event,
    pub sb_id: u64,
    pub block_size: u32,
    pub total_blocks: u64,
    pub free_blocks: u64,
}

impl SuperblockV2Record {
    pub fn new(event: SuperblockV2Event) -> Self { Self { event, sb_id: 0, block_size: 4096, total_blocks: 0, free_blocks: 0 } }
}

/// Superblock v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SuperblockV2BridgeStats { pub total_events: u64, pub syncs: u64, pub freezes: u64, pub remounts: u64 }

/// Main bridge superblock v2
#[derive(Debug)]
pub struct BridgeSuperblockV2 { pub stats: SuperblockV2BridgeStats }

impl BridgeSuperblockV2 {
    pub fn new() -> Self { Self { stats: SuperblockV2BridgeStats { total_events: 0, syncs: 0, freezes: 0, remounts: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &SuperblockV2Record) {
        self.stats.total_events += 1;
        match rec.event {
            SuperblockV2Event::Sync | SuperblockV2Event::QuotaSync => self.stats.syncs += 1,
            SuperblockV2Event::Freeze | SuperblockV2Event::Thaw => self.stats.freezes += 1,
            SuperblockV2Event::Remount | SuperblockV2Event::StatFs => self.stats.remounts += 1,
        }
    }
}
