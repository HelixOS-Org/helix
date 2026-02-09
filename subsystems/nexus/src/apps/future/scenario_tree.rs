// SPDX-License-Identifier: GPL-2.0
//! # Apps Scenario Tree
//!
//! Branching scenario trees for application behavior futures. Each node in the
//! tree represents a potential application state — will the app spawn threads?
//! allocate a burst of memory? issue an I/O storm? The tree fans out at
//! decision points, with each edge carrying an estimated probability.
//!
//! The engine builds, prunes, and queries these trees to answer questions like
//! "What is the most likely execution path?" and "What is the expected resource
//! need across all plausible futures?"
//!
//! This is the apps engine reasoning about branching futures.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TREE_DEPTH: usize = 16;
const MAX_CHILDREN: usize = 8;
const MAX_NODES: usize = 4096;
const MAX_APPS: usize = 256;
const PRUNE_THRESHOLD: f64 = 0.01;
const EMA_ALPHA: f64 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0xdeadbeef_cafebabe;

// ============================================================================
// UTILITY FUNCTIONS
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

#[inline]
fn ema_update(current: f64, new_sample: f64, alpha: f64) -> f64 {
    alpha * new_sample + (1.0 - alpha) * current
}

// ============================================================================
// ACTION TYPES
// ============================================================================

/// Possible actions an application may take at a scenario branch point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppAction {
    SpawnThread,
    AllocateMemory,
    DeallocateMemory,
    IoBurstRead,
    IoBurstWrite,
    CpuIntensive,
    Idle,
    Exit,
}

impl AppAction {
    fn as_str(&self) -> &'static str {
        match self {
            AppAction::SpawnThread => "spawn_thread",
            AppAction::AllocateMemory => "alloc_mem",
            AppAction::DeallocateMemory => "dealloc_mem",
            AppAction::IoBurstRead => "io_read",
            AppAction::IoBurstWrite => "io_write",
            AppAction::CpuIntensive => "cpu_heavy",
            AppAction::Idle => "idle",
            AppAction::Exit => "exit",
        }
    }

    fn resource_cost(&self) -> f64 {
        match self {
            AppAction::SpawnThread => 8.0,
            AppAction::AllocateMemory => 12.0,
            AppAction::DeallocateMemory => -4.0,
            AppAction::IoBurstRead => 6.0,
            AppAction::IoBurstWrite => 7.0,
            AppAction::CpuIntensive => 15.0,
            AppAction::Idle => 0.5,
            AppAction::Exit => -20.0,
        }
    }

    fn from_index(i: usize) -> Self {
        match i % 8 {
            0 => AppAction::SpawnThread,
            1 => AppAction::AllocateMemory,
            2 => AppAction::DeallocateMemory,
            3 => AppAction::IoBurstRead,
            4 => AppAction::IoBurstWrite,
            5 => AppAction::CpuIntensive,
            6 => AppAction::Idle,
            _ => AppAction::Exit,
        }
    }
}

// ============================================================================
// SCENARIO NODE
// ============================================================================

/// A single node in the scenario tree, representing one possible application state.
#[derive(Debug, Clone)]
pub struct ScenarioNode {
    pub node_id: u64,
    pub action: AppAction,
    pub probability: f64,
    pub cumulative_probability: f64,
    pub resource_estimate: f64,
    pub depth: usize,
    pub children: Vec<usize>,
    pub label: String,
}

