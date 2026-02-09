//! # Apps Network Stack Manager
//!
//! Application network stack integration:
//! - Per-app socket tracking and aggregation
//! - TCP connection state machine monitoring
//! - UDP flow tracking
//! - Network bandwidth accounting per app
//! - Connection rate limiting
//! - DNS resolution caching per app

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TCP state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// Connection 4-tuple
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnTuple {
    pub local_addr: [u8; 4],
    pub local_port: u16,
    pub remote_addr: [u8; 4],
    pub remote_port: u16,
}

impl ConnTuple {
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &self.local_addr {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= self.local_port as u64;
        h = h.wrapping_mul(0x100000001b3);
        for &b in &self.remote_addr {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= self.remote_port as u64;
        h = h.wrapping_mul(0x100000001b3);
        h
    }
}

/// TCP connection info
#[derive(Debug, Clone)]
pub struct TcpConnInfo {
    pub tuple: ConnTuple,
    pub state: TcpState,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
    pub retransmits: u32,
    pub rtt_us: u32,
    pub rtt_var_us: u32,
    pub cwnd: u32,
    pub ssthresh: u32,
    pub mss: u16,
    pub established_ns: u64,
}

impl TcpConnInfo {
    pub fn new(tuple: ConnTuple, ts: u64) -> Self {
        Self {
            tuple, state: TcpState::Closed,
            bytes_sent: 0, bytes_recv: 0,
            packets_sent: 0, packets_recv: 0,
            retransmits: 0, rtt_us: 0, rtt_var_us: 0,
            cwnd: 10, ssthresh: 65535, mss: 1460,
            established_ns: ts,
        }
    }

    #[inline(always)]
    pub fn goodput_bps(&self, duration_ns: u64) -> f64 {
        if duration_ns == 0 { return 0.0; }
        (self.bytes_sent + self.bytes_recv) as f64 / (duration_ns as f64 / 1_000_000_000.0)
    }

    #[inline(always)]
    pub fn retransmit_rate(&self) -> f64 {
        if self.packets_sent == 0 { return 0.0; }
        self.retransmits as f64 / self.packets_sent as f64
    }
}

/// UDP flow info
#[derive(Debug, Clone)]
pub struct UdpFlowInfo {
    pub tuple: ConnTuple,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
    pub drops: u64,
    pub first_seen_ns: u64,
}

impl UdpFlowInfo {
    pub fn new(tuple: ConnTuple, ts: u64) -> Self {
        Self {
            tuple, bytes_sent: 0, bytes_recv: 0,
            packets_sent: 0, packets_recv: 0, drops: 0,
            first_seen_ns: ts,
        }
    }
}

/// DNS cache entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DnsCacheEntry {
    pub name_hash: u64,
    pub resolved_addr: [u8; 4],
    pub ttl_secs: u32,
    pub resolve_ns: u64,
    pub hit_count: u64,
}

impl DnsCacheEntry {
    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        let expiry = self.resolve_ns + (self.ttl_secs as u64) * 1_000_000_000;
        now >= expiry
    }
}

/// Per-app network state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppNetState {
    pub process_id: u64,
    pub tcp_conns: BTreeMap<u64, TcpConnInfo>,
    pub udp_flows: BTreeMap<u64, UdpFlowInfo>,
    pub dns_cache: BTreeMap<u64, DnsCacheEntry>,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub total_connections: u64,
    pub active_connections: u32,
    pub connection_rate_limit: Option<u32>,
    pub connections_this_second: u32,
    pub rate_limit_hits: u64,
}

