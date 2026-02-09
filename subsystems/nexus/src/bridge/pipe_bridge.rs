//! # Bridge Pipe Bridge
//!
//! Pipe and FIFO syscall bridging:
//! - Pipe buffer management and ring buffer tracking
//! - Reader/writer state synchronization
//! - Splice/vmsplice/tee operation bridging
//! - Pipe capacity tuning (F_SETPIPE_SZ)
//! - Broken pipe detection and reporting
//! - Per-pipe throughput statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Pipe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeState {
    Open,
    ReadClosed,
    WriteClosed,
    BothClosed,
    Broken,
}

/// Pipe buffer page
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PipeBuffer {
    pub page_index: u32,
    pub offset: u32,
    pub len: u32,
    pub flags: u32,
}

impl PipeBuffer {
    pub fn new(page: u32, offset: u32, len: u32) -> Self {
        Self { page_index: page, offset, len, flags: 0 }
    }
    #[inline(always)]
    pub fn remaining(&self) -> u32 { 4096u32.saturating_sub(self.offset + self.len) }
}

/// Pipe instance
#[derive(Debug, Clone)]
pub struct PipeInstance {
    pub pipe_id: u64,
    pub state: PipeState,
    pub reader_pid: u64,
    pub writer_pid: u64,
    pub capacity_pages: u32,
    pub used_pages: u32,
    pub bytes_buffered: u64,
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub splice_count: u64,
    pub blocked_reads: u64,
    pub blocked_writes: u64,
    pub broken_pipe_signals: u32,
    pub created_ts: u64,
    pub last_write_ts: u64,
    pub last_read_ts: u64,
}

impl PipeInstance {
    pub fn new(id: u64, reader: u64, writer: u64, ts: u64) -> Self {
        Self {
            pipe_id: id, state: PipeState::Open, reader_pid: reader,
            writer_pid: writer, capacity_pages: 16, used_pages: 0,
            bytes_buffered: 0, total_bytes_written: 0, total_bytes_read: 0,
            write_count: 0, read_count: 0, splice_count: 0,
            blocked_reads: 0, blocked_writes: 0, broken_pipe_signals: 0,
            created_ts: ts, last_write_ts: 0, last_read_ts: 0,
        }
    }

    #[inline(always)]
    pub fn capacity_bytes(&self) -> u64 { self.capacity_pages as u64 * 4096 }

    pub fn write(&mut self, bytes: u64, ts: u64) -> bool {
        if self.state == PipeState::ReadClosed || self.state == PipeState::BothClosed {
            self.broken_pipe_signals += 1;
            self.state = PipeState::Broken;
            return false;
        }
        if self.bytes_buffered + bytes > self.capacity_bytes() {
            self.blocked_writes += 1;
            return false;
        }
        self.bytes_buffered += bytes;
        self.total_bytes_written += bytes;
        self.write_count += 1;
        self.last_write_ts = ts;
        self.used_pages = ((self.bytes_buffered + 4095) / 4096) as u32;
        true
    }

    pub fn read(&mut self, bytes: u64, ts: u64) -> u64 {
        let actual = bytes.min(self.bytes_buffered);
        if actual == 0 && self.state != PipeState::WriteClosed {
            self.blocked_reads += 1;
            return 0;
        }
        self.bytes_buffered -= actual;
        self.total_bytes_read += actual;
        self.read_count += 1;
        self.last_read_ts = ts;
        self.used_pages = ((self.bytes_buffered + 4095) / 4096) as u32;
        actual
    }

    #[inline]
    pub fn close_reader(&mut self) {
        self.state = match self.state {
            PipeState::Open => PipeState::ReadClosed,
            PipeState::WriteClosed => PipeState::BothClosed,
            other => other,
        };
    }

    #[inline]
    pub fn close_writer(&mut self) {
        self.state = match self.state {
            PipeState::Open => PipeState::WriteClosed,
            PipeState::ReadClosed => PipeState::BothClosed,
            other => other,
        };
    }

    #[inline(always)]
    pub fn set_capacity(&mut self, pages: u32) { self.capacity_pages = pages.max(1); }

    #[inline(always)]
    pub fn fill_ratio(&self) -> f64 {
        let cap = self.capacity_bytes();
        if cap == 0 { 0.0 } else { self.bytes_buffered as f64 / cap as f64 }
    }

    #[inline(always)]
    pub fn throughput_bps(&self, current_ts: u64) -> f64 {
        let elapsed = current_ts.saturating_sub(self.created_ts);
        if elapsed == 0 { 0.0 } else { self.total_bytes_written as f64 / (elapsed as f64 / 1_000_000_000.0) }
    }
}

/// Splice operation record
#[derive(Debug, Clone)]
pub struct SpliceRecord {
    pub src_fd: i32,
    pub dst_fd: i32,
    pub bytes: u64,
    pub flags: u32,
    pub timestamp: u64,
    pub zero_copy: bool,
}

/// Pipe bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PipeBridgeStats {
    pub total_pipes: usize,
    pub active_pipes: usize,
    pub broken_pipes: usize,
    pub total_bytes_transferred: u64,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_splices: u64,
    pub total_blocked_reads: u64,
    pub total_blocked_writes: u64,
    pub avg_fill_ratio: f64,
}

