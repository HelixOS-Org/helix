// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Setsockopt (socket option configuration)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetoptResult {
    Success,
    InvalidArg,
    PermDenied,
    NotSupported,
    BadFd,
    NoProto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetoptCategory {
    Buffer,
    Timeout,
    Keepalive,
    Congestion,
    Multicast,
    Security,
    Routing,
    Performance,
    Debug,
}

#[derive(Debug, Clone)]
pub struct SetoptRecord {
    pub fd: u64,
    pub level: u32,
    pub optname: u32,
    pub category: SetoptCategory,
    pub old_value: Option<i64>,
    pub new_value: i64,
    pub result: SetoptResult,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct SocketOptionHistory {
    pub fd: u64,
    pub changes: Vec<SetoptRecord>,
    pub total_sets: u64,
    pub total_failures: u64,
}

impl SocketOptionHistory {
    pub fn new(fd: u64) -> Self {
        Self { fd, changes: Vec::new(), total_sets: 0, total_failures: 0 }
    }

    pub fn record(&mut self, rec: SetoptRecord) {
        match rec.result {
            SetoptResult::Success => self.total_sets += 1,
            _ => self.total_failures += 1,
        }
        if self.changes.len() < 256 { self.changes.push(rec); }
    }

    pub fn success_rate(&self) -> u64 {
        let total = self.total_sets + self.total_failures;
        if total == 0 { 100 } else { (self.total_sets * 100) / total }
    }

    pub fn last_value_for(&self, optname: u32) -> Option<i64> {
        self.changes.iter().rev()
            .find(|r| r.optname == optname && r.result == SetoptResult::Success)
            .map(|r| r.new_value)
    }
}

#[derive(Debug, Clone)]
pub struct TcpTuningProfile {
    pub nodelay: bool,
    pub cork: bool,
    pub keepalive_secs: u32,
    pub keepalive_intvl: u32,
    pub keepalive_cnt: u32,
    pub max_seg: u32,
    pub window_clamp: u32,
    pub congestion_hash: u64,
    pub fastopen_qlen: u32,
}

impl TcpTuningProfile {
    pub fn default_profile() -> Self {
        Self {
            nodelay: false, cork: false,
            keepalive_secs: 7200, keepalive_intvl: 75,
            keepalive_cnt: 9, max_seg: 536,
            window_clamp: 0, congestion_hash: 0,
            fastopen_qlen: 0,
        }
    }

    pub fn is_low_latency(&self) -> bool { self.nodelay && !self.cork }
    pub fn is_throughput_optimized(&self) -> bool { !self.nodelay && self.cork }
}

#[derive(Debug, Clone)]
pub struct SetsockoptAppStats {
    pub total_sets: u64,
    pub total_failures: u64,
    pub per_category: BTreeMap<u8, u64>,
}

pub struct AppSetsockopt {
    histories: BTreeMap<u64, SocketOptionHistory>,
    tcp_profiles: BTreeMap<u64, TcpTuningProfile>,
    stats: SetsockoptAppStats,
}

impl AppSetsockopt {
    pub fn new() -> Self {
        Self {
            histories: BTreeMap::new(),
            tcp_profiles: BTreeMap::new(),
            stats: SetsockoptAppStats {
                total_sets: 0, total_failures: 0,
                per_category: BTreeMap::new(),
            },
        }
    }

    pub fn register_socket(&mut self, fd: u64) {
        self.histories.insert(fd, SocketOptionHistory::new(fd));
    }

    pub fn record_set(&mut self, rec: SetoptRecord) {
        let cat_key = rec.category as u8;
        *self.stats.per_category.entry(cat_key).or_insert(0) += 1;
        match rec.result {
            SetoptResult::Success => self.stats.total_sets += 1,
            _ => self.stats.total_failures += 1,
        }
        let fd = rec.fd;
        self.histories.entry(fd)
            .or_insert_with(|| SocketOptionHistory::new(fd))
            .record(rec);
    }

    pub fn stats(&self) -> &SetsockoptAppStats { &self.stats }
}

// ============================================================================
// Merged from setsockopt_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOptV2Result { Success, InvalidOpt, PermDenied, InvalidValue }

/// Setsockopt v2 request
#[derive(Debug, Clone)]
pub struct SetsockoptV2Request {
    pub fd: i32,
    pub level: u16,
    pub optname: u16,
    pub value: u64,
}

impl SetsockoptV2Request {
    pub fn new(fd: i32, level: u16, optname: u16, value: u64) -> Self { Self { fd, level, optname, value } }
}

/// Setsockopt v2 app stats
#[derive(Debug, Clone)]
pub struct SetsockoptV2AppStats { pub total_sets: u64, pub successes: u64, pub failures: u64, pub buf_changes: u64 }

/// Main app setsockopt v2
#[derive(Debug)]
pub struct AppSetsockoptV2 { pub stats: SetsockoptV2AppStats }

impl AppSetsockoptV2 {
    pub fn new() -> Self { Self { stats: SetsockoptV2AppStats { total_sets: 0, successes: 0, failures: 0, buf_changes: 0 } } }
    pub fn set_opt(&mut self, req: &SetsockoptV2Request) -> SetOptV2Result {
        self.stats.total_sets += 1;
        self.stats.successes += 1;
        SetOptV2Result::Success
    }
}
