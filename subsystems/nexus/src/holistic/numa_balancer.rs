//! # Holistic NUMA Balancer
//!
//! NUMA-aware memory and task balancing:
//! - Per-node memory pressure tracking
//! - Automatic page migration on access patterns
//! - Task-to-node affinity scoring
//! - Interconnect bandwidth monitoring
//! - Distance-aware load balancing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use libm::sqrt;

// ============================================================================
// NUMA TYPES
// ============================================================================

/// NUMA node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaNodeState {
    /// Normal operation
    Normal,
    /// Under memory pressure
    Pressured,
    /// Overcommitted
    Overcommitted,
    /// Offline
    Offline,
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaMigReason {
    /// Access locality (page faulted from remote)
    AccessLocality,
    /// Memory pressure on source node
    MemoryPressure,
    /// Rebalancing
    Rebalance,
    /// Explicit hint
    Hint,
}

// ============================================================================
// NUMA NODE
// ============================================================================

/// NUMA node info
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID
    pub node_id: u32,
    /// State
    pub state: NumaNodeState,
    /// Total memory (pages)
    pub total_pages: u64,
    /// Free pages
    pub free_pages: u64,
    /// Active tasks
    pub active_tasks: u32,
    /// Remote access count (this period)
    pub remote_accesses: u64,
    /// Local access count (this period)
    pub local_accesses: u64,
    /// CPU capacity (abstract units)
    pub cpu_capacity: u32,
    /// CPU utilization (0..1)
    pub cpu_utilization: f64,
    /// Memory pressure EMA (0..1)
    pub mem_pressure_ema: f64,
}

impl NumaNode {
    pub fn new(node_id: u32, total_pages: u64, cpu_capacity: u32) -> Self {
        Self {
            node_id,
            state: NumaNodeState::Normal,
            total_pages,
            free_pages: total_pages,
            active_tasks: 0,
            remote_accesses: 0,
            local_accesses: 0,
            cpu_capacity,
            cpu_utilization: 0.0,
            mem_pressure_ema: 0.0,
        }
    }

    /// Memory utilization
    #[inline]
    pub fn mem_utilization(&self) -> f64 {
        if self.total_pages == 0 {
            return 0.0;
        }
        1.0 - (self.free_pages as f64 / self.total_pages as f64)
    }

    /// Locality ratio (local / total accesses)
    #[inline]
    pub fn locality_ratio(&self) -> f64 {
        let total = self.local_accesses + self.remote_accesses;
        if total == 0 {
            return 1.0;
        }
        self.local_accesses as f64 / total as f64
    }

    /// Update pressure EMA
    #[inline]
    pub fn update_pressure(&mut self) {
        let current = self.mem_utilization();
        self.mem_pressure_ema = 0.8 * self.mem_pressure_ema + 0.2 * current;

        self.state = if self.mem_pressure_ema > 0.95 {
            NumaNodeState::Overcommitted
        } else if self.mem_pressure_ema > 0.8 {
            NumaNodeState::Pressured
        } else {
            NumaNodeState::Normal
        };
    }

    /// Score for placing a task (lower is better)
    #[inline]
    pub fn placement_score(&self, distance: u32) -> f64 {
        let mem_factor = self.mem_utilization();
        let cpu_factor = self.cpu_utilization;
        let distance_factor = distance as f64 / 10.0;
        mem_factor * 0.4 + cpu_factor * 0.4 + distance_factor * 0.2
    }
}

// ============================================================================
// DISTANCE MATRIX
// ============================================================================

/// NUMA distance matrix
#[derive(Debug)]
pub struct NumaDistanceMatrix {
    /// Distances (node_a * max_nodes + node_b -> distance)
    distances: Vec<u32>,
    /// Number of nodes
    node_count: usize,
}

impl NumaDistanceMatrix {
    pub fn new(node_count: usize) -> Self {
        let mut distances = alloc::vec![10; node_count * node_count];
        // Default: local=10, remote=20
        for i in 0..node_count {
            for j in 0..node_count {
                distances[i * node_count + j] = if i == j { 10 } else { 20 };
            }
        }
        Self { distances, node_count }
    }

    /// Set distance
    #[inline]
    pub fn set_distance(&mut self, from: usize, to: usize, dist: u32) {
        if from < self.node_count && to < self.node_count {
            self.distances[from * self.node_count + to] = dist;
            self.distances[to * self.node_count + from] = dist;
        }
    }

    /// Get distance
    #[inline]
    pub fn distance(&self, from: usize, to: usize) -> u32 {
        if from < self.node_count && to < self.node_count {
            self.distances[from * self.node_count + to]
        } else {
            u32::MAX
        }
    }

    /// Find nearest node with capacity
    #[inline]
    pub fn nearest_with_capacity(&self, from: usize, nodes: &BTreeMap<u32, NumaNode>) -> Option<u32> {
        let mut candidates: Vec<(u32, u32)> = Vec::new();
        for (&nid, node) in nodes {
            if nid as usize != from && node.free_pages > 0 && node.state != NumaNodeState::Offline {
                let dist = self.distance(from, nid as usize);
                candidates.push((nid, dist));
            }
        }
        candidates.sort_by_key(|c| c.1);
        candidates.first().map(|c| c.0)
    }
}

// ============================================================================
// MIGRATION ENTRY
// ============================================================================

