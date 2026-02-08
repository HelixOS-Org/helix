// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Accept (connection acceptance tracking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptVariant {
    Accept,
    Accept4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptFlag {
    NonBlock,
    CloseOnExec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptResult {
    Success,
    WouldBlock,
    ConnAborted,
    Interrupted,
    Invalid,
    TooManyFiles,
    NoMem,
}

#[derive(Debug, Clone)]
pub struct AcceptedConnection {
    pub listen_fd: u64,
    pub new_fd: u64,
    pub peer_addr_hash: u64,
    pub accept_latency_ns: u64,
    pub timestamp: u64,
    pub nonblocking: bool,
    pub cloexec: bool,
}

impl AcceptedConnection {
    pub fn new(listen_fd: u64, new_fd: u64, peer: &[u8], latency: u64, ts: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in peer { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            listen_fd, new_fd, peer_addr_hash: h,
            accept_latency_ns: latency, timestamp: ts,
            nonblocking: false, cloexec: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListenerAcceptState {
    pub fd: u64,
    pub total_accepted: u64,
    pub total_failed: u64,
    pub total_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub connections: Vec<AcceptedConnection>,
    pub max_tracked: usize,
}

impl ListenerAcceptState {
    pub fn new(fd: u64, max_tracked: usize) -> Self {
        Self {
            fd, total_accepted: 0, total_failed: 0,
            total_latency_ns: 0, min_latency_ns: u64::MAX,
            max_latency_ns: 0, connections: Vec::new(),
            max_tracked,
        }
    }

    pub fn record_accept(&mut self, conn: AcceptedConnection) {
        let lat = conn.accept_latency_ns;
        self.total_latency_ns += lat;
        if lat < self.min_latency_ns { self.min_latency_ns = lat; }
        if lat > self.max_latency_ns { self.max_latency_ns = lat; }
        self.total_accepted += 1;
        if self.connections.len() < self.max_tracked {
            self.connections.push(conn);
        }
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.total_accepted == 0 { 0 } else { self.total_latency_ns / self.total_accepted }
    }

    pub fn success_rate(&self) -> u64 {
        let total = self.total_accepted + self.total_failed;
        if total == 0 { 100 } else { (self.total_accepted * 100) / total }
    }
}

#[derive(Debug, Clone)]
pub struct AcceptAppStats {
    pub total_accepts: u64,
    pub total_failures: u64,
    pub total_connections: u64,
    pub avg_latency_ns: u64,
}

pub struct AppAccept {
    listeners: BTreeMap<u64, ListenerAcceptState>,
    stats: AcceptAppStats,
}

impl AppAccept {
    pub fn new() -> Self {
        Self {
            listeners: BTreeMap::new(),
            stats: AcceptAppStats {
                total_accepts: 0, total_failures: 0,
                total_connections: 0, avg_latency_ns: 0,
            },
        }
    }

    pub fn register_listener(&mut self, fd: u64, max_tracked: usize) {
        self.listeners.insert(fd, ListenerAcceptState::new(fd, max_tracked));
    }

    pub fn record_accept(&mut self, fd: u64, conn: AcceptedConnection) {
        if let Some(l) = self.listeners.get_mut(&fd) {
            l.record_accept(conn);
            self.stats.total_accepts += 1;
            self.stats.total_connections += 1;
        }
    }

    pub fn record_failure(&mut self, fd: u64) {
        if let Some(l) = self.listeners.get_mut(&fd) {
            l.total_failed += 1;
            self.stats.total_failures += 1;
        }
    }

    pub fn stats(&self) -> &AcceptAppStats { &self.stats }
}

// ============================================================================
// Merged from accept_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptV2Flag {
    None,
    NonBlock,
    CloseOnExec,
}

/// Accept v2 request
#[derive(Debug, Clone)]
pub struct AcceptV2Request {
    pub listen_fd: i32,
    pub flags: AcceptV2Flag,
}

impl AcceptV2Request {
    pub fn new(listen_fd: i32) -> Self {
        Self {
            listen_fd,
            flags: AcceptV2Flag::None,
        }
    }
}

/// Accept v2 app stats
#[derive(Debug, Clone)]
pub struct AcceptV2AppStats {
    pub total_accepts: u64,
    pub successful: u64,
    pub would_block: u64,
    pub errors: u64,
}

/// Main app accept v2
#[derive(Debug)]
pub struct AppAcceptV2 {
    pub stats: AcceptV2AppStats,
}

impl AppAcceptV2 {
    pub fn new() -> Self {
        Self {
            stats: AcceptV2AppStats {
                total_accepts: 0,
                successful: 0,
                would_block: 0,
                errors: 0,
            },
        }
    }
    pub fn accept(&mut self, req: &AcceptV2Request) -> i32 {
        self.stats.total_accepts += 1;
        self.stats.successful += 1;
        self.stats.total_accepts as i32
    }
}
