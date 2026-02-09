// SPDX-License-Identifier: GPL-2.0
//! Apps preadv_app — preadv/pwritev vectored I/O application layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// I/O vector
#[derive(Debug)]
pub struct IoVec {
    pub base: u64,
    pub len: u64,
}

/// Vectored I/O direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectoredIoDir {
    Read,
    Write,
}

/// Vectored I/O flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectoredIoFlag {
    None,
    Append,
    DSyncWrite,
    SyncWrite,
    HighPriority,
    NoWait,
}

/// Vectored I/O operation
#[derive(Debug)]
pub struct VectoredIoOp {
    pub fd: u64,
    pub direction: VectoredIoDir,
    pub offset: i64,
    pub iov_count: u32,
    pub total_bytes: u64,
    pub flags: VectoredIoFlag,
    pub started_at: u64,
    pub completed_at: u64,
    pub bytes_transferred: u64,
}

/// Per-FD vectored I/O tracker
#[derive(Debug)]
pub struct FdVectoredTracker {
    pub fd: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub max_iov_count: u32,
    pub avg_iov_size: f64,
    pub total_iovecs: u64,
}

impl FdVectoredTracker {
    pub fn new(fd: u64) -> Self {
        Self { fd, read_ops: 0, write_ops: 0, read_bytes: 0, write_bytes: 0, max_iov_count: 0, avg_iov_size: 0.0, total_iovecs: 0 }
    }

    #[inline]
    pub fn record(&mut self, op: &VectoredIoOp) {
        match op.direction {
            VectoredIoDir::Read => { self.read_ops += 1; self.read_bytes += op.bytes_transferred; }
            VectoredIoDir::Write => { self.write_ops += 1; self.write_bytes += op.bytes_transferred; }
        }
        if op.iov_count > self.max_iov_count { self.max_iov_count = op.iov_count; }
        self.total_iovecs += op.iov_count as u64;
        let total_ops = self.read_ops + self.write_ops;
        if total_ops > 0 { self.avg_iov_size = self.total_iovecs as f64 / total_ops as f64; }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PreadvAppStats {
    pub tracked_fds: u32,
    pub total_read_ops: u64,
    pub total_write_ops: u64,
    pub total_bytes: u64,
    pub max_iov_count: u32,
}

/// Main app preadv
pub struct AppPreadv {
    trackers: BTreeMap<u64, FdVectoredTracker>,
    ops: Vec<VectoredIoOp>,
}

impl AppPreadv {
    pub fn new() -> Self { Self { trackers: BTreeMap::new(), ops: Vec::new() } }

    #[inline(always)]
    pub fn track(&mut self, fd: u64) { self.trackers.insert(fd, FdVectoredTracker::new(fd)); }

    #[inline]
    pub fn record_op(&mut self, op: VectoredIoOp) {
        let fd = op.fd;
        if let Some(t) = self.trackers.get_mut(&fd) { t.record(&op); }
        self.ops.push(op);
    }

    #[inline(always)]
    pub fn untrack(&mut self, fd: u64) { self.trackers.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> PreadvAppStats {
        let reads: u64 = self.trackers.values().map(|t| t.read_ops).sum();
        let writes: u64 = self.trackers.values().map(|t| t.write_ops).sum();
        let bytes: u64 = self.trackers.values().map(|t| t.read_bytes + t.write_bytes).sum();
        let max_iov: u32 = self.trackers.values().map(|t| t.max_iov_count).max().unwrap_or(0);
        PreadvAppStats { tracked_fds: self.trackers.len() as u32, total_read_ops: reads, total_write_ops: writes, total_bytes: bytes, max_iov_count: max_iov }
    }
}
