// SPDX-License-Identifier: GPL-2.0
//! Apps truncate_app — file truncation/extension.

extern crate alloc;

use alloc::vec::Vec;

/// Truncate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncateType {
    Truncate,
    Ftruncate,
}

/// Truncate operation
#[derive(Debug)]
pub struct TruncateOp {
    pub id: u64,
    pub target: TruncateType,
    pub fd_or_path_hash: u64,
    pub old_size: u64,
    pub new_size: u64,
    pub pid: u64,
    pub timestamp: u64,
    pub success: bool,
}

impl TruncateOp {
    pub fn new(id: u64, tt: TruncateType, target: u64, old: u64, new: u64, pid: u64, now: u64) -> Self {
        Self { id, target: tt, fd_or_path_hash: target, old_size: old, new_size: new, pid, timestamp: now, success: false }
    }

    pub fn size_delta(&self) -> i64 { self.new_size as i64 - self.old_size as i64 }
    pub fn is_shrink(&self) -> bool { self.new_size < self.old_size }
    pub fn is_extend(&self) -> bool { self.new_size > self.old_size }
}

/// Stats
#[derive(Debug, Clone)]
pub struct TruncateAppStats {
    pub total_ops: u32,
    pub successful: u32,
    pub failed: u32,
    pub shrinks: u32,
    pub extends: u32,
    pub total_bytes_freed: u64,
    pub total_bytes_allocated: u64,
}

/// Main truncate app
pub struct AppTruncate {
    ops: Vec<TruncateOp>,
    next_id: u64,
}

impl AppTruncate {
    pub fn new() -> Self { Self { ops: Vec::new(), next_id: 1 } }

    pub fn truncate(&mut self, tt: TruncateType, target: u64, old: u64, new: u64, pid: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.ops.push(TruncateOp::new(id, tt, target, old, new, pid, now));
        id
    }

    pub fn complete(&mut self, id: u64) {
        if let Some(op) = self.ops.iter_mut().find(|o| o.id == id) { op.success = true; }
    }

    pub fn stats(&self) -> TruncateAppStats {
        let ok = self.ops.iter().filter(|o| o.success).count() as u32;
        let fail = self.ops.len() as u32 - ok;
        let shrinks = self.ops.iter().filter(|o| o.success && o.is_shrink()).count() as u32;
        let extends = self.ops.iter().filter(|o| o.success && o.is_extend()).count() as u32;
        let freed: u64 = self.ops.iter().filter(|o| o.success && o.is_shrink()).map(|o| o.old_size - o.new_size).sum();
        let alloc: u64 = self.ops.iter().filter(|o| o.success && o.is_extend()).map(|o| o.new_size - o.old_size).sum();
        TruncateAppStats { total_ops: self.ops.len() as u32, successful: ok, failed: fail, shrinks, extends, total_bytes_freed: freed, total_bytes_allocated: alloc }
    }
}

// ============================================================================
// Merged from truncate_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncateV2Op {
    Truncate,
    Ftruncate,
    FallocKeepSize,
    FallocPunchHole,
    FallocCollapseRange,
    FallocZeroRange,
    FallocInsertRange,
    FallocUnshareRange,
}

/// Operation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncateV2Result {
    Success,
    Permission,
    IsDirectory,
    TextBusy,
    InvalidArg,
    NoSpace,
    NotSupported,
    BadFd,
}

/// A truncate/fallocate operation record.
#[derive(Debug, Clone)]
pub struct TruncateV2Record {
    pub record_id: u64,
    pub pid: u64,
    pub fd: i32,
    pub op: TruncateV2Op,
    pub offset: u64,
    pub length: u64,
    pub old_size: u64,
    pub new_size: u64,
    pub result: TruncateV2Result,
    pub timestamp: u64,
}

impl TruncateV2Record {
    pub fn new(record_id: u64, pid: u64, fd: i32, op: TruncateV2Op) -> Self {
        Self {
            record_id,
            pid,
            fd,
            op,
            offset: 0,
            length: 0,
            old_size: 0,
            new_size: 0,
            result: TruncateV2Result::Success,
            timestamp: 0,
        }
    }

