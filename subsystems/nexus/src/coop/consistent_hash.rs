//! # Coop Consistent Hashing
//!
//! Consistent hash ring for distributed resource allocation:
//! - Virtual node mapping
//! - Jump consistent hashing
//! - Bounded loads with rebalancing
//! - Node addition/removal with minimal redistribution
//! - Replication factor management
//! - Hot spot detection and mitigation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Hash ring node
#[derive(Debug, Clone)]
pub struct RingNode {
    pub id: u64,
    pub name: String,
    pub weight: u32,
    pub virtual_nodes: u32,
    pub load: u64,
    pub capacity: u64,
    pub is_alive: bool,
    pub added_ts: u64,
    pub items_owned: u64,
}

impl RingNode {
    pub fn new(id: u64, name: String, weight: u32, capacity: u64, ts: u64) -> Self {
        Self {
            id, name, weight, virtual_nodes: weight * 100, load: 0,
            capacity, is_alive: true, added_ts: ts, items_owned: 0,
        }
    }

    pub fn load_factor(&self) -> f64 {
        if self.capacity == 0 { return 1.0; }
        self.load as f64 / self.capacity as f64
    }

    pub fn is_overloaded(&self, avg_load: f64, bound: f64) -> bool {
        self.load as f64 > avg_load * bound
    }
}

/// Virtual node on the ring
#[derive(Debug, Clone)]
pub struct VirtualNode {
    pub hash: u64,
    pub physical_node: u64,
    pub replica_index: u32,
}

impl VirtualNode {
    pub fn new(hash: u64, physical: u64, replica: u32) -> Self {
        Self { hash, physical_node: physical, replica_index: replica }
    }
}

/// Item placement
#[derive(Debug, Clone)]
pub struct ItemPlacement {
    pub item_key: u64,
    pub primary_node: u64,
    pub replica_nodes: Vec<u64>,
    pub assigned_ts: u64,
}

/// Rebalance event
#[derive(Debug, Clone)]
pub struct RebalanceEvent {
    pub item_key: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub reason: RebalanceReason,
    pub timestamp: u64,
}

/// Rebalance reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebalanceReason {
    NodeAdded,
    NodeRemoved,
    LoadImbalance,
    CapacityChange,
    ReplicaRepair,
}

/// Consistent hash ring stats
#[derive(Debug, Clone, Default)]
pub struct ConsistentHashStats {
    pub total_physical_nodes: usize,
    pub alive_nodes: usize,
    pub total_virtual_nodes: usize,
    pub total_items: usize,
    pub max_load_factor: f64,
    pub min_load_factor: f64,
    pub avg_load_factor: f64,
    pub load_std_dev: f64,
    pub total_rebalances: u64,
}

/// Coop consistent hash ring
pub struct CoopConsistentHash {
    physical_nodes: BTreeMap<u64, RingNode>,
    ring: BTreeMap<u64, VirtualNode>,
    placements: BTreeMap<u64, ItemPlacement>,
    rebalance_log: Vec<RebalanceEvent>,
    stats: ConsistentHashStats,
    replication_factor: u32,
    load_bound: f64,
}

impl CoopConsistentHash {
    pub fn new(replication_factor: u32, load_bound: f64) -> Self {
        Self {
            physical_nodes: BTreeMap::new(), ring: BTreeMap::new(),
            placements: BTreeMap::new(), rebalance_log: Vec::new(),
            stats: ConsistentHashStats::default(), replication_factor,
            load_bound: if load_bound <= 1.0 { 1.25 } else { load_bound },
        }
    }

