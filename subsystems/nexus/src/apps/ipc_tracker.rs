//! # Apps IPC Tracker
//!
//! Inter-process communication tracking and analysis:
//! - Pipe throughput monitoring
//! - Unix domain socket analytics
//! - Shared memory segment tracking
//! - Message queue monitoring
//! - Semaphore state tracking
//! - IPC namespace isolation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IPC mechanism type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcType {
    Pipe,
    UnixStream,
    UnixDgram,
    SharedMemory,
    MessageQueue,
    Semaphore,
    Signal,
    Futex,
    Eventfd,
}

/// IPC endpoint state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcEndpointState {
    Open,
    Connected,
    Listening,
    Closing,
    Closed,
}

/// IPC channel
#[derive(Debug, Clone)]
pub struct IpcChannel {
    pub id: u64,
    pub ipc_type: IpcType,
    pub state: IpcEndpointState,
    pub pid_a: u64,
    pub pid_b: Option<u64>,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub msgs_sent: u64,
    pub msgs_recv: u64,
    pub errors: u64,
    pub buffer_size: u32,
    pub buffer_used: u32,
    pub created_ts: u64,
    pub last_activity_ts: u64,
    pub avg_latency_ns: u64,
}

impl IpcChannel {
    pub fn new(id: u64, ipc_type: IpcType, pid_a: u64) -> Self {
        Self {
            id, ipc_type, state: IpcEndpointState::Open, pid_a, pid_b: None,
            bytes_sent: 0, bytes_recv: 0, msgs_sent: 0, msgs_recv: 0,
            errors: 0, buffer_size: 65536, buffer_used: 0,
            created_ts: 0, last_activity_ts: 0, avg_latency_ns: 0,
        }
    }

    pub fn connect(&mut self, pid_b: u64) { self.pid_b = Some(pid_b); self.state = IpcEndpointState::Connected; }

    pub fn send(&mut self, bytes: u64, ts: u64) {
        self.bytes_sent += bytes; self.msgs_sent += 1; self.last_activity_ts = ts;
    }

    pub fn recv(&mut self, bytes: u64, ts: u64) {
        self.bytes_recv += bytes; self.msgs_recv += 1; self.last_activity_ts = ts;
    }

    pub fn close(&mut self) { self.state = IpcEndpointState::Closed; }

    pub fn throughput_bps(&self, now: u64) -> f64 {
        let elapsed = now.saturating_sub(self.created_ts);
        if elapsed == 0 { 0.0 } else { (self.bytes_sent + self.bytes_recv) as f64 / (elapsed as f64 / 1_000_000_000.0) }
    }

    pub fn buffer_util(&self) -> f64 { if self.buffer_size == 0 { 0.0 } else { self.buffer_used as f64 / self.buffer_size as f64 * 100.0 } }
}

/// Shared memory segment
#[derive(Debug, Clone)]
pub struct ShmSegment {
    pub id: u64,
    pub key: u64,
    pub size: u64,
    pub owner_pid: u64,
    pub attached_pids: Vec<u64>,
    pub nattach: u32,
    pub created_ts: u64,
    pub last_attach_ts: u64,
    pub last_detach_ts: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

impl ShmSegment {
    pub fn new(id: u64, key: u64, size: u64, owner: u64) -> Self {
        Self {
            id, key, size, owner_pid: owner, attached_pids: Vec::new(),
            nattach: 0, created_ts: 0, last_attach_ts: 0, last_detach_ts: 0,
            read_bytes: 0, write_bytes: 0,
        }
    }

    pub fn attach(&mut self, pid: u64, ts: u64) {
        if !self.attached_pids.contains(&pid) { self.attached_pids.push(pid); }
        self.nattach += 1; self.last_attach_ts = ts;
    }

