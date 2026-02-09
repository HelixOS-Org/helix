// SPDX-License-Identifier: GPL-2.0
//! Coop socket — cooperative socket sharing with SO_REUSEPORT and load balancing

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopSocketType {
    Stream,
    Dgram,
    Raw,
    SeqPacket,
}

/// Coop socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopSocketState {
    Created,
    Bound,
    Listening,
    Connected,
    Shared,
    Closed,
}

/// Load balance mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopLbMode {
    RoundRobin,
    Random,
    HashBased,
    LeastConn,
    WeightedRR,
}

/// Socket group for sharing
#[derive(Debug, Clone)]
pub struct CoopSocketGroup {
    pub group_id: u64,
    pub members: Vec<u64>,
    pub lb_mode: CoopLbMode,
    pub rr_index: u64,
    pub total_dispatched: u64,
    pub weights: LinearMap<u32, 64>,
}

impl CoopSocketGroup {
    pub fn new(group_id: u64, lb_mode: CoopLbMode) -> Self {
        Self { group_id, members: Vec::new(), lb_mode, rr_index: 0, total_dispatched: 0, weights: LinearMap::new() }
    }

    #[inline]
    pub fn add_member(&mut self, sock_id: u64, weight: u32) {
        if !self.members.contains(&sock_id) {
            self.members.push(sock_id);
            self.weights.insert(sock_id, weight);
        }
    }

    #[inline(always)]
    pub fn remove_member(&mut self, sock_id: u64) {
        self.members.retain(|&id| id != sock_id);
        self.weights.remove(sock_id);
    }

    pub fn dispatch(&mut self, hash_seed: u64) -> Option<u64> {
        if self.members.is_empty() { return None; }
        let target = match self.lb_mode {
            CoopLbMode::RoundRobin => {
                let idx = self.rr_index as usize % self.members.len();
                self.rr_index += 1;
                self.members[idx]
            }
            CoopLbMode::Random | CoopLbMode::HashBased => {
                let idx = (hash_seed % self.members.len() as u64) as usize;
                self.members[idx]
            }
            _ => self.members[0],
        };
        self.total_dispatched += 1;
        Some(target)
    }
}

/// Coop socket instance
#[derive(Debug, Clone)]
pub struct CoopSocketInstance {
    pub sock_id: u64,
    pub sock_type: CoopSocketType,
    pub state: CoopSocketState,
    pub group_id: Option<u64>,
    pub connections_handled: u64,
    pub bytes_transferred: u64,
}

impl CoopSocketInstance {
    pub fn new(sock_id: u64, sock_type: CoopSocketType) -> Self {
        Self { sock_id, sock_type, state: CoopSocketState::Created, group_id: None, connections_handled: 0, bytes_transferred: 0 }
    }

    #[inline(always)]
    pub fn join_group(&mut self, group_id: u64) {
        self.group_id = Some(group_id);
        self.state = CoopSocketState::Shared;
    }
}

/// Coop socket stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopSocketStats {
    pub total_sockets: u64,
    pub total_groups: u64,
    pub total_dispatched: u64,
    pub shared_sockets: u64,
}

/// Main coop socket manager
#[derive(Debug)]
pub struct CoopSocket {
    pub sockets: BTreeMap<u64, CoopSocketInstance>,
    pub groups: BTreeMap<u64, CoopSocketGroup>,
    pub stats: CoopSocketStats,
}

impl CoopSocket {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            groups: BTreeMap::new(),
            stats: CoopSocketStats { total_sockets: 0, total_groups: 0, total_dispatched: 0, shared_sockets: 0 },
        }
    }

    #[inline(always)]
    pub fn create_socket(&mut self, sock_id: u64, sock_type: CoopSocketType) {
        self.sockets.insert(sock_id, CoopSocketInstance::new(sock_id, sock_type));
        self.stats.total_sockets += 1;
    }

    #[inline(always)]
    pub fn create_group(&mut self, group_id: u64, mode: CoopLbMode) {
        self.groups.insert(group_id, CoopSocketGroup::new(group_id, mode));
        self.stats.total_groups += 1;
    }

    #[inline]
    pub fn join(&mut self, sock_id: u64, group_id: u64) -> bool {
        if let Some(sock) = self.sockets.get_mut(&sock_id) {
            sock.join_group(group_id);
            if let Some(group) = self.groups.get_mut(&group_id) {
                group.add_member(sock_id, 1);
            }
            self.stats.shared_sockets += 1;
            true
        } else { false }
    }
}

// ============================================================================
// Merged from socket_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketPoolCoopEvent { PoolShare, FdMigrate, SocketRecycle, LoadBalance }

/// Socket pool coop record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketPoolCoopRecord {
    pub event: SocketPoolCoopEvent,
    pub pool_size: u32,
    pub active_fds: u32,
    pub recycled: u32,
}

impl SocketPoolCoopRecord {
    pub fn new(event: SocketPoolCoopEvent) -> Self { Self { event, pool_size: 0, active_fds: 0, recycled: 0 } }
}

/// Socket pool coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketPoolCoopStats { pub total_events: u64, pub shares: u64, pub migrations: u64, pub recycled: u64 }

/// Main coop socket v2
#[derive(Debug)]
pub struct CoopSocketV2 { pub stats: SocketPoolCoopStats }

impl CoopSocketV2 {
    pub fn new() -> Self { Self { stats: SocketPoolCoopStats { total_events: 0, shares: 0, migrations: 0, recycled: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &SocketPoolCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            SocketPoolCoopEvent::PoolShare | SocketPoolCoopEvent::LoadBalance => self.stats.shares += 1,
            SocketPoolCoopEvent::FdMigrate => self.stats.migrations += 1,
            SocketPoolCoopEvent::SocketRecycle => self.stats.recycled += rec.recycled as u64,
        }
    }
}
