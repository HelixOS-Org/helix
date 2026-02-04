//! NUMA topology analysis and optimization.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// NUMA ANALYZER
// ============================================================================

/// Analyzes cross-NUMA memory access patterns
pub struct NumaAnalyzer {
    /// Cross-node access counts [from_node][to_node]
    cross_node_accesses: Vec<Vec<u64>>,
    /// Local access counts by node
    local_accesses: Vec<u64>,
    /// Task-to-node mapping
    task_home_node: BTreeMap<u64, u32>,
    /// Number of NUMA nodes
    num_nodes: u32,
}

impl NumaAnalyzer {
    /// Create new NUMA analyzer
    pub fn new(num_nodes: u32) -> Self {
        let n = num_nodes as usize;
        Self {
            cross_node_accesses: vec![vec![0; n]; n],
            local_accesses: vec![0; n],
            task_home_node: BTreeMap::new(),
            num_nodes,
        }
    }

    /// Record memory access
    pub fn record_access(&mut self, task_id: u64, accessing_node: u32, memory_node: u32) {
        if accessing_node as usize >= self.local_accesses.len()
            || memory_node as usize >= self.local_accesses.len()
        {
            return;
        }

        if accessing_node == memory_node {
            self.local_accesses[accessing_node as usize] += 1;
        } else {
            self.cross_node_accesses[accessing_node as usize][memory_node as usize] += 1;
        }

        // Track task home node
        self.task_home_node
            .entry(task_id)
            .and_modify(|_n| {
                // Update if this node is more frequently accessed
                // Simple: just track most recent
            })
            .or_insert(memory_node);
    }

    /// Get local access ratio for a node
    pub fn local_ratio(&self, node: u32) -> f64 {
        if node as usize >= self.local_accesses.len() {
            return 0.0;
        }

        let local = self.local_accesses[node as usize];
        let remote: u64 = self.cross_node_accesses[node as usize].iter().sum();
        let total = local + remote;

        if total == 0 {
            1.0
        } else {
            local as f64 / total as f64
        }
    }

    /// Get cross-node traffic matrix
    pub fn cross_node_traffic(&self) -> Vec<Vec<u64>> {
        self.cross_node_accesses.clone()
    }

    /// Recommend node for task based on memory access patterns
    pub fn recommend_node(&self, task_id: u64) -> Option<u32> {
        self.task_home_node.get(&task_id).copied()
    }

    /// Get node with most memory for a task
    pub fn best_node_for_task(&self, task_id: u64, memory_locations: &[u32]) -> u32 {
        if memory_locations.is_empty() {
            return self.recommend_node(task_id).unwrap_or(0);
        }

        // Count memory per node
        let mut node_counts = vec![0u32; self.num_nodes as usize];
        for &node in memory_locations {
            if (node as usize) < node_counts.len() {
                node_counts[node as usize] += 1;
            }
        }

        // Return node with most memory
        node_counts
            .iter()
            .enumerate()
            .max_by_key(|&(_, &c)| c)
            .map(|(i, _)| i as u32)
            .unwrap_or(0)
    }

    /// Should migrate memory?
    pub fn should_migrate(&self, _task_id: u64, current_node: u32, target_node: u32) -> bool {
        if current_node == target_node {
            return false;
        }

        // Calculate benefit
        let current_local = self.local_ratio(current_node);

        // Simple heuristic: migrate if local ratio is low
        current_local < 0.5
    }

    /// Get overall NUMA efficiency
    pub fn numa_efficiency(&self) -> f64 {
        let local: u64 = self.local_accesses.iter().sum();
        let remote: u64 = self.cross_node_accesses.iter().flat_map(|r| r).sum();
        let total = local + remote;

        if total == 0 {
            1.0
        } else {
            local as f64 / total as f64
        }
    }
}
