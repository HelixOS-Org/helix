//! NEAT Genome - the genetic encoding of a neural network.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::connection::ConnectionGene;
use crate::neuroevo::innovation::InnovationTracker;
use crate::neuroevo::node::{NodeGene, NodeType};
use crate::neuroevo::types::{InnovationNumber, NodeId};
use crate::neuroevo::utils::{lcg_next, random_weight};

/// NEAT Genome - the genetic encoding of a neural network
#[derive(Debug, Clone)]
pub struct NeatGenome {
    /// Node genes
    pub nodes: Vec<NodeGene>,
    /// Connection genes (sorted by innovation number)
    pub connections: Vec<ConnectionGene>,
    /// Fitness score
    pub fitness: f64,
    /// Adjusted fitness (for speciation)
    pub adjusted_fitness: f64,
    /// Species ID
    pub species_id: usize,
    /// Number of input nodes
    pub num_inputs: usize,
    /// Number of output nodes
    pub num_outputs: usize,
    /// Generation born
    pub generation: u32,
}

impl NeatGenome {
    /// Create a minimal genome (no hidden nodes, direct input-output connections)
    pub fn minimal(
        num_inputs: usize,
        num_outputs: usize,
        output_activation: ActivationFunction,
        tracker: &mut InnovationTracker,
    ) -> Self {
        let mut nodes = Vec::with_capacity(num_inputs + num_outputs + 1);
        let mut connections = Vec::new();

        // Create bias node
        let bias_id = tracker.new_node_id();
        nodes.push(NodeGene::new(
            bias_id,
            NodeType::Bias,
            ActivationFunction::Identity,
        ));

        // Create input nodes
        let input_ids: Vec<NodeId> = (0..num_inputs)
            .map(|_| {
                let id = tracker.new_node_id();
                nodes.push(NodeGene::input(id));
                id
            })
            .collect();

        // Create output nodes
        let output_ids: Vec<NodeId> = (0..num_outputs)
            .map(|_| {
                let id = tracker.new_node_id();
                nodes.push(NodeGene::output(id, output_activation));
                id
            })
            .collect();

        // Create full connections from inputs/bias to outputs
        for &input_id in &input_ids {
            for &output_id in &output_ids {
                let innovation = tracker.get_or_create(input_id, output_id);
                let weight = random_weight(innovation);
                connections.push(ConnectionGene::new(input_id, output_id, weight, innovation));
            }
        }

        // Connect bias to outputs
        for &output_id in &output_ids {
            let innovation = tracker.get_or_create(bias_id, output_id);
            let weight = random_weight(innovation);
            connections.push(ConnectionGene::new(bias_id, output_id, weight, innovation));
        }

        Self {
            nodes,
            connections,
            fitness: 0.0,
            adjusted_fitness: 0.0,
            species_id: 0,
            num_inputs,
            num_outputs,
            generation: 0,
        }
    }

    /// Get the complexity of this genome
    #[inline(always)]
    pub fn complexity(&self) -> usize {
        self.nodes.len() + self.connections.iter().filter(|c| c.enabled).count()
    }

