//! # Apps Network Filter
//!
//! Per-application network filtering and firewall:
//! - Ingress/egress packet filtering
//! - Connection tracking per process
//! - Rate limiting and traffic shaping
//! - Protocol-level filtering
//! - Network namespace integration
//! - BPF-style filter programs

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Filter action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    Accept,
    Drop,
    Reject,
    Redirect,
    Log,
    RateLimit,
    Mark,
}

/// Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetProtocol {
    Tcp,
    Udp,
    Icmp,
    Icmpv6,
    Sctp,
    Raw,
    Any,
}

/// Direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterDirection {
    Ingress,
    Egress,
    Both,
}

/// IP address (simplified)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpAddr {
    pub octets: [u8; 16],
    pub is_v6: bool,
    pub prefix_len: u8,
}

impl IpAddr {
    pub fn v4(a: u8, b: u8, c: u8, d: u8, prefix: u8) -> Self {
        let mut octets = [0u8; 16];
        octets[0] = a; octets[1] = b; octets[2] = c; octets[3] = d;
        Self { octets, is_v6: false, prefix_len: prefix }
    }

    pub fn any_v4() -> Self { Self::v4(0, 0, 0, 0, 0) }

    pub fn matches(&self, other: &IpAddr) -> bool {
        if self.prefix_len == 0 { return true; }
        let bytes = if self.is_v6 { 16 } else { 4 };
        let full_bytes = (self.prefix_len / 8) as usize;
        if full_bytes > bytes { return false; }
        for i in 0..full_bytes {
            if self.octets[i] != other.octets[i] { return false; }
        }
        let remaining = self.prefix_len % 8;
        if remaining > 0 && full_bytes < bytes {
            let mask = 0xFFu8 << (8 - remaining);
            if (self.octets[full_bytes] & mask) != (other.octets[full_bytes] & mask) { return false; }
        }
        true
    }
}

/// Port range
#[derive(Debug, Clone, Copy)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl PortRange {
    pub fn single(port: u16) -> Self { Self { start: port, end: port } }
    pub fn range(start: u16, end: u16) -> Self { Self { start, end } }
    pub fn any() -> Self { Self { start: 0, end: 65535 } }
    pub fn contains(&self, port: u16) -> bool { port >= self.start && port <= self.end }
}

/// Filter rule
#[derive(Debug, Clone)]
pub struct FilterRule {
    pub id: u64,
    pub name: String,
    pub priority: u16,
    pub direction: FilterDirection,
    pub protocol: NetProtocol,
    pub src_addr: IpAddr,
    pub dst_addr: IpAddr,
    pub src_ports: PortRange,
    pub dst_ports: PortRange,
    pub action: FilterAction,
    pub enabled: bool,
    pub hit_count: u64,
    pub byte_count: u64,
    pub rate_limit_pps: Option<u32>,
    pub log_enabled: bool,
}

impl FilterRule {
    pub fn new(id: u64, name: String, action: FilterAction) -> Self {
        Self {
            id, name, priority: 100, direction: FilterDirection::Both,
            protocol: NetProtocol::Any, src_addr: IpAddr::any_v4(),
            dst_addr: IpAddr::any_v4(), src_ports: PortRange::any(),
            dst_ports: PortRange::any(), action, enabled: true,
            hit_count: 0, byte_count: 0, rate_limit_pps: None, log_enabled: false,
        }
    }

    pub fn matches_packet(&self, proto: NetProtocol, src: &IpAddr, dst: &IpAddr, src_port: u16, dst_port: u16) -> bool {
        if !self.enabled { return false; }
        if self.protocol != NetProtocol::Any && self.protocol != proto { return false; }
        if !self.src_addr.matches(src) { return false; }
        if !self.dst_addr.matches(dst) { return false; }
        if !self.src_ports.contains(src_port) { return false; }
        if !self.dst_ports.contains(dst_port) { return false; }
        true
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    New,
    Established,
    Related,
    TimeWait,
    CloseWait,
    Closed,
}

/// Connection tracking entry
#[derive(Debug, Clone)]
pub struct ConnTrackEntry {
    pub id: u64,
    pub pid: u64,
    pub protocol: NetProtocol,
    pub src_addr: IpAddr,
    pub dst_addr: IpAddr,
    pub src_port: u16,
    pub dst_port: u16,
    pub state: ConnState,
    pub packets_in: u64,
    pub packets_out: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub created_ts: u64,
    pub last_seen_ts: u64,
    pub timeout_ns: u64,
}

impl ConnTrackEntry {
    pub fn new(id: u64, pid: u64, proto: NetProtocol) -> Self {
        Self {
            id, pid, protocol: proto,
            src_addr: IpAddr::any_v4(), dst_addr: IpAddr::any_v4(),
            src_port: 0, dst_port: 0, state: ConnState::New,
            packets_in: 0, packets_out: 0, bytes_in: 0, bytes_out: 0,
            created_ts: 0, last_seen_ts: 0, timeout_ns: 120_000_000_000,
        }
    }

    pub fn is_expired(&self, now: u64) -> bool {
        now.saturating_sub(self.last_seen_ts) > self.timeout_ns
    }

