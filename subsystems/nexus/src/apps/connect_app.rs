// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Connect (outbound connection management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectState {
    Pending,
    InProgress,
    Established,
    Refused,
    TimedOut,
    Unreachable,
    Reset,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectRetryPolicy {
    None,
    Linear(u32),
    Exponential(u32),
    Jittered(u32),
}

#[derive(Debug, Clone)]
pub struct ConnectAttempt {
    pub fd: u64,
    pub dest_addr_hash: u64,
    pub dest_port: u16,
    pub state: ConnectState,
    pub attempt_nr: u32,
    pub start_ns: u64,
    pub end_ns: u64,
    pub error_code: Option<u32>,
}

impl ConnectAttempt {
    pub fn new(fd: u64, addr: &[u8], port: u16, start: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in addr { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            fd, dest_addr_hash: h, dest_port: port,
            state: ConnectState::Pending, attempt_nr: 1,
            start_ns: start, end_ns: 0, error_code: None,
        }
    }

    pub fn complete(&mut self, state: ConnectState, end: u64) {
        self.state = state;
        self.end_ns = end;
    }

    pub fn latency_ns(&self) -> u64 {
        if self.end_ns > self.start_ns { self.end_ns - self.start_ns } else { 0 }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.state,
            ConnectState::Established | ConnectState::Refused |
            ConnectState::TimedOut | ConnectState::Unreachable |
            ConnectState::Failed)
    }
}

#[derive(Debug, Clone)]
pub struct ConnectTargetStats {
    pub addr_hash: u64,
    pub port: u16,
    pub total_attempts: u64,
    pub successful: u64,
    pub total_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub last_error: Option<ConnectState>,
}

impl ConnectTargetStats {
    pub fn new(addr_hash: u64, port: u16) -> Self {
        Self {
            addr_hash, port, total_attempts: 0, successful: 0,
            total_latency_ns: 0, min_latency_ns: u64::MAX,
            max_latency_ns: 0, last_error: None,
        }
    }

    pub fn record(&mut self, attempt: &ConnectAttempt) {
        self.total_attempts += 1;
        let lat = attempt.latency_ns();
        self.total_latency_ns += lat;
        if lat < self.min_latency_ns { self.min_latency_ns = lat; }
        if lat > self.max_latency_ns { self.max_latency_ns = lat; }
        if attempt.state == ConnectState::Established {
            self.successful += 1;
        } else if attempt.is_terminal() {
            self.last_error = Some(attempt.state);
        }
    }

    pub fn success_rate(&self) -> u64 {
        if self.total_attempts == 0 { 0 }
        else { (self.successful * 100) / self.total_attempts }
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.total_attempts == 0 { 0 }
        else { self.total_latency_ns / self.total_attempts }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectAppStats {
    pub total_connects: u64,
    pub successful: u64,
    pub failed: u64,
    pub timed_out: u64,
    pub refused: u64,
}

pub struct AppConnect {
    targets: BTreeMap<u64, ConnectTargetStats>,
    pending: BTreeMap<u64, ConnectAttempt>,
    stats: ConnectAppStats,
}

impl AppConnect {
    pub fn new() -> Self {
        Self {
            targets: BTreeMap::new(),
            pending: BTreeMap::new(),
            stats: ConnectAppStats {
                total_connects: 0, successful: 0,
                failed: 0, timed_out: 0, refused: 0,
            },
        }
    }

    pub fn start_connect(&mut self, attempt: ConnectAttempt) {
        self.stats.total_connects += 1;
        self.pending.insert(attempt.fd, attempt);
    }

    pub fn complete_connect(&mut self, fd: u64, state: ConnectState, end_ns: u64) {
        if let Some(mut attempt) = self.pending.remove(&fd) {
            attempt.complete(state, end_ns);
            match state {
                ConnectState::Established => self.stats.successful += 1,
                ConnectState::Refused => self.stats.refused += 1,
                ConnectState::TimedOut => self.stats.timed_out += 1,
                _ => self.stats.failed += 1,
            }
            let key = attempt.dest_addr_hash ^ (attempt.dest_port as u64);
            let target = self.targets.entry(key)
                .or_insert_with(|| ConnectTargetStats::new(attempt.dest_addr_hash, attempt.dest_port));
            target.record(&attempt);
        }
    }

    pub fn stats(&self) -> &ConnectAppStats { &self.stats }
}
