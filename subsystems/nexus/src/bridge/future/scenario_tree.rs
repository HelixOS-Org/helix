// SPDX-License-Identifier: GPL-2.0
//! # Bridge Scenario Tree
//!
//! Branching scenario trees for syscall futures. Each node is a possible system
//! state, edges represent possible events (syscall, interrupt, timer). Builds
//! trees up to depth 8 and evaluates them minimax-style to choose optimal
//! syscall routing. The bridge looking at every fork in the road simultaneously.
//!
//! Every future is a tree — this module grows and prunes them.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TREE_DEPTH: usize = 8;
const MAX_CHILDREN_PER_NODE: usize = 6;
const MAX_NODES: usize = 2048;
const MAX_SCENARIOS: usize = 128;
const PRUNE_THRESHOLD: f32 = 0.01;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DEFAULT_SEED: u64 = 0xDEAD_BEEF_CAFE_1234;

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

fn rand_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 1_000_000) as f32 / 1_000_000.0
}

// ============================================================================
// EVENT TYPES
// ============================================================================

/// Type of event that transitions between scenario nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    /// A syscall invocation with its number
    Syscall(u32),
    /// A hardware interrupt
    Interrupt(u16),
    /// A timer expiration
    Timer,
    /// A resource allocation event
    ResourceAlloc,
    /// A scheduling preemption
    Preemption,
    /// An IPC message arrival
    IpcMessage,
}

impl EventKind {
    fn to_bytes(&self) -> [u8; 8] {
        let val: u64 = match self {
            EventKind::Syscall(n) => 0x1000_0000 | *n as u64,
            EventKind::Interrupt(n) => 0x2000_0000 | *n as u64,
            EventKind::Timer => 0x3000_0000,
            EventKind::ResourceAlloc => 0x4000_0000,
            EventKind::Preemption => 0x5000_0000,
            EventKind::IpcMessage => 0x6000_0000,
        };
        val.to_le_bytes()
    }
}

// ============================================================================
// SCENARIO NODE
// ============================================================================

/// A single node in the scenario tree, representing one possible system state.
#[derive(Debug, Clone)]
pub struct ScenarioNode {
    /// FNV-1a hash of this state
    pub state_hash: u64,
    /// Probability of reaching this node from the root
    pub probability: f32,
    /// Estimated outcome quality at this node (higher = better)
    pub outcome_estimate: f32,
    /// Depth in the tree (root = 0)
    pub depth: u8,
    /// Event that led to this node from its parent
    pub triggering_event: Option<EventKind>,
    /// Indices of children in the arena
    pub children: Vec<usize>,
    /// Minimax value computed during evaluation
    pub minimax_value: f32,
    /// Whether this node has been pruned
    pub pruned: bool,
}

