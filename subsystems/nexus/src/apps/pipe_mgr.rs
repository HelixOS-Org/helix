//! # Apps Pipe Manager
//!
//! Pipe usage tracking and optimization:
//! - Named and anonymous pipe lifecycle
//! - Reader/writer endpoint tracking
//! - Throughput and buffer utilization
//! - Splice/tee zero-copy tracking
//! - Broken pipe detection
//! - Pipe chain analysis (A|B|C patterns)

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Pipe type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeKind {
    Anonymous,
    Named,
    Socketpair,
}

/// Pipe endpoint state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointState {
    Open,
    Closed,
    Broken,
}

/// Pipe instance
#[derive(Debug, Clone)]
pub struct PipeInstance {
    pub id: u64,
    pub kind: PipeKind,
    pub read_fd: i32,
    pub write_fd: i32,
    pub reader_pid: u64,
    pub writer_pid: u64,
    pub reader_state: EndpointState,
    pub writer_state: EndpointState,
    pub buffer_size: usize,
    pub buffer_used: usize,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub write_ops: u64,
    pub read_ops: u64,
    pub write_blocks: u64,
    pub read_blocks: u64,
    pub splice_ops: u64,
    pub splice_bytes: u64,
    pub created_ts: u64,
    pub last_io_ts: u64,
    pub broken_ts: u64,
}

impl PipeInstance {
    pub fn new(id: u64, kind: PipeKind, rfd: i32, wfd: i32, reader: u64, writer: u64, buf_size: usize, ts: u64) -> Self {
        Self {
            id, kind, read_fd: rfd, write_fd: wfd, reader_pid: reader, writer_pid: writer,
            reader_state: EndpointState::Open, writer_state: EndpointState::Open,
            buffer_size: buf_size, buffer_used: 0,
            bytes_written: 0, bytes_read: 0, write_ops: 0, read_ops: 0,
            write_blocks: 0, read_blocks: 0, splice_ops: 0, splice_bytes: 0,
            created_ts: ts, last_io_ts: 0, broken_ts: 0,
        }
    }

    #[inline]
    pub fn record_write(&mut self, bytes: usize, blocked: bool, ts: u64) {
        self.bytes_written += bytes as u64;
        self.write_ops += 1;
        if blocked { self.write_blocks += 1; }
        self.buffer_used = (self.buffer_used + bytes).min(self.buffer_size);
        self.last_io_ts = ts;
    }

    #[inline]
    pub fn record_read(&mut self, bytes: usize, blocked: bool, ts: u64) {
        self.bytes_read += bytes as u64;
        self.read_ops += 1;
        if blocked { self.read_blocks += 1; }
        self.buffer_used = self.buffer_used.saturating_sub(bytes);
        self.last_io_ts = ts;
    }

    #[inline]
    pub fn record_splice(&mut self, bytes: usize, ts: u64) {
        self.splice_ops += 1;
        self.splice_bytes += bytes as u64;
        self.last_io_ts = ts;
    }

    #[inline]
    pub fn close_reader(&mut self, ts: u64) {
        self.reader_state = EndpointState::Closed;
        if self.writer_state == EndpointState::Open {
            self.writer_state = EndpointState::Broken;
            self.broken_ts = ts;
        }
    }

    #[inline]
    pub fn close_writer(&mut self, ts: u64) {
        self.writer_state = EndpointState::Closed;
        if self.reader_state == EndpointState::Open {
            // reader gets EOF, not broken
        }
    }

    #[inline(always)]
    pub fn is_broken(&self) -> bool {
        self.reader_state == EndpointState::Broken || self.writer_state == EndpointState::Broken
    }

    #[inline(always)]
    pub fn is_closed(&self) -> bool {
        self.reader_state == EndpointState::Closed && self.writer_state == EndpointState::Closed
    }

    #[inline(always)]
    pub fn fill_ratio(&self) -> f64 {
        if self.buffer_size == 0 { return 0.0; }
        self.buffer_used as f64 / self.buffer_size as f64
    }

    #[inline(always)]
    pub fn throughput_bps(&self, elapsed_ns: u64) -> f64 {
        if elapsed_ns == 0 { return 0.0; }
        ((self.bytes_written + self.bytes_read) as f64 * 1_000_000_000.0) / elapsed_ns as f64
    }