    pub fn detach(&mut self, pid: u64, ts: u64) {
        self.attached_pids.retain(|&p| p != pid);
        self.nattach = self.nattach.saturating_sub(1);
        self.last_detach_ts = ts;
    }
}

/// Per-process IPC summary
#[derive(Debug, Clone)]
pub struct ProcessIpcSummary {
    pub pid: u64,
    pub channels: Vec<u64>,
    pub shm_segments: Vec<u64>,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub total_msgs: u64,
    pub active_peers: Vec<u64>,
}

impl ProcessIpcSummary {
    pub fn new(pid: u64) -> Self {
        Self { pid, channels: Vec::new(), shm_segments: Vec::new(), total_bytes_sent: 0, total_bytes_recv: 0, total_msgs: 0, active_peers: Vec::new() }
    }
}

/// IPC tracker stats
#[derive(Debug, Clone, Default)]
pub struct IpcTrackerStats {
    pub total_channels: usize,
    pub active_channels: usize,
    pub total_shm_segments: usize,
    pub total_bytes_transferred: u64,
    pub total_messages: u64,
    pub tracked_processes: usize,
    pub total_errors: u64,
}

/// Apps IPC tracker
pub struct AppsIpcTracker {
    channels: BTreeMap<u64, IpcChannel>,
    shm_segments: BTreeMap<u64, ShmSegment>,
    process_summaries: BTreeMap<u64, ProcessIpcSummary>,
    stats: IpcTrackerStats,
    next_id: u64,
}

impl AppsIpcTracker {
    pub fn new() -> Self {
        Self { channels: BTreeMap::new(), shm_segments: BTreeMap::new(), process_summaries: BTreeMap::new(), stats: IpcTrackerStats::default(), next_id: 1 }
    }

    pub fn create_channel(&mut self, ipc_type: IpcType, pid: u64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut ch = IpcChannel::new(id, ipc_type, pid);
        ch.created_ts = ts;
        self.channels.insert(id, ch);
        self.process_summaries.entry(pid).or_insert_with(|| ProcessIpcSummary::new(pid)).channels.push(id);
        id
    }

    pub fn connect(&mut self, ch_id: u64, pid_b: u64) {
        if let Some(ch) = self.channels.get_mut(&ch_id) { ch.connect(pid_b); }
        self.process_summaries.entry(pid_b).or_insert_with(|| ProcessIpcSummary::new(pid_b)).channels.push(ch_id);
    }

    pub fn send(&mut self, ch_id: u64, bytes: u64, ts: u64) {
        if let Some(ch) = self.channels.get_mut(&ch_id) { ch.send(bytes, ts); }
    }

    pub fn recv(&mut self, ch_id: u64, bytes: u64, ts: u64) {
        if let Some(ch) = self.channels.get_mut(&ch_id) { ch.recv(bytes, ts); }
    }

    pub fn close_channel(&mut self, ch_id: u64) {
        if let Some(ch) = self.channels.get_mut(&ch_id) { ch.close(); }
    }

    pub fn create_shm(&mut self, key: u64, size: u64, owner: u64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut seg = ShmSegment::new(id, key, size, owner);
        seg.created_ts = ts;
        self.shm_segments.insert(id, seg);
        self.process_summaries.entry(owner).or_insert_with(|| ProcessIpcSummary::new(owner)).shm_segments.push(id);
        id
    }

    pub fn attach_shm(&mut self, seg_id: u64, pid: u64, ts: u64) {
        if let Some(s) = self.shm_segments.get_mut(&seg_id) { s.attach(pid, ts); }
    }

    pub fn detach_shm(&mut self, seg_id: u64, pid: u64, ts: u64) {
        if let Some(s) = self.shm_segments.get_mut(&seg_id) { s.detach(pid, ts); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_channels = self.channels.len();
        self.stats.active_channels = self.channels.values().filter(|c| c.state == IpcEndpointState::Connected || c.state == IpcEndpointState::Open).count();
        self.stats.total_shm_segments = self.shm_segments.len();
        self.stats.total_bytes_transferred = self.channels.values().map(|c| c.bytes_sent + c.bytes_recv).sum();
        self.stats.total_messages = self.channels.values().map(|c| c.msgs_sent + c.msgs_recv).sum();
        self.stats.tracked_processes = self.process_summaries.len();
        self.stats.total_errors = self.channels.values().map(|c| c.errors).sum();
    }

    pub fn channel(&self, id: u64) -> Option<&IpcChannel> { self.channels.get(&id) }
    pub fn shm(&self, id: u64) -> Option<&ShmSegment> { self.shm_segments.get(&id) }
    pub fn stats(&self) -> &IpcTrackerStats { &self.stats }
}