impl ScenarioNode {
    fn new(state_hash: u64, probability: f32, depth: u8) -> Self {
        Self {
            state_hash,
            probability,
            outcome_estimate: 0.0,
            depth,
            triggering_event: None,
            children: Vec::new(),
            minimax_value: 0.0,
            pruned: false,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty() || self.pruned
    }
}

// ============================================================================
// PATH RESULT
// ============================================================================

/// A path through the scenario tree from root to a leaf.
#[derive(Debug, Clone)]
pub struct ScenarioPath {
    /// Sequence of node indices in the arena
    pub node_indices: Vec<usize>,
    /// Total probability along this path
    pub probability: f32,
    /// Outcome value at the leaf
    pub outcome: f32,
    /// Events taken along the path
    pub events: Vec<EventKind>,
}

// ============================================================================
// SCENARIO TREE STATS
// ============================================================================

/// Statistics for the scenario tree engine.
#[derive(Debug, Clone)]
pub struct ScenarioTreeStats {
    pub total_trees_built: u64,
    pub total_nodes_created: u64,
    pub total_nodes_pruned: u64,
    pub avg_branch_factor: f32,
    pub avg_tree_depth: f32,
    pub best_path_evaluations: u64,
    pub worst_path_evaluations: u64,
    pub expected_value_queries: u64,
}

impl ScenarioTreeStats {
    fn new() -> Self {
        Self {
            total_trees_built: 0,
            total_nodes_created: 0,
            total_nodes_pruned: 0,
            avg_branch_factor: 0.0,
            avg_tree_depth: 0.0,
            best_path_evaluations: 0,
            worst_path_evaluations: 0,
            expected_value_queries: 0,
        }
    }
}

// ============================================================================
// TRANSITION TABLE
// ============================================================================

/// Records observed event transitions and their probabilities.
#[derive(Debug, Clone)]
struct TransitionEntry {
    from_hash: u64,
    events: Vec<(EventKind, f32, u64)>, // (event, probability, resulting_state_hash)
    total_observations: u64,
}

// ============================================================================
// BRIDGE SCENARIO TREE
// ============================================================================

/// Branching scenario tree engine for syscall future prediction.
///
/// Builds trees of possible futures up to depth 8. Each node represents a
/// possible system state, and edges are events. Uses minimax evaluation to
/// find optimal and worst-case paths through the tree.
pub struct BridgeScenarioTree {
    /// Arena of all nodes in the current tree
    arena: Vec<ScenarioNode>,
    /// Root index in the arena (always 0 when tree is built)
    root: Option<usize>,
    /// Transition table: state_hash -> possible next events
    transitions: BTreeMap<u64, TransitionEntry>,
    /// Outcome estimator: state_hash -> estimated quality
    outcome_cache: BTreeMap<u64, f32>,
    /// Running statistics
    stats: ScenarioTreeStats,
    /// PRNG state
    rng: u64,
    /// Maximum depth for tree building
    max_depth: usize,
    /// EMA of tree quality across builds
    quality_ema: f32,
    /// History of scenario evaluations: scenario_id -> expected value
    evaluation_history: BTreeMap<u64, Vec<f32>>,
}

impl BridgeScenarioTree {
    /// Create a new scenario tree engine.
    pub fn new() -> Self {
        Self {
            arena: Vec::new(),
            root: None,
            transitions: BTreeMap::new(),
            outcome_cache: BTreeMap::new(),
            stats: ScenarioTreeStats::new(),
            rng: DEFAULT_SEED,
            max_depth: MAX_TREE_DEPTH,
            quality_ema: 0.5,
            evaluation_history: BTreeMap::new(),
        }
    }

    /// Record a state transition for future tree building.
    pub fn record_transition(
        &mut self,
        from_state: u64,
        event: EventKind,
        to_state: u64,
        observed_outcome: f32,
    ) {
        let entry = self.transitions.entry(from_state).or_insert_with(|| {
            TransitionEntry {
                from_hash: from_state,
                events: Vec::new(),
                total_observations: 0,
            }
        });
        entry.total_observations += 1;

        let mut found = false;
        for (ev, prob, dest) in entry.events.iter_mut() {
            if core::mem::discriminant(ev) == core::mem::discriminant(&event)
                && *dest == to_state
            {
                // Update probability with EMA
                let obs_count = entry.total_observations as f32;
                *prob = *prob * (1.0 - 1.0 / obs_count) + 1.0 / obs_count;
                found = true;
                break;
            }
        }
        if !found && entry.events.len() < MAX_CHILDREN_PER_NODE {
            let initial_prob = 1.0 / (entry.events.len() + 1) as f32;
            entry.events.push((event, initial_prob, to_state));
        }

        // Normalize probabilities
        let total: f32 = entry.events.iter().map(|(_, p, _)| *p).sum();
        if total > 0.0 {
            for (_, p, _) in entry.events.iter_mut() {
                *p /= total;
            }
        }

        // Update outcome cache
        let cached = self.outcome_cache.entry(to_state).or_insert(0.5);
        *cached = *cached * (1.0 - EMA_ALPHA) + observed_outcome * EMA_ALPHA;

        if self.transitions.len() > MAX_SCENARIOS {
            // Evict least observed
            let mut min_obs = u64::MAX;
            let mut min_key = 0u64;
            for (k, v) in self.transitions.iter() {
                if v.total_observations < min_obs {
                    min_obs = v.total_observations;
                    min_key = *k;
                }
            }
            self.transitions.remove(&min_key);
        }
    }

