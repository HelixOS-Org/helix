// SPDX-License-Identifier: GPL-2.0
//! Apps fallocate_app — file space preallocation.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Fallocate mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallocateMode {
    Default,
    KeepSize,
    PunchHole,
    CollapseRange,
    ZeroRange,
    InsertRange,
    Unshare,
}

/// Fallocate operation
#[derive(Debug)]
pub struct FallocateOp {
    pub id: u64,
    pub fd: u64,
    pub mode: FallocateMode,
    pub offset: u64,
    pub length: u64,
    pub timestamp: u64,
    pub success: bool,
}

impl FallocateOp {
    pub fn new(id: u64, fd: u64, mode: FallocateMode, offset: u64, length: u64, now: u64) -> Self {
        Self { id, fd, mode, offset, length, timestamp: now, success: false }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct FallocateAppStats {
    pub total_ops: u32,
    pub successful: u32,
    pub failed: u32,
    pub total_bytes_allocated: u64,
    pub total_bytes_punched: u64,
}

/// Main fallocate app
pub struct AppFallocate {
    ops: BTreeMap<u64, FallocateOp>,
    next_id: u64,
}

impl AppFallocate {
    pub fn new() -> Self { Self { ops: BTreeMap::new(), next_id: 1 } }

    pub fn fallocate(&mut self, fd: u64, mode: FallocateMode, offset: u64, length: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.ops.insert(id, FallocateOp::new(id, fd, mode, offset, length, now));
        id
    }

    pub fn complete(&mut self, id: u64) {
        if let Some(op) = self.ops.get_mut(&id) { op.success = true; }
    }

    pub fn stats(&self) -> FallocateAppStats {
        let ok = self.ops.values().filter(|o| o.success).count() as u32;
        let fail = self.ops.len() as u32 - ok;
        let allocated: u64 = self.ops.values().filter(|o| o.success && o.mode == FallocateMode::Default).map(|o| o.length).sum();
        let punched: u64 = self.ops.values().filter(|o| o.success && o.mode == FallocateMode::PunchHole).map(|o| o.length).sum();
        FallocateAppStats { total_ops: self.ops.len() as u32, successful: ok, failed: fail, total_bytes_allocated: allocated, total_bytes_punched: punched }
    }
}

// ============================================================================
// Merged from fallocate_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallocateV2Mode {
    Default,
    KeepSize,
    PunchHole,
    CollapseRange,
    ZeroRange,
    InsertRange,
    Unshare,
}

/// Fallocate v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallocateV2Result {
    Success,
    NoSpace,
    BadFd,
    NotSupported,
    Invalid,
    PermissionDenied,
    TextBusy,
    Error,
}

/// Fallocate v2 record
#[derive(Debug, Clone)]
pub struct FallocateV2Record {
    pub fd: i32,
    pub mode: FallocateV2Mode,
    pub result: FallocateV2Result,
    pub offset: u64,
    pub length: u64,
    pub latency_ns: u64,
}

impl FallocateV2Record {
    pub fn new(fd: i32, mode: FallocateV2Mode, offset: u64, length: u64) -> Self {
        Self { fd, mode, result: FallocateV2Result::Success, offset, length, latency_ns: 0 }
    }

    pub fn is_deallocating(&self) -> bool {
        matches!(self.mode, FallocateV2Mode::PunchHole | FallocateV2Mode::CollapseRange)
    }

    pub fn is_allocating(&self) -> bool {
        matches!(self.mode, FallocateV2Mode::Default | FallocateV2Mode::InsertRange)
    }
}

/// Fallocate v2 app stats
#[derive(Debug, Clone)]
pub struct FallocateV2AppStats {
    pub total_calls: u64,
    pub allocated_bytes: u64,
    pub deallocated_bytes: u64,
    pub punch_holes: u64,
    pub errors: u64,
}

/// Main app fallocate v2
#[derive(Debug)]
pub struct AppFallocateV2 {
    pub stats: FallocateV2AppStats,
}

impl AppFallocateV2 {
    pub fn new() -> Self {
        Self { stats: FallocateV2AppStats { total_calls: 0, allocated_bytes: 0, deallocated_bytes: 0, punch_holes: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &FallocateV2Record) {
        self.stats.total_calls += 1;
        if rec.result != FallocateV2Result::Success { self.stats.errors += 1; return; }
        if rec.is_allocating() { self.stats.allocated_bytes += rec.length; }
        if rec.is_deallocating() { self.stats.deallocated_bytes += rec.length; }
        if rec.mode == FallocateV2Mode::PunchHole { self.stats.punch_holes += 1; }
    }

    pub fn net_allocation(&self) -> i64 {
        self.stats.allocated_bytes as i64 - self.stats.deallocated_bytes as i64
    }
}
