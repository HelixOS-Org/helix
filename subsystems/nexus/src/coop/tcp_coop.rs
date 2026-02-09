// SPDX-License-Identifier: GPL-2.0
//! Coop TCP — cooperative TCP connection management with shared congestion state

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop TCP congestion algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopTcpCongestion {
    Reno,
    Cubic,
    Bbr,
    BbrV2,
    Vegas,
    Westwood,
    Dctcp,
    Ecn,
    Shared,
}

/// Coop TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopTcpState {
    Idle,
    SynSent,
    Established,
    CloseWait,
    FinWait,
    TimeWait,
    Closed,
    Sharing,
}

/// Shared congestion state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SharedCwndState {
    pub group_id: u64,
    pub total_cwnd: u64,
    pub member_count: u32,
    pub total_rtt_us: u64,
    pub rtt_samples: u64,
    pub total_loss_events: u64,
}

impl SharedCwndState {
    pub fn new(group_id: u64) -> Self {
        Self { group_id, total_cwnd: 0, member_count: 0, total_rtt_us: 0, rtt_samples: 0, total_loss_events: 0 }
    }

    #[inline(always)]
    pub fn avg_rtt_us(&self) -> u64 {
        if self.rtt_samples == 0 { 0 } else { self.total_rtt_us / self.rtt_samples }
    }

    #[inline(always)]
    pub fn fair_share_cwnd(&self) -> u64 {
        if self.member_count == 0 { 0 } else { self.total_cwnd / self.member_count as u64 }
    }

    #[inline(always)]
    pub fn record_rtt(&mut self, rtt_us: u64) {
        self.total_rtt_us += rtt_us;
        self.rtt_samples += 1;
    }

    #[inline(always)]
    pub fn record_loss(&mut self) {
        self.total_loss_events += 1;
    }
}

/// Coop TCP connection
#[derive(Debug, Clone)]
pub struct CoopTcpConnection {
    pub conn_id: u64,
    pub state: CoopTcpState,
    pub congestion: CoopTcpCongestion,
    pub cwnd: u64,
    pub ssthresh: u64,
    pub srtt_us: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub retransmits: u64,
    pub group_id: Option<u64>,
}

impl CoopTcpConnection {
    pub fn new(conn_id: u64) -> Self {
        Self {
            conn_id, state: CoopTcpState::Idle, congestion: CoopTcpCongestion::Cubic,
            cwnd: 10, ssthresh: u64::MAX, srtt_us: 0, bytes_sent: 0, bytes_received: 0,
            retransmits: 0, group_id: None,
        }
    }

    #[inline(always)]
    pub fn send(&mut self, bytes: u64) { self.bytes_sent += bytes; }
    #[inline(always)]
    pub fn receive(&mut self, bytes: u64) { self.bytes_received += bytes; }
    #[inline(always)]
    pub fn retransmit(&mut self) { self.retransmits += 1; self.ssthresh = self.cwnd / 2; self.cwnd = self.ssthresh; }
    #[inline(always)]
    pub fn join_group(&mut self, gid: u64) { self.group_id = Some(gid); self.state = CoopTcpState::Sharing; }
    #[inline(always)]
    pub fn retransmit_rate(&self) -> f64 {
        if self.bytes_sent == 0 { 0.0 } else { self.retransmits as f64 / (self.bytes_sent / 1460) as f64 }
    }
}

/// Coop TCP stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopTcpStats {
    pub total_connections: u64,
    pub shared_connections: u64,
    pub total_bytes_sent: u64,
    pub total_retransmits: u64,
}

/// Main coop TCP manager
#[derive(Debug)]
pub struct CoopTcp {
    pub connections: BTreeMap<u64, CoopTcpConnection>,
    pub groups: BTreeMap<u64, SharedCwndState>,
    pub stats: CoopTcpStats,
}

impl CoopTcp {
    pub fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
            groups: BTreeMap::new(),
            stats: CoopTcpStats { total_connections: 0, shared_connections: 0, total_bytes_sent: 0, total_retransmits: 0 },
        }
    }

    #[inline(always)]
    pub fn create_connection(&mut self, conn_id: u64) {
        self.connections.insert(conn_id, CoopTcpConnection::new(conn_id));
        self.stats.total_connections += 1;
    }

    #[inline(always)]
    pub fn create_group(&mut self, group_id: u64) {
        self.groups.insert(group_id, SharedCwndState::new(group_id));
    }

    #[inline]
    pub fn join_group(&mut self, conn_id: u64, group_id: u64) -> bool {
        if let Some(conn) = self.connections.get_mut(&conn_id) {
            conn.join_group(group_id);
            if let Some(group) = self.groups.get_mut(&group_id) {
                group.member_count += 1;
                group.total_cwnd += conn.cwnd;
            }
            self.stats.shared_connections += 1;
            true
        } else { false }
    }
}

// ============================================================================
// Merged from tcp_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpCoopV2Event { ConnPool, WindowShare, CongestionSync, KeepAliveGroup }

/// TCP coop record
#[derive(Debug, Clone)]
pub struct TcpCoopV2Record {
    pub event: TcpCoopV2Event,
    pub connections: u32,
    pub window_size: u32,
    pub rtt_us: u32,
}

impl TcpCoopV2Record {
    pub fn new(event: TcpCoopV2Event) -> Self { Self { event, connections: 0, window_size: 0, rtt_us: 0 } }
}

/// TCP coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TcpCoopV2Stats { pub total_events: u64, pub pooled: u64, pub syncs: u64, pub keepalives: u64 }

/// Main coop TCP v2
#[derive(Debug)]
pub struct CoopTcpV2 { pub stats: TcpCoopV2Stats }

impl CoopTcpV2 {
    pub fn new() -> Self { Self { stats: TcpCoopV2Stats { total_events: 0, pooled: 0, syncs: 0, keepalives: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &TcpCoopV2Record) {
        self.stats.total_events += 1;
        match rec.event {
            TcpCoopV2Event::ConnPool => self.stats.pooled += 1,
            TcpCoopV2Event::WindowShare | TcpCoopV2Event::CongestionSync => self.stats.syncs += 1,
            TcpCoopV2Event::KeepAliveGroup => self.stats.keepalives += 1,
        }
    }
}