impl ScenarioNode {
    fn new(node_id: u64, action: AppAction, probability: f64, depth: usize) -> Self {
        let label_bytes = [
            action.as_str().as_bytes(),
            b"_d",
            &[(depth as u8) + b'0'],
        ]
        .concat();
        let label = String::from_utf8(label_bytes).unwrap_or_default();
        Self {
            node_id,
            action,
            probability,
            cumulative_probability: probability,
            resource_estimate: action.resource_cost() * probability,
            depth,
            children: Vec::new(),
            label,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn expected_cost(&self) -> f64 {
        self.resource_estimate * self.cumulative_probability
    }
}

// ============================================================================
// APP TREE STATE
// ============================================================================

/// Per-application tree tracking state.
#[derive(Debug, Clone)]
struct AppTreeState {
    app_id: u64,
    root_index: usize,
    node_count: usize,
    max_depth_reached: usize,
    total_probability_mass: f64,
    last_build_tick: u64,
    observation_count: u64,
    action_frequency: LinearMap<u64, 64>,
}

impl AppTreeState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            root_index: 0,
            node_count: 0,
            max_depth_reached: 0,
            total_probability_mass: 0.0,
            last_build_tick: 0,
            observation_count: 0,
            action_frequency: LinearMap::new(),
        }
    }

    fn record_action(&mut self, action: AppAction) {
        let key = fnv1a_hash(action.as_str().as_bytes());
        let count = self.action_frequency.entry(key).or_insert(0);
        *count += 1;
        self.observation_count += 1;
    }

    fn action_probability(&self, action: AppAction) -> f64 {
        if self.observation_count == 0 {
            return 1.0 / 8.0;
        }
        let key = fnv1a_hash(action.as_str().as_bytes());
        let count = self.action_frequency.get(key).copied().unwrap_or(0);
        count as f64 / self.observation_count as f64
    }
}

// ============================================================================
// SCENARIO TREE STATS
// ============================================================================

/// Statistics for the scenario tree engine.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ScenarioTreeStats {
    pub total_trees_built: u64,
    pub total_nodes_created: u64,
    pub total_nodes_pruned: u64,
    pub average_tree_depth: f64,
    pub average_branch_factor: f64,
    pub total_queries: u64,
    pub worst_case_queries: u64,
    pub most_likely_queries: u64,
}

impl ScenarioTreeStats {
    fn new() -> Self {
        Self {
            total_trees_built: 0,
            total_nodes_created: 0,
            total_nodes_pruned: 0,
            average_tree_depth: 0.0,
            average_branch_factor: 0.0,
            total_queries: 0,
            worst_case_queries: 0,
            most_likely_queries: 0,
        }
    }
}

// ============================================================================
// APPS SCENARIO TREE ENGINE
// ============================================================================

/// Main scenario tree engine for application behavior prediction.
///
/// Builds and manages branching scenario trees that represent possible future
/// execution paths of applications. Each tree is rooted at the current observed
/// state and fans out based on historically observed action frequencies.
pub struct AppsScenarioTree {
    nodes: Vec<ScenarioNode>,
    app_states: BTreeMap<u64, AppTreeState>,
    stats: ScenarioTreeStats,
    rng_state: u64,
    tick: u64,
    build_count: u64,
    total_expected_resource: f64,
    ema_branch_factor: f64,
    ema_depth: f64,
}

