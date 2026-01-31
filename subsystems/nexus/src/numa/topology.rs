//! NUMA topology detection and management.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use super::types::{CpuId, Distance, NodeId};

// ============================================================================
// NUMA NODE
// ============================================================================

/// NUMA node information
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID
    pub id: NodeId,
    /// CPUs on this node
    pub cpus: Vec<CpuId>,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Free memory (bytes)
    pub free_memory: u64,
    /// Huge pages total
    pub huge_pages_total: u32,
    /// Huge pages free
    pub huge_pages_free: u32,
    /// Local access latency (ns)
    pub local_latency_ns: u32,
}

impl NumaNode {
    /// Create new NUMA node
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            cpus: Vec::new(),
            total_memory: 0,
            free_memory: 0,
            huge_pages_total: 0,
            huge_pages_free: 0,
            local_latency_ns: 100,
        }
    }

    /// Add CPU
    pub fn with_cpu(mut self, cpu: CpuId) -> Self {
        self.cpus.push(cpu);
        self
    }

    /// Add CPUs
    pub fn with_cpus(mut self, cpus: &[CpuId]) -> Self {
        self.cpus.extend_from_slice(cpus);
        self
    }

    /// Set memory
    pub fn with_memory(mut self, total: u64, free: u64) -> Self {
        self.total_memory = total;
        self.free_memory = free;
        self
    }

    /// Memory usage ratio
    pub fn memory_usage(&self) -> f64 {
        if self.total_memory == 0 {
            0.0
        } else {
            1.0 - (self.free_memory as f64 / self.total_memory as f64)
        }
    }

    /// CPU count
    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }

    /// Has CPU?
    pub fn has_cpu(&self, cpu: CpuId) -> bool {
        self.cpus.contains(&cpu)
    }
}

// ============================================================================
// NUMA TOPOLOGY
// ============================================================================

/// NUMA topology information
#[derive(Debug, Clone)]
pub struct NumaTopology {
    /// Number of nodes
    pub node_count: u32,
    /// Node information
    pub nodes: Vec<NumaNode>,
    /// Distance matrix
    pub distances: Vec<Vec<Distance>>,
    /// Inter-node bandwidths
    pub bandwidths: Vec<Vec<u64>>,
}

impl NumaTopology {
    /// Create new topology
    pub fn new(node_count: u32) -> Self {
        let mut nodes = Vec::with_capacity(node_count as usize);
        for i in 0..node_count {
            nodes.push(NumaNode::new(i));
        }

        // Initialize distance matrix with local distance
        let n = node_count as usize;
        let mut distances = vec![vec![10; n]; n];
        for i in 0..n {
            distances[i][i] = 10; // Local is always 10
        }

        let bandwidths = vec![vec![0; n]; n];

        Self {
            node_count,
            nodes,
            distances,
            bandwidths,
        }
    }

    /// Set distance between nodes
    pub fn set_distance(&mut self, from: NodeId, to: NodeId, distance: Distance) {
        let f = from as usize;
        let t = to as usize;
        if f < self.distances.len() && t < self.distances[f].len() {
            self.distances[f][t] = distance;
            self.distances[t][f] = distance; // Symmetric
        }
    }

    /// Get distance between nodes
    pub fn get_distance(&self, from: NodeId, to: NodeId) -> Distance {
        let f = from as usize;
        let t = to as usize;
        if f < self.distances.len() && t < self.distances[f].len() {
            self.distances[f][t]
        } else {
            255 // Unknown
        }
    }

    /// Set bandwidth between nodes
    pub fn set_bandwidth(&mut self, from: NodeId, to: NodeId, bandwidth: u64) {
        let f = from as usize;
        let t = to as usize;
        if f < self.bandwidths.len() && t < self.bandwidths[f].len() {
            self.bandwidths[f][t] = bandwidth;
            self.bandwidths[t][f] = bandwidth; // Symmetric
        }
    }

    /// Get bandwidth between nodes
    pub fn get_bandwidth(&self, from: NodeId, to: NodeId) -> u64 {
        let f = from as usize;
        let t = to as usize;
        if f < self.bandwidths.len() && t < self.bandwidths[f].len() {
            self.bandwidths[f][t]
        } else {
            0
        }
    }

    /// Find node with most free memory
    pub fn node_with_most_memory(&self) -> Option<NodeId> {
        self.nodes
            .iter()
            .max_by_key(|n| n.free_memory)
            .map(|n| n.id)
    }

    /// Find node for CPU
    pub fn node_for_cpu(&self, cpu: CpuId) -> Option<NodeId> {
        self.nodes.iter().find(|n| n.has_cpu(cpu)).map(|n| n.id)
    }

    /// Get nearest nodes (sorted by distance)
    pub fn nearest_nodes(&self, from: NodeId) -> Vec<NodeId> {
        let mut nodes: Vec<_> = (0..self.node_count)
            .filter(|&n| n != from)
            .map(|n| (n, self.get_distance(from, n)))
            .collect();

        nodes.sort_by_key(|(_, d)| *d);
        nodes.into_iter().map(|(n, _)| n).collect()
    }
}
