// SPDX-License-Identifier: GPL-2.0
//! Bridge fallocate — file space preallocation with punch-hole and collapse-range

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fallocate mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallocateMode {
    Allocate,
    KeepSize,
    PunchHole,
    CollapseRange,
    ZeroRange,
    InsertRange,
    Unshare,
}

/// Fallocate state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallocateState {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Fallocate operation
#[derive(Debug, Clone)]
pub struct FallocateOp {
    pub op_id: u64,
    pub fd: i32,
    pub mode: FallocateMode,
    pub offset: u64,
    pub len: u64,
    pub state: FallocateState,
    pub blocks_allocated: u64,
    pub blocks_freed: u64,
    pub start_ns: u64,
    pub end_ns: u64,
}

impl FallocateOp {
    pub fn new(op_id: u64, fd: i32, mode: FallocateMode, offset: u64, len: u64) -> Self {
        Self {
            op_id,
            fd,
            mode,
            offset,
            len,
            state: FallocateState::Pending,
            blocks_allocated: 0,
            blocks_freed: 0,
            start_ns: 0,
            end_ns: 0,
        }
    }

    #[inline(always)]
    pub fn start(&mut self, ts_ns: u64) {
        self.state = FallocateState::InProgress;
        self.start_ns = ts_ns;
    }

    #[inline]
    pub fn complete(&mut self, ts_ns: u64, blocks_alloc: u64, blocks_free: u64) {
        self.state = FallocateState::Completed;
        self.end_ns = ts_ns;
        self.blocks_allocated = blocks_alloc;
        self.blocks_freed = blocks_free;
    }

    #[inline(always)]
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    #[inline(always)]
    pub fn is_destructive(&self) -> bool {
        matches!(self.mode, FallocateMode::PunchHole | FallocateMode::CollapseRange | FallocateMode::InsertRange)
    }

    #[inline(always)]
    pub fn net_blocks(&self) -> i64 {
        self.blocks_allocated as i64 - self.blocks_freed as i64
    }
}

/// File space tracker
#[derive(Debug, Clone)]
pub struct FileSpaceTracker {
    pub fd: i32,
    pub allocated_blocks: u64,
    pub hole_blocks: u64,
    pub total_ops: u64,
    pub prealloc_ops: u64,
    pub punch_ops: u64,
    pub collapse_ops: u64,
}

impl FileSpaceTracker {
    pub fn new(fd: i32) -> Self {
        Self {
            fd,
            allocated_blocks: 0,
            hole_blocks: 0,
            total_ops: 0,
            prealloc_ops: 0,
            punch_ops: 0,
            collapse_ops: 0,
        }
    }

    pub fn record_op(&mut self, op: &FallocateOp) {
        self.total_ops += 1;
        match op.mode {
            FallocateMode::Allocate | FallocateMode::KeepSize => {
                self.allocated_blocks += op.blocks_allocated;
                self.prealloc_ops += 1;
            }
            FallocateMode::PunchHole | FallocateMode::ZeroRange => {
                self.hole_blocks += op.blocks_freed;
                self.punch_ops += 1;
            }
            FallocateMode::CollapseRange | FallocateMode::InsertRange => {
                self.collapse_ops += 1;
            }
            _ => {}
        }
    }

    #[inline(always)]
    pub fn fragmentation_estimate(&self) -> f64 {
        let total = self.allocated_blocks + self.hole_blocks;
        if total == 0 { 0.0 } else { self.hole_blocks as f64 / total as f64 }
    }
}

/// Fallocate bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FallocateBridgeStats {
    pub total_ops: u64,
    pub total_blocks_allocated: u64,
    pub total_blocks_freed: u64,
    pub prealloc_ops: u64,
    pub punch_ops: u64,
    pub collapse_ops: u64,
}

/// Main bridge fallocate
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeFallocate {
    pub file_trackers: BTreeMap<i32, FileSpaceTracker>,
    pub stats: FallocateBridgeStats,
    pub next_op_id: u64,
}

impl BridgeFallocate {
    pub fn new() -> Self {
        Self {
            file_trackers: BTreeMap::new(),
            stats: FallocateBridgeStats {
                total_ops: 0,
                total_blocks_allocated: 0,
                total_blocks_freed: 0,
                prealloc_ops: 0,
                punch_ops: 0,
                collapse_ops: 0,
            },
            next_op_id: 1,
        }
    }

    pub fn fallocate(&mut self, fd: i32, mode: FallocateMode, offset: u64, len: u64, ts_ns: u64, blocks_alloc: u64, blocks_free: u64) -> u64 {
        let id = self.next_op_id;
        self.next_op_id += 1;
        let mut op = FallocateOp::new(id, fd, mode, offset, len);
        op.start(ts_ns);
        op.complete(ts_ns, blocks_alloc, blocks_free);
        self.stats.total_ops += 1;
        self.stats.total_blocks_allocated += blocks_alloc;
        self.stats.total_blocks_freed += blocks_free;
        match mode {
            FallocateMode::Allocate | FallocateMode::KeepSize => self.stats.prealloc_ops += 1,
            FallocateMode::PunchHole | FallocateMode::ZeroRange => self.stats.punch_ops += 1,
            FallocateMode::CollapseRange | FallocateMode::InsertRange => self.stats.collapse_ops += 1,
            _ => {}
        }
        let tracker = self.file_trackers.entry(fd).or_insert_with(|| FileSpaceTracker::new(fd));
        tracker.record_op(&op);
        id
    }

    #[inline(always)]
    pub fn net_block_change(&self) -> i64 {
        self.stats.total_blocks_allocated as i64 - self.stats.total_blocks_freed as i64
    }
}

// ============================================================================
// Merged from fallocate_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallocateV2Mode { Allocate, PunchHole, CollapseRange, ZeroRange, InsertRange }

/// Fallocate v2 record
#[derive(Debug, Clone)]
pub struct FallocateV2Record {
    pub mode: FallocateV2Mode,
    pub fd: i32,
    pub offset: u64,
    pub len: u64,
}

impl FallocateV2Record {
    pub fn new(mode: FallocateV2Mode, fd: i32) -> Self { Self { mode, fd, offset: 0, len: 0 } }
}

/// Fallocate v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FallocateV2BridgeStats { pub total_ops: u64, pub allocs: u64, pub punches: u64, pub zeros: u64 }

/// Main bridge fallocate v2
#[derive(Debug)]
pub struct BridgeFallocateV2 { pub stats: FallocateV2BridgeStats }

impl BridgeFallocateV2 {
    pub fn new() -> Self { Self { stats: FallocateV2BridgeStats { total_ops: 0, allocs: 0, punches: 0, zeros: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &FallocateV2Record) {
        self.stats.total_ops += 1;
        match rec.mode {
            FallocateV2Mode::Allocate | FallocateV2Mode::InsertRange => self.stats.allocs += 1,
            FallocateV2Mode::PunchHole | FallocateV2Mode::CollapseRange => self.stats.punches += 1,
            FallocateV2Mode::ZeroRange => self.stats.zeros += 1,
        }
    }
}
