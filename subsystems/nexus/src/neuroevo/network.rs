//! Neural network phenotype built from NEAT genomes.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::genome::NeatGenome;
use crate::neuroevo::node::NodeType;
use crate::neuroevo::types::NodeId;

/// Neural network built from a NEAT genome
pub struct NeatNetwork {
    /// Node values (indexed by position in sorted order)
    node_values: Vec<f64>,
    /// Previous node values (for recurrence)
    prev_values: Vec<f64>,
    /// Node IDs in sorted order
    node_ids: Vec<NodeId>,
    /// Node type for each position
    node_types: Vec<NodeType>,
    /// Activation functions
    activations: Vec<ActivationFunction>,
    /// Node biases
    biases: Vec<f64>,
    /// Connections: (from_idx, to_idx, weight, enabled)
    connections: Vec<(usize, usize, f64, bool)>,
    /// Number of inputs
    num_inputs: usize,
    /// Number of outputs
    num_outputs: usize,
}

impl NeatNetwork {
    /// Build a neural network from a genome
    pub fn from_genome(genome: &NeatGenome) -> Self {
        // Create sorted list of nodes
        let mut nodes: Vec<_> = genome.nodes.iter().collect();
        nodes.sort_by_key(|n| (n.layer, n.id));

        let node_ids: Vec<NodeId> = nodes.iter().map(|n| n.id).collect();
        let node_types: Vec<NodeType> = nodes.iter().map(|n| n.node_type).collect();
        let activations: Vec<ActivationFunction> = nodes.iter().map(|n| n.activation).collect();
        let biases: Vec<f64> = nodes.iter().map(|n| n.bias).collect();

        // Create ID to index mapping
        let id_to_idx: BTreeMap<NodeId, usize> = node_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();

        // Convert connections
        let connections: Vec<(usize, usize, f64, bool)> = genome
            .connections
            .iter()
            .filter_map(|c| {
                let from_idx = id_to_idx.get(&c.from)?;
                let to_idx = id_to_idx.get(&c.to)?;
                Some((*from_idx, *to_idx, c.weight, c.enabled))
            })
            .collect();

        let num_nodes = nodes.len();

        Self {
            node_values: vec![0.0; num_nodes],
            prev_values: vec![0.0; num_nodes],
            node_ids,
            node_types,
            activations,
            biases,
            connections,
            num_inputs: genome.num_inputs,
            num_outputs: genome.num_outputs,
        }
    }

    /// Activate the network with given inputs
    pub fn activate(&mut self, inputs: &[f64]) -> Vec<f64> {
        // Store previous values for recurrence
        self.prev_values.copy_from_slice(&self.node_values);

        // Reset node values
        for v in &mut self.node_values {
            *v = 0.0;
        }

        // Set bias nodes to 1.0
        for (i, &ntype) in self.node_types.iter().enumerate() {
            if ntype == NodeType::Bias {
                self.node_values[i] = 1.0;
            }
        }

        // Set input values
        let mut input_idx = 0;
        for (i, &ntype) in self.node_types.iter().enumerate() {
            if ntype == NodeType::Input && input_idx < inputs.len() {
                self.node_values[i] = inputs[input_idx];
                input_idx += 1;
            }
        }

        // Propagate through network (multiple iterations for recurrence)
        for _ in 0..3 {
            let mut new_values = self.node_values.clone();

            for &(from, to, weight, enabled) in &self.connections {
                if enabled {
                    new_values[to] += self.node_values[from] * weight;
                }
            }

            // Apply activations
            for (i, &ntype) in self.node_types.iter().enumerate() {
                if ntype == NodeType::Hidden || ntype == NodeType::Output {
                    new_values[i] = self.activations[i].apply(new_values[i] + self.biases[i]);
                }
            }

            self.node_values = new_values;
        }

        // Extract outputs
        let mut outputs = Vec::with_capacity(self.num_outputs);
        for (i, &ntype) in self.node_types.iter().enumerate() {
            if ntype == NodeType::Output {
                outputs.push(self.node_values[i]);
            }
        }

        outputs
    }

    /// Reset network state
    pub fn reset(&mut self) {
        for v in &mut self.node_values {
            *v = 0.0;
        }
        for v in &mut self.prev_values {
            *v = 0.0;
        }
    }
}
