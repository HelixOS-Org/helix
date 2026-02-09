// SPDX-License-Identifier: GPL-2.0
//! Bridge splice_bridge — zero-copy pipe/socket data transfer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Splice operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceOp {
    Splice,
    Tee,
    Vmsplice,
    SendFile,
    CopyFileRange,
}

/// Splice flags
#[derive(Debug, Clone, Copy)]
pub struct SpliceFlags {
    pub bits: u32,
}

impl SpliceFlags {
    pub const MOVE: u32 = 1;
    pub const NONBLOCK: u32 = 2;
    pub const MORE: u32 = 4;
    pub const GIFT: u32 = 8;

    pub fn new(bits: u32) -> Self { Self { bits } }
    #[inline(always)]
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    #[inline(always)]
    pub fn is_nonblock(&self) -> bool { self.has(Self::NONBLOCK) }
}

/// Splice endpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    Pipe,
    Socket,
    RegularFile,
    BlockDevice,
    CharDevice,
}

/// Pipe buffer page reference
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PipeBuffer {
    pub page_id: u64,
    pub offset: u32,
    pub len: u32,
    pub flags: u32,
    pub ref_count: u32,
}

impl PipeBuffer {
    pub fn new(page_id: u64, offset: u32, len: u32) -> Self {
        Self { page_id, offset, len, flags: 0, ref_count: 1 }
    }

    #[inline(always)]
    pub fn is_zero_copy(&self) -> bool { self.ref_count > 1 }
}

/// Splice transfer record
#[derive(Debug, Clone)]
pub struct SpliceTransfer {
    pub id: u64,
    pub op: SpliceOp,
    pub flags: SpliceFlags,
    pub src_type: EndpointType,
    pub dst_type: EndpointType,
    pub src_fd: i32,
    pub dst_fd: i32,
    pub requested_bytes: u64,
    pub transferred_bytes: u64,
    pub pages_moved: u32,
    pub zero_copy: bool,
    pub started_at: u64,
    pub completed_at: u64,
}

impl SpliceTransfer {
    pub fn new(id: u64, op: SpliceOp, src_fd: i32, dst_fd: i32, bytes: u64, now: u64) -> Self {
        Self {
            id, op, flags: SpliceFlags::new(0),
            src_type: EndpointType::Pipe, dst_type: EndpointType::Pipe,
            src_fd, dst_fd, requested_bytes: bytes, transferred_bytes: 0,
            pages_moved: 0, zero_copy: false, started_at: now, completed_at: 0,
        }
    }

    #[inline]
    pub fn complete(&mut self, transferred: u64, pages: u32, now: u64) {
        self.transferred_bytes = transferred;
        self.pages_moved = pages;
        self.completed_at = now;
    }

    #[inline]
    pub fn throughput_mbps(&self) -> f64 {
        let lat = self.completed_at.saturating_sub(self.started_at);
        if lat == 0 { return 0.0; }
        (self.transferred_bytes as f64 * 1_000_000_000.0) / (lat as f64 * 1024.0 * 1024.0)
    }

    #[inline(always)]
    pub fn efficiency(&self) -> f64 {
        if self.requested_bytes == 0 { return 0.0; }
        self.transferred_bytes as f64 / self.requested_bytes as f64
    }
}

/// Bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SpliceBridgeStats {
    pub total_operations: u64,
    pub total_bytes: u64,
    pub total_pages_moved: u64,
    pub zero_copy_ops: u64,
    pub ops_by_type: BTreeMap<u8, u64>,
    pub avg_throughput_mbps: f64,
}

/// Main splice bridge
#[repr(align(64))]
pub struct BridgeSplice {
    history: Vec<SpliceTransfer>,
    active: BTreeMap<u64, SpliceTransfer>,
    next_id: u64,
    max_history: usize,
}

impl BridgeSplice {
    pub fn new() -> Self {
        Self { history: Vec::new(), active: BTreeMap::new(), next_id: 1, max_history: 4096 }
    }

    #[inline]
    pub fn begin_splice(&mut self, op: SpliceOp, src_fd: i32, dst_fd: i32, bytes: u64, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.active.insert(id, SpliceTransfer::new(id, op, src_fd, dst_fd, bytes, now));
        id
    }

