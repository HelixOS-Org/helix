// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge sendfile — Zero-copy file-to-socket/file transfer
//!
//! Bridges the sendfile(2) system call with splice-based fallback,
//! page cache interaction, and scatter-gather I/O batching.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Sendfile transfer mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendfileMode {
    Direct,
    Splice,
    CopyFallback,
    Mmap,
    Zerocopy,
}

/// Transfer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendfileState {
    Idle,
    Reading,
    Writing,
    Splicing,
    Completed,
    Error,
    Cancelled,
}

/// Source file descriptor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendfileSrcType {
    RegularFile,
    BlockDevice,
    PageCache,
    TmpFs,
    ProcFs,
}

/// A sendfile transfer descriptor.
#[derive(Debug, Clone)]
pub struct SendfileTransfer {
    pub transfer_id: u64,
    pub src_fd: i32,
    pub dst_fd: i32,
    pub offset: u64,
    pub count: u64,
    pub bytes_sent: u64,
    pub mode: SendfileMode,
    pub state: SendfileState,
    pub src_type: SendfileSrcType,
    pub page_cache_hits: u64,
    pub page_cache_misses: u64,
    pub splice_pages: u64,
    pub start_time: u64,
    pub end_time: u64,
}

impl SendfileTransfer {
    pub fn new(transfer_id: u64, src_fd: i32, dst_fd: i32, offset: u64, count: u64) -> Self {
        Self {
            transfer_id,
            src_fd,
            dst_fd,
            offset,
            count,
            bytes_sent: 0,
            mode: SendfileMode::Direct,
            state: SendfileState::Idle,
            src_type: SendfileSrcType::RegularFile,
            page_cache_hits: 0,
            page_cache_misses: 0,
            splice_pages: 0,
            start_time: 0,
            end_time: 0,
        }
    }

    #[inline]
    pub fn progress_percent(&self) -> f64 {
        if self.count == 0 {
            return 100.0;
        }
        (self.bytes_sent as f64 / self.count as f64) * 100.0
    }

    #[inline]
    pub fn throughput_bps(&self) -> u64 {
        let elapsed = if self.end_time > self.start_time {
            self.end_time - self.start_time
        } else {
            1
        };
        (self.bytes_sent * 1_000_000) / elapsed
    }

    #[inline]
    pub fn advance(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.state = if self.bytes_sent >= self.count {
            SendfileState::Completed
        } else {
            SendfileState::Writing
        };
    }
}

/// Pipe buffer for splice operations.
#[derive(Debug, Clone)]
pub struct SendfilePipeBuf {
    pub pipe_id: u64,
    pub capacity: usize,
    pub used: usize,
    pub pages: u32,
    pub flags: u32,
}

impl SendfilePipeBuf {
    pub fn new(pipe_id: u64, capacity: usize) -> Self {
        Self {
            pipe_id,
            capacity,
            used: 0,
            pages: 0,
            flags: 0,
        }
    }

    #[inline(always)]
    pub fn available(&self) -> usize {
        self.capacity - self.used
    }
}

/// Statistics for the sendfile bridge.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SendfileBridgeStats {
    pub total_transfers: u64,
    pub completed_transfers: u64,
    pub failed_transfers: u64,
    pub total_bytes_sent: u64,
    pub zerocopy_transfers: u64,
    pub splice_transfers: u64,
    pub copy_fallback_count: u64,
    pub page_cache_hit_rate: f64,
}

/// Main bridge sendfile manager.
#[repr(align(64))]
pub struct BridgeSendfile {
    pub transfers: BTreeMap<u64, SendfileTransfer>,
    pub pipe_bufs: BTreeMap<u64, SendfilePipeBuf>,
    pub next_transfer_id: u64,
    pub next_pipe_id: u64,
    pub stats: SendfileBridgeStats,
}

impl BridgeSendfile {
    pub fn new() -> Self {
        Self {
            transfers: BTreeMap::new(),
            pipe_bufs: BTreeMap::new(),
            next_transfer_id: 1,
            next_pipe_id: 1,
            stats: SendfileBridgeStats {
                total_transfers: 0,
                completed_transfers: 0,
                failed_transfers: 0,
                total_bytes_sent: 0,
                zerocopy_transfers: 0,
                splice_transfers: 0,
                copy_fallback_count: 0,
                page_cache_hit_rate: 0.0,
            },
        }
    }

    pub fn start_transfer(
        &mut self,
        src_fd: i32,
        dst_fd: i32,
        offset: u64,
        count: u64,
        mode: SendfileMode,
    ) -> u64 {
        let id = self.next_transfer_id;
        self.next_transfer_id += 1;
        let mut xfer = SendfileTransfer::new(id, src_fd, dst_fd, offset, count);
        xfer.mode = mode;
        xfer.state = SendfileState::Reading;
        self.transfers.insert(id, xfer);
        self.stats.total_transfers += 1;
        match mode {
            SendfileMode::Zerocopy => self.stats.zerocopy_transfers += 1,
            SendfileMode::Splice => self.stats.splice_transfers += 1,
            SendfileMode::CopyFallback => self.stats.copy_fallback_count += 1,
            _ => {}
        }
        id
    }