impl AppNetState {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            tcp_conns: BTreeMap::new(),
            udp_flows: BTreeMap::new(),
            dns_cache: BTreeMap::new(),
            total_bytes_sent: 0,
            total_bytes_recv: 0,
            total_connections: 0,
            active_connections: 0,
            connection_rate_limit: None,
            connections_this_second: 0,
            rate_limit_hits: 0,
        }
    }

    pub fn add_tcp_conn(&mut self, conn: TcpConnInfo) -> bool {
        if let Some(limit) = self.connection_rate_limit {
            if self.connections_this_second >= limit {
                self.rate_limit_hits += 1;
                return false;
            }
        }
        let key = conn.tuple.hash();
        self.tcp_conns.insert(key, conn);
        self.total_connections += 1;
        self.active_connections += 1;
        self.connections_this_second += 1;
        true
    }

    #[inline]
    pub fn remove_tcp_conn(&mut self, tuple: &ConnTuple) {
        let key = tuple.hash();
        if self.tcp_conns.remove(&key).is_some() {
            self.active_connections = self.active_connections.saturating_sub(1);
        }
    }

    #[inline(always)]
    pub fn record_send(&mut self, bytes: u64) { self.total_bytes_sent += bytes; }
    #[inline(always)]
    pub fn record_recv(&mut self, bytes: u64) { self.total_bytes_recv += bytes; }

    #[inline(always)]
    pub fn reset_rate_counter(&mut self) { self.connections_this_second = 0; }

    #[inline(always)]
    pub fn expire_dns(&mut self, now: u64) {
        self.dns_cache.retain(|_, entry| !entry.is_expired(now));
    }
}

/// Apps network manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppsNetMgrStats {
    pub total_processes: usize,
    pub total_tcp_conns: usize,
    pub total_udp_flows: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub total_rate_limit_hits: u64,
}

/// Apps Network Stack Manager
pub struct AppsNetMgr {
    states: BTreeMap<u64, AppNetState>,
    stats: AppsNetMgrStats,
}

impl AppsNetMgr {
    pub fn new() -> Self {
        Self { states: BTreeMap::new(), stats: AppsNetMgrStats::default() }
    }

    #[inline(always)]
    pub fn register(&mut self, pid: u64) {
        self.states.entry(pid).or_insert_with(|| AppNetState::new(pid));
    }

    #[inline(always)]
    pub fn add_tcp(&mut self, pid: u64, conn: TcpConnInfo) -> bool {
        self.states.get_mut(&pid).map(|s| s.add_tcp_conn(conn)).unwrap_or(false)
    }

    #[inline(always)]
    pub fn remove_tcp(&mut self, pid: u64, tuple: &ConnTuple) {
        if let Some(s) = self.states.get_mut(&pid) { s.remove_tcp_conn(tuple); }
    }

    #[inline(always)]
    pub fn record_send(&mut self, pid: u64, bytes: u64) {
        if let Some(s) = self.states.get_mut(&pid) { s.record_send(bytes); }
    }

    #[inline(always)]
    pub fn record_recv(&mut self, pid: u64, bytes: u64) {
        if let Some(s) = self.states.get_mut(&pid) { s.record_recv(bytes); }
    }

    #[inline(always)]
    pub fn set_rate_limit(&mut self, pid: u64, limit: u32) {
        if let Some(s) = self.states.get_mut(&pid) { s.connection_rate_limit = Some(limit); }
    }

    #[inline(always)]
    pub fn tick_rate_counters(&mut self) {
        for s in self.states.values_mut() { s.reset_rate_counter(); }
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) { self.states.remove(&pid); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.states.len();
        self.stats.total_tcp_conns = self.states.values().map(|s| s.tcp_conns.len()).sum();
        self.stats.total_udp_flows = self.states.values().map(|s| s.udp_flows.len()).sum();
        self.stats.total_bytes_sent = self.states.values().map(|s| s.total_bytes_sent).sum();
        self.stats.total_bytes_recv = self.states.values().map(|s| s.total_bytes_recv).sum();
        self.stats.total_rate_limit_hits = self.states.values().map(|s| s.rate_limit_hits).sum();
    }

    #[inline(always)]
    pub fn app_state(&self, pid: u64) -> Option<&AppNetState> { self.states.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &AppsNetMgrStats { &self.stats }
}