    pub fn size_delta(&self) -> i64 {
        self.new_size as i64 - self.old_size as i64
    }
}

/// Per-file truncate tracking.
#[derive(Debug, Clone)]
pub struct FileTruncateV2State {
    pub inode: u64,
    pub current_size: u64,
    pub allocated_blocks: u64,
    pub hole_bytes: u64,
    pub truncate_count: u64,
    pub fallocate_count: u64,
    pub punch_hole_count: u64,
}

impl FileTruncateV2State {
    pub fn new(inode: u64, current_size: u64) -> Self {
        Self {
            inode,
            current_size,
            allocated_blocks: 0,
            hole_bytes: 0,
            truncate_count: 0,
            fallocate_count: 0,
            punch_hole_count: 0,
        }
    }

    pub fn apply_truncate(&mut self, new_size: u64) {
        self.current_size = new_size;
        self.truncate_count += 1;
    }

    pub fn apply_punch_hole(&mut self, offset: u64, length: u64) {
        let end = offset + length;
        if end <= self.current_size {
            self.hole_bytes += length;
        }
        self.punch_hole_count += 1;
    }

    pub fn sparseness_ratio(&self) -> f64 {
        if self.current_size == 0 {
            return 0.0;
        }
        self.hole_bytes as f64 / self.current_size as f64
    }
}

/// Statistics for truncate V2 app.
#[derive(Debug, Clone)]
pub struct TruncateV2AppStats {
    pub total_truncates: u64,
    pub total_fallocates: u64,
    pub total_punch_holes: u64,
    pub total_zero_ranges: u64,
    pub total_collapse_ranges: u64,
    pub total_insert_ranges: u64,
    pub bytes_reclaimed: u64,
    pub bytes_allocated: u64,
}

/// Main apps truncate V2 manager.
pub struct AppTruncateV2 {
    pub files: BTreeMap<u64, FileTruncateV2State>,
    pub recent_records: Vec<TruncateV2Record>,
    pub next_record_id: u64,
    pub stats: TruncateV2AppStats,
}

impl AppTruncateV2 {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            recent_records: Vec::new(),
            next_record_id: 1,
            stats: TruncateV2AppStats {
                total_truncates: 0,
                total_fallocates: 0,
                total_punch_holes: 0,
                total_zero_ranges: 0,
                total_collapse_ranges: 0,
                total_insert_ranges: 0,
                bytes_reclaimed: 0,
                bytes_allocated: 0,
            },
        }
    }

    pub fn record_op(
        &mut self,
        pid: u64,
        fd: i32,
        inode: u64,
        op: TruncateV2Op,
        offset: u64,
        length: u64,
    ) -> u64 {
        let id = self.next_record_id;
        self.next_record_id += 1;
        let mut rec = TruncateV2Record::new(id, pid, fd, op);
        rec.offset = offset;
        rec.length = length;
        let state = self.files.entry(inode).or_insert_with(|| FileTruncateV2State::new(inode, 0));
        match op {
            TruncateV2Op::Truncate | TruncateV2Op::Ftruncate => {
                rec.old_size = state.current_size;
                state.apply_truncate(length);
                rec.new_size = length;
                self.stats.total_truncates += 1;
            }
            TruncateV2Op::FallocPunchHole => {
                state.apply_punch_hole(offset, length);
                self.stats.total_punch_holes += 1;
                self.stats.bytes_reclaimed += length;
            }
            TruncateV2Op::FallocZeroRange => {
                self.stats.total_zero_ranges += 1;
            }
            TruncateV2Op::FallocCollapseRange => {
                self.stats.total_collapse_ranges += 1;
            }
            TruncateV2Op::FallocInsertRange => {
                self.stats.total_insert_ranges += 1;
            }
            _ => {
                self.stats.total_fallocates += 1;
                self.stats.bytes_allocated += length;
            }
        }
        self.recent_records.push(rec);
        id
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

// ============================================================================
// Merged from truncate_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncateV3Mode {
    Truncate,
    Ftruncate,
    FallocPunchHole,
    FallocCollapseRange,
    FallocZeroRange,
    CowBreak,
}

