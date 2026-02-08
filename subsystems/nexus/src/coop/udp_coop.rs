// SPDX-License-Identifier: GPL-2.0
//! Coop UDP — cooperative UDP socket sharing with multicast coordination

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop UDP sharing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopUdpShareMode {
    Exclusive,
    ReusePort,
    ReuseAddr,
    LoadBalance,
    Multicast,
}

/// Coop UDP state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopUdpState {
    Idle,
    Bound,
    Connected,
    Sharing,
    Closed,
}

/// Shared UDP port group
#[derive(Debug, Clone)]
pub struct SharedUdpPort {
    pub port: u16,
    pub members: Vec<u64>,
    pub mode: CoopUdpShareMode,
    pub total_datagrams: u64,
    pub total_bytes: u64,
    pub distribution: BTreeMap<u64, u64>,
}

impl SharedUdpPort {
    pub fn new(port: u16, mode: CoopUdpShareMode) -> Self {
        Self { port, members: Vec::new(), mode, total_datagrams: 0, total_bytes: 0, distribution: BTreeMap::new() }
    }

    pub fn add_member(&mut self, sock_id: u64) {
        if !self.members.contains(&sock_id) {
            self.members.push(sock_id);
            self.distribution.insert(sock_id, 0);
        }
    }

    pub fn remove_member(&mut self, sock_id: u64) {
        self.members.retain(|&id| id != sock_id);
        self.distribution.remove(&sock_id);
    }

    pub fn dispatch(&mut self, dgram_bytes: u64, seed: u64) -> Option<u64> {
        if self.members.is_empty() { return None; }
        let idx = (seed % self.members.len() as u64) as usize;
        let target = self.members[idx];
        self.total_datagrams += 1;
        self.total_bytes += dgram_bytes;
        *self.distribution.entry(target).or_insert(0) += 1;
        Some(target)
    }

    pub fn balance_score(&self) -> f64 {
        if self.members.is_empty() { return 0.0; }
        let avg = self.total_datagrams as f64 / self.members.len() as f64;
        if avg == 0.0 { return 1.0; }
        let variance: f64 = self.distribution.values()
            .map(|&c| { let d = c as f64 - avg; d * d })
            .sum::<f64>() / self.members.len() as f64;
        1.0 / (1.0 + libm::sqrt(variance) / avg)
    }
}

/// Coop UDP socket
#[derive(Debug, Clone)]
pub struct CoopUdpSocket {
    pub sock_id: u64,
    pub state: CoopUdpState,
    pub port: u16,
    pub sent: u64,
    pub received: u64,
    pub drops: u64,
}

impl CoopUdpSocket {
    pub fn new(sock_id: u64) -> Self {
        Self { sock_id, state: CoopUdpState::Idle, port: 0, sent: 0, received: 0, drops: 0 }
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.received + self.drops;
        if total == 0 { 0.0 } else { self.drops as f64 / total as f64 }
    }
}

/// Coop UDP stats
#[derive(Debug, Clone)]
pub struct CoopUdpStats {
    pub total_sockets: u64,
    pub shared_ports: u64,
    pub total_datagrams: u64,
    pub total_drops: u64,
}

/// Main coop UDP manager
#[derive(Debug)]
pub struct CoopUdp {
    pub sockets: BTreeMap<u64, CoopUdpSocket>,
    pub ports: BTreeMap<u16, SharedUdpPort>,
    pub stats: CoopUdpStats,
}

impl CoopUdp {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            ports: BTreeMap::new(),
            stats: CoopUdpStats { total_sockets: 0, shared_ports: 0, total_datagrams: 0, total_drops: 0 },
        }
    }

    pub fn create_socket(&mut self, sock_id: u64) {
        self.sockets.insert(sock_id, CoopUdpSocket::new(sock_id));
        self.stats.total_sockets += 1;
    }

    pub fn share_port(&mut self, port: u16, sock_id: u64, mode: CoopUdpShareMode) {
        let group = self.ports.entry(port).or_insert_with(|| {
            self.stats.shared_ports += 1;
            SharedUdpPort::new(port, mode)
        });
        group.add_member(sock_id);
    }
}

// ============================================================================
// Merged from udp_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpCoopV2Event { MulticastGroup, BroadcastShare, PortReuse, BufferPool }

/// UDP coop record
#[derive(Debug, Clone)]
pub struct UdpCoopV2Record {
    pub event: UdpCoopV2Event,
    pub group_members: u32,
    pub datagrams: u64,
    pub bytes: u64,
}

impl UdpCoopV2Record {
    pub fn new(event: UdpCoopV2Event) -> Self { Self { event, group_members: 0, datagrams: 0, bytes: 0 } }
}

/// UDP coop stats
#[derive(Debug, Clone)]
pub struct UdpCoopV2Stats { pub total_events: u64, pub multicasts: u64, pub port_reuses: u64, pub bytes_pooled: u64 }

/// Main coop UDP v2
#[derive(Debug)]
pub struct CoopUdpV2 { pub stats: UdpCoopV2Stats }

impl CoopUdpV2 {
    pub fn new() -> Self { Self { stats: UdpCoopV2Stats { total_events: 0, multicasts: 0, port_reuses: 0, bytes_pooled: 0 } } }
    pub fn record(&mut self, rec: &UdpCoopV2Record) {
        self.stats.total_events += 1;
        match rec.event {
            UdpCoopV2Event::MulticastGroup | UdpCoopV2Event::BroadcastShare => self.stats.multicasts += 1,
            UdpCoopV2Event::PortReuse => self.stats.port_reuses += 1,
            UdpCoopV2Event::BufferPool => self.stats.bytes_pooled += rec.bytes,
        }
    }
}
