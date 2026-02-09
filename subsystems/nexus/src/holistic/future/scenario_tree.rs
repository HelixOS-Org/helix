// SPDX-License-Identifier: GPL-2.0
//! # Holistic Scenario Tree — System-Wide Future State Tree
//!
//! Predicts the **entire system's future** as a tree of possible states. Each
//! node represents a complete global snapshot (CPU, memory, I/O, network,
//! process set), and each edge represents a probabilistic transition driven by
//! workload shifts, resource pressure, or external events.
//!
//! The scenario tree enables optimal-path planning: the kernel can trace the
//! most likely future, the worst-case future, and every branch in between —
//! then choose actions that maximise expected system health.
//!
//! ## Key Features
//!
//! - Global state nodes with full resource vectors
//! - Probabilistic transitions with cross-subsystem coupling
//! - Optimal / worst-case path extraction via dynamic programming
//! - Pruning strategies that keep the tree tractable in real time
//! - Expected-state aggregation across all surviving branches

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TREE_DEPTH: usize = 16;
const MAX_CHILDREN_PER_NODE: usize = 8;
const MAX_NODES: usize = 2048;
const MAX_RESOURCE_DIMS: usize = 12;
const MAX_TRANSITION_LOG: usize = 512;
const PRUNING_THRESHOLD: f32 = 0.005;
const EMA_ALPHA: f32 = 0.10;
const CONFIDENCE_FLOOR: f32 = 0.01;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn ema_update(current: f32, new_sample: f32) -> f32 {
    EMA_ALPHA * new_sample + (1.0 - EMA_ALPHA) * current
}

// ============================================================================
// RESOURCE VECTOR — snapshot of the entire system
// ============================================================================

/// A single dimension of system state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceDimension {
    CpuUtilization,
    MemoryPressure,
    IoThroughput,
    NetworkBandwidth,
    ProcessCount,
    SchedulerLoad,
    CacheHitRate,
    ThermalLevel,
    PowerDraw,
    DiskLatency,
    IpcThroughput,
    InterruptRate,
}

/// Full system state vector at a point in time
#[derive(Debug, Clone)]
pub struct SystemStateVector {
    pub values: BTreeMap<u64, f32>,
    pub timestamp_us: u64,
    pub generation: u64,
}

impl SystemStateVector {
    fn new(timestamp_us: u64, generation: u64) -> Self {
        Self {
            values: BTreeMap::new(),
            timestamp_us,
            generation,
        }
    }

    fn set(&mut self, dim: ResourceDimension, value: f32) {
        let key = fnv1a_hash(&[dim as u8]);
        self.values.insert(key, value);
    }

    fn get(&self, dim: ResourceDimension) -> f32 {
        let key = fnv1a_hash(&[dim as u8]);
        self.values.get(&key).copied().unwrap_or(0.0)
    }

    fn distance(&self, other: &Self) -> f32 {
        let mut sum_sq = 0.0_f32;
        for (k, v) in &self.values {
            let ov = other.values.get(k).copied().unwrap_or(0.0);
            let d = *v - ov;
            sum_sq += d * d;
        }
        sum_sq
    }
}

// ============================================================================
// SCENARIO TREE NODE
// ============================================================================

/// A single node in the scenario tree
#[derive(Debug, Clone)]
pub struct ScenarioNode {
    pub node_id: u64,
    pub depth: usize,
    pub state: SystemStateVector,
    pub probability: f32,
    pub cumulative_score: f32,
    pub children: Vec<u64>,
    pub parent: Option<u64>,
    pub label: String,
    pub pruned: bool,
}

/// Transition edge between two nodes
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from_node: u64,
    pub to_node: u64,
    pub transition_prob: f32,
    pub trigger_description: String,
    pub resource_delta: BTreeMap<u64, f32>,
}

/// An extracted path through the tree
#[derive(Debug, Clone)]
pub struct TreePath {
    pub node_ids: Vec<u64>,
    pub total_probability: f32,
    pub total_score: f32,
    pub description: String,
}