    /// Build a scenario tree rooted at the given system state.
    pub fn build_tree(&mut self, root_state: u64) {
        self.arena.clear();
        let root = ScenarioNode::new(root_state, 1.0, 0);
        self.arena.push(root);
        self.root = Some(0);

        let mut stack: Vec<usize> = Vec::new();
        stack.push(0);

        while let Some(node_idx) = stack.pop() {
            if self.arena.len() >= MAX_NODES {
                break;
            }
            let depth = self.arena[node_idx].depth;
            if depth as usize >= self.max_depth {
                // Assign leaf outcome
                let sh = self.arena[node_idx].state_hash;
                let outcome = self.outcome_cache.get(&sh).copied().unwrap_or(0.5);
                self.arena[node_idx].outcome_estimate = outcome;
                continue;
            }

            let state_hash = self.arena[node_idx].state_hash;
            let parent_prob = self.arena[node_idx].probability;

            // Look up transitions from this state
            if let Some(entry) = self.transitions.get(&state_hash).cloned() {
                for (event, prob, dest_hash) in entry.events.iter() {
                    if self.arena.len() >= MAX_NODES {
                        break;
                    }
                    let child_prob = parent_prob * prob;
                    if child_prob < PRUNE_THRESHOLD * 0.1 {
                        continue;
                    }
                    let mut child = ScenarioNode::new(*dest_hash, child_prob, depth + 1);
                    child.triggering_event = Some(*event);
                    let child_idx = self.arena.len();
                    self.arena.push(child);
                    self.arena[node_idx].children.push(child_idx);
                    stack.push(child_idx);
                }
            } else {
                // No transitions known: generate synthetic children
                let sh = state_hash;
                let outcome = self.outcome_cache.get(&sh).copied().unwrap_or(0.5);
                self.arena[node_idx].outcome_estimate = outcome;
            }
        }

        // Propagate minimax values bottom-up
        self.propagate_minimax(0, true);

        self.stats.total_trees_built += 1;
        self.stats.total_nodes_created += self.arena.len() as u64;

        // Update average depth and branch factor
        let (depth_sum, leaf_count, branch_sum, internal_count) = self.tree_metrics();
        if leaf_count > 0 {
            let avg_d = depth_sum as f32 / leaf_count as f32;
            self.stats.avg_tree_depth =
                self.stats.avg_tree_depth * (1.0 - EMA_ALPHA) + avg_d * EMA_ALPHA;
        }
        if internal_count > 0 {
            let avg_b = branch_sum as f32 / internal_count as f32;
            self.stats.avg_branch_factor =
                self.stats.avg_branch_factor * (1.0 - EMA_ALPHA) + avg_b * EMA_ALPHA;
        }
    }

    fn propagate_minimax(&mut self, node_idx: usize, maximizing: bool) -> f32 {
        if node_idx >= self.arena.len() {
            return 0.0;
        }
        if self.arena[node_idx].is_leaf() {
            let val = self.arena[node_idx].outcome_estimate;
            self.arena[node_idx].minimax_value = val;
            return val;
        }

        let children = self.arena[node_idx].children.clone();
        let mut best = if maximizing { f32::NEG_INFINITY } else { f32::INFINITY };

        for &child_idx in &children {
            let child_val = self.propagate_minimax(child_idx, !maximizing);
            if maximizing {
                if child_val > best {
                    best = child_val;
                }
            } else if child_val < best {
                best = child_val;
            }
        }

        if best.is_infinite() {
            best = 0.5;
        }
        self.arena[node_idx].minimax_value = best;
        best
    }