    #[inline]
    pub fn complete_splice(&mut self, id: u64, transferred: u64, pages: u32, now: u64) -> bool {
        if let Some(mut transfer) = self.active.remove(&id) {
            transfer.complete(transferred, pages, now);
            if self.history.len() >= self.max_history { self.history.drain(..self.max_history / 4); }
            self.history.push(transfer);
            true
        } else { false }
    }

    pub fn stats(&self) -> SpliceBridgeStats {
        let total_bytes: u64 = self.history.iter().map(|t| t.transferred_bytes).sum();
        let total_pages: u64 = self.history.iter().map(|t| t.pages_moved as u64).sum();
        let zc = self.history.iter().filter(|t| t.zero_copy).count() as u64;
        let mut by_type = BTreeMap::new();
        for t in &self.history { *by_type.entry(t.op as u8).or_insert(0u64) += 1; }
        let tps: Vec<f64> = self.history.iter().filter(|t| t.completed_at > 0).map(|t| t.throughput_mbps()).collect();
        let avg_tp = if tps.is_empty() { 0.0 } else { tps.iter().sum::<f64>() / tps.len() as f64 };
        SpliceBridgeStats {
            total_operations: self.history.len() as u64, total_bytes,
            total_pages_moved: total_pages, zero_copy_ops: zc,
            ops_by_type: by_type, avg_throughput_mbps: avg_tp,
        }
    }
}

// ============================================================================
// Merged from splice_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct SpliceV2Flags(pub u32);

impl SpliceV2Flags {
    pub const MOVE: u32 = 1;
    pub const NONBLOCK: u32 = 2;
    pub const MORE: u32 = 4;
    pub const GIFT: u32 = 8;
    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Splice v2 operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV2Op {
    Splice,
    Tee,
    Vmsplice,
    SendFile,
    CopyFileRange,
}

/// Pipe v2 buffer
#[derive(Debug)]
#[repr(align(64))]
pub struct PipeV2Buffer {
    pub id: u64,
    pub capacity: usize,
    pub used: usize,
    pub pages: u32,
    pub readers: u32,
    pub writers: u32,
}

impl PipeV2Buffer {
    pub fn new(id: u64, capacity: usize) -> Self { Self { id, capacity, used: 0, pages: (capacity / 4096) as u32, readers: 0, writers: 0 } }
    #[inline(always)]
    pub fn available(&self) -> usize { self.capacity - self.used }
    #[inline(always)]
    pub fn utilization(&self) -> f64 { self.used as f64 / self.capacity as f64 }
}

/// Transfer record
#[derive(Debug, Clone)]
pub struct SpliceV2Transfer {
    pub op: SpliceV2Op,
    pub src_fd: i32,
    pub dst_fd: i32,
    pub bytes_moved: u64,
    pub flags: SpliceV2Flags,
    pub duration_ns: u64,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SpliceV2BridgeStats {
    pub total_pipes: u32,
    pub total_transfers: u64,
    pub total_bytes: u64,
    pub splice_ops: u64,
    pub tee_ops: u64,
    pub sendfile_ops: u64,
    pub avg_throughput: f64,
}

/// Main splice v2 bridge
#[repr(align(64))]
pub struct BridgeSpliceV2 {
    pipes: BTreeMap<u64, PipeV2Buffer>,
    transfers: Vec<SpliceV2Transfer>,
    next_pipe_id: u64,
    max_transfers: usize,
}

impl BridgeSpliceV2 {
    pub fn new() -> Self { Self { pipes: BTreeMap::new(), transfers: Vec::new(), next_pipe_id: 1, max_transfers: 4096 } }

    #[inline]
    pub fn create_pipe(&mut self, capacity: usize) -> u64 {
        let id = self.next_pipe_id; self.next_pipe_id += 1;
        self.pipes.insert(id, PipeV2Buffer::new(id, capacity));
        id
    }

    #[inline(always)]
    pub fn destroy_pipe(&mut self, id: u64) { self.pipes.remove(&id); }