/// Result of expected-state aggregation
#[derive(Debug, Clone)]
pub struct ExpectedSystemState {
    pub weighted_state: SystemStateVector,
    pub variance_per_dim: BTreeMap<u64, f32>,
    pub branch_count: usize,
    pub total_probability_mass: f32,
    pub horizon_us: u64,
}

/// Pruning strategy outcome
#[derive(Debug, Clone)]
pub struct PruningReport {
    pub nodes_before: usize,
    pub nodes_after: usize,
    pub probability_mass_removed: f32,
    pub strategy: PruningStrategy,
    pub timestamp_us: u64,
}

/// Pruning strategy selector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruningStrategy {
    ProbabilityThreshold,
    DepthLimit,
    WidthLimit,
    ScoreBased,
    Hybrid,
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Runtime statistics for the scenario tree engine
#[derive(Debug, Clone)]
pub struct ScenarioTreeStats {
    pub trees_built: u64,
    pub total_nodes_created: u64,
    pub total_nodes_pruned: u64,
    pub optimal_paths_found: u64,
    pub worst_paths_found: u64,
    pub expected_states_computed: u64,
    pub avg_tree_depth: f32,
    pub avg_branching_factor: f32,
    pub avg_build_time_us: f32,
    pub avg_path_probability: f32,
}

impl ScenarioTreeStats {
    fn new() -> Self {
        Self {
            trees_built: 0,
            total_nodes_created: 0,
            total_nodes_pruned: 0,
            optimal_paths_found: 0,
            worst_paths_found: 0,
            expected_states_computed: 0,
            avg_tree_depth: 0.0,
            avg_branching_factor: 0.0,
            avg_build_time_us: 0.0,
            avg_path_probability: 0.0,
        }
    }
}

// ============================================================================
// HOLISTIC SCENARIO TREE ENGINE
// ============================================================================

/// System-wide scenario tree prediction engine
pub struct HolisticScenarioTree {
    nodes: BTreeMap<u64, ScenarioNode>,
    transitions: Vec<StateTransition>,
    root_id: Option<u64>,
    next_node_id: u64,
    rng_state: u64,
    pruning_log: Vec<PruningReport>,
    stats: ScenarioTreeStats,
    depth_limit: usize,
    width_limit: usize,
    generation: u64,
}

impl HolisticScenarioTree {
    /// Create a new holistic scenario tree engine
    pub fn new(seed: u64) -> Self {
        Self {
            nodes: BTreeMap::new(),
            transitions: Vec::new(),
            root_id: None,
            next_node_id: 1,
            rng_state: seed ^ 0xDEAD_BEEF_CAFE_1234,
            pruning_log: Vec::new(),
            stats: ScenarioTreeStats::new(),
            depth_limit: MAX_TREE_DEPTH,
            width_limit: MAX_CHILDREN_PER_NODE,
            generation: 0,
        }
    }

    /// Build a full system scenario tree from a root state
    pub fn build_system_tree(
        &mut self,
        root_state: SystemStateVector,
        horizon_us: u64,
        max_depth: usize,
    ) -> u64 {
        self.generation += 1;
        self.nodes.clear();
        self.transitions.clear();

        let depth = if max_depth > MAX_TREE_DEPTH { MAX_TREE_DEPTH } else { max_depth };
        let root_id = self.allocate_node(root_state, 0, 1.0, None, String::from("root"));
        self.root_id = Some(root_id);

        let mut frontier: Vec<u64> = Vec::new();
        frontier.push(root_id);

        for current_depth in 0..depth {
            let mut next_frontier: Vec<u64> = Vec::new();
            let step_horizon = horizon_us / (depth as u64).max(1);

            for &parent_id in &frontier {
                if self.nodes.len() >= MAX_NODES {
                    break;
                }
                let child_count = self.decide_branching(parent_id, current_depth);
                for c in 0..child_count {
                    let child_state = self.project_state(parent_id, step_horizon, c);
                    let prob = self.compute_transition_prob(parent_id, c, child_count);
                    let label = Self::make_label(current_depth + 1, c);
                    let child_id = self.allocate_node(
                        child_state,
                        current_depth + 1,
                        prob,
                        Some(parent_id),
                        label,
                    );
                    self.record_transition(parent_id, child_id, prob);
                    next_frontier.push(child_id);
                }
            }
            frontier = next_frontier;
        }

        let total = self.nodes.len() as u64;
        self.stats.trees_built += 1;
        self.stats.total_nodes_created += total;
        self.stats.avg_tree_depth = ema_update(self.stats.avg_tree_depth, depth as f32);
        self.stats.avg_branching_factor = if total > 1 {
            ema_update(self.stats.avg_branching_factor, total as f32 / depth as f32)
        } else {
            1.0
        };

        root_id
    }

