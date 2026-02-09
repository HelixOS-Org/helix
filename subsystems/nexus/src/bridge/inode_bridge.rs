// SPDX-License-Identifier: GPL-2.0
//! Bridge inode — inode operation bridge with alloc/free tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Inode bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeBridgeOp {
    Alloc,
    Free,
    Read,
    Write,
    Truncate,
    SetAttr,
    GetAttr,
    Permission,
    Evict,
    Writeback,
}

/// Inode bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeBridgeResult {
    Success,
    NoSpace,
    PermissionDenied,
    Stale,
    IoError,
    Error,
}

/// Inode bridge record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InodeBridgeRecord {
    pub op: InodeBridgeOp,
    pub result: InodeBridgeResult,
    pub inode: u64,
    pub size: u64,
    pub blocks: u64,
    pub latency_ns: u64,
}

impl InodeBridgeRecord {
    pub fn new(op: InodeBridgeOp, inode: u64) -> Self {
        Self { op, result: InodeBridgeResult::Success, inode, size: 0, blocks: 0, latency_ns: 0 }
    }
}

/// Inode bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InodeBridgeStats {
    pub total_ops: u64,
    pub allocs: u64,
    pub frees: u64,
    pub writebacks: u64,
    pub errors: u64,
    pub total_latency_ns: u64,
}

/// Main bridge inode
#[derive(Debug)]
pub struct BridgeInode {
    pub stats: InodeBridgeStats,
}

impl BridgeInode {
    pub fn new() -> Self {
        Self { stats: InodeBridgeStats { total_ops: 0, allocs: 0, frees: 0, writebacks: 0, errors: 0, total_latency_ns: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &InodeBridgeRecord) {
        self.stats.total_ops += 1;
        self.stats.total_latency_ns += rec.latency_ns;
        match rec.op {
            InodeBridgeOp::Alloc => self.stats.allocs += 1,
            InodeBridgeOp::Free | InodeBridgeOp::Evict => self.stats.frees += 1,
            InodeBridgeOp::Writeback => self.stats.writebacks += 1,
            _ => {}
        }
        if rec.result != InodeBridgeResult::Success { self.stats.errors += 1; }
    }
}

// ============================================================================
// Merged from inode_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeV2Event { Alloc, Free, Chmod, Chown, Truncate, SetAttr }

/// Inode v2 record
#[derive(Debug, Clone)]
pub struct InodeV2Record {
    pub event: InodeV2Event,
    pub inode: u64,
    pub mode: u32,
    pub size: u64,
    pub nlinks: u32,
}

impl InodeV2Record {
    pub fn new(event: InodeV2Event, inode: u64) -> Self { Self { event, inode, mode: 0, size: 0, nlinks: 1 } }
}

/// Inode v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InodeV2BridgeStats { pub total_events: u64, pub allocs: u64, pub frees: u64, pub attr_changes: u64 }

/// Main bridge inode v2
#[derive(Debug)]
pub struct BridgeInodeV2 { pub stats: InodeV2BridgeStats }

impl BridgeInodeV2 {
    pub fn new() -> Self { Self { stats: InodeV2BridgeStats { total_events: 0, allocs: 0, frees: 0, attr_changes: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &InodeV2Record) {
        self.stats.total_events += 1;
        match rec.event {
            InodeV2Event::Alloc => self.stats.allocs += 1,
            InodeV2Event::Free => self.stats.frees += 1,
            _ => self.stats.attr_changes += 1,
        }
    }
}
