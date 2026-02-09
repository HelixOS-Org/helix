// SPDX-License-Identifier: GPL-2.0
//! Apps pipe_app — pipe/pipe2 application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Pipe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeAppState {
    Open,
    ReadClosed,
    WriteClosed,
    Closed,
}

/// Pipe instance
#[derive(Debug)]
pub struct PipeAppInstance {
    pub read_fd: u64,
    pub write_fd: u64,
    pub state: PipeAppState,
    pub capacity: u32,
    pub used: u32,
    pub flags: u32,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
    pub reader_pid: u64,
    pub writer_pid: u64,
}

impl PipeAppInstance {
    pub fn new(rfd: u64, wfd: u64, cap: u32) -> Self {
        Self { read_fd: rfd, write_fd: wfd, state: PipeAppState::Open, capacity: cap, used: 0, flags: 0, total_writes: 0, total_reads: 0, total_bytes_written: 0, total_bytes_read: 0, reader_pid: 0, writer_pid: 0 }
    }

    #[inline]
    pub fn write(&mut self, bytes: u32) -> bool {
        if self.used + bytes > self.capacity { return false; }
        self.used += bytes;
        self.total_writes += 1;
        self.total_bytes_written += bytes as u64;
        true
    }

    #[inline]
    pub fn read(&mut self, bytes: u32) -> u32 {
        let avail = if bytes > self.used { self.used } else { bytes };
        self.used -= avail;
        self.total_reads += 1;
        self.total_bytes_read += avail as u64;
        avail
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PipeAppStats {
    pub total_pipes: u32,
    pub open_pipes: u32,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_bytes: u64,
}

/// Main app pipe
pub struct AppPipe {
    pipes: BTreeMap<u64, PipeAppInstance>,
    next_fd: u64,
}

impl AppPipe {
    pub fn new() -> Self { Self { pipes: BTreeMap::new(), next_fd: 1 } }

    #[inline]
    pub fn create_pipe(&mut self, capacity: u32) -> (u64, u64) {
        let rfd = self.next_fd; self.next_fd += 1;
        let wfd = self.next_fd; self.next_fd += 1;
        self.pipes.insert(rfd, PipeAppInstance::new(rfd, wfd, capacity));
        (rfd, wfd)
    }

    #[inline(always)]
    pub fn write(&mut self, rfd: u64, bytes: u32) -> bool {
        if let Some(p) = self.pipes.get_mut(&rfd) { p.write(bytes) } else { false }
    }

    #[inline(always)]
    pub fn read(&mut self, rfd: u64, bytes: u32) -> u32 {
        if let Some(p) = self.pipes.get_mut(&rfd) { p.read(bytes) } else { 0 }
    }

    #[inline(always)]
    pub fn close_read(&mut self, rfd: u64) {
        if let Some(p) = self.pipes.get_mut(&rfd) { p.state = PipeAppState::ReadClosed; }
    }

    #[inline(always)]
    pub fn close_write(&mut self, rfd: u64) {
        if let Some(p) = self.pipes.get_mut(&rfd) { p.state = PipeAppState::WriteClosed; }
    }

    #[inline(always)]
    pub fn destroy(&mut self, rfd: u64) { self.pipes.remove(&rfd); }

    #[inline]
    pub fn stats(&self) -> PipeAppStats {
        let open = self.pipes.values().filter(|p| p.state == PipeAppState::Open).count() as u32;
        let writes: u64 = self.pipes.values().map(|p| p.total_writes).sum();
        let reads: u64 = self.pipes.values().map(|p| p.total_reads).sum();
        let bytes: u64 = self.pipes.values().map(|p| p.total_bytes_written).sum();
        PipeAppStats { total_pipes: self.pipes.len() as u32, open_pipes: open, total_writes: writes, total_reads: reads, total_bytes: bytes }
    }
}

// ============================================================================
// Merged from pipe_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeV2State {
    Open,
    ReadClosed,
    WriteClosed,
    BothClosed,
    Broken,
}

/// Pipe creation flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeV2Flag {
    CloseExec,
    Direct,
    NonBlock,
}

/// A pipe V2 instance.
#[derive(Debug, Clone)]
pub struct PipeV2Instance {
    pub pipe_id: u64,
    pub read_fd: i32,
    pub write_fd: i32,
    pub state: PipeV2State,
    pub capacity: usize,
    pub current_usage: usize,
    pub flags: Vec<PipeV2Flag>,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub splice_pages: u64,
    pub write_blocked_count: u64,
    pub read_blocked_count: u64,
    pub owner_pid: u64,
}

impl PipeV2Instance {
    pub fn new(pipe_id: u64, read_fd: i32, write_fd: i32) -> Self {
        Self {
            pipe_id,
            read_fd,
            write_fd,
            state: PipeV2State::Open,
            capacity: 65536,
            current_usage: 0,
            flags: Vec::new(),
            bytes_written: 0,
            bytes_read: 0,
            splice_pages: 0,
            write_blocked_count: 0,
            read_blocked_count: 0,
            owner_pid: 0,
        }
    }