    pub fn advance_transfer(&mut self, transfer_id: u64, bytes: u64) -> bool {
        if let Some(xfer) = self.transfers.get_mut(&transfer_id) {
            xfer.advance(bytes);
            self.stats.total_bytes_sent += bytes;
            if xfer.state == SendfileState::Completed {
                self.stats.completed_transfers += 1;
            }
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }
}

// ============================================================================
// Merged from sendfile_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendfileV2Mode {
    Kernel,
    ZeroCopy,
    SpliceBackend,
    DmaCopy,
    Fallback,
}

/// Sendfile v2 state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendfileV2State {
    Idle,
    Sending,
    Blocked,
    Completed,
    Error,
}

/// Sendfile operation
#[derive(Debug, Clone)]
pub struct SendfileV2Op {
    pub op_id: u64,
    pub in_fd: i32,
    pub out_fd: i32,
    pub offset: u64,
    pub count: u64,
    pub bytes_sent: u64,
    pub mode: SendfileV2Mode,
    pub state: SendfileV2State,
    pub start_ns: u64,
    pub end_ns: u64,
    pub page_cache_hits: u64,
    pub page_cache_misses: u64,
}

impl SendfileV2Op {
    pub fn new(op_id: u64, in_fd: i32, out_fd: i32, offset: u64, count: u64) -> Self {
        Self {
            op_id,
            in_fd,
            out_fd,
            offset,
            count,
            bytes_sent: 0,
            mode: SendfileV2Mode::Kernel,
            state: SendfileV2State::Idle,
            start_ns: 0,
            end_ns: 0,
            page_cache_hits: 0,
            page_cache_misses: 0,
        }
    }

    #[inline(always)]
    pub fn start(&mut self, ts_ns: u64) {
        self.state = SendfileV2State::Sending;
        self.start_ns = ts_ns;
    }

    #[inline]
    pub fn progress(&mut self, bytes: u64, cache_hit: bool) {
        self.bytes_sent += bytes;
        if cache_hit {
            self.page_cache_hits += 1;
        } else {
            self.page_cache_misses += 1;
        }
    }

    #[inline(always)]
    pub fn complete(&mut self, ts_ns: u64) {
        self.state = SendfileV2State::Completed;
        self.end_ns = ts_ns;
    }

    #[inline(always)]
    pub fn throughput_bps(&self) -> u64 {
        let dur = self.end_ns.saturating_sub(self.start_ns);
        if dur == 0 { 0 } else { (self.bytes_sent * 8 * 1_000_000_000) / dur }
    }

    #[inline(always)]
    pub fn completion_pct(&self) -> f64 {
        if self.count == 0 { 0.0 } else { (self.bytes_sent as f64 / self.count as f64) * 100.0 }
    }

    #[inline(always)]
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.page_cache_hits + self.page_cache_misses;
        if total == 0 { 0.0 } else { self.page_cache_hits as f64 / total as f64 }
    }
}

/// Sendfile v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SendfileV2BridgeStats {
    pub total_operations: u64,
    pub total_bytes_sent: u64,
    pub zero_copy_ops: u64,
    pub fallback_ops: u64,
    pub total_duration_ns: u64,
}

/// Main bridge sendfile v2
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeSendfileV2 {
    pub active_ops: BTreeMap<u64, SendfileV2Op>,
    pub stats: SendfileV2BridgeStats,
    pub next_op_id: u64,
}

impl BridgeSendfileV2 {
    pub fn new() -> Self {
        Self {
            active_ops: BTreeMap::new(),
            stats: SendfileV2BridgeStats {
                total_operations: 0,
                total_bytes_sent: 0,
                zero_copy_ops: 0,
                fallback_ops: 0,
                total_duration_ns: 0,
            },
            next_op_id: 1,
        }
    }

    #[inline]
    pub fn start_sendfile(&mut self, in_fd: i32, out_fd: i32, offset: u64, count: u64, ts_ns: u64) -> u64 {
        let id = self.next_op_id;
        self.next_op_id += 1;
        let mut op = SendfileV2Op::new(id, in_fd, out_fd, offset, count);
        op.start(ts_ns);
        self.active_ops.insert(id, op);
        self.stats.total_operations += 1;
        id
    }

    pub fn complete_sendfile(&mut self, op_id: u64, ts_ns: u64) -> Option<u64> {
        if let Some(op) = self.active_ops.get_mut(&op_id) {
            op.complete(ts_ns);
            let bytes = op.bytes_sent;
            self.stats.total_bytes_sent += bytes;
            self.stats.total_duration_ns += op.end_ns.saturating_sub(op.start_ns);
            match op.mode {
                SendfileV2Mode::ZeroCopy | SendfileV2Mode::DmaCopy => self.stats.zero_copy_ops += 1,
                SendfileV2Mode::Fallback => self.stats.fallback_ops += 1,
                _ => {}
            }
            Some(bytes)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn avg_throughput_bps(&self) -> u64 {
        if self.stats.total_duration_ns == 0 { 0 }
        else { (self.stats.total_bytes_sent * 8 * 1_000_000_000) / self.stats.total_duration_ns }
    }
}
