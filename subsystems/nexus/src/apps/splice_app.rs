// SPDX-License-Identifier: GPL-2.0
//! Apps splice_app — splice/tee/vmsplice zero-copy I/O application layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Splice operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceOpType {
    Splice,
    Tee,
    Vmsplice,
}

/// Splice flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceFlag {
    Move,
    NonBlock,
    More,
    Gift,
}

/// Pipe buffer state
#[derive(Debug)]
#[repr(align(64))]
pub struct PipeBufferState {
    pub pipe_fd: u64,
    pub capacity: u64,
    pub used: u64,
    pub pages: u32,
    pub max_pages: u32,
}

impl PipeBufferState {
    pub fn new(fd: u64, cap: u64, max_pg: u32) -> Self {
        Self { pipe_fd: fd, capacity: cap, used: 0, pages: 0, max_pages: max_pg }
    }
}

/// Splice transfer record
#[derive(Debug)]
pub struct SpliceTransfer {
    pub op_type: SpliceOpType,
    pub fd_in: u64,
    pub fd_out: u64,
    pub offset_in: i64,
    pub offset_out: i64,
    pub requested: u64,
    pub transferred: u64,
    pub flags: u32,
    pub timestamp: u64,
    pub zero_copy: bool,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SpliceAppStats {
    pub splice_count: u64,
    pub tee_count: u64,
    pub vmsplice_count: u64,
    pub total_bytes: u64,
    pub zero_copy_bytes: u64,
    pub pipe_buffers_tracked: u32,
}

/// Main app splice
pub struct AppSplice {
    transfers: Vec<SpliceTransfer>,
    pipe_buffers: BTreeMap<u64, PipeBufferState>,
    splice_count: u64,
    tee_count: u64,
    vmsplice_count: u64,
    total_bytes: u64,
    zero_copy_bytes: u64,
}

impl AppSplice {
    pub fn new() -> Self {
        Self { transfers: Vec::new(), pipe_buffers: BTreeMap::new(), splice_count: 0, tee_count: 0, vmsplice_count: 0, total_bytes: 0, zero_copy_bytes: 0 }
    }

    #[inline(always)]
    pub fn track_pipe(&mut self, fd: u64, capacity: u64, max_pg: u32) {
        self.pipe_buffers.insert(fd, PipeBufferState::new(fd, capacity, max_pg));
    }

    #[inline]
    pub fn record(&mut self, xfer: SpliceTransfer) {
        match xfer.op_type {
            SpliceOpType::Splice => self.splice_count += 1,
            SpliceOpType::Tee => self.tee_count += 1,
            SpliceOpType::Vmsplice => self.vmsplice_count += 1,
        }
        self.total_bytes += xfer.transferred;
        if xfer.zero_copy { self.zero_copy_bytes += xfer.transferred; }
        self.transfers.push(xfer);
    }

    #[inline(always)]
    pub fn untrack_pipe(&mut self, fd: u64) { self.pipe_buffers.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> SpliceAppStats {
        SpliceAppStats {
            splice_count: self.splice_count, tee_count: self.tee_count, vmsplice_count: self.vmsplice_count,
            total_bytes: self.total_bytes, zero_copy_bytes: self.zero_copy_bytes,
            pipe_buffers_tracked: self.pipe_buffers.len() as u32,
        }
    }
}