    fn tree_metrics(&self) -> (u64, u64, u64, u64) {
        let mut depth_sum = 0u64;
        let mut leaf_count = 0u64;
        let mut branch_sum = 0u64;
        let mut internal_count = 0u64;
        for node in &self.arena {
            if node.is_leaf() {
                depth_sum += node.depth as u64;
                leaf_count += 1;
            } else {
                branch_sum += node.children.len() as u64;
                internal_count += 1;
            }
        }
        (depth_sum, leaf_count, branch_sum, internal_count)
    }

    /// Find the best path through the scenario tree (highest outcome).
    pub fn best_path(&mut self) -> Option<ScenarioPath> {
        self.stats.best_path_evaluations += 1;
        let root_idx = self.root?;
        let mut path = ScenarioPath {
            node_indices: Vec::new(),
            probability: 1.0,
            outcome: 0.0,
            events: Vec::new(),
        };
        self.trace_path(root_idx, true, &mut path);
        Some(path)
    }

    /// Find the worst path through the scenario tree (lowest outcome).
    pub fn worst_path(&mut self) -> Option<ScenarioPath> {
        self.stats.worst_path_evaluations += 1;
        let root_idx = self.root?;
        let mut path = ScenarioPath {
            node_indices: Vec::new(),
            probability: 1.0,
            outcome: 0.0,
            events: Vec::new(),
        };
        self.trace_path(root_idx, false, &mut path);
        Some(path)
    }

    fn trace_path(&self, node_idx: usize, maximize: bool, path: &mut ScenarioPath) {
        if node_idx >= self.arena.len() {
            return;
        }
        let node = &self.arena[node_idx];
        path.node_indices.push(node_idx);
        if let Some(ev) = node.triggering_event {
            path.events.push(ev);
        }

        if node.is_leaf() {
            path.probability = node.probability;
            path.outcome = node.minimax_value;
            return;
        }

        let mut best_child: Option<usize> = None;
        let mut best_val = if maximize { f32::NEG_INFINITY } else { f32::INFINITY };

        for &child_idx in &node.children {
            if child_idx < self.arena.len() {
                let cv = self.arena[child_idx].minimax_value;
                if maximize && cv > best_val {
                    best_val = cv;
                    best_child = Some(child_idx);
                } else if !maximize && cv < best_val {
                    best_val = cv;
                    best_child = Some(child_idx);
                }
            }
        }

        if let Some(child) = best_child {
            self.trace_path(child, maximize, path);
        }
    }

    /// Compute the probability-weighted expected value across all leaf nodes.
    pub fn expected_value(&mut self) -> f32 {
        self.stats.expected_value_queries += 1;
        if self.arena.is_empty() {
            return 0.0;
        }
        let mut weighted_sum = 0.0f32;
        let mut total_prob = 0.0f32;
        for node in &self.arena {
            if node.is_leaf() && !node.pruned {
                weighted_sum += node.probability * node.outcome_estimate;
                total_prob += node.probability;
            }
        }
        if total_prob > 0.0 {
            let ev = weighted_sum / total_prob;
            // Store in evaluation history
            let root_hash = self.arena.first().map(|n| n.state_hash).unwrap_or(0);
            let hist = self.evaluation_history.entry(root_hash).or_insert_with(Vec::new);
            hist.push(ev);
            if hist.len() > 64 {
                hist.remove(0);
            }
            self.quality_ema = self.quality_ema * (1.0 - EMA_ALPHA) + ev * EMA_ALPHA;
            ev
        } else {
            0.5
        }
    }

    /// Prune nodes with probability below threshold.
    pub fn prune_unlikely(&mut self, threshold: f32) -> usize {
        let thresh = if threshold <= 0.0 { PRUNE_THRESHOLD } else { threshold };
        let mut pruned_count = 0usize;
        for i in 0..self.arena.len() {
            if self.arena[i].probability < thresh && !self.arena[i].pruned && i != 0 {
                self.arena[i].pruned = true;
                self.arena[i].children.clear();
                pruned_count += 1;
            }
        }
        self.stats.total_nodes_pruned += pruned_count as u64;
        pruned_count
    }