    #[inline(always)]
    pub fn record_transfer(&mut self, transfer: SpliceV2Transfer) {
        if self.transfers.len() >= self.max_transfers { self.transfers.drain(..self.max_transfers / 2); }
        self.transfers.push(transfer);
    }

    #[inline]
    pub fn stats(&self) -> SpliceV2BridgeStats {
        let bytes: u64 = self.transfers.iter().map(|t| t.bytes_moved).sum();
        let splice = self.transfers.iter().filter(|t| t.op == SpliceV2Op::Splice).count() as u64;
        let tee = self.transfers.iter().filter(|t| t.op == SpliceV2Op::Tee).count() as u64;
        let sf = self.transfers.iter().filter(|t| t.op == SpliceV2Op::SendFile).count() as u64;
        let dur_sum: u64 = self.transfers.iter().map(|t| t.duration_ns).sum();
        let avg_tp = if dur_sum == 0 { 0.0 } else { bytes as f64 / (dur_sum as f64 / 1_000_000_000.0) };
        SpliceV2BridgeStats { total_pipes: self.pipes.len() as u32, total_transfers: self.transfers.len() as u64, total_bytes: bytes, splice_ops: splice, tee_ops: tee, sendfile_ops: sf, avg_throughput: avg_tp }
    }
}

// ============================================================================
// Merged from splice_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV3Op {
    Splice,
    Tee,
    Vmsplice,
}

/// Splice v3 flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV3Flag {
    Move,
    NonBlock,
    More,
    Gift,
}

/// Pipe buffer info v3
#[derive(Debug)]
#[repr(align(64))]
pub struct PipeV3Buffer {
    pub pipe_id: u64,
    pub capacity: u32,
    pub used: u32,
    pub pages: u32,
}

/// Splice v3 transfer
#[derive(Debug)]
pub struct SpliceV3Transfer {
    pub op: SpliceV3Op,
    pub fd_in: u64,
    pub fd_out: u64,
    pub bytes_requested: u64,
    pub bytes_transferred: u64,
    pub flags: u32,
    pub timestamp: u64,
    pub zero_copy: bool,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SpliceV3BridgeStats {
    pub total_splices: u64,
    pub total_tees: u64,
    pub total_vmsplices: u64,
    pub total_bytes: u64,
    pub zero_copy_bytes: u64,
}

/// Main bridge splice v3
#[repr(align(64))]
pub struct BridgeSpliceV3 {
    pipe_buffers: BTreeMap<u64, PipeV3Buffer>,
    total_splices: u64,
    total_tees: u64,
    total_vmsplices: u64,
    total_bytes: u64,
    zero_copy_bytes: u64,
}

impl BridgeSpliceV3 {
    pub fn new() -> Self {
        Self { pipe_buffers: BTreeMap::new(), total_splices: 0, total_tees: 0, total_vmsplices: 0, total_bytes: 0, zero_copy_bytes: 0 }
    }

    #[inline(always)]
    pub fn register_pipe(&mut self, id: u64, cap: u32) {
        self.pipe_buffers.insert(id, PipeV3Buffer { pipe_id: id, capacity: cap, used: 0, pages: 0 });
    }

    #[inline]
    pub fn transfer(&mut self, xfer: &SpliceV3Transfer) {
        match xfer.op {
            SpliceV3Op::Splice => self.total_splices += 1,
            SpliceV3Op::Tee => self.total_tees += 1,
            SpliceV3Op::Vmsplice => self.total_vmsplices += 1,
        }
        self.total_bytes += xfer.bytes_transferred;
        if xfer.zero_copy { self.zero_copy_bytes += xfer.bytes_transferred; }
    }

    #[inline(always)]
    pub fn unregister_pipe(&mut self, id: u64) { self.pipe_buffers.remove(&id); }

    #[inline(always)]
    pub fn stats(&self) -> SpliceV3BridgeStats {
        SpliceV3BridgeStats { total_splices: self.total_splices, total_tees: self.total_tees, total_vmsplices: self.total_vmsplices, total_bytes: self.total_bytes, zero_copy_bytes: self.zero_copy_bytes }
    }
}

// ============================================================================
// Merged from splice_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV4Op {
    Splice,
    Tee,
    Vmsplice,
    FanOut,
    Gather,
}

