// SPDX-License-Identifier: GPL-2.0
//! Coop netns — cooperative network namespace management with shared interfaces

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop netns state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopNetnsState {
    Active,
    Migrating,
    Shared,
    Suspended,
    Destroyed,
}

/// Coop veth state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopVethState {
    Down,
    Up,
    Bridged,
    Failed,
}

/// Cooperative veth pair
#[derive(Debug, Clone)]
pub struct CoopVethPair {
    pub pair_id: u64,
    pub ns_a: u64,
    pub ns_b: u64,
    pub state: CoopVethState,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub drops: u64,
}

impl CoopVethPair {
    pub fn new(pair_id: u64, ns_a: u64, ns_b: u64) -> Self {
        Self {
            pair_id, ns_a, ns_b, state: CoopVethState::Down,
            tx_bytes: 0, rx_bytes: 0, tx_packets: 0, rx_packets: 0, drops: 0,
        }
    }

    #[inline(always)]
    pub fn bring_up(&mut self) { self.state = CoopVethState::Up; }
    #[inline(always)]
    pub fn transmit(&mut self, bytes: u64) { self.tx_bytes += bytes; self.tx_packets += 1; }
    #[inline(always)]
    pub fn receive(&mut self, bytes: u64) { self.rx_bytes += bytes; self.rx_packets += 1; }
    #[inline(always)]
    pub fn drop_pkt(&mut self) { self.drops += 1; }

    #[inline(always)]
    pub fn total_throughput(&self) -> u64 { self.tx_bytes + self.rx_bytes }
    #[inline(always)]
    pub fn drop_rate(&self) -> f64 {
        let total = self.tx_packets + self.rx_packets + self.drops;
        if total == 0 { 0.0 } else { self.drops as f64 / total as f64 }
    }
}

/// Cooperative network namespace
#[derive(Debug, Clone)]
pub struct CoopNetNamespace {
    pub ns_id: u64,
    pub name_hash: u64,
    pub state: CoopNetnsState,
    pub interfaces: Vec<u64>,
    pub veth_pairs: Vec<u64>,
    pub shared_with: Vec<u64>,
    pub process_count: u32,
}

impl CoopNetNamespace {
    pub fn new(ns_id: u64, name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            ns_id, name_hash: h, state: CoopNetnsState::Active,
            interfaces: Vec::new(), veth_pairs: Vec::new(), shared_with: Vec::new(), process_count: 0,
        }
    }

    #[inline(always)]
    pub fn add_interface(&mut self, if_id: u64) {
        if !self.interfaces.contains(&if_id) { self.interfaces.push(if_id); }
    }

    #[inline]
    pub fn share_with(&mut self, other_ns: u64) {
        if !self.shared_with.contains(&other_ns) {
            self.shared_with.push(other_ns);
            self.state = CoopNetnsState::Shared;
        }
    }

    #[inline(always)]
    pub fn attach_process(&mut self) { self.process_count += 1; }
    #[inline(always)]
    pub fn detach_process(&mut self) { self.process_count = self.process_count.saturating_sub(1); }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.process_count == 0 }

    #[inline(always)]
    pub fn destroy(&mut self) { self.state = CoopNetnsState::Destroyed; }
}

/// Coop netns stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopNetnsStats {
    pub total_namespaces: u64,
    pub shared_namespaces: u64,
    pub total_veth_pairs: u64,
    pub total_migrations: u64,
}

/// Main coop netns manager
#[derive(Debug)]
pub struct CoopNetns {
    pub namespaces: BTreeMap<u64, CoopNetNamespace>,
    pub veth_pairs: BTreeMap<u64, CoopVethPair>,
    pub stats: CoopNetnsStats,
}

impl CoopNetns {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            veth_pairs: BTreeMap::new(),
            stats: CoopNetnsStats { total_namespaces: 0, shared_namespaces: 0, total_veth_pairs: 0, total_migrations: 0 },
        }
    }

    #[inline(always)]
    pub fn create_namespace(&mut self, ns_id: u64, name: &[u8]) {
        self.namespaces.insert(ns_id, CoopNetNamespace::new(ns_id, name));
        self.stats.total_namespaces += 1;
    }

    #[inline]
    pub fn create_veth_pair(&mut self, pair_id: u64, ns_a: u64, ns_b: u64) {
        self.veth_pairs.insert(pair_id, CoopVethPair::new(pair_id, ns_a, ns_b));
        if let Some(ns) = self.namespaces.get_mut(&ns_a) { ns.veth_pairs.push(pair_id); }
        if let Some(ns) = self.namespaces.get_mut(&ns_b) { ns.veth_pairs.push(pair_id); }
        self.stats.total_veth_pairs += 1;
    }

    #[inline]
    pub fn share_namespaces(&mut self, ns_a: u64, ns_b: u64) {
        if let Some(ns) = self.namespaces.get_mut(&ns_a) { ns.share_with(ns_b); }
        if let Some(ns) = self.namespaces.get_mut(&ns_b) { ns.share_with(ns_a); }
        self.stats.shared_namespaces += 1;
    }

    #[inline]
    pub fn destroy_namespace(&mut self, ns_id: u64) -> bool {
        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            if ns.is_empty() { ns.destroy(); true } else { false }
        } else { false }
    }
}