    /// Return the maximum depth of the current tree.
    pub fn tree_depth(&self) -> usize {
        let mut max_d = 0usize;
        for node in &self.arena {
            if node.depth as usize > max_d {
                max_d = node.depth as usize;
            }
        }
        max_d
    }

    /// Compute the average branching factor of the tree.
    pub fn branch_factor(&self) -> f32 {
        let mut sum = 0usize;
        let mut count = 0usize;
        for node in &self.arena {
            if !node.is_leaf() {
                sum += node.children.len();
                count += 1;
            }
        }
        if count > 0 {
            sum as f32 / count as f32
        } else {
            0.0
        }
    }

    /// Get the total number of nodes in the current tree.
    pub fn node_count(&self) -> usize {
        self.arena.len()
    }

    /// Get the number of leaf nodes in the current tree.
    pub fn leaf_count(&self) -> usize {
        self.arena.iter().filter(|n| n.is_leaf()).count()
    }

    /// Get current statistics.
    pub fn stats(&self) -> &ScenarioTreeStats {
        &self.stats
    }

    /// Get the EMA quality score across all tree evaluations.
    pub fn quality_score(&self) -> f32 {
        self.quality_ema
    }

    /// Evaluate a specific scenario identifier and return its expected value.
    pub fn evaluate_scenario(&mut self, scenario_id: u64) -> f32 {
        let hash_bytes = scenario_id.to_le_bytes();
        let state_hash = fnv1a_hash(&hash_bytes);
        self.build_tree(state_hash);
        self.expected_value()
    }

    /// Generate synthetic transitions for testing / cold-start.
    pub fn generate_synthetic(&mut self, base_state: u64, breadth: usize) {
        let depth_limit = if self.max_depth > 4 { 4 } else { self.max_depth };
        let b = if breadth > MAX_CHILDREN_PER_NODE { MAX_CHILDREN_PER_NODE } else { breadth };

        let mut stack: Vec<(u64, usize)> = Vec::new();
        stack.push((base_state, 0));

        while let Some((state, depth)) = stack.pop() {
            if depth >= depth_limit {
                continue;
            }
            for i in 0..b {
                let event = match i % 4 {
                    0 => EventKind::Syscall(xorshift64(&mut self.rng) as u32 % 256),
                    1 => EventKind::Interrupt(xorshift64(&mut self.rng) as u16 % 16),
                    2 => EventKind::Timer,
                    _ => EventKind::IpcMessage,
                };
                let next_bytes = [state.to_le_bytes(), (i as u64).to_le_bytes()].concat();
                let next_hash = fnv1a_hash(&next_bytes);
                let outcome = rand_f32(&mut self.rng);
                self.record_transition(state, event, next_hash, outcome);
                if depth + 1 < depth_limit {
                    stack.push((next_hash, depth + 1));
                }
            }
        }
    }

    /// Get all paths from root to leaves, sorted by outcome descending.
    pub fn all_paths_sorted(&self) -> Vec<ScenarioPath> {
        let mut paths = Vec::new();
        if self.arena.is_empty() {
            return paths;
        }
        let mut stack: Vec<(usize, Vec<usize>, Vec<EventKind>)> = Vec::new();
        stack.push((0, Vec::new(), Vec::new()));

        while let Some((idx, mut indices, mut events)) = stack.pop() {
            if idx >= self.arena.len() {
                continue;
            }
            indices.push(idx);
            if let Some(ev) = self.arena[idx].triggering_event {
                events.push(ev);
            }
            if self.arena[idx].is_leaf() {
                paths.push(ScenarioPath {
                    node_indices: indices,
                    probability: self.arena[idx].probability,
                    outcome: self.arena[idx].outcome_estimate,
                    events,
                });
            } else {
                for &child_idx in &self.arena[idx].children {
                    stack.push((child_idx, indices.clone(), events.clone()));
                }
            }
        }
        paths.sort_by(|a, b| b.outcome.partial_cmp(&a.outcome).unwrap_or(core::cmp::Ordering::Equal));
        paths
    }
}
