// SPDX-License-Identifier: GPL-2.0
//! Apps sendfile_app — zero-copy file transfer.

extern crate alloc;

use alloc::vec::Vec;

/// Sendfile transfer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendfileState {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Sendfile transfer
#[derive(Debug)]
pub struct SendfileTransfer {
    pub id: u64,
    pub in_fd: u64,
    pub out_fd: u64,
    pub offset: u64,
    pub count: u64,
    pub transferred: u64,
    pub state: SendfileState,
    pub timestamp: u64,
    pub duration_ns: u64,
}

impl SendfileTransfer {
    pub fn new(id: u64, in_fd: u64, out_fd: u64, offset: u64, count: u64, now: u64) -> Self {
        Self { id, in_fd, out_fd, offset, count, transferred: 0, state: SendfileState::Pending, timestamp: now, duration_ns: 0 }
    }

    pub fn complete(&mut self, transferred: u64, dur: u64) {
        self.transferred = transferred;
        self.state = SendfileState::Completed;
        self.duration_ns = dur;
    }

    pub fn throughput_bps(&self) -> u64 {
        if self.duration_ns == 0 { 0 } else { self.transferred * 1_000_000_000 / self.duration_ns }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SendfileAppStats {
    pub total_transfers: u32,
    pub completed: u32,
    pub failed: u32,
    pub total_bytes: u64,
    pub avg_throughput_bps: u64,
}

/// Main sendfile app
pub struct AppSendfile {
    transfers: Vec<SendfileTransfer>,
    next_id: u64,
}

impl AppSendfile {
    pub fn new() -> Self { Self { transfers: Vec::new(), next_id: 1 } }

    pub fn sendfile(&mut self, in_fd: u64, out_fd: u64, offset: u64, count: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.transfers.push(SendfileTransfer::new(id, in_fd, out_fd, offset, count, now));
        id
    }

    pub fn stats(&self) -> SendfileAppStats {
        let completed = self.transfers.iter().filter(|t| t.state == SendfileState::Completed).count() as u32;
        let failed = self.transfers.iter().filter(|t| t.state == SendfileState::Failed).count() as u32;
        let bytes: u64 = self.transfers.iter().map(|t| t.transferred).sum();
        let thrus: Vec<u64> = self.transfers.iter().filter(|t| t.state == SendfileState::Completed).map(|t| t.throughput_bps()).collect();
        let avg = if thrus.is_empty() { 0 } else { thrus.iter().sum::<u64>() / thrus.len() as u64 };
        SendfileAppStats { total_transfers: self.transfers.len() as u32, completed, failed, total_bytes: bytes, avg_throughput_bps: avg }
    }
}

// ============================================================================
// Merged from sendfile_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendfileV2Dir {
    FileToSocket,
    FileToFile,
    FileToPipe,
}

/// Sendfile v2 transfer
#[derive(Debug)]
pub struct SendfileV2Transfer {
    pub id: u64,
    pub out_fd: u64,
    pub in_fd: u64,
    pub direction: SendfileV2Dir,
    pub offset: u64,
    pub count: u64,
    pub transferred: u64,
    pub zero_copy: bool,
    pub timestamp: u64,
}

/// FD sendfile tracker
#[derive(Debug)]
pub struct FdSendfileTracker {
    pub fd: u64,
    pub total_sends: u64,
    pub total_bytes: u64,
    pub zero_copy_bytes: u64,
    pub avg_transfer_size: u64,
}

impl FdSendfileTracker {
    pub fn new(fd: u64) -> Self {
        Self { fd, total_sends: 0, total_bytes: 0, zero_copy_bytes: 0, avg_transfer_size: 0 }
    }

    pub fn record(&mut self, bytes: u64, zc: bool) {
        self.total_sends += 1;
        self.total_bytes += bytes;
        if zc { self.zero_copy_bytes += bytes; }
        self.avg_transfer_size = self.total_bytes / self.total_sends;
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SendfileV2AppStats {
    pub tracked_fds: u32,
    pub total_sends: u64,
    pub total_bytes: u64,
    pub zero_copy_bytes: u64,
}

/// Main app sendfile v2
pub struct AppSendfileV2 {
    trackers: BTreeMap<u64, FdSendfileTracker>,
}

impl AppSendfileV2 {
    pub fn new() -> Self { Self { trackers: BTreeMap::new() } }

    pub fn track(&mut self, fd: u64) { self.trackers.insert(fd, FdSendfileTracker::new(fd)); }

    pub fn sendfile(&mut self, out_fd: u64, bytes: u64, zero_copy: bool) {
        if let Some(t) = self.trackers.get_mut(&out_fd) { t.record(bytes, zero_copy); }
    }

    pub fn untrack(&mut self, fd: u64) { self.trackers.remove(&fd); }

    pub fn stats(&self) -> SendfileV2AppStats {
        let sends: u64 = self.trackers.values().map(|t| t.total_sends).sum();
        let bytes: u64 = self.trackers.values().map(|t| t.total_bytes).sum();
        let zc: u64 = self.trackers.values().map(|t| t.zero_copy_bytes).sum();
        SendfileV2AppStats { tracked_fds: self.trackers.len() as u32, total_sends: sends, total_bytes: bytes, zero_copy_bytes: zc }
    }
}
