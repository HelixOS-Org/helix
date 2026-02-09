// SPDX-License-Identifier: GPL-2.0
//! # Cooperation World Model
//!
//! The cooperation engine's view of the inter-process world. Models trust
//! networks between participants, resource flow graphs, and cooperation
//! topology. Predicts future cooperation patterns and detects network
//! entropy shifts that signal regime changes.
//!
//! A cooperation engine that understands its environment can anticipate
//! needs, pre-position resources, and foster deeper collaboration.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_NODES: usize = 256;
const MAX_EDGES: usize = 1024;
const MAX_PREDICTION_HISTORY: usize = 128;
const TRUST_DECAY_RATE: f32 = 0.99;
const ENTROPY_SMOOTHING: f32 = 0.08;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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

// ============================================================================
// TOPOLOGY TYPES
// ============================================================================

/// A node in the cooperation topology (a participant)
#[derive(Debug, Clone)]
pub struct TopoNode {
    pub name: String,
    pub id: u64,
    /// Trust score (0.0 – 1.0)
    pub trust: f32,
    /// Resource contribution (arbitrary units)
    pub resource_contribution: f32,
    /// Resource consumption (arbitrary units)
    pub resource_consumption: f32,
    /// Cooperation frequency (interactions per tick)
    pub cooperation_frequency: f32,
    /// Number of active edges (connections)
    pub edge_count: u32,
    /// Last interaction tick
    pub last_interaction_tick: u64,
    /// Total interactions
    pub total_interactions: u64,
}

/// An edge in the cooperation topology (a relationship)
#[derive(Debug, Clone)]
pub struct TopoEdge {
    pub source_id: u64,
    pub target_id: u64,
    /// Mutual trust between nodes (0.0 – 1.0)
    pub mutual_trust: f32,
    /// Resource flow rate along this edge
    pub flow_rate: f32,
    /// Direction: positive = source→target dominance
    pub flow_direction: f32,
    /// Interaction count on this edge
    pub interactions: u64,
    /// Edge weight for graph algorithms
    pub weight: f32,
}

/// A cooperation prediction
#[derive(Debug, Clone, Copy)]
pub struct CoopPrediction {
    pub predicted_trust: f32,
    pub actual_trust: f32,
    pub error: f32,
    pub predicted_flow: f32,
    pub actual_flow: f32,
    pub tick: u64,
}

// ============================================================================
// WORLD MODEL STATS
// ============================================================================

/// Aggregate world model statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct WorldModelStats {
    pub nodes_tracked: usize,
    pub edges_tracked: usize,
    pub avg_trust: f32,
    pub network_density: f32,
    pub total_resource_flow: f32,
    pub network_entropy: f32,
    pub prediction_accuracy: f32,
    pub cooperation_temperature: f32,
}

// ============================================================================
// COOPERATION WORLD MODEL
// ============================================================================

/// The cooperation engine's model of the inter-process world — trust
/// networks, resource flow, topology, and cooperation prediction.
#[derive(Debug)]
pub struct CoopWorldModel {
    /// Topology nodes (keyed by FNV hash of participant name)
    nodes: BTreeMap<u64, TopoNode>,
    /// Topology edges (keyed by combined hash of source+target)
    edges: BTreeMap<u64, TopoEdge>,
    /// Prediction history for accuracy tracking
    predictions: Vec<CoopPrediction>,
    pred_write_idx: usize,
    /// EMA of prediction error
    avg_prediction_error: f32,
    /// EMA of network entropy
    entropy_ema: f32,
    /// Total resource flow observed
    total_flow: f32,
    /// Monotonic tick
    tick: u64,
    /// PRNG state for prediction jitter
    rng_state: u64,
}