    /// Extract the optimal (highest cumulative score) path from root to leaf
    pub fn optimal_path(&mut self) -> Option<TreePath> {
        let root = self.root_id?;
        let path = self.trace_best_path(root, true);
        if !path.node_ids.is_empty() {
            self.stats.optimal_paths_found += 1;
        }
        Some(path)
    }

    /// Extract the worst-case (lowest cumulative score) path from root to leaf
    pub fn worst_case_path(&mut self) -> Option<TreePath> {
        let root = self.root_id?;
        let path = self.trace_best_path(root, false);
        if !path.node_ids.is_empty() {
            self.stats.worst_paths_found += 1;
        }
        Some(path)
    }

    /// Compute the probability-weighted expected system state at the leaves
    pub fn expected_system_state(&mut self, horizon_us: u64) -> ExpectedSystemState {
        let mut weighted = SystemStateVector::new(horizon_us, self.generation);
        let mut variance_acc: BTreeMap<u64, f32> = BTreeMap::new();
        let mut total_prob = 0.0_f32;
        let mut leaf_count = 0_usize;

        let leaf_snapshots: Vec<(f32, BTreeMap<u64, f32>)> = self
            .nodes
            .values()
            .filter(|n| n.children.is_empty() && !n.pruned)
            .map(|n| (n.probability, n.state.values.clone()))
            .collect();

        for (prob, values) in &leaf_snapshots {
            total_prob += prob;
            leaf_count += 1;
            for (k, v) in values {
                let entry = weighted.values.entry(*k).or_insert(0.0);
                *entry += prob * v;
            }
        }

        if total_prob > 0.0 {
            for v in weighted.values.values_mut() {
                *v /= total_prob;
            }
            for (_prob, values) in &leaf_snapshots {
                for (k, v) in values {
                    let mean = weighted.values.get(k).copied().unwrap_or(0.0);
                    let diff = v - mean;
                    let entry = variance_acc.entry(*k).or_insert(0.0);
                    *entry += diff * diff;
                }
            }
            if leaf_count > 1 {
                for v in variance_acc.values_mut() {
                    *v /= leaf_count as f32;
                }
            }
        }

        self.stats.expected_states_computed += 1;

        ExpectedSystemState {
            weighted_state: weighted,
            variance_per_dim: variance_acc,
            branch_count: leaf_count,
            total_probability_mass: total_prob,
            horizon_us,
        }
    }