/// Bridge pipe manager
#[repr(align(64))]
pub struct BridgePipeBridge {
    pipes: BTreeMap<u64, PipeInstance>,
    splice_history: VecDeque<SpliceRecord>,
    max_splice_history: usize,
    next_id: u64,
    stats: PipeBridgeStats,
}

impl BridgePipeBridge {
    pub fn new() -> Self {
        Self {
            pipes: BTreeMap::new(), splice_history: VecDeque::new(),
            max_splice_history: 512, next_id: 1,
            stats: PipeBridgeStats::default(),
        }
    }

    #[inline]
    pub fn create_pipe(&mut self, reader: u64, writer: u64, ts: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pipes.insert(id, PipeInstance::new(id, reader, writer, ts));
        id
    }

    #[inline(always)]
    pub fn write(&mut self, pipe_id: u64, bytes: u64, ts: u64) -> bool {
        if let Some(p) = self.pipes.get_mut(&pipe_id) { p.write(bytes, ts) } else { false }
    }

    #[inline(always)]
    pub fn read(&mut self, pipe_id: u64, bytes: u64, ts: u64) -> u64 {
        if let Some(p) = self.pipes.get_mut(&pipe_id) { p.read(bytes, ts) } else { 0 }
    }

    #[inline(always)]
    pub fn splice(&mut self, src_fd: i32, dst_fd: i32, bytes: u64, flags: u32, ts: u64) {
        self.splice_history.push_back(SpliceRecord { src_fd, dst_fd, bytes, flags, timestamp: ts, zero_copy: flags & 0x4 != 0 });
        if self.splice_history.len() > self.max_splice_history { self.splice_history.pop_front(); }
    }

    #[inline(always)]
    pub fn close_reader(&mut self, pipe_id: u64) {
        if let Some(p) = self.pipes.get_mut(&pipe_id) { p.close_reader(); }
    }

    #[inline(always)]
    pub fn close_writer(&mut self, pipe_id: u64) {
        if let Some(p) = self.pipes.get_mut(&pipe_id) { p.close_writer(); }
    }

    #[inline(always)]
    pub fn set_capacity(&mut self, pipe_id: u64, pages: u32) {
        if let Some(p) = self.pipes.get_mut(&pipe_id) { p.set_capacity(pages); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_pipes = self.pipes.len();
        self.stats.active_pipes = self.pipes.values().filter(|p| p.state == PipeState::Open).count();
        self.stats.broken_pipes = self.pipes.values().filter(|p| p.state == PipeState::Broken).count();
        self.stats.total_bytes_transferred = self.pipes.values().map(|p| p.total_bytes_written).sum();
        self.stats.total_writes = self.pipes.values().map(|p| p.write_count).sum();
        self.stats.total_reads = self.pipes.values().map(|p| p.read_count).sum();
        self.stats.total_splices = self.splice_history.len() as u64;
        self.stats.total_blocked_reads = self.pipes.values().map(|p| p.blocked_reads).sum();
        self.stats.total_blocked_writes = self.pipes.values().map(|p| p.blocked_writes).sum();
        let fills: Vec<f64> = self.pipes.values().filter(|p| p.state == PipeState::Open).map(|p| p.fill_ratio()).collect();
        self.stats.avg_fill_ratio = if fills.is_empty() { 0.0 } else { fills.iter().sum::<f64>() / fills.len() as f64 };
    }

    #[inline(always)]
    pub fn pipe(&self, id: u64) -> Option<&PipeInstance> { self.pipes.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &PipeBridgeStats { &self.stats }
}

// ============================================================================
// Merged from pipe_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeV2Op {
    Create,
    Read,
    Write,
    Splice,
    Tee,
    Vmsplice,
    SetSize,
    Close,
}

/// Pipe v2 flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeV2Flag {
    None,
    Nonblock,
    Cloexec,
    Direct,
}

/// Pipe v2 record
#[derive(Debug, Clone)]
pub struct PipeV2Record {
    pub op: PipeV2Op,
    pub flag: PipeV2Flag,
    pub bytes: u64,
    pub capacity: u32,
    pub fds: [i32; 2],
}

impl PipeV2Record {
    pub fn new(op: PipeV2Op) -> Self {
        Self { op, flag: PipeV2Flag::None, bytes: 0, capacity: 65536, fds: [-1, -1] }
    }
}

/// Pipe v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PipeV2BridgeStats {
    pub total_ops: u64,
    pub pipes_created: u64,
    pub bytes_transferred: u64,
    pub splices: u64,
}

/// Main bridge pipe v2
#[derive(Debug)]
pub struct BridgePipeV2 {
    pub stats: PipeV2BridgeStats,
}

impl BridgePipeV2 {
    pub fn new() -> Self {
        Self { stats: PipeV2BridgeStats { total_ops: 0, pipes_created: 0, bytes_transferred: 0, splices: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &PipeV2Record) {
        self.stats.total_ops += 1;
        match rec.op {
            PipeV2Op::Create => self.stats.pipes_created += 1,
            PipeV2Op::Read | PipeV2Op::Write => self.stats.bytes_transferred += rec.bytes,
            PipeV2Op::Splice | PipeV2Op::Tee | PipeV2Op::Vmsplice => self.stats.splices += 1,
            _ => {}
        }
    }
}