/// Splice V4 flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV4Flag {
    Move,
    NonBlock,
    More,
    Gift,
    FdInPipe,
    FdOutPipe,
}

/// Pipe page reference.
#[derive(Debug, Clone)]
pub struct SpliceV4PageRef {
    pub page_pfn: u64,
    pub offset: u32,
    pub length: u32,
    pub ref_count: u32,
    pub is_gift: bool,
}

impl SpliceV4PageRef {
    pub fn new(page_pfn: u64, offset: u32, length: u32) -> Self {
        Self {
            page_pfn,
            offset,
            length,
            ref_count: 1,
            is_gift: false,
        }
    }

    #[inline(always)]
    pub fn add_ref(&mut self) {
        self.ref_count += 1;
    }

    #[inline(always)]
    pub fn drop_ref(&mut self) -> bool {
        self.ref_count = self.ref_count.saturating_sub(1);
        self.ref_count == 0
    }
}

/// A pipe descriptor for splice operations.
#[derive(Debug, Clone)]
pub struct SpliceV4Pipe {
    pub pipe_id: u64,
    pub max_pages: u32,
    pub used_pages: u32,
    pub readers: u32,
    pub writers: u32,
    pub pages: VecDeque<SpliceV4PageRef>,
    pub bytes_spliced: u64,
    pub bytes_teed: u64,
}

impl SpliceV4Pipe {
    pub fn new(pipe_id: u64, max_pages: u32) -> Self {
        Self {
            pipe_id,
            max_pages,
            used_pages: 0,
            readers: 1,
            writers: 1,
            pages: VecDeque::new(),
            bytes_spliced: 0,
            bytes_teed: 0,
        }
    }

    #[inline(always)]
    pub fn available_pages(&self) -> u32 {
        self.max_pages.saturating_sub(self.used_pages)
    }

    #[inline]
    pub fn push_page(&mut self, page: SpliceV4PageRef) -> bool {
        if self.used_pages >= self.max_pages {
            return false;
        }
        let bytes = page.length as u64;
        self.pages.push_back(page);
        self.used_pages += 1;
        self.bytes_spliced += bytes;
        true
    }

    #[inline]
    pub fn pop_page(&mut self) -> Option<SpliceV4PageRef> {
        if let Some(page) = self.pages.pop() {
            self.used_pages = self.used_pages.saturating_sub(1);
            Some(page)
        } else {
            None
        }
    }
}

/// A splice transfer operation.
#[derive(Debug, Clone)]
pub struct SpliceV4Transfer {
    pub transfer_id: u64,
    pub op: SpliceV4Op,
    pub src_fd: i32,
    pub dst_fds: Vec<i32>,
    pub offset_in: u64,
    pub offset_out: u64,
    pub length: u64,
    pub bytes_transferred: u64,
    pub flags: Vec<SpliceV4Flag>,
    pub pipe_id: Option<u64>,
}

impl SpliceV4Transfer {
    pub fn new(transfer_id: u64, op: SpliceV4Op, src_fd: i32, dst_fd: i32) -> Self {
        Self {
            transfer_id,
            op,
            src_fd,
            dst_fds: alloc::vec![dst_fd],
            offset_in: 0,
            offset_out: 0,
            length: 0,
            bytes_transferred: 0,
            flags: Vec::new(),
            pipe_id: None,
        }
    }

    #[inline(always)]
    pub fn add_fanout_dst(&mut self, fd: i32) {
        self.dst_fds.push(fd);
    }

    #[inline(always)]
    pub fn is_complete(&self) -> bool {
        self.length > 0 && self.bytes_transferred >= self.length
    }
}

/// Statistics for splice V4 bridge.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SpliceV4BridgeStats {
    pub total_splices: u64,
    pub total_tees: u64,
    pub total_vmsplices: u64,
    pub total_fanouts: u64,
    pub total_bytes: u64,
    pub pages_moved: u64,
    pub pages_copied: u64,
    pub pipe_count: u64,
}