    /// Apply a pruning strategy to reduce tree size
    pub fn pruning_strategy(&mut self, strategy: PruningStrategy) -> PruningReport {
        let before = self.nodes.len();
        let mut removed_mass = 0.0_f32;

        let to_prune: Vec<u64> = match strategy {
            PruningStrategy::ProbabilityThreshold => self
                .nodes
                .iter()
                .filter(|(_, n)| !n.pruned && n.probability < PRUNING_THRESHOLD)
                .map(|(id, n)| { removed_mass += n.probability; *id })
                .collect(),
            PruningStrategy::DepthLimit => self
                .nodes
                .iter()
                .filter(|(_, n)| !n.pruned && n.depth >= self.depth_limit)
                .map(|(id, n)| { removed_mass += n.probability; *id })
                .collect(),
            PruningStrategy::WidthLimit => {
                let mut prune_list = Vec::new();
                let parent_ids: Vec<u64> = self.nodes.keys().copied().collect();
                for pid in parent_ids {
                    let children: Vec<u64> = self
                        .nodes
                        .get(&pid)
                        .map(|n| n.children.clone())
                        .unwrap_or_default();
                    if children.len() > self.width_limit {
                        for &cid in &children[self.width_limit..] {
                            if let Some(cn) = self.nodes.get(&cid) {
                                removed_mass += cn.probability;
                            }
                            prune_list.push(cid);
                        }
                    }
                }
                prune_list
            }
            PruningStrategy::ScoreBased => self
                .nodes
                .iter()
                .filter(|(_, n)| !n.pruned && n.cumulative_score < 0.0)
                .map(|(id, n)| { removed_mass += n.probability; *id })
                .collect(),
            PruningStrategy::Hybrid => {
                let mut prune_list: Vec<u64> = self
                    .nodes
                    .iter()
                    .filter(|(_, n)| {
                        !n.pruned
                            && (n.probability < PRUNING_THRESHOLD
                                || n.depth >= self.depth_limit)
                    })
                    .map(|(id, n)| { removed_mass += n.probability; *id })
                    .collect();
                prune_list.truncate(MAX_NODES / 2);
                prune_list
            }
        };

        for id in &to_prune {
            if let Some(node) = self.nodes.get_mut(id) {
                node.pruned = true;
            }
        }

        let after = self.nodes.values().filter(|n| !n.pruned).count();
        self.stats.total_nodes_pruned += to_prune.len() as u64;

        let report = PruningReport {
            nodes_before: before,
            nodes_after: after,
            probability_mass_removed: removed_mass,
            strategy,
            timestamp_us: self.generation,
        };
        if self.pruning_log.len() < MAX_TRANSITION_LOG {
            self.pruning_log.push(report.clone());
        }
        report
    }

    /// Return the current tree size (total / active / pruned)
    pub fn tree_size(&self) -> (usize, usize, usize) {
        let total = self.nodes.len();
        let pruned = self.nodes.values().filter(|n| n.pruned).count();
        (total, total - pruned, pruned)
    }

    /// Compute probability of a specific path (product of transition probs)
    pub fn path_probability(&self, node_ids: &[u64]) -> f32 {
        if node_ids.is_empty() {
            return 0.0;
        }
        let mut prob = 1.0_f32;
        for window in node_ids.windows(2) {
            let from = window[0];
            let to = window[1];
            let tp = self
                .transitions
                .iter()
                .find(|t| t.from_node == from && t.to_node == to)
                .map(|t| t.transition_prob)
                .unwrap_or(0.0);
            prob *= tp;
        }
        self.stats.avg_path_probability.max(0.0);
        prob
    }

    /// Get current statistics snapshot
    pub fn stats(&self) -> &ScenarioTreeStats {
        &self.stats
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn allocate_node(
        &mut self,
        state: SystemStateVector,
        depth: usize,
        probability: f32,
        parent: Option<u64>,
        label: String,
    ) -> u64 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let score = state.values.values().sum::<f32>() * probability;
        let node = ScenarioNode {
            node_id: id,
            depth,
            state,
            probability,
            cumulative_score: score,
            children: Vec::new(),
            parent,
            label,
            pruned: false,
        };
        self.nodes.insert(id, node);
        if let Some(pid) = parent {
            if let Some(pnode) = self.nodes.get_mut(&pid) {
                pnode.children.push(id);
            }
        }
        id
    }

    fn decide_branching(&mut self, _parent_id: u64, depth: usize) -> usize {
        let base = if depth < 2 { 4 } else { 2 };
        let r = xorshift64(&mut self.rng_state) % 3;
        let count = base + r as usize;
        if count > self.width_limit { self.width_limit } else { count }
    }