impl CoopWorldModel {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            predictions: Vec::new(),
            pred_write_idx: 0,
            avg_prediction_error: 0.5,
            entropy_ema: 0.5,
            total_flow: 0.0,
            tick: 0,
            rng_state: 0xD0E1_DC0B_DEAD_BEEF,
        }
    }

    /// Update cooperation topology with a new interaction between participants
    pub fn update_topology(
        &mut self,
        source_name: &str,
        target_name: &str,
        trust_delta: f32,
        resource_flow: f32,
    ) {
        self.tick += 1;
        let src_id = fnv1a_hash(source_name.as_bytes());
        let tgt_id = fnv1a_hash(target_name.as_bytes());

        // Update source node
        let src = self.nodes.entry(src_id).or_insert_with(|| TopoNode {
            name: String::from(source_name),
            id: src_id,
            trust: 0.5,
            resource_contribution: 0.0,
            resource_consumption: 0.0,
            cooperation_frequency: 0.0,
            edge_count: 0,
            last_interaction_tick: 0,
            total_interactions: 0,
        });
        src.total_interactions += 1;
        src.last_interaction_tick = self.tick;
        src.resource_contribution += resource_flow.max(0.0);
        src.cooperation_frequency = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * src.cooperation_frequency;
        src.trust = (src.trust + trust_delta * 0.1).max(0.0).min(1.0);

        // Update target node
        let tgt = self.nodes.entry(tgt_id).or_insert_with(|| TopoNode {
            name: String::from(target_name),
            id: tgt_id,
            trust: 0.5,
            resource_contribution: 0.0,
            resource_consumption: 0.0,
            cooperation_frequency: 0.0,
            edge_count: 0,
            last_interaction_tick: 0,
            total_interactions: 0,
        });
        tgt.total_interactions += 1;
        tgt.last_interaction_tick = self.tick;
        tgt.resource_consumption += resource_flow.max(0.0);
        tgt.cooperation_frequency = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * tgt.cooperation_frequency;
        tgt.trust = (tgt.trust + trust_delta * 0.1).max(0.0).min(1.0);

        // Update edge
        let edge_key = src_id.wrapping_mul(FNV_PRIME) ^ tgt_id;
        let is_new = !self.edges.contains_key(&edge_key);
        let edge = self.edges.entry(edge_key).or_insert_with(|| TopoEdge {
            source_id: src_id,
            target_id: tgt_id,
            mutual_trust: 0.5,
            flow_rate: 0.0,
            flow_direction: 0.0,
            interactions: 0,
            weight: 1.0,
        });
        edge.interactions += 1;
        edge.flow_rate = EMA_ALPHA * resource_flow + (1.0 - EMA_ALPHA) * edge.flow_rate;
        edge.mutual_trust = (edge.mutual_trust + trust_delta * 0.05).max(0.0).min(1.0);
        edge.weight = edge.mutual_trust * edge.flow_rate.max(0.01);

        let flow_bias = resource_flow - edge.flow_rate;
        edge.flow_direction = EMA_ALPHA * flow_bias + (1.0 - EMA_ALPHA) * edge.flow_direction;

        self.total_flow += resource_flow.abs();

        // Update edge counts if this is a new edge
        if is_new {
            if let Some(s) = self.nodes.get_mut(&src_id) {
                s.edge_count += 1;
            }
            if let Some(t) = self.nodes.get_mut(&tgt_id) {
                t.edge_count += 1;
            }
        }
    }

    /// Evaluate trust network health: avg trust × connectivity
    pub fn trust_network_health(&self) -> f32 {
        if self.nodes.is_empty() {
            return 0.0;
        }

        let avg_trust = self.nodes.values().map(|n| n.trust).sum::<f32>() / self.nodes.len() as f32;

        let max_edges = self.nodes.len() * (self.nodes.len().saturating_sub(1)) / 2;
        let density = if max_edges > 0 {
            (self.edges.len() as f32 / max_edges as f32).min(1.0)
        } else {
            0.0
        };

        // Decay stale trust
        let active_ratio = self
            .nodes
            .values()
            .filter(|n| self.tick - n.last_interaction_tick < 100)
            .count() as f32
            / self.nodes.len() as f32;

        avg_trust * 0.40 + density * 0.30 + active_ratio * 0.30
    }

    /// Compute total resource flow through the network
    pub fn resource_flow(&self) -> f32 {
        self.edges.values().map(|e| e.flow_rate).sum()
    }

    /// Predict future cooperation likelihood between two participants
    pub fn predict_cooperation(&mut self, source_name: &str, target_name: &str) -> f32 {
        self.tick += 1;
        let src_id = fnv1a_hash(source_name.as_bytes());
        let tgt_id = fnv1a_hash(target_name.as_bytes());
        let edge_key = src_id.wrapping_mul(FNV_PRIME) ^ tgt_id;

        let base_prediction = if let Some(edge) = self.edges.get(&edge_key) {
            let trust_factor = edge.mutual_trust;
            let frequency_factor = (edge.interactions as f32 / 100.0).min(1.0);
            trust_factor * 0.60 + frequency_factor * 0.40
        } else {
            // No history — use node-level trust as prior
            let src_trust = self.nodes.get(&src_id).map(|n| n.trust).unwrap_or(0.3);
            let tgt_trust = self.nodes.get(&tgt_id).map(|n| n.trust).unwrap_or(0.3);
            (src_trust + tgt_trust) / 2.0 * 0.5
        };

        // Add small jitter to avoid overconfident predictions
        let jitter_raw = xorshift64(&mut self.rng_state);
        let jitter = ((jitter_raw % 100) as f32 / 1000.0) - 0.05;

        (base_prediction + jitter).max(0.0).min(1.0)
    }

    /// Compute network entropy: higher = more chaotic cooperation patterns
    pub fn network_entropy(&mut self) -> f32 {
        if self.nodes.is_empty() {
            return 0.0;
        }

        // Shannon entropy over node cooperation frequencies
        let total_freq: f32 = self
            .nodes
            .values()
            .map(|n| n.cooperation_frequency)
            .sum::<f32>()
            .max(f32::EPSILON);

        let mut entropy = 0.0_f32;
        for node in self.nodes.values() {
            let p = node.cooperation_frequency / total_freq;
            if p > f32::EPSILON {
                entropy -= p * libm::log2f(p);
            }
        }

        // Normalize by log2(N)
        let max_entropy = libm::log2f(self.nodes.len() as f32).max(1.0);
        let normalized = (entropy / max_entropy).max(0.0).min(1.0);

        self.entropy_ema =
            ENTROPY_SMOOTHING * normalized + (1.0 - ENTROPY_SMOOTHING) * self.entropy_ema;
        self.entropy_ema
    }

    /// Get aggregate statistics
    pub fn stats(&mut self) -> WorldModelStats {
        let avg_trust = if self.nodes.is_empty() {
            0.0
        } else {
            self.nodes.values().map(|n| n.trust).sum::<f32>() / self.nodes.len() as f32
        };

        let max_edges = self.nodes.len() * self.nodes.len().saturating_sub(1) / 2;
        let density = if max_edges > 0 {
            (self.edges.len() as f32 / max_edges as f32).min(1.0)
        } else {
            0.0
        };

        let entropy = self.network_entropy();

        // Cooperation temperature: high interactions + high trust = warm
        let temperature = if self.nodes.is_empty() {
            0.0
        } else {
            let avg_freq = self
                .nodes
                .values()
                .map(|n| n.cooperation_frequency)
                .sum::<f32>()
                / self.nodes.len() as f32;
            (avg_trust * 0.5 + avg_freq.min(1.0) * 0.5).min(1.0)
        };

        WorldModelStats {
            nodes_tracked: self.nodes.len(),
            edges_tracked: self.edges.len(),
            avg_trust,
            network_density: density,
            total_resource_flow: self.resource_flow(),
            network_entropy: entropy,
            prediction_accuracy: 1.0 - self.avg_prediction_error,
            cooperation_temperature: temperature,
        }
    }
}