/// Page migration entry
#[derive(Debug, Clone)]
pub struct NumaPageMigration {
    /// Page frame number
    pub pfn: u64,
    /// Source node
    pub src_node: u32,
    /// Destination node
    pub dst_node: u32,
    /// Reason
    pub reason: NumaMigReason,
    /// Distance between nodes
    pub distance: u32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// NUMA balancer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticNumaBalancerStats {
    /// Active nodes
    pub active_nodes: usize,
    /// Pressured nodes
    pub pressured_nodes: usize,
    /// Average locality
    pub avg_locality: f64,
    /// Locality imbalance (std dev)
    pub locality_imbalance: f64,
    /// Total migrations
    pub total_migrations: u64,
    /// Pending migrations
    pub pending_migrations: usize,
}

/// System-wide NUMA balancer
pub struct HolisticNumaBalancer {
    /// Nodes
    nodes: BTreeMap<u32, NumaNode>,
    /// Distance matrix
    pub distances: NumaDistanceMatrix,
    /// Pending migrations
    pending: Vec<NumaPageMigration>,
    /// Total migrations executed
    total_migrations: u64,
    /// Stats
    stats: HolisticNumaBalancerStats,
}

impl HolisticNumaBalancer {
    pub fn new(node_count: usize) -> Self {
        Self {
            nodes: BTreeMap::new(),
            distances: NumaDistanceMatrix::new(node_count),
            pending: Vec::new(),
            total_migrations: 0,
            stats: HolisticNumaBalancerStats::default(),
        }
    }

    /// Register node
    #[inline(always)]
    pub fn register_node(&mut self, node_id: u32, total_pages: u64, cpu_capacity: u32) {
        self.nodes.insert(node_id, NumaNode::new(node_id, total_pages, cpu_capacity));
        self.update_stats();
    }

    /// Record remote access (triggers potential migration)
    pub fn record_remote_access(&mut self, pfn: u64, access_node: u32, home_node: u32) {
        if let Some(node) = self.nodes.get_mut(&access_node) {
            node.remote_accesses += 1;
        }
        if let Some(node) = self.nodes.get_mut(&home_node) {
            node.local_accesses += 1;
        }

        // Consider migration if access node has capacity
        if let Some(access) = self.nodes.get(&access_node) {
            if access.free_pages > 0 {
                let dist = self.distances.distance(home_node as usize, access_node as usize);
                self.pending.push(NumaPageMigration {
                    pfn,
                    src_node: home_node,
                    dst_node: access_node,
                    reason: NumaMigReason::AccessLocality,
                    distance: dist,
                });
            }
        }
        self.update_stats();
    }

    /// Evaluate rebalancing
    pub fn evaluate_rebalance(&mut self) {
        // Update pressures
        let node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        for &nid in &node_ids {
            if let Some(node) = self.nodes.get_mut(&nid) {
                node.update_pressure();
            }
        }

        // Find pressured nodes and try to offload
        let pressured: Vec<u32> = self.nodes.iter()
            .filter(|(_, n)| matches!(n.state, NumaNodeState::Pressured | NumaNodeState::Overcommitted))
            .map(|(&id, _)| id)
            .collect();

        for &src_nid in &pressured {
            if let Some(target) = self.distances.nearest_with_capacity(src_nid as usize, &self.nodes) {
                let dist = self.distances.distance(src_nid as usize, target as usize);
                self.pending.push(NumaPageMigration {
                    pfn: 0, // placeholder
                    src_node: src_nid,
                    dst_node: target,
                    reason: NumaMigReason::MemoryPressure,
                    distance: dist,
                });
            }
        }
        self.update_stats();
    }

    /// Execute pending migrations
    pub fn execute_migrations(&mut self, max_migrations: usize) -> usize {
        let to_execute: Vec<NumaPageMigration> = self.pending.drain(..self.pending.len().min(max_migrations)).collect();
        let mut executed = 0;

        for mig in &to_execute {
            if let Some(src) = self.nodes.get_mut(&mig.src_node) {
                if src.free_pages < src.total_pages {
                    src.free_pages += 1;
                }
            }
            if let Some(dst) = self.nodes.get_mut(&mig.dst_node) {
                if dst.free_pages > 0 {
                    dst.free_pages -= 1;
                    executed += 1;
                }
            }
        }
        self.total_migrations += executed as u64;
        self.update_stats();
        executed
    }

    fn update_stats(&mut self) {
        self.stats.active_nodes = self.nodes.values()
            .filter(|n| n.state != NumaNodeState::Offline)
            .count();
        self.stats.pressured_nodes = self.nodes.values()
            .filter(|n| matches!(n.state, NumaNodeState::Pressured | NumaNodeState::Overcommitted))
            .count();

        let localities: Vec<f64> = self.nodes.values().map(|n| n.locality_ratio()).collect();
        if !localities.is_empty() {
            let sum: f64 = localities.iter().sum();
            let mean = sum / localities.len() as f64;
            self.stats.avg_locality = mean;

            let var: f64 = localities.iter().map(|l| (l - mean) * (l - mean)).sum::<f64>() / localities.len() as f64;
            self.stats.locality_imbalance = sqrt(var);
        }
        self.stats.total_migrations = self.total_migrations;
        self.stats.pending_migrations = self.pending.len();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticNumaBalancerStats {
        &self.stats
    }
}
