// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Shutdown (socket shutdown management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownHow {
    Read,
    Write,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownResult {
    Success,
    NotConnected,
    BadFd,
    NotSocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownLingerState {
    Disabled,
    Active(u32),
    TimedOut,
    Complete,
}

#[derive(Debug, Clone)]
pub struct ShutdownRecord {
    pub fd: u64,
    pub how: ShutdownHow,
    pub result: ShutdownResult,
    pub pending_send_bytes: u64,
    pub pending_recv_bytes: u64,
    pub linger_state: ShutdownLingerState,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct SocketShutdownState {
    pub fd: u64,
    pub read_shut: bool,
    pub write_shut: bool,
    pub linger_timeout_sec: u32,
    pub bytes_drained: u64,
    pub time_in_linger_ns: u64,
    pub graceful: bool,
}

impl SocketShutdownState {
    pub fn new(fd: u64) -> Self {
        Self {
            fd, read_shut: false, write_shut: false,
            linger_timeout_sec: 0, bytes_drained: 0,
            time_in_linger_ns: 0, graceful: true,
        }
    }

    pub fn shutdown(&mut self, how: ShutdownHow) {
        match how {
            ShutdownHow::Read => self.read_shut = true,
            ShutdownHow::Write => self.write_shut = true,
            ShutdownHow::Both => {
                self.read_shut = true;
                self.write_shut = true;
            }
        }
    }

    pub fn is_fully_shut(&self) -> bool { self.read_shut && self.write_shut }
    pub fn is_half_shut(&self) -> bool { self.read_shut ^ self.write_shut }
}

#[derive(Debug, Clone)]
pub struct ShutdownAppStats {
    pub total_shutdowns: u64,
    pub read_shutdowns: u64,
    pub write_shutdowns: u64,
    pub both_shutdowns: u64,
    pub graceful_count: u64,
    pub linger_timeouts: u64,
}

pub struct AppShutdown {
    sockets: BTreeMap<u64, SocketShutdownState>,
    records: Vec<ShutdownRecord>,
    stats: ShutdownAppStats,
}

impl AppShutdown {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            records: Vec::new(),
            stats: ShutdownAppStats {
                total_shutdowns: 0, read_shutdowns: 0,
                write_shutdowns: 0, both_shutdowns: 0,
                graceful_count: 0, linger_timeouts: 0,
            },
        }
    }

    pub fn register_socket(&mut self, fd: u64) {
        self.sockets.insert(fd, SocketShutdownState::new(fd));
    }

    pub fn shutdown(&mut self, fd: u64, how: ShutdownHow, ts: u64) -> ShutdownResult {
        if let Some(s) = self.sockets.get_mut(&fd) {
            s.shutdown(how);
            self.stats.total_shutdowns += 1;
            match how {
                ShutdownHow::Read => self.stats.read_shutdowns += 1,
                ShutdownHow::Write => self.stats.write_shutdowns += 1,
                ShutdownHow::Both => self.stats.both_shutdowns += 1,
            }
            self.records.push(ShutdownRecord {
                fd, how, result: ShutdownResult::Success,
                pending_send_bytes: 0, pending_recv_bytes: 0,
                linger_state: ShutdownLingerState::Disabled,
                timestamp: ts,
            });
            ShutdownResult::Success
        } else {
            ShutdownResult::BadFd
        }
    }

    pub fn stats(&self) -> &ShutdownAppStats { &self.stats }
}

// ============================================================================
// Merged from shutdown_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownV2How { Read, Write, Both }

/// Shutdown v2 request
#[derive(Debug, Clone)]
pub struct ShutdownV2Request {
    pub fd: i32,
    pub how: ShutdownV2How,
    pub linger_ms: u32,
}

impl ShutdownV2Request {
    pub fn new(fd: i32, how: ShutdownV2How) -> Self { Self { fd, how, linger_ms: 0 } }
}

/// Shutdown v2 app stats
#[derive(Debug, Clone)]
pub struct ShutdownV2AppStats { pub total_shutdowns: u64, pub reads_shut: u64, pub writes_shut: u64, pub both_shut: u64 }

/// Main app shutdown v2
#[derive(Debug)]
pub struct AppShutdownV2 { pub stats: ShutdownV2AppStats }

impl AppShutdownV2 {
    pub fn new() -> Self { Self { stats: ShutdownV2AppStats { total_shutdowns: 0, reads_shut: 0, writes_shut: 0, both_shut: 0 } } }
    pub fn shutdown(&mut self, req: &ShutdownV2Request) {
        self.stats.total_shutdowns += 1;
        match req.how {
            ShutdownV2How::Read => self.stats.reads_shut += 1,
            ShutdownV2How::Write => self.stats.writes_shut += 1,
            ShutdownV2How::Both => self.stats.both_shut += 1,
        }
    }
}