    pub fn write(&mut self, bytes: usize) -> bool {
        if self.state == PipeV2State::ReadClosed || self.state == PipeV2State::BothClosed {
            return false;
        }
        if self.current_usage + bytes > self.capacity {
            self.write_blocked_count += 1;
            return false;
        }
        self.current_usage += bytes;
        self.bytes_written += bytes as u64;
        true
    }

    pub fn read(&mut self, max_bytes: usize) -> usize {
        if self.current_usage == 0 {
            if self.state == PipeV2State::WriteClosed {
                return 0; // EOF
            }
            self.read_blocked_count += 1;
            return 0;
        }
        let to_read = core::cmp::min(max_bytes, self.current_usage);
        self.current_usage -= to_read;
        self.bytes_read += to_read as u64;
        to_read
    }

    #[inline]
    pub fn set_capacity(&mut self, new_cap: usize) -> bool {
        if new_cap < self.current_usage {
            return false;
        }
        self.capacity = new_cap;
        true
    }

    #[inline]
    pub fn close_read(&mut self) {
        self.state = match self.state {
            PipeV2State::Open => PipeV2State::ReadClosed,
            PipeV2State::WriteClosed => PipeV2State::BothClosed,
            other => other,
        };
    }

    #[inline]
    pub fn close_write(&mut self) {
        self.state = match self.state {
            PipeV2State::Open => PipeV2State::WriteClosed,
            PipeV2State::ReadClosed => PipeV2State::BothClosed,
            other => other,
        };
    }

    #[inline]
    pub fn utilization_percent(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        (self.current_usage as f64 / self.capacity as f64) * 100.0
    }
}

/// Statistics for pipe V2 app.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PipeV2AppStats {
    pub total_pipes: u64,
    pub active_pipes: u64,
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
    pub total_splice_pages: u64,
    pub capacity_changes: u64,
    pub broken_pipes: u64,
    pub blocked_writes: u64,
}

/// Main apps pipe V2 manager.
pub struct AppPipeV2 {
    pub pipes: BTreeMap<u64, PipeV2Instance>,
    pub fd_to_pipe: BTreeMap<i32, u64>,
    pub next_pipe_id: u64,
    pub stats: PipeV2AppStats,
}

impl AppPipeV2 {
    pub fn new() -> Self {
        Self {
            pipes: BTreeMap::new(),
            fd_to_pipe: BTreeMap::new(),
            next_pipe_id: 1,
            stats: PipeV2AppStats {
                total_pipes: 0,
                active_pipes: 0,
                total_bytes_written: 0,
                total_bytes_read: 0,
                total_splice_pages: 0,
                capacity_changes: 0,
                broken_pipes: 0,
                blocked_writes: 0,
            },
        }
    }

    pub fn create_pipe(&mut self, read_fd: i32, write_fd: i32, pid: u64) -> u64 {
        let id = self.next_pipe_id;
        self.next_pipe_id += 1;
        let mut pipe = PipeV2Instance::new(id, read_fd, write_fd);
        pipe.owner_pid = pid;
        self.fd_to_pipe.insert(read_fd, id);
        self.fd_to_pipe.insert(write_fd, id);
        self.pipes.insert(id, pipe);
        self.stats.total_pipes += 1;
        self.stats.active_pipes += 1;
        id
    }

    pub fn write_to_pipe(&mut self, pipe_id: u64, bytes: usize) -> bool {
        if let Some(pipe) = self.pipes.get_mut(&pipe_id) {
            let ok = pipe.write(bytes);
            if ok {
                self.stats.total_bytes_written += bytes as u64;
            } else {
                self.stats.blocked_writes += 1;
            }
            ok
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn pipe_count(&self) -> usize {
        self.pipes.len()
    }
}

// ============================================================================
// Merged from pipe_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeV3Op { Create, SetSize, GetSize }

/// Pipe v3 flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeV3Flag { None, Nonblock, Cloexec, Direct }

/// Pipe v3 record
#[derive(Debug, Clone)]
pub struct PipeV3Record {
    pub op: PipeV3Op,
    pub flags: u32,
    pub capacity: u32,
    pub fds: [i32; 2],
    pub pid: u32,
}

impl PipeV3Record {
    pub fn new(op: PipeV3Op) -> Self { Self { op, flags: 0, capacity: 65536, fds: [-1, -1], pid: 0 } }
}

/// Pipe v3 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PipeV3AppStats { pub total_ops: u64, pub created: u64, pub resized: u64 }

/// Main app pipe v3
#[derive(Debug)]
pub struct AppPipeV3 { pub stats: PipeV3AppStats }

impl AppPipeV3 {
    pub fn new() -> Self { Self { stats: PipeV3AppStats { total_ops: 0, created: 0, resized: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &PipeV3Record) {
        self.stats.total_ops += 1;
        match rec.op {
            PipeV3Op::Create => self.stats.created += 1,
            PipeV3Op::SetSize => self.stats.resized += 1,
            _ => {}
        }
    }
}