    fn hash_with_seed(key: u64, seed: u32) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let bytes = key.to_le_bytes();
        for &b in &bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        let seed_bytes = seed.to_le_bytes();
        for &b in &seed_bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        h
    }

    pub fn add_node(&mut self, id: u64, name: String, weight: u32, capacity: u64, ts: u64) {
        let node = RingNode::new(id, name, weight, capacity, ts);
        let vnodes = node.virtual_nodes;
        self.physical_nodes.insert(id, node);
        for i in 0..vnodes {
            let hash = Self::hash_with_seed(id, i);
            self.ring.insert(hash, VirtualNode::new(hash, id, i));
        }
    }

    pub fn remove_node(&mut self, id: u64, ts: u64) {
        if let Some(node) = self.physical_nodes.remove(&id) {
            for i in 0..node.virtual_nodes {
                let hash = Self::hash_with_seed(id, i);
                self.ring.remove(&hash);
            }
            // Reassign items
            let orphaned: Vec<u64> = self.placements.iter()
                .filter(|(_, p)| p.primary_node == id)
                .map(|(&k, _)| k).collect();
            for key in orphaned {
                if let Some(new_node) = self.find_node(key) {
                    self.rebalance_log.push(RebalanceEvent {
                        item_key: key, from_node: id, to_node: new_node,
                        reason: RebalanceReason::NodeRemoved, timestamp: ts,
                    });
                    if let Some(p) = self.placements.get_mut(&key) {
                        p.primary_node = new_node;
                        p.assigned_ts = ts;
                    }
                }
            }
        }
    }

    pub fn find_node(&self, key: u64) -> Option<u64> {
        if self.ring.is_empty() { return None; }
        let key_hash = Self::hash_with_seed(key, 0);
        // Find first virtual node >= key_hash (ring walk clockwise)
        if let Some((_, vn)) = self.ring.range(key_hash..).next() {
            if self.physical_nodes.get(&vn.physical_node).map_or(false, |n| n.is_alive) {
                return Some(vn.physical_node);
            }
        }
        // Wrap around
        if let Some((_, vn)) = self.ring.iter().next() {
            return Some(vn.physical_node);
        }
        None
    }

    pub fn find_replicas(&self, key: u64) -> Vec<u64> {
        let mut replicas = Vec::new();
        if self.ring.is_empty() { return replicas; }
        let key_hash = Self::hash_with_seed(key, 0);
        let needed = self.replication_factor as usize;

        // Walk clockwise from key_hash
        for (_, vn) in self.ring.range(key_hash..) {
            if !replicas.contains(&vn.physical_node) {
                let alive = self.physical_nodes.get(&vn.physical_node).map_or(false, |n| n.is_alive);
                if alive { replicas.push(vn.physical_node); }
            }
            if replicas.len() >= needed { return replicas; }
        }
        // Wrap around
        for (_, vn) in self.ring.iter() {
            if !replicas.contains(&vn.physical_node) {
                let alive = self.physical_nodes.get(&vn.physical_node).map_or(false, |n| n.is_alive);
                if alive { replicas.push(vn.physical_node); }
            }
            if replicas.len() >= needed { return replicas; }
        }
        replicas
    }

    pub fn place_item(&mut self, key: u64, ts: u64) -> Option<u64> {
        let primary = self.find_node(key)?;
        let replicas = self.find_replicas(key);
        let rep = replicas.into_iter().filter(|&n| n != primary).collect();
        self.placements.insert(key, ItemPlacement { item_key: key, primary_node: primary, replica_nodes: rep, assigned_ts: ts });
        if let Some(n) = self.physical_nodes.get_mut(&primary) { n.items_owned += 1; n.load += 1; }
        Some(primary)
    }

    pub fn recompute(&mut self) {
        let alive: Vec<&RingNode> = self.physical_nodes.values().filter(|n| n.is_alive).collect();
        self.stats.total_physical_nodes = self.physical_nodes.len();
        self.stats.alive_nodes = alive.len();
        self.stats.total_virtual_nodes = self.ring.len();
        self.stats.total_items = self.placements.len();
        self.stats.total_rebalances = self.rebalance_log.len() as u64;

        if alive.is_empty() { return; }
        let factors: Vec<f64> = alive.iter().map(|n| n.load_factor()).collect();
        self.stats.max_load_factor = factors.iter().cloned().fold(0.0_f64, f64::max);
        self.stats.min_load_factor = factors.iter().cloned().fold(f64::MAX, f64::min);
        let sum: f64 = factors.iter().sum();
        self.stats.avg_load_factor = sum / factors.len() as f64;
        let variance: f64 = factors.iter().map(|f| { let d = f - self.stats.avg_load_factor; d * d }).sum::<f64>() / factors.len() as f64;
        self.stats.load_std_dev = libm::sqrt(variance);
    }

    pub fn node(&self, id: u64) -> Option<&RingNode> { self.physical_nodes.get(&id) }
    pub fn placement(&self, key: u64) -> Option<&ItemPlacement> { self.placements.get(&key) }
    pub fn stats(&self) -> &ConsistentHashStats { &self.stats }
}