    #[inline]
    pub fn block_ratio(&self) -> f64 {
        let total = self.write_ops + self.read_ops;
        if total == 0 { return 0.0; }
        (self.write_blocks + self.read_blocks) as f64 / total as f64
    }

    #[inline]
    pub fn zero_copy_ratio(&self) -> f64 {
        let total = self.bytes_written + self.splice_bytes;
        if total == 0 { return 0.0; }
        self.splice_bytes as f64 / total as f64
    }
}

/// Pipe chain (pipeline pattern)
#[derive(Debug, Clone)]
pub struct PipeChain {
    pub chain_id: u64,
    pub pipe_ids: Vec<u64>,
    pub pids: Vec<u64>,
    pub total_throughput: u64,
    pub bottleneck_idx: Option<usize>,
}

impl PipeChain {
    pub fn new(id: u64) -> Self {
        Self { chain_id: id, pipe_ids: Vec::new(), pids: Vec::new(), total_throughput: 0, bottleneck_idx: None }
    }
}

/// Pipe manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PipeMgrStats {
    pub total_pipes: usize,
    pub active_pipes: usize,
    pub broken_pipes: usize,
    pub total_bytes: u64,
    pub total_splice_bytes: u64,
    pub high_contention_pipes: usize,
    pub pipe_chains: usize,
}

/// Apps pipe manager
pub struct AppsPipeMgr {
    pipes: BTreeMap<u64, PipeInstance>,
    chains: BTreeMap<u64, PipeChain>,
    next_id: u64,
    next_chain_id: u64,
    stats: PipeMgrStats,
}

impl AppsPipeMgr {
    pub fn new() -> Self {
        Self {
            pipes: BTreeMap::new(), chains: BTreeMap::new(),
            next_id: 1, next_chain_id: 1,
            stats: PipeMgrStats::default(),
        }
    }

    #[inline]
    pub fn create(&mut self, kind: PipeKind, rfd: i32, wfd: i32, reader: u64, writer: u64, buf_size: usize, ts: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pipes.insert(id, PipeInstance::new(id, kind, rfd, wfd, reader, writer, buf_size, ts));
        id
    }

    #[inline(always)]
    pub fn record_write(&mut self, id: u64, bytes: usize, blocked: bool, ts: u64) {
        if let Some(p) = self.pipes.get_mut(&id) { p.record_write(bytes, blocked, ts); }
    }

    #[inline(always)]
    pub fn record_read(&mut self, id: u64, bytes: usize, blocked: bool, ts: u64) {
        if let Some(p) = self.pipes.get_mut(&id) { p.record_read(bytes, blocked, ts); }
    }

    #[inline(always)]
    pub fn record_splice(&mut self, id: u64, bytes: usize, ts: u64) {
        if let Some(p) = self.pipes.get_mut(&id) { p.record_splice(bytes, ts); }
    }

    #[inline(always)]
    pub fn close_reader(&mut self, id: u64, ts: u64) {
        if let Some(p) = self.pipes.get_mut(&id) { p.close_reader(ts); }
    }

    #[inline(always)]
    pub fn close_writer(&mut self, id: u64, ts: u64) {
        if let Some(p) = self.pipes.get_mut(&id) { p.close_writer(ts); }
    }

    #[inline(always)]
    pub fn destroy(&mut self, id: u64) { self.pipes.remove(&id); }

    #[inline(always)]
    pub fn gc_closed(&mut self) { self.pipes.retain(|_, p| !p.is_closed()); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_pipes = self.pipes.len();
        self.stats.active_pipes = self.pipes.values().filter(|p| !p.is_broken() && !p.is_closed()).count();
        self.stats.broken_pipes = self.pipes.values().filter(|p| p.is_broken()).count();
        self.stats.total_bytes = self.pipes.values().map(|p| p.bytes_written + p.bytes_read).sum();
        self.stats.total_splice_bytes = self.pipes.values().map(|p| p.splice_bytes).sum();
        self.stats.high_contention_pipes = self.pipes.values().filter(|p| p.block_ratio() > 0.3).count();
        self.stats.pipe_chains = self.chains.len();
    }

    #[inline(always)]
    pub fn pipe(&self, id: u64) -> Option<&PipeInstance> { self.pipes.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &PipeMgrStats { &self.stats }
}
