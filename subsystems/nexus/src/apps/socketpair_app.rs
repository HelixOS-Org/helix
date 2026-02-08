// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Socketpair (bidirectional socket pairs)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketpairDomain {
    Unix,
    LocalIpc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketpairType {
    Stream,
    Dgram,
    Seqpacket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketpairState {
    Active,
    HalfClosed,
    FullyClosed,
}

#[derive(Debug, Clone)]
pub struct SocketpairInstance {
    pub id: u64,
    pub fd_a: u64,
    pub fd_b: u64,
    pub domain: SocketpairDomain,
    pub pair_type: SocketpairType,
    pub state: SocketpairState,
    pub nonblocking: bool,
    pub cloexec: bool,
    pub bytes_a_to_b: u64,
    pub bytes_b_to_a: u64,
    pub msgs_a_to_b: u64,
    pub msgs_b_to_a: u64,
    pub buf_size: u32,
}

impl SocketpairInstance {
    pub fn new(id: u64, fd_a: u64, fd_b: u64, domain: SocketpairDomain, pair_type: SocketpairType) -> Self {
        Self {
            id, fd_a, fd_b, domain, pair_type,
            state: SocketpairState::Active,
            nonblocking: false, cloexec: false,
            bytes_a_to_b: 0, bytes_b_to_a: 0,
            msgs_a_to_b: 0, msgs_b_to_a: 0,
            buf_size: 65536,
        }
    }

    pub fn send_a_to_b(&mut self, bytes: u64) {
        self.bytes_a_to_b += bytes;
        self.msgs_a_to_b += 1;
    }

    pub fn send_b_to_a(&mut self, bytes: u64) {
        self.bytes_b_to_a += bytes;
        self.msgs_b_to_a += 1;
    }

    pub fn close_fd(&mut self, fd: u64) {
        if fd == self.fd_a || fd == self.fd_b {
            match self.state {
                SocketpairState::Active => self.state = SocketpairState::HalfClosed,
                SocketpairState::HalfClosed => self.state = SocketpairState::FullyClosed,
                _ => {}
            }
        }
    }

    pub fn total_bytes(&self) -> u64 { self.bytes_a_to_b + self.bytes_b_to_a }
    pub fn total_msgs(&self) -> u64 { self.msgs_a_to_b + self.msgs_b_to_a }

    pub fn direction_ratio(&self) -> u64 {
        let total = self.total_bytes();
        if total == 0 { 50 } else { (self.bytes_a_to_b * 100) / total }
    }
}

#[derive(Debug, Clone)]
pub struct SocketpairAppStats {
    pub total_pairs: u64,
    pub active_pairs: u64,
    pub total_bytes: u64,
    pub total_msgs: u64,
}

pub struct AppSocketpair {
    pairs: BTreeMap<u64, SocketpairInstance>,
    fd_to_pair: BTreeMap<u64, u64>,
    next_id: u64,
    stats: SocketpairAppStats,
}

impl AppSocketpair {
    pub fn new() -> Self {
        Self {
            pairs: BTreeMap::new(),
            fd_to_pair: BTreeMap::new(),
            next_id: 1,
            stats: SocketpairAppStats {
                total_pairs: 0, active_pairs: 0,
                total_bytes: 0, total_msgs: 0,
            },
        }
    }

    pub fn create_pair(&mut self, fd_a: u64, fd_b: u64, domain: SocketpairDomain, pair_type: SocketpairType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let inst = SocketpairInstance::new(id, fd_a, fd_b, domain, pair_type);
        self.pairs.insert(id, inst);
        self.fd_to_pair.insert(fd_a, id);
        self.fd_to_pair.insert(fd_b, id);
        self.stats.total_pairs += 1;
        self.stats.active_pairs += 1;
        id
    }

    pub fn close_fd(&mut self, fd: u64) {
        if let Some(&pair_id) = self.fd_to_pair.get(&fd) {
            if let Some(pair) = self.pairs.get_mut(&pair_id) {
                pair.close_fd(fd);
                if pair.state == SocketpairState::FullyClosed {
                    if self.stats.active_pairs > 0 { self.stats.active_pairs -= 1; }
                }
            }
        }
    }

    pub fn stats(&self) -> &SocketpairAppStats { &self.stats }
}