impl AppsScenarioTree {
    /// Create a new scenario tree engine.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            app_states: BTreeMap::new(),
            stats: ScenarioTreeStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            build_count: 0,
            total_expected_resource: 0.0,
            ema_branch_factor: 2.0,
            ema_depth: 3.0,
        }
    }

    /// Record an observed action for an application to improve future trees.
    #[inline]
    pub fn record_observation(&mut self, app_id: u64, action: AppAction) {
        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            return;
        }
        let state = self.app_states.entry(app_id).or_insert_with(|| AppTreeState::new(app_id));
        state.record_action(action);
    }

    /// Build a full scenario tree for the given application.
    ///
    /// Expands from the root outward, branching at each depth by assigning
    /// probabilities derived from observed action frequencies. Stops at
    /// `MAX_TREE_DEPTH` or when probabilities fall below `PRUNE_THRESHOLD`.
    pub fn build_app_tree(&mut self, app_id: u64) -> usize {
        self.tick += 1;
        let state = match self.app_states.get_mut(&app_id) {
            Some(s) => s,
            None => {
                self.app_states.insert(app_id, AppTreeState::new(app_id));
                self.app_states.get_mut(&app_id).unwrap()
            }
        };
        state.last_build_tick = self.tick;

        // Clear previous nodes for this build
        self.nodes.clear();

        let root_id = fnv1a_hash(&app_id.to_le_bytes());
        let root = ScenarioNode::new(root_id, AppAction::Idle, 1.0, 0);
        self.nodes.push(root);
        state.root_index = 0;

        let mut frontier: Vec<usize> = Vec::new();
        frontier.push(0);
        let mut max_depth: usize = 0;
        let mut total_branches: u64 = 0;
        let mut branch_points: u64 = 0;

        while let Some(parent_idx) = frontier.pop() {
            let parent_depth = self.nodes[parent_idx].depth;
            let parent_cum_prob = self.nodes[parent_idx].cumulative_probability;

            if parent_depth >= MAX_TREE_DEPTH || self.nodes.len() >= MAX_NODES {
                break;
            }

            let mut children_added = 0usize;
            for action_i in 0..8usize {
                let action = AppAction::from_index(action_i);
                let prob = state.action_probability(action);
                let cum_prob = parent_cum_prob * prob;

                if cum_prob < PRUNE_THRESHOLD {
                    continue;
                }

                let nid = fnv1a_hash(&[
                    &app_id.to_le_bytes()[..],
                    &(parent_idx as u64).to_le_bytes()[..],
                    &(action_i as u64).to_le_bytes()[..],
                ].concat());

                let mut node = ScenarioNode::new(nid, action, prob, parent_depth + 1);
                node.cumulative_probability = cum_prob;
                node.resource_estimate = action.resource_cost() * cum_prob;

                let child_idx = self.nodes.len();
                if child_idx >= MAX_NODES {
                    break;
                }
                self.nodes.push(node);
                self.nodes[parent_idx].children.push(child_idx);
                children_added += 1;

                if parent_depth + 1 < MAX_TREE_DEPTH && cum_prob > PRUNE_THRESHOLD * 2.0 {
                    frontier.push(child_idx);
                }
            }

            if children_added > 0 {
                total_branches += children_added as u64;
                branch_points += 1;
            }
            if parent_depth + 1 > max_depth {
                max_depth = parent_depth + 1;
            }
        }

        state.node_count = self.nodes.len();
        state.max_depth_reached = max_depth;
        state.total_probability_mass = self.nodes.iter().filter(|n| n.is_leaf()).map(|n| n.cumulative_probability).sum();

        let bf = if branch_points > 0 { total_branches as f64 / branch_points as f64 } else { 0.0 };
        self.ema_branch_factor = ema_update(self.ema_branch_factor, bf, EMA_ALPHA);
        self.ema_depth = ema_update(self.ema_depth, max_depth as f64, EMA_ALPHA);

        self.build_count += 1;
        self.stats.total_trees_built += 1;
        self.stats.total_nodes_created += self.nodes.len() as u64;
        self.stats.average_tree_depth = self.ema_depth;
        self.stats.average_branch_factor = self.ema_branch_factor;

        self.nodes.len()
    }

    /// Return the most likely execution path from root to leaf.
    ///
    /// At each node, picks the child with the highest individual probability
    /// and descends until hitting a leaf.
    pub fn most_likely_path(&mut self) -> Vec<AppAction> {
        self.stats.most_likely_queries += 1;
        self.stats.total_queries += 1;

        if self.nodes.is_empty() {
            return Vec::new();
        }

        let mut path = Vec::new();
        let mut current = 0usize;

        loop {
            let node = &self.nodes[current];
            if node.is_leaf() {
                path.push(node.action);
                break;
            }
            path.push(node.action);

            let best_child = node
                .children
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    let pa = self.nodes[a].probability;
                    let pb = self.nodes[b].probability;
                    pa.partial_cmp(&pb).unwrap_or(core::cmp::Ordering::Equal)
                });

            match best_child {
                Some(idx) => current = idx,
                None => break,
            }
        }

        path
    }

    /// Return the worst-case execution path — highest cumulative resource cost.
    pub fn worst_case(&mut self) -> (Vec<AppAction>, f64) {
        self.stats.worst_case_queries += 1;
        self.stats.total_queries += 1;

        if self.nodes.is_empty() {
            return (Vec::new(), 0.0);
        }

        let mut worst_cost = f64::MIN;
        let mut worst_leaf = 0usize;

        for (i, node) in self.nodes.iter().enumerate() {
            if node.is_leaf() {
                let cost = node.resource_estimate;
                if cost > worst_cost {
                    worst_cost = cost;
                    worst_leaf = i;
                }
            }
        }

        // Trace path back from leaf to root by walking depth
        let mut path = Vec::new();
        let target_depth = self.nodes[worst_leaf].depth;
        let mut current_target = worst_leaf;

        for _d in (0..=target_depth).rev() {
            path.push(self.nodes[current_target].action);
            // Find parent: a node whose children contain current_target
            let mut found_parent = false;
            for (i, node) in self.nodes.iter().enumerate() {
                if node.children.contains(&current_target) {
                    current_target = i;
                    found_parent = true;
                    break;
                }
            }
            if !found_parent {
                break;
            }
        }

        path.reverse();
        (path, worst_cost)
    }

    /// Compute the expected resource need across all leaf scenarios,
    /// weighted by their cumulative probability.
    pub fn expected_resource_need(&self) -> f64 {
        let mut total = 0.0;
        let mut prob_sum = 0.0;

        for node in &self.nodes {
            if node.is_leaf() {
                total += node.resource_estimate;
                prob_sum += node.cumulative_probability;
            }
        }

        if prob_sum > 0.0 {
            total / prob_sum
        } else {
            0.0
        }
    }

    /// Prune branches whose cumulative probability falls below threshold.
    /// Returns the number of nodes pruned.
    pub fn prune_tree(&mut self, threshold: f64) -> usize {
        let cutoff = if threshold > 0.0 { threshold } else { PRUNE_THRESHOLD };
        let mut pruned = 0usize;
        let mut keep = Vec::with_capacity(self.nodes.len());
        keep.resize(self.nodes.len(), true);

        for i in 0..self.nodes.len() {
            if self.nodes[i].cumulative_probability < cutoff && i != 0 {
                keep[i] = false;
                pruned += 1;
            }
        }

        // Remove pruned children references
        for i in 0..self.nodes.len() {
            if keep[i] {
                let node = &mut self.nodes[i];
                node.children.retain(|&child| child < keep.len() && keep[child]);
            }
        }

        self.stats.total_nodes_pruned += pruned as u64;
        pruned
    }

    /// Return the total number of scenarios (leaf nodes) in the current tree.
    #[inline(always)]
    pub fn scenario_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_leaf()).count()
    }

    /// Return a snapshot of engine statistics.
    #[inline(always)]
    pub fn stats(&self) -> &ScenarioTreeStats {
        &self.stats
    }

    /// Get the total number of nodes currently in the tree.
    #[inline(always)]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Generate a stochastic branch using xorshift PRNG to add randomized
    /// future exploration paths beyond the deterministic frequency model.
    pub fn stochastic_branch(&mut self, parent_idx: usize) -> usize {
        if parent_idx >= self.nodes.len() || self.nodes.len() >= MAX_NODES {
            return 0;
        }

        let parent_depth = self.nodes[parent_idx].depth;
        let parent_cum = self.nodes[parent_idx].cumulative_probability;

        if parent_depth >= MAX_TREE_DEPTH {
            return 0;
        }

        let rand_val = xorshift64(&mut self.rng_state);
        let action = AppAction::from_index((rand_val % 8) as usize);
        let prob = 0.05 + (rand_val % 100) as f64 * 0.005;
        let cum = parent_cum * prob;

        let nid = fnv1a_hash(&rand_val.to_le_bytes());
        let mut node = ScenarioNode::new(nid, action, prob, parent_depth + 1);
        node.cumulative_probability = cum;
        node.resource_estimate = action.resource_cost() * cum;

        let child_idx = self.nodes.len();
        self.nodes.push(node);
        self.nodes[parent_idx].children.push(child_idx);
        self.stats.total_nodes_created += 1;

        1
    }

    /// Get the depth distribution of the current tree.
    #[inline]
    pub fn depth_distribution(&self) -> BTreeMap<usize, usize> {
        let mut dist = BTreeMap::new();
        for node in &self.nodes {
            let count = dist.entry(node.depth).or_insert(0);
            *count += 1;
        }
        dist
    }

    /// Retrieve the EMA-smoothed average branch factor.
    #[inline(always)]
    pub fn avg_branch_factor(&self) -> f64 {
        self.ema_branch_factor
    }

    /// Retrieve the EMA-smoothed average tree depth.
    #[inline(always)]
    pub fn avg_tree_depth(&self) -> f64 {
        self.ema_depth
    }
}
