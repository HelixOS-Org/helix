// SPDX-License-Identifier: GPL-2.0
//! Holistic numa_policy — NUMA memory allocation and placement policies.

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// NUMA policy type (mirrors Linux mbind/set_mempolicy)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaPolicy {
    Default,
    Preferred,
    Bind,
    Interleave,
    Local,
    WeightedInterleave,
}

/// NUMA node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Online,
    HasMemory,
    HasCpu,
    Offline,
    Draining,
}

/// Memory migration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationMode {
    Lazy,
    Eager,
    OnFault,
    Background,
    Disabled,
}

/// NUMA distance entry (node-to-node latency)
#[derive(Debug, Clone, Copy)]
pub struct NumaDistance {
    pub from_node: u32,
    pub to_node: u32,
    pub distance: u32,
}

/// NUMA node memory info
#[derive(Debug, Clone)]
pub struct NodeMemInfo {
    pub node_id: u32,
    pub total_pages: u64,
    pub free_pages: u64,
    pub active_pages: u64,
    pub inactive_pages: u64,
    pub dirty_pages: u64,
    pub slab_pages: u64,
    pub huge_pages_total: u64,
    pub huge_pages_free: u64,
}

impl NodeMemInfo {
    pub fn new(node_id: u32, total_pages: u64) -> Self {
        Self {
            node_id, total_pages, free_pages: total_pages,
            active_pages: 0, inactive_pages: 0, dirty_pages: 0,
            slab_pages: 0, huge_pages_total: 0, huge_pages_free: 0,
        }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.total_pages == 0 { return 0.0; }
        1.0 - (self.free_pages as f64 / self.total_pages as f64)
    }

    #[inline]
    pub fn pressure(&self) -> f64 {
        if self.total_pages == 0 { return 1.0; }
        let used = self.total_pages.saturating_sub(self.free_pages);
        used as f64 / self.total_pages as f64
    }
}

/// Process NUMA binding
#[derive(Debug, Clone)]
pub struct ProcessNumaBinding {
    pub pid: u64,
    pub policy: NumaPolicy,
    pub preferred_node: Option<u32>,
    pub node_mask: u64,
    pub migration_mode: MigrationMode,
    pub pages_local: u64,
    pub pages_remote: u64,
    pub pages_migrated: u64,
    pub migration_failures: u64,
}

impl ProcessNumaBinding {
    pub fn new(pid: u64, policy: NumaPolicy) -> Self {
        Self {
            pid, policy, preferred_node: None, node_mask: u64::MAX,
            migration_mode: MigrationMode::Lazy,
            pages_local: 0, pages_remote: 0, pages_migrated: 0,
            migration_failures: 0,
        }
    }

    #[inline]
    pub fn locality_ratio(&self) -> f64 {
        let total = self.pages_local + self.pages_remote;
        if total == 0 { return 1.0; }
        self.pages_local as f64 / total as f64
    }

    #[inline(always)]
    pub fn is_node_allowed(&self, node: u32) -> bool {
        if node >= 64 { return false; }
        (self.node_mask >> node) & 1 != 0
    }
}

/// NUMA balancing event
#[derive(Debug, Clone)]
pub struct NumaBalanceEvent {
    pub pid: u64,
    pub from_node: u32,
    pub to_node: u32,
    pub pages: u64,
    pub timestamp: u64,
    pub latency_ns: u64,
}

/// NUMA policy stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NumaPolicyStats {
    pub total_nodes: u32,
    pub online_nodes: u32,
    pub total_bindings: u32,
    pub total_migrations: u64,
    pub avg_locality: f64,
    pub total_remote_accesses: u64,
    pub node_pressure: ArrayMap<f64, 32>,
}

/// Main NUMA policy manager
pub struct HolisticNumaPolicy {
    nodes: BTreeMap<u32, NodeMemInfo>,
    distances: Vec<NumaDistance>,
    bindings: BTreeMap<u64, ProcessNumaBinding>,
    events: Vec<NumaBalanceEvent>,
    max_events: usize,
}

impl HolisticNumaPolicy {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(), distances: Vec::new(),
            bindings: BTreeMap::new(), events: Vec::new(),
            max_events: 8192,
        }
    }

    #[inline(always)]
    pub fn add_node(&mut self, node_id: u32, total_pages: u64) {
        self.nodes.insert(node_id, NodeMemInfo::new(node_id, total_pages));
    }

    #[inline(always)]
    pub fn set_distance(&mut self, from: u32, to: u32, distance: u32) {
        self.distances.push(NumaDistance { from_node: from, to_node: to, distance });
    }

    #[inline]
    pub fn get_distance(&self, from: u32, to: u32) -> u32 {
        if from == to { return 10; } // local distance
        self.distances.iter()
            .find(|d| d.from_node == from && d.to_node == to)
            .map(|d| d.distance).unwrap_or(255)
    }

    #[inline]
    pub fn bind_process(&mut self, pid: u64, policy: NumaPolicy, preferred: Option<u32>) {
        let mut binding = ProcessNumaBinding::new(pid, policy);
        binding.preferred_node = preferred;
        self.bindings.insert(pid, binding);
    }

    #[inline(always)]
    pub fn unbind_process(&mut self, pid: u64) -> bool { self.bindings.remove(&pid).is_some() }

    pub fn select_node(&self, pid: u64) -> Option<u32> {
        let binding = self.bindings.get(&pid)?;
        match binding.policy {
            NumaPolicy::Preferred => binding.preferred_node,
            NumaPolicy::Local => Some(0), // placeholder
            NumaPolicy::Bind => {
                // Select least-loaded allowed node
                self.nodes.iter()
                    .filter(|(&id, _)| binding.is_node_allowed(id))
                    .min_by(|(_, a), (_, b)| a.pressure().partial_cmp(&b.pressure()).unwrap_or(core::cmp::Ordering::Equal))
                    .map(|(&id, _)| id)
            }
            NumaPolicy::Interleave => {
                // Simple round-robin
                let allowed: Vec<u32> = self.nodes.keys()
                    .filter(|&&id| binding.is_node_allowed(id))
                    .copied().collect();
                if allowed.is_empty() { None }
                else { Some(allowed[(binding.pages_local + binding.pages_remote) as usize % allowed.len()]) }
            }
            _ => self.nodes.keys().next().copied(),
        }
    }

    #[inline]
    pub fn record_migration(&mut self, pid: u64, from: u32, to: u32, pages: u64, lat_ns: u64, now: u64) {
        if let Some(b) = self.bindings.get_mut(&pid) { b.pages_migrated += pages; }
        if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 4); }
        self.events.push(NumaBalanceEvent { pid, from_node: from, to_node: to, pages, timestamp: now, latency_ns: lat_ns });
    }

    pub fn stats(&self) -> NumaPolicyStats {
        let online = self.nodes.len() as u32;
        let total_mig: u64 = self.bindings.values().map(|b| b.pages_migrated).sum();
        let remote: u64 = self.bindings.values().map(|b| b.pages_remote).sum();
        let localities: Vec<f64> = self.bindings.values().map(|b| b.locality_ratio()).collect();
        let avg_loc = if localities.is_empty() { 1.0 } else { localities.iter().sum::<f64>() / localities.len() as f64 };
        let mut node_pressure = BTreeMap::new();
        for (&id, info) in &self.nodes { node_pressure.insert(id, info.pressure()); }
        NumaPolicyStats {
            total_nodes: self.nodes.len() as u32, online_nodes: online,
            total_bindings: self.bindings.len() as u32,
            total_migrations: total_mig, avg_locality: avg_loc,
            total_remote_accesses: remote, node_pressure,
        }
    }
}