/// Main bridge splice V4 manager.
#[repr(align(64))]
pub struct BridgeSpliceV4 {
    pub pipes: BTreeMap<u64, SpliceV4Pipe>,
    pub transfers: BTreeMap<u64, SpliceV4Transfer>,
    pub next_pipe_id: u64,
    pub next_transfer_id: u64,
    pub stats: SpliceV4BridgeStats,
}

impl BridgeSpliceV4 {
    pub fn new() -> Self {
        Self {
            pipes: BTreeMap::new(),
            transfers: BTreeMap::new(),
            next_pipe_id: 1,
            next_transfer_id: 1,
            stats: SpliceV4BridgeStats {
                total_splices: 0,
                total_tees: 0,
                total_vmsplices: 0,
                total_fanouts: 0,
                total_bytes: 0,
                pages_moved: 0,
                pages_copied: 0,
                pipe_count: 0,
            },
        }
    }

    #[inline]
    pub fn create_pipe(&mut self, max_pages: u32) -> u64 {
        let id = self.next_pipe_id;
        self.next_pipe_id += 1;
        let pipe = SpliceV4Pipe::new(id, max_pages);
        self.pipes.insert(id, pipe);
        self.stats.pipe_count += 1;
        id
    }

    pub fn start_splice(
        &mut self,
        src_fd: i32,
        dst_fd: i32,
        length: u64,
        op: SpliceV4Op,
    ) -> u64 {
        let id = self.next_transfer_id;
        self.next_transfer_id += 1;
        let mut xfer = SpliceV4Transfer::new(id, op, src_fd, dst_fd);
        xfer.length = length;
        self.transfers.insert(id, xfer);
        match op {
            SpliceV4Op::Splice => self.stats.total_splices += 1,
            SpliceV4Op::Tee => self.stats.total_tees += 1,
            SpliceV4Op::Vmsplice => self.stats.total_vmsplices += 1,
            SpliceV4Op::FanOut => self.stats.total_fanouts += 1,
            _ => {}
        }
        id
    }

    #[inline(always)]
    pub fn pipe_count(&self) -> usize {
        self.pipes.len()
    }

    #[inline(always)]
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }
}

// ============================================================================
// Merged from splice_v5_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV5Op {
    Splice,
    Tee,
    Vmsplice,
    SendfileSplice,
    PipeRelay,
    DmaOffload,
}

/// Splice v5 flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV5Flag {
    Move,
    Nonblock,
    More,
    Gift,
    NoCopy,
    ForceDma,
}

/// Splice v5 pipe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceV5PipeState {
    Empty,
    Partial,
    Full,
    Draining,
    DmaActive,
    Closed,
}

/// Page accounting entry
#[derive(Debug, Clone)]
pub struct SpliceV5Page {
    pub page_id: u64,
    pub offset: u32,
    pub len: u32,
    pub ref_count: u32,
    pub dma_mapped: bool,
    pub can_merge: bool,
}

impl SpliceV5Page {
    pub fn new(page_id: u64, offset: u32, len: u32) -> Self {
        Self {
            page_id,
            offset,
            len,
            ref_count: 1,
            dma_mapped: false,
            can_merge: true,
        }
    }

    #[inline(always)]
    pub fn remaining(&self) -> u32 {
        4096u32.saturating_sub(self.offset + self.len)
    }

    #[inline]
    pub fn merge(&mut self, extra_len: u32) -> bool {
        if self.can_merge && self.remaining() >= extra_len {
            self.len += extra_len;
            true
        } else {
            false
        }
    }
}

/// Splice v5 pipe buffer
#[derive(Debug, Clone)]
pub struct SpliceV5Pipe {
    pub pipe_id: u32,
    pub max_pages: u32,
    pub state: SpliceV5PipeState,
    pub pages: VecDeque<SpliceV5Page>,
    pub total_bytes: u64,
    pub total_ops: u64,
    pub zero_copy_bytes: u64,
    pub dma_bytes: u64,
    pub copy_bytes: u64,
    pub merged_count: u64,
}

