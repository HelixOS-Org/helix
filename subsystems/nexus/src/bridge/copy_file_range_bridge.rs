// SPDX-License-Identifier: GPL-2.0
//! Bridge copy_file_range — efficient server-side file copy between fds

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Copy file range mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyRangeMode {
    ServerSide,
    Reflink,
    CoW,
    FallbackCopy,
    CrossDevice,
}

/// Copy file range state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyRangeState {
    Pending,
    InProgress,
    Completed,
    ShortCopy,
    Error,
}

/// Copy range operation
#[derive(Debug, Clone)]
pub struct CopyRangeOp {
    pub op_id: u64,
    pub src_fd: i32,
    pub dst_fd: i32,
    pub src_offset: u64,
    pub dst_offset: u64,
    pub requested_len: u64,
    pub copied_len: u64,
    pub mode: CopyRangeMode,
    pub state: CopyRangeState,
    pub start_ns: u64,
    pub end_ns: u64,
    pub reflink_blocks: u64,
    pub cow_triggers: u64,
}

impl CopyRangeOp {
    pub fn new(op_id: u64, src_fd: i32, dst_fd: i32, src_off: u64, dst_off: u64, len: u64) -> Self {
        Self {
            op_id,
            src_fd,
            dst_fd,
            src_offset: src_off,
            dst_offset: dst_off,
            requested_len: len,
            copied_len: 0,
            mode: CopyRangeMode::ServerSide,
            state: CopyRangeState::Pending,
            start_ns: 0,
            end_ns: 0,
            reflink_blocks: 0,
            cow_triggers: 0,
        }
    }

    pub fn start(&mut self, ts_ns: u64, mode: CopyRangeMode) {
        self.state = CopyRangeState::InProgress;
        self.mode = mode;
        self.start_ns = ts_ns;
    }

    pub fn progress(&mut self, bytes: u64) {
        self.copied_len += bytes;
    }

    pub fn complete(&mut self, ts_ns: u64) {
        self.end_ns = ts_ns;
        self.state = if self.copied_len >= self.requested_len {
            CopyRangeState::Completed
        } else {
            CopyRangeState::ShortCopy
        };
    }

    pub fn throughput_bps(&self) -> u64 {
        let dur = self.end_ns.saturating_sub(self.start_ns);
        if dur == 0 { 0 } else { (self.copied_len * 8 * 1_000_000_000) / dur }
    }

    pub fn completion_pct(&self) -> f64 {
        if self.requested_len == 0 { 0.0 } else { (self.copied_len as f64 / self.requested_len as f64) * 100.0 }
    }
}

/// Copy file range bridge stats
#[derive(Debug, Clone)]
pub struct CopyRangeBridgeStats {
    pub total_ops: u64,
    pub total_bytes_copied: u64,
    pub reflink_ops: u64,
    pub cow_ops: u64,
    pub fallback_ops: u64,
    pub short_copies: u64,
}

/// Main bridge copy_file_range
#[derive(Debug)]
pub struct BridgeCopyFileRange {
    pub active_ops: BTreeMap<u64, CopyRangeOp>,
    pub stats: CopyRangeBridgeStats,
    pub next_op_id: u64,
}

impl BridgeCopyFileRange {
    pub fn new() -> Self {
        Self {
            active_ops: BTreeMap::new(),
            stats: CopyRangeBridgeStats {
                total_ops: 0,
                total_bytes_copied: 0,
                reflink_ops: 0,
                cow_ops: 0,
                fallback_ops: 0,
                short_copies: 0,
            },
            next_op_id: 1,
        }
    }

    pub fn start_copy(&mut self, src_fd: i32, dst_fd: i32, src_off: u64, dst_off: u64, len: u64, ts_ns: u64, mode: CopyRangeMode) -> u64 {
        let id = self.next_op_id;
        self.next_op_id += 1;
        let mut op = CopyRangeOp::new(id, src_fd, dst_fd, src_off, dst_off, len);
        op.start(ts_ns, mode);
        self.active_ops.insert(id, op);
        self.stats.total_ops += 1;
        match mode {
            CopyRangeMode::Reflink => self.stats.reflink_ops += 1,
            CopyRangeMode::CoW => self.stats.cow_ops += 1,
            CopyRangeMode::FallbackCopy => self.stats.fallback_ops += 1,
            _ => {}
        }
        id
    }

    pub fn complete_copy(&mut self, op_id: u64, ts_ns: u64) -> Option<u64> {
        if let Some(op) = self.active_ops.get_mut(&op_id) {
            op.complete(ts_ns);
            let copied = op.copied_len;
            self.stats.total_bytes_copied += copied;
            if op.state == CopyRangeState::ShortCopy {
                self.stats.short_copies += 1;
            }
            Some(copied)
        } else {
            None
        }
    }

    pub fn reflink_rate(&self) -> f64 {
        if self.stats.total_ops == 0 { 0.0 } else { self.stats.reflink_ops as f64 / self.stats.total_ops as f64 }
    }
}