/// Truncate v3 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncateV3Result {
    Success,
    PermissionDenied,
    IsDirectory,
    TextBusy,
    FileTooLarge,
    ReadOnlyFs,
    CowFailed,
    Error,
}

/// Block state after truncate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncBlockState {
    Freed,
    Zeroed,
    CowBroken,
    Hole,
    Unshared,
}

/// Truncate v3 operation record
#[derive(Debug, Clone)]
pub struct TruncateV3Record {
    pub path_hash: u64,
    pub fd: i32,
    pub mode: TruncateV3Mode,
    pub result: TruncateV3Result,
    pub old_size: u64,
    pub new_size: u64,
    pub blocks_freed: u64,
    pub blocks_allocated: u64,
    pub cow_blocks_broken: u64,
    pub holes_created: u64,
    pub duration_ns: u64,
}

impl TruncateV3Record {
    pub fn new(path: &[u8], mode: TruncateV3Mode, old_size: u64, new_size: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            path_hash: h,
            fd: -1,
            mode,
            result: TruncateV3Result::Success,
            old_size,
            new_size,
            blocks_freed: if new_size < old_size { (old_size - new_size) / 4096 } else { 0 },
            blocks_allocated: if new_size > old_size { (new_size - old_size) / 4096 } else { 0 },
            cow_blocks_broken: 0,
            holes_created: 0,
            duration_ns: 0,
        }
    }

    pub fn is_shrink(&self) -> bool { self.new_size < self.old_size }
    pub fn is_grow(&self) -> bool { self.new_size > self.old_size }
    pub fn is_zero_range(&self) -> bool { self.mode == TruncateV3Mode::FallocZeroRange }
    pub fn size_delta(&self) -> i64 { self.new_size as i64 - self.old_size as i64 }
    pub fn net_blocks(&self) -> i64 { self.blocks_allocated as i64 - self.blocks_freed as i64 }
}

/// Truncate v3 app stats
#[derive(Debug, Clone)]
pub struct TruncateV3AppStats {
    pub total_ops: u64,
    pub shrink_ops: u64,
    pub grow_ops: u64,
    pub zero_ops: u64,
    pub cow_breaks: u64,
    pub total_blocks_freed: u64,
    pub total_blocks_allocated: u64,
    pub total_holes: u64,
    pub failures: u64,
}

/// Main app truncate v3
#[derive(Debug)]
pub struct AppTruncateV3 {
    pub stats: TruncateV3AppStats,
}

impl AppTruncateV3 {
    pub fn new() -> Self {
        Self {
            stats: TruncateV3AppStats {
                total_ops: 0,
                shrink_ops: 0,
                grow_ops: 0,
                zero_ops: 0,
                cow_breaks: 0,
                total_blocks_freed: 0,
                total_blocks_allocated: 0,
                total_holes: 0,
                failures: 0,
            },
        }
    }

    pub fn record(&mut self, record: &TruncateV3Record) {
        self.stats.total_ops += 1;
        match record.result {
            TruncateV3Result::Success => {
                if record.new_size == 0 { self.stats.zero_ops += 1; }
                else if record.is_shrink() { self.stats.shrink_ops += 1; }
                else if record.is_grow() { self.stats.grow_ops += 1; }
                if record.cow_blocks_broken > 0 { self.stats.cow_breaks += 1; }
                self.stats.total_blocks_freed += record.blocks_freed;
                self.stats.total_blocks_allocated += record.blocks_allocated;
                self.stats.total_holes += record.holes_created;
            }
            _ => self.stats.failures += 1,
        }
    }

    pub fn net_block_change(&self) -> i64 {
        self.stats.total_blocks_allocated as i64 - self.stats.total_blocks_freed as i64
    }
}