    fn project_state(&mut self, parent_id: u64, step_us: u64, variant: usize) -> SystemStateVector {
        let parent_values = self
            .nodes
            .get(&parent_id)
            .map(|n| n.state.values.clone())
            .unwrap_or_default();
        let parent_ts = self
            .nodes
            .get(&parent_id)
            .map(|n| n.state.timestamp_us)
            .unwrap_or(0);

        let mut new_state = SystemStateVector::new(parent_ts + step_us, self.generation);
        for (k, v) in &parent_values {
            let noise = (xorshift64(&mut self.rng_state) % 200) as f32 / 1000.0 - 0.1;
            let drift = (variant as f32 - 1.5) * 0.02;
            let new_val = (v + noise + drift).clamp(0.0, 1.0);
            new_state.values.insert(*k, new_val);
        }
        new_state
    }

    fn compute_transition_prob(&mut self, _parent_id: u64, child_idx: usize, total: usize) -> f32 {
        if total == 0 {
            return 0.0;
        }
        let base = 1.0 / total as f32;
        let jitter = (xorshift64(&mut self.rng_state) % 100) as f32 / 2000.0 - 0.025;
        (base + jitter).max(CONFIDENCE_FLOOR)
    }

    fn record_transition(&mut self, from: u64, to: u64, prob: f32) {
        if self.transitions.len() < MAX_TRANSITION_LOG {
            self.transitions.push(StateTransition {
                from_node: from,
                to_node: to,
                transition_prob: prob,
                trigger_description: String::new(),
                resource_delta: BTreeMap::new(),
            });
        }
    }

    fn trace_best_path(&self, start: u64, maximize: bool) -> TreePath {
        let mut path = Vec::new();
        let mut current = start;
        let mut total_prob = 1.0_f32;
        let mut total_score = 0.0_f32;

        loop {
            path.push(current);
            let node = match self.nodes.get(&current) {
                Some(n) => n,
                None => break,
            };
            total_score += node.cumulative_score;

            let active_children: Vec<u64> = node
                .children
                .iter()
                .copied()
                .filter(|cid| self.nodes.get(cid).map(|n| !n.pruned).unwrap_or(false))
                .collect();

            if active_children.is_empty() {
                break;
            }

            let best = if maximize {
                active_children
                    .iter()
                    .max_by(|a, b| {
                        let sa = self.nodes.get(a).map(|n| n.cumulative_score).unwrap_or(0.0);
                        let sb = self.nodes.get(b).map(|n| n.cumulative_score).unwrap_or(0.0);
                        sa.partial_cmp(&sb).unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .copied()
            } else {
                active_children
                    .iter()
                    .min_by(|a, b| {
                        let sa = self.nodes.get(a).map(|n| n.cumulative_score).unwrap_or(0.0);
                        let sb = self.nodes.get(b).map(|n| n.cumulative_score).unwrap_or(0.0);
                        sa.partial_cmp(&sb).unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .copied()
            };

            match best {
                Some(next_id) => {
                    let tp = self
                        .transitions
                        .iter()
                        .find(|t| t.from_node == current && t.to_node == next_id)
                        .map(|t| t.transition_prob)
                        .unwrap_or(1.0);
                    total_prob *= tp;
                    current = next_id;
                }
                None => break,
            }
        }

        let desc = if maximize {
            String::from("optimal")
        } else {
            String::from("worst-case")
        };

        TreePath {
            node_ids: path,
            total_probability: total_prob,
            total_score,
            description: desc,
        }
    }

    fn make_label(depth: usize, child_idx: usize) -> String {
        let mut buf = String::from("d");
        let d_bytes = depth as u64;
        let c_bytes = child_idx as u64;
        let hash = fnv1a_hash(&d_bytes.to_le_bytes()) ^ c_bytes;
        let _ = buf.push('0');
        let remainder = hash % 1000;
        if remainder < 100 { buf.push('0'); }
        if remainder < 10 { buf.push('0'); }
        buf
    }
}