    pub fn total_bytes(&self) -> u64 { self.bytes_in + self.bytes_out }
}

/// Rate limiter (token bucket)
#[derive(Debug, Clone)]
pub struct RateLimiter {
    pub pid: u64,
    pub max_pps: u32,
    pub max_bps: u64,
    pub tokens_pps: u32,
    pub tokens_bps: u64,
    pub last_refill_ts: u64,
    pub dropped_packets: u64,
    pub dropped_bytes: u64,
}

impl RateLimiter {
    pub fn new(pid: u64, max_pps: u32, max_bps: u64) -> Self {
        Self { pid, max_pps, max_bps, tokens_pps: max_pps, tokens_bps: max_bps, last_refill_ts: 0, dropped_packets: 0, dropped_bytes: 0 }
    }

    pub fn refill(&mut self, now: u64) {
        let elapsed_ns = now.saturating_sub(self.last_refill_ts);
        let elapsed_s_frac = elapsed_ns as f64 / 1_000_000_000.0;
        let pps_refill = (self.max_pps as f64 * elapsed_s_frac) as u32;
        let bps_refill = (self.max_bps as f64 * elapsed_s_frac) as u64;
        self.tokens_pps = (self.tokens_pps + pps_refill).min(self.max_pps);
        self.tokens_bps = (self.tokens_bps + bps_refill).min(self.max_bps);
        self.last_refill_ts = now;
    }

    pub fn try_consume(&mut self, bytes: u64) -> bool {
        if self.tokens_pps == 0 || self.tokens_bps < bytes {
            self.dropped_packets += 1;
            self.dropped_bytes += bytes;
            return false;
        }
        self.tokens_pps -= 1;
        self.tokens_bps -= bytes;
        true
    }
}

/// Net filter stats
#[derive(Debug, Clone, Default)]
pub struct NetFilterStats {
    pub total_rules: usize,
    pub active_rules: usize,
    pub total_connections: usize,
    pub active_connections: usize,
    pub total_packets_filtered: u64,
    pub total_bytes_filtered: u64,
    pub total_drops: u64,
    pub total_accepts: u64,
    pub rate_limited_pids: usize,
}

/// Apps network filter
pub struct AppsNetFilter {
    rules: BTreeMap<u64, FilterRule>,
    connections: BTreeMap<u64, ConnTrackEntry>,
    rate_limiters: BTreeMap<u64, RateLimiter>,
    stats: NetFilterStats,
    next_id: u64,
    default_action: FilterAction,
}

impl AppsNetFilter {
    pub fn new(default_action: FilterAction) -> Self {
        Self { rules: BTreeMap::new(), connections: BTreeMap::new(), rate_limiters: BTreeMap::new(), stats: NetFilterStats::default(), next_id: 1, default_action }
    }

    pub fn add_rule(&mut self, name: String, action: FilterAction) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.rules.insert(id, FilterRule::new(id, name, action));
        id
    }

    pub fn remove_rule(&mut self, id: u64) -> bool { self.rules.remove(&id).is_some() }

    pub fn enable_rule(&mut self, id: u64) { if let Some(r) = self.rules.get_mut(&id) { r.enabled = true; } }
    pub fn disable_rule(&mut self, id: u64) { if let Some(r) = self.rules.get_mut(&id) { r.enabled = false; } }

    pub fn evaluate(&mut self, proto: NetProtocol, src: &IpAddr, dst: &IpAddr, src_port: u16, dst_port: u16) -> FilterAction {
        let mut sorted: Vec<&mut FilterRule> = self.rules.values_mut().collect();
        sorted.sort_by_key(|r| r.priority);
        for rule in sorted.iter_mut() {
            if rule.matches_packet(proto, src, dst, src_port, dst_port) {
                rule.hit_count += 1;
                return rule.action;
            }
        }
        self.default_action
    }

    pub fn track_connection(&mut self, pid: u64, proto: NetProtocol) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.connections.insert(id, ConnTrackEntry::new(id, pid, proto));
        id
    }

    pub fn add_rate_limiter(&mut self, pid: u64, max_pps: u32, max_bps: u64) {
        self.rate_limiters.insert(pid, RateLimiter::new(pid, max_pps, max_bps));
    }

    pub fn check_rate_limit(&mut self, pid: u64, bytes: u64, now: u64) -> bool {
        if let Some(rl) = self.rate_limiters.get_mut(&pid) {
            rl.refill(now);
            rl.try_consume(bytes)
        } else {
            true
        }
    }

    pub fn expire_connections(&mut self, now: u64) {
        let expired: Vec<u64> = self.connections.iter()
            .filter(|(_, c)| c.is_expired(now))
            .map(|(&id, _)| id).collect();
        for id in expired { self.connections.remove(&id); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_rules = self.rules.len();
        self.stats.active_rules = self.rules.values().filter(|r| r.enabled).count();
        self.stats.total_connections = self.connections.len();
        self.stats.active_connections = self.connections.values().filter(|c| c.state != ConnState::Closed).count();
        self.stats.total_packets_filtered = self.rules.values().map(|r| r.hit_count).sum();
        self.stats.total_bytes_filtered = self.rules.values().map(|r| r.byte_count).sum();
        self.stats.rate_limited_pids = self.rate_limiters.len();
    }

    pub fn rule(&self, id: u64) -> Option<&FilterRule> { self.rules.get(&id) }
    pub fn connection(&self, id: u64) -> Option<&ConnTrackEntry> { self.connections.get(&id) }
    pub fn stats(&self) -> &NetFilterStats { &self.stats }
}