    /// Check if a connection would create a cycle
    pub fn would_create_cycle(&self, from: NodeId, to: NodeId) -> bool {
        // Build adjacency list
        let mut adj: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
        for conn in &self.connections {
            if conn.enabled {
                adj.entry(conn.from).or_default().push(conn.to);
            }
        }
        // Add the proposed connection
        adj.entry(from).or_default().push(to);

        // DFS to detect cycle
        let mut visited = BTreeMap::new();
        let mut rec_stack = BTreeMap::new();

        for node in self.nodes.iter() {
            if self.has_cycle_util(node.id, &adj, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    fn has_cycle_util(
        &self,
        node: NodeId,
        adj: &BTreeMap<NodeId, Vec<NodeId>>,
        visited: &mut BTreeMap<NodeId, bool>,
        rec_stack: &mut BTreeMap<NodeId, bool>,
    ) -> bool {
        if *rec_stack.get(&node).unwrap_or(&false) {
            return true;
        }
        if *visited.get(&node).unwrap_or(&false) {
            return false;
        }

        visited.insert(node, true);
        rec_stack.insert(node, true);

        if let Some(neighbors) = adj.get(&node) {
            for &neighbor in neighbors {
                if self.has_cycle_util(neighbor, adj, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.insert(node, false);
        false
    }

    /// Add a new connection mutation
    pub fn mutate_add_connection(&mut self, tracker: &mut InnovationTracker, seed: u64) {
        // Find eligible node pairs
        let mut candidates = Vec::new();
        for from_node in &self.nodes {
            if from_node.node_type == NodeType::Output {
                continue; // Can't have outputs as sources
            }
            for to_node in &self.nodes {
                if to_node.node_type == NodeType::Input || to_node.node_type == NodeType::Bias {
                    continue; // Can't have inputs as targets
                }
                if from_node.id == to_node.id {
                    continue; // No self-connections
                }
                // Check if connection already exists
                let exists = self
                    .connections
                    .iter()
                    .any(|c| c.from == from_node.id && c.to == to_node.id);
                if !exists && !self.would_create_cycle(from_node.id, to_node.id) {
                    candidates.push((from_node.id, to_node.id));
                }
            }
        }

        if candidates.is_empty() {
            return;
        }

        // Pick a random candidate
        let idx = (seed as usize) % candidates.len();
        let (from, to) = candidates[idx];

        let innovation = tracker.get_or_create(from, to);
        let weight = random_weight(seed);
        self.connections
            .push(ConnectionGene::new(from, to, weight, innovation));
    }

    /// Add a new node by splitting an existing connection
    pub fn mutate_add_node(&mut self, tracker: &mut InnovationTracker, seed: u64) {
        // Find enabled connections to split
        let enabled: Vec<usize> = self
            .connections
            .iter()
            .enumerate()
            .filter(|(_, c)| c.enabled)
            .map(|(i, _)| i)
            .collect();

        if enabled.is_empty() {
            return;
        }

        // Pick a random connection
        let conn_idx = enabled[(seed as usize) % enabled.len()];
        let old_conn = &self.connections[conn_idx];
        let from = old_conn.from;
        let to = old_conn.to;
        let old_weight = old_conn.weight;

        // Disable old connection
        self.connections[conn_idx].enabled = false;

        // Create new hidden node
        let new_node_id = tracker.new_node_id();
        let activation = ActivationFunction::random(seed / 7);
        self.nodes.push(NodeGene::hidden(new_node_id, activation));

        // Create two new connections: from -> new -> to
        let innov1 = tracker.get_or_create(from, new_node_id);
        let innov2 = tracker.get_or_create(new_node_id, to);

        // First connection has weight 1.0, second has old weight (preserves behavior)
        self.connections
            .push(ConnectionGene::new(from, new_node_id, 1.0, innov1));
        self.connections
            .push(ConnectionGene::new(new_node_id, to, old_weight, innov2));
    }

    /// Mutate connection weights
    pub fn mutate_weights(&mut self, mutation_rate: f64, perturbation_power: f64, seed: u64) {
        let mut rng_state = seed;
        for conn in &mut self.connections {
            rng_state = lcg_next(rng_state);
            let rand = (rng_state as f64) / (u64::MAX as f64);

            if rand < mutation_rate {
                rng_state = lcg_next(rng_state);
                let rand2 = (rng_state as f64) / (u64::MAX as f64);

                if rand2 < 0.1 {
                    // 10% chance of complete reset
                    conn.weight = random_weight(rng_state);
                } else {
                    // 90% chance of perturbation
                    let perturbation =
                        ((rng_state as f64) / (u64::MAX as f64) * 2.0 - 1.0) * perturbation_power;
                    conn.weight += perturbation;
                    conn.weight = conn.weight.clamp(-5.0, 5.0);
                }
            }
        }
    }

    /// Compute compatibility distance for speciation
    pub fn compatibility_distance(&self, other: &NeatGenome, c1: f64, c2: f64, c3: f64) -> f64 {
        let mut excess = 0.0;
        let mut disjoint = 0.0;
        let mut matching = 0;
        let mut weight_diff_sum = 0.0;

        let max_innov_self = self
            .connections
            .iter()
            .map(|c| c.innovation)
            .max()
            .unwrap_or(0);
        let max_innov_other = other
            .connections
            .iter()
            .map(|c| c.innovation)
            .max()
            .unwrap_or(0);
        let _max_innov = max_innov_self.max(max_innov_other);
        let min_max = max_innov_self.min(max_innov_other);

        let other_map: BTreeMap<InnovationNumber, &ConnectionGene> = other
            .connections
            .iter()
            .map(|c| (c.innovation, c))
            .collect();

        let self_innovations: alloc::collections::BTreeSet<InnovationNumber> =
            self.connections.iter().map(|c| c.innovation).collect();

        for conn in &self.connections {
            if let Some(other_conn) = other_map.get(&conn.innovation) {
                matching += 1;
                weight_diff_sum += libm::fabs(conn.weight - other_conn.weight);
            } else if conn.innovation > min_max {
                excess += 1.0;
            } else {
                disjoint += 1.0;
            }
        }

        // Count excess/disjoint from other genome
        for conn in &other.connections {
            if !self_innovations.contains(&conn.innovation) {
                if conn.innovation > min_max {
                    excess += 1.0;
                } else {
                    disjoint += 1.0;
                }
            }
        }

        let n = self.connections.len().max(other.connections.len()).max(1) as f64;
        let avg_weight_diff = if matching > 0 {
            weight_diff_sum / matching as f64
        } else {
            0.0
        };

        (c1 * excess / n) + (c2 * disjoint / n) + (c3 * avg_weight_diff)
    }

    /// Crossover two genomes
    pub fn crossover(parent1: &NeatGenome, parent2: &NeatGenome, seed: u64) -> NeatGenome {
        // More fit parent determines structure
        let (fitter, less_fit) = if parent1.fitness >= parent2.fitness {
            (parent1, parent2)
        } else {
            (parent2, parent1)
        };

        let mut child = fitter.clone();

        // Create map of less fit parent's connections
        let less_fit_map: BTreeMap<InnovationNumber, &ConnectionGene> = less_fit
            .connections
            .iter()
            .map(|c| (c.innovation, c))
            .collect();

        let mut rng = seed;
        for conn in &mut child.connections {
            if let Some(other_conn) = less_fit_map.get(&conn.innovation) {
                // Matching gene - randomly inherit weight
                rng = lcg_next(rng);
                if rng % 2 == 0 {
                    conn.weight = other_conn.weight;
                }

                // If either parent has it disabled, chance of inheriting disabled
                if !conn.enabled || !other_conn.enabled {
                    rng = lcg_next(rng);
                    conn.enabled = rng % 100 >= 75; // 75% chance of being disabled
                }
            }
        }

        child.fitness = 0.0;
        child.adjusted_fitness = 0.0;
        child.generation = parent1.generation.max(parent2.generation) + 1;

        child
    }
}
