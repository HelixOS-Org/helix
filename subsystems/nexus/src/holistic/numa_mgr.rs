// SPDX-License-Identifier: GPL-2.0
//! Holistic numa_mgr — NUMA topology and allocation manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// NUMA distance
#[derive(Debug, Clone, Copy)]
pub struct NumaDistance {
    pub from_node: u32,
    pub to_node: u32,
    pub distance: u32,
}

/// NUMA node
#[derive(Debug)]
pub struct NumaNode {
    pub id: u32,
    pub cpus: Vec<u32>,
    pub total_memory: u64,
    pub free_memory: u64,
    pub local_allocs: u64,
    pub remote_allocs: u64,
    pub migrations: u64,
}

impl NumaNode {
    pub fn new(id: u32, total_mem: u64) -> Self {
        Self { id, cpus: Vec::new(), total_memory: total_mem, free_memory: total_mem, local_allocs: 0, remote_allocs: 0, migrations: 0 }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, cpu: u32) { self.cpus.push(cpu); }
    #[inline(always)]
    pub fn utilization(&self) -> f64 { if self.total_memory == 0 { 0.0 } else { 1.0 - self.free_memory as f64 / self.total_memory as f64 } }
    #[inline(always)]
    pub fn local_ratio(&self) -> f64 { let total = self.local_allocs + self.remote_allocs; if total == 0 { 1.0 } else { self.local_allocs as f64 / total as f64 } }
}

/// NUMA allocation policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaPolicy {
    Default,
    Bind,
    Interleave,
    Preferred,
    Local,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NumaMgrStats {
    pub total_nodes: u32,
    pub total_cpus: u32,
    pub total_memory: u64,
    pub free_memory: u64,
    pub total_local_allocs: u64,
    pub total_remote_allocs: u64,
    pub avg_local_ratio: f64,
}

/// Main NUMA manager
pub struct HolisticNumaMgr {
    nodes: BTreeMap<u32, NumaNode>,
    distances: Vec<NumaDistance>,
    default_policy: NumaPolicy,
}

impl HolisticNumaMgr {
    pub fn new() -> Self { Self { nodes: BTreeMap::new(), distances: Vec::new(), default_policy: NumaPolicy::Default } }

    #[inline(always)]
    pub fn add_node(&mut self, id: u32, total_mem: u64) { self.nodes.insert(id, NumaNode::new(id, total_mem)); }

    #[inline(always)]
    pub fn set_distance(&mut self, from: u32, to: u32, dist: u32) {
        self.distances.push(NumaDistance { from_node: from, to_node: to, distance: dist });
    }

    #[inline(always)]
    pub fn nearest_node(&self, from: u32) -> Option<u32> {
        self.distances.iter().filter(|d| d.from_node == from && d.to_node != from)
            .min_by_key(|d| d.distance).map(|d| d.to_node)
    }

    #[inline]
    pub fn allocate(&mut self, node_id: u32, size: u64, local: bool) -> bool {
        if let Some(n) = self.nodes.get_mut(&node_id) {
            if n.free_memory >= size { n.free_memory -= size; if local { n.local_allocs += 1; } else { n.remote_allocs += 1; } true }
            else { false }
        } else { false }
    }

    #[inline]
    pub fn stats(&self) -> NumaMgrStats {
        let cpus: u32 = self.nodes.values().map(|n| n.cpus.len() as u32).sum();
        let total_mem: u64 = self.nodes.values().map(|n| n.total_memory).sum();
        let free_mem: u64 = self.nodes.values().map(|n| n.free_memory).sum();
        let local: u64 = self.nodes.values().map(|n| n.local_allocs).sum();
        let remote: u64 = self.nodes.values().map(|n| n.remote_allocs).sum();
        let ratios: Vec<f64> = self.nodes.values().map(|n| n.local_ratio()).collect();
        let avg_ratio = if ratios.is_empty() { 1.0 } else { ratios.iter().sum::<f64>() / ratios.len() as f64 };
        NumaMgrStats { total_nodes: self.nodes.len() as u32, total_cpus: cpus, total_memory: total_mem, free_memory: free_mem, total_local_allocs: local, total_remote_allocs: remote, avg_local_ratio: avg_ratio }
    }
}