impl SpliceV5Pipe {
    pub fn new(pipe_id: u32, max_pages: u32) -> Self {
        Self {
            pipe_id,
            max_pages,
            state: SpliceV5PipeState::Empty,
            pages: VecDeque::new(),
            total_bytes: 0,
            total_ops: 0,
            zero_copy_bytes: 0,
            dma_bytes: 0,
            copy_bytes: 0,
            merged_count: 0,
        }
    }

    pub fn splice_in(&mut self, bytes: u32, page_id: u64, zero_copy: bool, dma: bool) -> u32 {
        self.total_ops += 1;
        if let Some(last) = self.pages.last_mut() {
            if last.merge(bytes) {
                self.merged_count += 1;
                self.total_bytes += bytes as u64;
                self.account(bytes, zero_copy, dma);
                self.update_state();
                return bytes;
            }
        }
        if self.pages.len() as u32 >= self.max_pages {
            return 0;
        }
        let mut page = SpliceV5Page::new(page_id, 0, bytes);
        page.dma_mapped = dma;
        self.pages.push_back(page);
        self.total_bytes += bytes as u64;
        self.account(bytes, zero_copy, dma);
        self.update_state();
        bytes
    }

    fn account(&mut self, bytes: u32, zero_copy: bool, dma: bool) {
        if dma {
            self.dma_bytes += bytes as u64;
        } else if zero_copy {
            self.zero_copy_bytes += bytes as u64;
        } else {
            self.copy_bytes += bytes as u64;
        }
    }

    #[inline]
    pub fn splice_out(&mut self) -> u32 {
        if let Some(page) = self.pages.first() {
            let len = page.len;
            self.pages.pop_front();
            self.update_state();
            len
        } else {
            0
        }
    }

    fn update_state(&mut self) {
        if self.pages.is_empty() {
            self.state = SpliceV5PipeState::Empty;
        } else if self.pages.len() as u32 >= self.max_pages {
            self.state = SpliceV5PipeState::Full;
        } else if self.pages.iter().any(|p| p.dma_mapped) {
            self.state = SpliceV5PipeState::DmaActive;
        } else {
            self.state = SpliceV5PipeState::Partial;
        }
    }

    #[inline(always)]
    pub fn zero_copy_rate(&self) -> f64 {
        if self.total_bytes == 0 { 0.0 }
        else { (self.zero_copy_bytes + self.dma_bytes) as f64 / self.total_bytes as f64 }
    }
}

/// Splice v5 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SpliceV5BridgeStats {
    pub total_ops: u64,
    pub total_bytes: u64,
    pub zero_copy_bytes: u64,
    pub dma_bytes: u64,
    pub total_pipes: u64,
}

/// Main bridge splice v5
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeSpliceV5 {
    pub pipes: BTreeMap<u32, SpliceV5Pipe>,
    pub stats: SpliceV5BridgeStats,
    pub next_pipe_id: u32,
}

impl BridgeSpliceV5 {
    pub fn new() -> Self {
        Self {
            pipes: BTreeMap::new(),
            stats: SpliceV5BridgeStats {
                total_ops: 0,
                total_bytes: 0,
                zero_copy_bytes: 0,
                dma_bytes: 0,
                total_pipes: 0,
            },
            next_pipe_id: 1,
        }
    }

    #[inline]
    pub fn create_pipe(&mut self, max_pages: u32) -> u32 {
        let id = self.next_pipe_id;
        self.next_pipe_id += 1;
        self.pipes.insert(id, SpliceV5Pipe::new(id, max_pages));
        self.stats.total_pipes += 1;
        id
    }

    pub fn splice(&mut self, pipe_id: u32, bytes: u32, page_id: u64, zero_copy: bool, dma: bool) -> u32 {
        if let Some(pipe) = self.pipes.get_mut(&pipe_id) {
            let transferred = pipe.splice_in(bytes, page_id, zero_copy, dma);
            self.stats.total_ops += 1;
            self.stats.total_bytes += transferred as u64;
            if dma { self.stats.dma_bytes += transferred as u64; }
            else if zero_copy { self.stats.zero_copy_bytes += transferred as u64; }
            transferred
        } else {
            0
        }
    }
}
