//! # Neuroevolution Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary neuroevolutionary algorithms that evolve
//! both neural network topologies AND weights simultaneously for kernel-level
//! intelligent systems.
//!
//! ## Key Features
//!
//! - **NEAT (NeuroEvolution of Augmenting Topologies)**: Evolves both structure and weights
//! - **HyperNEAT**: Indirect encoding using compositional pattern-producing networks
//! - **ES-HyperNEAT**: Evolves substrate topology based on network geometry
//! - **Novelty Search**: Exploration-based search without explicit objectives
//! - **MAP-Elites**: Quality-Diversity algorithm for illuminating solution spaces
//! - **CoDeepNEAT**: Coevolution of deep neural network modules
//!
//! ## Kernel Applications
//!
//! - Evolving optimal scheduling policies
//! - Generating memory management heuristics
//! - Creating adaptive interrupt handlers
//! - Discovering novel kernel optimization strategies

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// GENETIC ENCODING TYPES
// ============================================================================

/// Unique identifier for nodes in the network
pub type NodeId = u32;

/// Unique identifier for connections (genes)
pub type InnovationNumber = u64;

/// Activation functions supported by neurons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationFunction {
    /// Identity function: f(x) = x
    Identity,
    /// Sigmoid: f(x) = 1 / (1 + e^(-x))
    Sigmoid,
    /// Hyperbolic tangent: f(x) = tanh(x)
    Tanh,
    /// Rectified Linear Unit: f(x) = max(0, x)
    ReLU,
    /// Leaky ReLU: f(x) = max(0.01x, x)
    LeakyReLU,
    /// Exponential Linear Unit
    ELU,
    /// Gaussian: f(x) = e^(-x^2)
    Gaussian,
    /// Sine function for HyperNEAT CPPNs
    Sine,
    /// Absolute value for CPPNs
    Abs,
    /// Step function: f(x) = x > 0 ? 1 : 0
    Step,
}

impl ActivationFunction {
    /// Apply the activation function
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::Identity => x,
            Self::Sigmoid => 1.0 / (1.0 + libm::exp(-x)),
            Self::Tanh => libm::tanh(x),
            Self::ReLU => {
                if x > 0.0 {
                    x
                } else {
                    0.0
                }
            },
            Self::LeakyReLU => {
                if x > 0.0 {
                    x
                } else {
                    0.01 * x
                }
            },
            Self::ELU => {
                if x > 0.0 {
                    x
                } else {
                    libm::exp(x) - 1.0
                }
            },
            Self::Gaussian => libm::exp(-x * x),
            Self::Sine => libm::sin(x),
            Self::Abs => libm::fabs(x),
            Self::Step => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            },
        }
    }

    /// Get a random activation function
    pub fn random(seed: u64) -> Self {
        match seed % 10 {
            0 => Self::Identity,
            1 => Self::Sigmoid,
            2 => Self::Tanh,
            3 => Self::ReLU,
            4 => Self::LeakyReLU,
            5 => Self::ELU,
            6 => Self::Gaussian,
            7 => Self::Sine,
            8 => Self::Abs,
            _ => Self::Step,
        }
    }
}

/// Type of node in the neural network
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// Input sensor
    Input,
    /// Hidden processing node
    Hidden,
    /// Output effector
    Output,
    /// Bias node (always outputs 1.0)
    Bias,
}

/// A node gene in the NEAT genome
#[derive(Debug, Clone)]
pub struct NodeGene {
    /// Unique node identifier
    pub id: NodeId,
    /// Type of this node
    pub node_type: NodeType,
    /// Activation function
    pub activation: ActivationFunction,
    /// Layer for feedforward ordering (-1 for input, 0+ for hidden/output)
    pub layer: i32,
    /// Position for substrate (x, y) in HyperNEAT
    pub position: (f64, f64),
    /// Bias value
    pub bias: f64,
    /// Response multiplier
    pub response: f64,
}

impl NodeGene {
    /// Create a new node gene
    pub fn new(id: NodeId, node_type: NodeType, activation: ActivationFunction) -> Self {
        Self {
            id,
            node_type,
            activation,
            layer: match node_type {
                NodeType::Input | NodeType::Bias => -1,
                NodeType::Output => 1,
                NodeType::Hidden => 0,
            },
            position: (0.0, 0.0),
            bias: 0.0,
            response: 1.0,
        }
    }

    /// Create an input node
    pub fn input(id: NodeId) -> Self {
        Self::new(id, NodeType::Input, ActivationFunction::Identity)
    }

    /// Create an output node
    pub fn output(id: NodeId, activation: ActivationFunction) -> Self {
        Self::new(id, NodeType::Output, activation)
    }

    /// Create a hidden node
    pub fn hidden(id: NodeId, activation: ActivationFunction) -> Self {
        Self::new(id, NodeType::Hidden, activation)
    }
}

/// A connection gene in the NEAT genome
#[derive(Debug, Clone)]
pub struct ConnectionGene {
    /// Source node
    pub from: NodeId,
    /// Destination node
    pub to: NodeId,
    /// Connection weight
    pub weight: f64,
    /// Is this connection enabled?
    pub enabled: bool,
    /// Global innovation number for crossover alignment
    pub innovation: InnovationNumber,
    /// Recurrent flag
    pub recurrent: bool,
}

impl ConnectionGene {
    /// Create a new connection gene
    pub fn new(from: NodeId, to: NodeId, weight: f64, innovation: InnovationNumber) -> Self {
        Self {
            from,
            to,
            weight,
            enabled: true,
            innovation,
            recurrent: false,
        }
    }

    /// Disable this connection
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if connection is active
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

// ============================================================================
// NEAT GENOME
// ============================================================================

/// Global innovation tracker for maintaining historical markings
pub struct InnovationTracker {
    /// Current innovation number
    current: InnovationNumber,
    /// Map of (from, to) -> innovation for existing connections
    innovations: BTreeMap<(NodeId, NodeId), InnovationNumber>,
    /// Current node ID
    current_node: NodeId,
}

impl InnovationTracker {
    /// Create a new innovation tracker
    pub fn new() -> Self {
        Self {
            current: 0,
            innovations: BTreeMap::new(),
            current_node: 0,
        }
    }

    /// Get or create innovation number for a connection
    pub fn get_or_create(&mut self, from: NodeId, to: NodeId) -> InnovationNumber {
        let key = (from, to);
        if let Some(&innov) = self.innovations.get(&key) {
            innov
        } else {
            self.current += 1;
            self.innovations.insert(key, self.current);
            self.current
        }
    }

    /// Get a new node ID
    pub fn new_node_id(&mut self) -> NodeId {
        self.current_node += 1;
        self.current_node
    }

    /// Reset for a new generation (keeps node IDs, clears innovation cache)
    pub fn new_generation(&mut self) {
        self.innovations.clear();
    }
}

impl Default for InnovationTracker {
    fn default() -> Self {
        Self::new()
    }
}

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
        let max_innov = max_innov_self.max(max_innov_other);
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

// ============================================================================
// NEURAL NETWORK PHENOTYPE
// ============================================================================

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
        let mut nodes: Vec<&NodeGene> = genome.nodes.iter().collect();
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

// ============================================================================
// SPECIATION
// ============================================================================

/// A species is a group of similar genomes
#[derive(Debug)]
pub struct Species {
    /// Species ID
    pub id: usize,
    /// Representative genome
    pub representative: NeatGenome,
    /// Member genome indices
    pub members: Vec<usize>,
    /// Average fitness
    pub avg_fitness: f64,
    /// Best fitness ever
    pub best_fitness: f64,
    /// Generations since improvement
    pub stagnation: u32,
    /// Offspring allocation
    pub offspring: usize,
}

impl Species {
    /// Create a new species with a representative
    pub fn new(id: usize, representative: NeatGenome) -> Self {
        Self {
            id,
            representative,
            members: Vec::new(),
            avg_fitness: 0.0,
            best_fitness: 0.0,
            stagnation: 0,
            offspring: 0,
        }
    }

    /// Check if a genome belongs to this species
    pub fn is_compatible(&self, genome: &NeatGenome, threshold: f64) -> bool {
        genome.compatibility_distance(&self.representative, 1.0, 1.0, 0.4) < threshold
    }

    /// Clear members for new generation
    pub fn clear_members(&mut self) {
        self.members.clear();
    }

    /// Add a member
    pub fn add_member(&mut self, idx: usize) {
        self.members.push(idx);
    }

    /// Update species statistics
    pub fn update_stats(&mut self, genomes: &[NeatGenome]) {
        if self.members.is_empty() {
            return;
        }

        let total_fitness: f64 = self.members.iter().map(|&i| genomes[i].fitness).sum();

        self.avg_fitness = total_fitness / self.members.len() as f64;

        let max_fitness = self
            .members
            .iter()
            .map(|&i| genomes[i].fitness)
            .fold(f64::NEG_INFINITY, f64::max);

        if max_fitness > self.best_fitness {
            self.best_fitness = max_fitness;
            self.stagnation = 0;
        } else {
            self.stagnation += 1;
        }

        // Update representative (use best member)
        if let Some(&best_idx) = self
            .members
            .iter()
            .max_by(|&&a, &&b| genomes[a].fitness.partial_cmp(&genomes[b].fitness).unwrap())
        {
            self.representative = genomes[best_idx].clone();
        }
    }
}

// ============================================================================
// NEAT POPULATION
// ============================================================================

/// Configuration for NEAT
#[derive(Debug, Clone)]
pub struct NeatConfig {
    /// Population size
    pub population_size: usize,
    /// Number of inputs
    pub num_inputs: usize,
    /// Number of outputs
    pub num_outputs: usize,
    /// Output activation function
    pub output_activation: ActivationFunction,
    /// Compatibility threshold for speciation
    pub compatibility_threshold: f64,
    /// Probability of adding a connection
    pub add_connection_prob: f64,
    /// Probability of adding a node
    pub add_node_prob: f64,
    /// Probability of mutating weights
    pub weight_mutation_prob: f64,
    /// Weight perturbation power
    pub weight_perturbation: f64,
    /// Elitism - number of best genomes to preserve
    pub elitism: usize,
    /// Survival threshold within species
    pub survival_threshold: f64,
    /// Stagnation limit before species is killed
    pub stagnation_limit: u32,
    /// Target number of species
    pub target_species: usize,
}

impl Default for NeatConfig {
    fn default() -> Self {
        Self {
            population_size: 150,
            num_inputs: 4,
            num_outputs: 2,
            output_activation: ActivationFunction::Tanh,
            compatibility_threshold: 3.0,
            add_connection_prob: 0.05,
            add_node_prob: 0.03,
            weight_mutation_prob: 0.8,
            weight_perturbation: 0.5,
            elitism: 2,
            survival_threshold: 0.2,
            stagnation_limit: 15,
            target_species: 10,
        }
    }
}

/// NEAT Population manager
pub struct NeatPopulation {
    /// Configuration
    pub config: NeatConfig,
    /// All genomes
    pub genomes: Vec<NeatGenome>,
    /// Species list
    pub species: Vec<Species>,
    /// Innovation tracker
    pub tracker: InnovationTracker,
    /// Current generation
    pub generation: u32,
    /// Best genome ever
    pub best_genome: Option<NeatGenome>,
    /// Best fitness ever
    pub best_fitness: f64,
    /// Random seed
    seed: u64,
}

impl NeatPopulation {
    /// Create a new NEAT population
    pub fn new(config: NeatConfig, seed: u64) -> Self {
        let mut tracker = InnovationTracker::new();
        let mut genomes = Vec::with_capacity(config.population_size);

        // Create initial population
        for _ in 0..config.population_size {
            let genome = NeatGenome::minimal(
                config.num_inputs,
                config.num_outputs,
                config.output_activation,
                &mut tracker,
            );
            genomes.push(genome);
        }

        Self {
            config,
            genomes,
            species: Vec::new(),
            tracker,
            generation: 0,
            best_genome: None,
            best_fitness: f64::NEG_INFINITY,
            seed,
        }
    }

    /// Speciate the population
    pub fn speciate(&mut self) {
        // Clear member lists
        for species in &mut self.species {
            species.clear_members();
        }

        // Assign genomes to species
        for (i, genome) in self.genomes.iter_mut().enumerate() {
            let mut found_species = false;

            for species in &mut self.species {
                if species.is_compatible(genome, self.config.compatibility_threshold) {
                    species.add_member(i);
                    genome.species_id = species.id;
                    found_species = true;
                    break;
                }
            }

            if !found_species {
                // Create new species
                let new_species_id = self.species.len();
                let mut new_species = Species::new(new_species_id, genome.clone());
                new_species.add_member(i);
                genome.species_id = new_species_id;
                self.species.push(new_species);
            }
        }

        // Remove empty species
        self.species.retain(|s| !s.members.is_empty());

        // Adjust compatibility threshold to target species count
        if self.species.len() < self.config.target_species {
            self.config.compatibility_threshold -= 0.3;
        } else if self.species.len() > self.config.target_species {
            self.config.compatibility_threshold += 0.3;
        }
        self.config.compatibility_threshold = self.config.compatibility_threshold.clamp(1.0, 10.0);
    }

    /// Calculate adjusted fitness for each genome
    pub fn calculate_adjusted_fitness(&mut self) {
        for species in &self.species {
            let species_size = species.members.len() as f64;
            for &idx in &species.members {
                self.genomes[idx].adjusted_fitness = self.genomes[idx].fitness / species_size;
            }
        }

        // Update species stats
        let genomes = &self.genomes;
        for species in &mut self.species {
            species.update_stats(genomes);
        }
    }

    /// Create next generation through selection and reproduction
    pub fn evolve(&mut self) {
        self.generation += 1;
        self.tracker.new_generation();

        // Calculate offspring allocation for each species
        let total_adjusted: f64 = self.genomes.iter().map(|g| g.adjusted_fitness).sum();

        if total_adjusted <= 0.0 {
            return;
        }

        for species in &mut self.species {
            let species_adjusted: f64 = species
                .members
                .iter()
                .map(|&i| self.genomes[i].adjusted_fitness)
                .sum();
            species.offspring = ((species_adjusted / total_adjusted)
                * self.config.population_size as f64)
                .round() as usize;
        }

        // Remove stagnant species (keep at least 2)
        if self.species.len() > 2 {
            self.species
                .retain(|s| s.stagnation < self.config.stagnation_limit);
        }

        // Create offspring
        let mut new_genomes = Vec::with_capacity(self.config.population_size);

        // Elitism: keep best genome overall
        if let Some(ref best) = self.best_genome {
            new_genomes.push(best.clone());
        }

        for species in &self.species {
            if species.members.is_empty() {
                continue;
            }

            // Sort members by fitness
            let mut sorted_members = species.members.clone();
            sorted_members.sort_by(|&a, &b| {
                self.genomes[b]
                    .fitness
                    .partial_cmp(&self.genomes[a].fitness)
                    .unwrap()
            });

            // Keep only top survivors
            let survivors =
                (sorted_members.len() as f64 * self.config.survival_threshold).ceil() as usize;
            let survivors = survivors.max(1);
            sorted_members.truncate(survivors);

            // Elitism within species
            for i in 0..self.config.elitism.min(sorted_members.len()) {
                if new_genomes.len() < self.config.population_size {
                    new_genomes.push(self.genomes[sorted_members[i]].clone());
                }
            }

            // Create offspring for this species
            for _ in 0..species.offspring {
                if new_genomes.len() >= self.config.population_size {
                    break;
                }

                self.seed = lcg_next(self.seed);
                let parent1_idx = sorted_members[self.seed as usize % sorted_members.len()];

                self.seed = lcg_next(self.seed);
                let offspring = if self.seed % 100 < 75 && sorted_members.len() > 1 {
                    // Crossover
                    self.seed = lcg_next(self.seed);
                    let parent2_idx = sorted_members[self.seed as usize % sorted_members.len()];
                    NeatGenome::crossover(
                        &self.genomes[parent1_idx],
                        &self.genomes[parent2_idx],
                        self.seed,
                    )
                } else {
                    // Asexual reproduction
                    self.genomes[parent1_idx].clone()
                };

                new_genomes.push(offspring);
            }
        }

        // Fill remaining slots with random offspring
        while new_genomes.len() < self.config.population_size {
            self.seed = lcg_next(self.seed);
            let idx = self.seed as usize % self.genomes.len();
            new_genomes.push(self.genomes[idx].clone());
        }

        // Mutate offspring (skip elites)
        let elite_count = self.config.elitism * self.species.len().max(1);
        for i in elite_count..new_genomes.len() {
            self.seed = lcg_next(self.seed);
            self.mutate_genome(&mut new_genomes[i]);
        }

        self.genomes = new_genomes;
    }

    /// Apply mutations to a genome
    fn mutate_genome(&mut self, genome: &mut NeatGenome) {
        // Weight mutation
        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < self.config.weight_mutation_prob {
            genome.mutate_weights(0.8, self.config.weight_perturbation, self.seed);
        }

        // Add connection
        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < self.config.add_connection_prob {
            genome.mutate_add_connection(&mut self.tracker, self.seed);
        }

        // Add node
        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < self.config.add_node_prob {
            genome.mutate_add_node(&mut self.tracker, self.seed);
        }
    }

    /// Update best genome tracking
    pub fn update_best(&mut self) {
        for genome in &self.genomes {
            if genome.fitness > self.best_fitness {
                self.best_fitness = genome.fitness;
                self.best_genome = Some(genome.clone());
            }
        }
    }

    /// Get the best network
    pub fn get_best_network(&self) -> Option<NeatNetwork> {
        self.best_genome.as_ref().map(NeatNetwork::from_genome)
    }
}

// ============================================================================
// HYPERNEAT - INDIRECT ENCODING
// ============================================================================

/// A point in the substrate space
#[derive(Debug, Clone, Copy)]
pub struct SubstratePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64, // For 3D substrates
}

impl SubstratePoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, z: 0.0 }
    }

    pub fn new_3d(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &SubstratePoint) -> f64 {
        libm::sqrt(
            (self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2),
        )
    }
}

/// Substrate configuration for HyperNEAT
#[derive(Debug, Clone)]
pub struct SubstrateConfig {
    /// Input node positions
    pub inputs: Vec<SubstratePoint>,
    /// Hidden node positions
    pub hidden: Vec<SubstratePoint>,
    /// Output node positions
    pub outputs: Vec<SubstratePoint>,
    /// Minimum weight threshold
    pub weight_threshold: f64,
    /// Maximum weight magnitude
    pub max_weight: f64,
}

impl SubstrateConfig {
    /// Create a 2D grid substrate
    pub fn grid_2d(input_size: usize, hidden_size: usize, output_size: usize) -> Self {
        let mut inputs = Vec::with_capacity(input_size);
        let mut hidden = Vec::with_capacity(hidden_size);
        let mut outputs = Vec::with_capacity(output_size);

        // Input layer at y = -1
        for i in 0..input_size {
            let x = (i as f64 / (input_size.max(1) - 1).max(1) as f64) * 2.0 - 1.0;
            inputs.push(SubstratePoint::new(x, -1.0));
        }

        // Hidden layer at y = 0
        for i in 0..hidden_size {
            let x = (i as f64 / (hidden_size.max(1) - 1).max(1) as f64) * 2.0 - 1.0;
            hidden.push(SubstratePoint::new(x, 0.0));
        }

        // Output layer at y = 1
        for i in 0..output_size {
            let x = (i as f64 / (output_size.max(1) - 1).max(1) as f64) * 2.0 - 1.0;
            outputs.push(SubstratePoint::new(x, 1.0));
        }

        Self {
            inputs,
            hidden,
            outputs,
            weight_threshold: 0.3,
            max_weight: 5.0,
        }
    }
}

/// HyperNEAT - uses a CPPN to generate substrate network weights
pub struct HyperNeat {
    /// The CPPN population
    pub cppn_population: NeatPopulation,
    /// Substrate configuration
    pub substrate: SubstrateConfig,
    /// Generated networks cache
    networks: Vec<Option<GeneratedNetwork>>,
}

/// A network generated from a CPPN query
pub struct GeneratedNetwork {
    /// Input count
    num_inputs: usize,
    /// Hidden count
    num_hidden: usize,
    /// Output count
    num_outputs: usize,
    /// Weights: (layer, from, to, weight)
    weights: Vec<(u8, usize, usize, f64)>,
    /// Node values
    values: Vec<f64>,
}

impl GeneratedNetwork {
    /// Create a new generated network
    pub fn new(num_inputs: usize, num_hidden: usize, num_outputs: usize) -> Self {
        let total_nodes = num_inputs + num_hidden + num_outputs;
        Self {
            num_inputs,
            num_hidden,
            num_outputs,
            weights: Vec::new(),
            values: vec![0.0; total_nodes],
        }
    }

    /// Add a connection
    pub fn add_connection(&mut self, layer: u8, from: usize, to: usize, weight: f64) {
        self.weights.push((layer, from, to, weight));
    }

    /// Activate the network
    pub fn activate(&mut self, inputs: &[f64]) -> Vec<f64> {
        // Reset values
        for v in &mut self.values {
            *v = 0.0;
        }

        // Set inputs
        for (i, &input) in inputs.iter().enumerate().take(self.num_inputs) {
            self.values[i] = input;
        }

        // Process connections by layer
        for layer in 0..=1 {
            let mut new_values = self.values.clone();

            for &(l, from, to, weight) in &self.weights {
                if l == layer {
                    new_values[to] += self.values[from] * weight;
                }
            }

            // Apply activation to target layer
            let start = if layer == 0 {
                self.num_inputs
            } else {
                self.num_inputs + self.num_hidden
            };
            let end = if layer == 0 {
                self.num_inputs + self.num_hidden
            } else {
                self.values.len()
            };

            for i in start..end {
                new_values[i] = libm::tanh(new_values[i]);
            }

            self.values = new_values;
        }

        // Extract outputs
        let output_start = self.num_inputs + self.num_hidden;
        self.values[output_start..].to_vec()
    }
}

impl HyperNeat {
    /// Create a new HyperNEAT system
    pub fn new(substrate: SubstrateConfig, seed: u64) -> Self {
        // CPPN has 4 inputs (x1, y1, x2, y2) and 1 or 2 outputs (weight, optional bias)
        let cppn_config = NeatConfig {
            population_size: 100,
            num_inputs: 4,  // Source x, y and target x, y
            num_outputs: 2, // Weight and expression (LEO)
            output_activation: ActivationFunction::Tanh,
            ..Default::default()
        };

        let cppn_population = NeatPopulation::new(cppn_config, seed);
        let networks = vec![None; cppn_population.genomes.len()];

        Self {
            cppn_population,
            substrate,
            networks,
        }
    }

    /// Generate a network from a CPPN
    pub fn generate_network(&self, cppn_idx: usize) -> GeneratedNetwork {
        let genome = &self.cppn_population.genomes[cppn_idx];
        let mut cppn = NeatNetwork::from_genome(genome);

        let mut network = GeneratedNetwork::new(
            self.substrate.inputs.len(),
            self.substrate.hidden.len(),
            self.substrate.outputs.len(),
        );

        // Query CPPN for input -> hidden connections
        for (i, input_pos) in self.substrate.inputs.iter().enumerate() {
            for (h, hidden_pos) in self.substrate.hidden.iter().enumerate() {
                let output = cppn.activate(&[input_pos.x, input_pos.y, hidden_pos.x, hidden_pos.y]);

                let weight = output[0] * self.substrate.max_weight;
                let expression = output.get(1).copied().unwrap_or(1.0);

                if libm::fabs(weight) > self.substrate.weight_threshold && expression > 0.0 {
                    network.add_connection(0, i, self.substrate.inputs.len() + h, weight);
                }
            }
        }

        // Query CPPN for hidden -> output connections
        for (h, hidden_pos) in self.substrate.hidden.iter().enumerate() {
            for (o, output_pos) in self.substrate.outputs.iter().enumerate() {
                let output =
                    cppn.activate(&[hidden_pos.x, hidden_pos.y, output_pos.x, output_pos.y]);

                let weight = output[0] * self.substrate.max_weight;
                let expression = output.get(1).copied().unwrap_or(1.0);

                if libm::fabs(weight) > self.substrate.weight_threshold && expression > 0.0 {
                    let from = self.substrate.inputs.len() + h;
                    let to = self.substrate.inputs.len() + self.substrate.hidden.len() + o;
                    network.add_connection(1, from, to, weight);
                }
            }
        }

        network
    }

    /// Evolve the population
    pub fn evolve(&mut self) {
        self.cppn_population.speciate();
        self.cppn_population.calculate_adjusted_fitness();
        self.cppn_population.evolve();
        self.cppn_population.update_best();

        // Invalidate network cache
        self.networks = vec![None; self.cppn_population.genomes.len()];
    }
}

// ============================================================================
// NOVELTY SEARCH
// ============================================================================

/// Behavior characterization for novelty search
#[derive(Debug, Clone)]
pub struct BehaviorVector {
    /// Behavior features
    pub features: Vec<f64>,
}

impl BehaviorVector {
    /// Create a new behavior vector
    pub fn new(features: Vec<f64>) -> Self {
        Self { features }
    }

    /// Compute distance to another behavior
    pub fn distance(&self, other: &BehaviorVector) -> f64 {
        if self.features.len() != other.features.len() {
            return f64::INFINITY;
        }

        let sum: f64 = self
            .features
            .iter()
            .zip(other.features.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum();

        libm::sqrt(sum)
    }
}

/// Novelty Search - exploration without explicit objectives
pub struct NoveltySearch {
    /// Archive of novel behaviors
    pub archive: Vec<BehaviorVector>,
    /// Archive threshold for adding new behaviors
    pub archive_threshold: f64,
    /// K-nearest neighbors for novelty calculation
    pub k_neighbors: usize,
    /// Maximum archive size
    pub max_archive_size: usize,
    /// Current population behaviors
    population_behaviors: Vec<BehaviorVector>,
}

impl NoveltySearch {
    /// Create a new novelty search instance
    pub fn new(archive_threshold: f64, k_neighbors: usize, max_archive_size: usize) -> Self {
        Self {
            archive: Vec::new(),
            archive_threshold,
            k_neighbors,
            max_archive_size,
            population_behaviors: Vec::new(),
        }
    }

    /// Calculate novelty score for a behavior
    pub fn novelty_score(&self, behavior: &BehaviorVector) -> f64 {
        // Combine archive and population for neighbor search
        let mut all_behaviors: Vec<&BehaviorVector> = self.archive.iter().collect();
        all_behaviors.extend(self.population_behaviors.iter());

        if all_behaviors.is_empty() {
            return f64::INFINITY; // First behavior is maximally novel
        }

        // Calculate distances to all behaviors
        let mut distances: Vec<f64> = all_behaviors.iter().map(|b| behavior.distance(b)).collect();

        // Sort to find k-nearest
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Average distance to k-nearest
        let k = self.k_neighbors.min(distances.len());
        distances[..k].iter().sum::<f64>() / k as f64
    }

    /// Add a behavior to the archive if novel enough
    pub fn add_to_archive(&mut self, behavior: BehaviorVector) {
        let novelty = self.novelty_score(&behavior);

        if novelty > self.archive_threshold {
            self.archive.push(behavior);

            // Prune archive if too large
            if self.archive.len() > self.max_archive_size {
                // Remove least novel
                self.prune_archive();
            }
        }
    }

    /// Set population behaviors for current generation
    pub fn set_population_behaviors(&mut self, behaviors: Vec<BehaviorVector>) {
        self.population_behaviors = behaviors;
    }

    /// Prune archive to max size by removing least novel
    fn prune_archive(&mut self) {
        if self.archive.len() <= self.max_archive_size {
            return;
        }

        // Calculate novelty for each archive member
        let mut scored: Vec<(usize, f64)> = Vec::new();
        for (i, behavior) in self.archive.iter().enumerate() {
            // Calculate novelty within archive only
            let mut distances: Vec<f64> = self
                .archive
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, b)| behavior.distance(b))
                .collect();
            distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let k = self.k_neighbors.min(distances.len());
            let avg_dist = if k > 0 {
                distances[..k].iter().sum::<f64>() / k as f64
            } else {
                0.0
            };
            scored.push((i, avg_dist));
        }

        // Sort by novelty (ascending, so least novel first)
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Keep only the most novel
        let to_remove = self.archive.len() - self.max_archive_size;
        let remove_indices: alloc::collections::BTreeSet<usize> =
            scored[..to_remove].iter().map(|(i, _)| *i).collect();

        self.archive = self
            .archive
            .iter()
            .enumerate()
            .filter(|(i, _)| !remove_indices.contains(i))
            .map(|(_, b)| b.clone())
            .collect();
    }
}

// ============================================================================
// MAP-ELITES: QUALITY-DIVERSITY ALGORITHM
// ============================================================================

/// A cell in the MAP-Elites archive
#[derive(Debug, Clone)]
pub struct EliteCell {
    /// The genome in this cell
    pub genome: NeatGenome,
    /// Behavior descriptor (determines cell position)
    pub behavior: Vec<f64>,
    /// Fitness (quality)
    pub fitness: f64,
}

/// MAP-Elites configuration
#[derive(Debug, Clone)]
pub struct MapElitesConfig {
    /// Number of dimensions in behavior space
    pub behavior_dims: usize,
    /// Number of bins per dimension
    pub bins_per_dim: usize,
    /// Behavior bounds (min, max) for each dimension
    pub behavior_bounds: Vec<(f64, f64)>,
    /// Batch size for parallel evaluation
    pub batch_size: usize,
}

/// MAP-Elites: Illuminating Search Spaces
pub struct MapElites {
    /// Configuration
    pub config: MapElitesConfig,
    /// Archive grid (flattened)
    pub archive: BTreeMap<usize, EliteCell>,
    /// Innovation tracker
    pub tracker: InnovationTracker,
    /// Best fitness found
    pub best_fitness: f64,
    /// Best genome
    pub best_genome: Option<NeatGenome>,
    /// Number of cells filled
    pub coverage: usize,
    /// Random seed
    seed: u64,
}

impl MapElites {
    /// Create a new MAP-Elites instance
    pub fn new(config: MapElitesConfig, seed: u64) -> Self {
        Self {
            config,
            archive: BTreeMap::new(),
            tracker: InnovationTracker::new(),
            best_fitness: f64::NEG_INFINITY,
            best_genome: None,
            coverage: 0,
            seed,
        }
    }

    /// Convert behavior descriptor to cell index
    pub fn behavior_to_cell(&self, behavior: &[f64]) -> usize {
        let mut index = 0;
        let mut multiplier = 1;

        for (i, &val) in behavior.iter().enumerate().take(self.config.behavior_dims) {
            let (min, max) = self
                .config
                .behavior_bounds
                .get(i)
                .copied()
                .unwrap_or((0.0, 1.0));
            let normalized = (val - min) / (max - min);
            let bin = ((normalized * self.config.bins_per_dim as f64) as usize)
                .min(self.config.bins_per_dim - 1);
            index += bin * multiplier;
            multiplier *= self.config.bins_per_dim;
        }

        index
    }

    /// Try to add a genome to the archive
    pub fn try_add(&mut self, genome: NeatGenome, behavior: Vec<f64>, fitness: f64) -> bool {
        let cell_idx = self.behavior_to_cell(&behavior);

        if let Some(existing) = self.archive.get(&cell_idx) {
            if fitness <= existing.fitness {
                return false;
            }
        } else {
            self.coverage += 1;
        }

        let cell = EliteCell {
            genome: genome.clone(),
            behavior,
            fitness,
        };

        self.archive.insert(cell_idx, cell);

        if fitness > self.best_fitness {
            self.best_fitness = fitness;
            self.best_genome = Some(genome);
        }

        true
    }

    /// Generate a new genome by mutation
    pub fn generate_offspring(&mut self, num_inputs: usize, num_outputs: usize) -> NeatGenome {
        self.seed = lcg_next(self.seed);

        if self.archive.is_empty() {
            // Create random initial genome
            return NeatGenome::minimal(
                num_inputs,
                num_outputs,
                ActivationFunction::Tanh,
                &mut self.tracker,
            );
        }

        // Select random parent from archive
        let parent_idx = self.seed as usize % self.archive.len();
        let parent = self.archive.values().nth(parent_idx).unwrap();

        let mut offspring = parent.genome.clone();

        // Apply mutations
        self.seed = lcg_next(self.seed);
        offspring.mutate_weights(0.8, 0.5, self.seed);

        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < 0.05 {
            offspring.mutate_add_connection(&mut self.tracker, self.seed);
        }

        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < 0.03 {
            offspring.mutate_add_node(&mut self.tracker, self.seed);
        }

        offspring
    }

    /// Get coverage statistics
    pub fn get_coverage_stats(&self) -> (usize, usize, f64) {
        let total_cells = self
            .config
            .bins_per_dim
            .pow(self.config.behavior_dims as u32);
        let coverage_pct = self.coverage as f64 / total_cells as f64 * 100.0;
        (self.coverage, total_cells, coverage_pct)
    }
}

// ============================================================================
// CODEEPNEAT: COEVOLUTION OF DEEP MODULES
// ============================================================================

/// A module blueprint in CoDeepNEAT
#[derive(Debug, Clone)]
pub struct ModuleBlueprint {
    /// Module ID
    pub id: u64,
    /// Number of layers in this module
    pub num_layers: usize,
    /// Layer configurations: (neurons, activation)
    pub layers: Vec<(usize, ActivationFunction)>,
    /// Skip connections within module
    pub skip_connections: Vec<(usize, usize)>,
    /// Fitness contribution
    pub fitness: f64,
    /// Age (generations)
    pub age: u32,
}

impl ModuleBlueprint {
    /// Create a new module blueprint
    pub fn new(id: u64, seed: u64) -> Self {
        let num_layers = ((seed % 5) + 1) as usize; // 1-5 layers
        let mut layers = Vec::with_capacity(num_layers);
        let mut rng = seed;

        for _ in 0..num_layers {
            rng = lcg_next(rng);
            let neurons = ((rng % 64) + 4) as usize; // 4-67 neurons
            rng = lcg_next(rng);
            let activation = ActivationFunction::random(rng);
            layers.push((neurons, activation));
        }

        Self {
            id,
            num_layers,
            layers,
            skip_connections: Vec::new(),
            fitness: 0.0,
            age: 0,
        }
    }

    /// Mutate this blueprint
    pub fn mutate(&mut self, seed: u64) {
        let mut rng = seed;

        // Mutate layer sizes
        for (neurons, _) in &mut self.layers {
            rng = lcg_next(rng);
            if rng % 100 < 30 {
                rng = lcg_next(rng);
                let delta = (rng % 8) as i64 - 4; // -4 to +3
                *neurons = ((*neurons as i64 + delta).max(1)) as usize;
            }
        }

        // Mutate activations
        for (_, activation) in &mut self.layers {
            rng = lcg_next(rng);
            if rng % 100 < 10 {
                *activation = ActivationFunction::random(rng);
            }
        }

        // Add/remove layers
        rng = lcg_next(rng);
        if rng % 100 < 5 && self.layers.len() < 10 {
            rng = lcg_next(rng);
            let neurons = ((rng % 64) + 4) as usize;
            rng = lcg_next(rng);
            let activation = ActivationFunction::random(rng);
            self.layers.push((neurons, activation));
        }

        if rng % 100 < 5 && self.layers.len() > 1 {
            self.layers.pop();
        }
    }
}

/// A complete network blueprint
#[derive(Debug, Clone)]
pub struct NetworkBlueprint {
    /// Blueprint ID
    pub id: u64,
    /// Module IDs in order
    pub modules: Vec<u64>,
    /// Connections between modules
    pub connections: Vec<(usize, usize)>,
    /// Fitness
    pub fitness: f64,
    /// Age
    pub age: u32,
}

impl NetworkBlueprint {
    /// Create a new network blueprint
    pub fn new(id: u64, module_ids: Vec<u64>) -> Self {
        // Create sequential connections
        let mut connections = Vec::new();
        for i in 0..module_ids.len().saturating_sub(1) {
            connections.push((i, i + 1));
        }

        Self {
            id,
            modules: module_ids,
            connections,
            fitness: 0.0,
            age: 0,
        }
    }

    /// Mutate this blueprint
    pub fn mutate(&mut self, available_modules: &[u64], seed: u64) {
        let mut rng = seed;

        // Swap modules
        for module in &mut self.modules {
            rng = lcg_next(rng);
            if rng % 100 < 20 && !available_modules.is_empty() {
                rng = lcg_next(rng);
                *module = available_modules[rng as usize % available_modules.len()];
            }
        }

        // Add module
        rng = lcg_next(rng);
        if rng % 100 < 10 && !available_modules.is_empty() && self.modules.len() < 10 {
            rng = lcg_next(rng);
            let new_module = available_modules[rng as usize % available_modules.len()];
            let pos = rng as usize % (self.modules.len() + 1);
            self.modules.insert(pos, new_module);

            // Update connections
            self.connections.clear();
            for i in 0..self.modules.len().saturating_sub(1) {
                self.connections.push((i, i + 1));
            }
        }

        // Remove module
        if rng % 100 < 10 && self.modules.len() > 1 {
            rng = lcg_next(rng);
            let pos = rng as usize % self.modules.len();
            self.modules.remove(pos);

            self.connections.clear();
            for i in 0..self.modules.len().saturating_sub(1) {
                self.connections.push((i, i + 1));
            }
        }
    }
}

/// CoDeepNEAT: Coevolution of Deep Network Modules
pub struct CoDeepNeat {
    /// Module population
    pub modules: Vec<ModuleBlueprint>,
    /// Network population
    pub networks: Vec<NetworkBlueprint>,
    /// Module population size
    pub module_pop_size: usize,
    /// Network population size
    pub network_pop_size: usize,
    /// Generation counter
    pub generation: u32,
    /// Next module ID
    next_module_id: u64,
    /// Next network ID
    next_network_id: u64,
    /// Random seed
    seed: u64,
}

impl CoDeepNeat {
    /// Create a new CoDeepNEAT instance
    pub fn new(module_pop_size: usize, network_pop_size: usize, seed: u64) -> Self {
        let mut rng = seed;
        let mut modules = Vec::with_capacity(module_pop_size);

        for i in 0..module_pop_size {
            rng = lcg_next(rng);
            modules.push(ModuleBlueprint::new(i as u64, rng));
        }

        let module_ids: Vec<u64> = modules.iter().map(|m| m.id).collect();
        let mut networks = Vec::with_capacity(network_pop_size);

        for i in 0..network_pop_size {
            rng = lcg_next(rng);
            let num_modules = ((rng % 3) + 1) as usize;
            let mut net_modules = Vec::with_capacity(num_modules);
            for _ in 0..num_modules {
                rng = lcg_next(rng);
                net_modules.push(module_ids[rng as usize % module_ids.len()]);
            }
            networks.push(NetworkBlueprint::new(i as u64, net_modules));
        }

        Self {
            modules,
            networks,
            module_pop_size,
            network_pop_size,
            generation: 0,
            next_module_id: module_pop_size as u64,
            next_network_id: network_pop_size as u64,
            seed: rng,
        }
    }

    /// Evolve both populations
    pub fn evolve(&mut self) {
        self.generation += 1;

        // Age all individuals
        for module in &mut self.modules {
            module.age += 1;
        }
        for network in &mut self.networks {
            network.age += 1;
        }

        // Sort by fitness
        self.modules
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        self.networks
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());

        // Evolve modules
        let module_survivors = self.module_pop_size / 2;
        self.modules.truncate(module_survivors);

        while self.modules.len() < self.module_pop_size {
            self.seed = lcg_next(self.seed);
            let parent_idx = self.seed as usize % module_survivors;
            let mut offspring = self.modules[parent_idx].clone();
            offspring.id = self.next_module_id;
            self.next_module_id += 1;
            offspring.age = 0;
            self.seed = lcg_next(self.seed);
            offspring.mutate(self.seed);
            self.modules.push(offspring);
        }

        // Evolve networks
        let network_survivors = self.network_pop_size / 2;
        self.networks.truncate(network_survivors);

        let module_ids: Vec<u64> = self.modules.iter().map(|m| m.id).collect();
        while self.networks.len() < self.network_pop_size {
            self.seed = lcg_next(self.seed);
            let parent_idx = self.seed as usize % network_survivors;
            let mut offspring = self.networks[parent_idx].clone();
            offspring.id = self.next_network_id;
            self.next_network_id += 1;
            offspring.age = 0;
            self.seed = lcg_next(self.seed);
            offspring.mutate(&module_ids, self.seed);
            self.networks.push(offspring);
        }
    }

    /// Get the best network
    pub fn get_best(&self) -> Option<&NetworkBlueprint> {
        self.networks
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
    }
}

// ============================================================================
// KERNEL NEUROEVOLUTION MANAGER
// ============================================================================

/// Types of kernel components that can be evolved
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEvolutionTarget {
    /// Scheduler policy
    Scheduler,
    /// Memory allocator
    MemoryAllocator,
    /// I/O scheduler
    IoScheduler,
    /// Interrupt handler
    InterruptHandler,
    /// Power management
    PowerManager,
    /// Cache policy
    CachePolicy,
}

/// Evolved kernel component descriptor
#[derive(Debug, Clone)]
pub struct EvolvedComponent {
    /// Target component type
    pub target: KernelEvolutionTarget,
    /// Generation evolved in
    pub generation: u32,
    /// Fitness achieved
    pub fitness: f64,
    /// Network genome
    pub genome: NeatGenome,
    /// Behavior descriptor
    pub behavior: Vec<f64>,
    /// Timestamp
    pub timestamp: u64,
}

/// Main neuroevolution manager for kernel components
pub struct KernelNeuroevoManager {
    /// NEAT populations for different targets
    pub populations: BTreeMap<KernelEvolutionTarget, NeatPopulation>,
    /// Evolved components registry
    pub evolved_components: Vec<EvolvedComponent>,
    /// Novelty search instance
    pub novelty_search: NoveltySearch,
    /// MAP-Elites for quality-diversity
    pub map_elites: Option<MapElites>,
    /// HyperNEAT for large-scale networks
    pub hyperneat: Option<HyperNeat>,
    /// CoDeepNEAT for modular networks
    pub codeepneat: Option<CoDeepNeat>,
    /// Total generations evolved
    pub total_generations: u64,
    /// Best overall fitness
    pub best_fitness: f64,
    /// Statistics
    pub stats: NeuroevoStats,
}

/// Neuroevolution statistics
#[derive(Debug, Clone, Default)]
pub struct NeuroevoStats {
    /// Total genomes evaluated
    pub genomes_evaluated: u64,
    /// Total mutations applied
    pub mutations_applied: u64,
    /// Successful improvements
    pub improvements: u64,
    /// Species count over time
    pub species_history: Vec<usize>,
    /// Best fitness over time
    pub fitness_history: Vec<f64>,
    /// Complexity over time
    pub complexity_history: Vec<usize>,
}

impl KernelNeuroevoManager {
    /// Create a new kernel neuroevolution manager
    pub fn new(seed: u64) -> Self {
        Self {
            populations: BTreeMap::new(),
            evolved_components: Vec::new(),
            novelty_search: NoveltySearch::new(0.5, 15, 500),
            map_elites: None,
            hyperneat: None,
            codeepneat: None,
            total_generations: 0,
            best_fitness: f64::NEG_INFINITY,
            stats: NeuroevoStats::default(),
        }
    }

    /// Initialize a population for a kernel target
    pub fn initialize_target(
        &mut self,
        target: KernelEvolutionTarget,
        num_inputs: usize,
        num_outputs: usize,
        seed: u64,
    ) {
        let config = NeatConfig {
            population_size: 100,
            num_inputs,
            num_outputs,
            output_activation: ActivationFunction::Tanh,
            ..Default::default()
        };

        let population = NeatPopulation::new(config, seed);
        self.populations.insert(target, population);
    }

    /// Evolve a target population
    pub fn evolve_target(&mut self, target: KernelEvolutionTarget) -> Option<()> {
        let population = self.populations.get_mut(&target)?;

        // Record stats before evolution
        self.stats.species_history.push(population.species.len());
        self.stats.fitness_history.push(population.best_fitness);
        if let Some(ref best) = population.best_genome {
            self.stats.complexity_history.push(best.complexity());
        }

        // Speciate and evolve
        population.speciate();
        population.calculate_adjusted_fitness();
        population.evolve();
        population.update_best();

        self.total_generations += 1;
        self.stats.genomes_evaluated += population.genomes.len() as u64;

        if population.best_fitness > self.best_fitness {
            self.best_fitness = population.best_fitness;
            self.stats.improvements += 1;
        }

        Some(())
    }

    /// Get the best network for a target
    pub fn get_best_network(&self, target: KernelEvolutionTarget) -> Option<NeatNetwork> {
        let population = self.populations.get(&target)?;
        population.get_best_network()
    }

    /// Register an evolved component
    pub fn register_component(&mut self, component: EvolvedComponent) {
        // Add to novelty search
        self.novelty_search
            .add_to_archive(BehaviorVector::new(component.behavior.clone()));

        self.evolved_components.push(component);
    }

    /// Get summary of evolution progress
    pub fn get_summary(&self) -> NeuroevoSummary {
        NeuroevoSummary {
            total_generations: self.total_generations,
            best_fitness: self.best_fitness,
            evolved_components: self.evolved_components.len(),
            archive_size: self.novelty_search.archive.len(),
            populations: self.populations.len(),
            genomes_evaluated: self.stats.genomes_evaluated,
            improvements: self.stats.improvements,
        }
    }
}

/// Summary of neuroevolution state
#[derive(Debug, Clone)]
pub struct NeuroevoSummary {
    pub total_generations: u64,
    pub best_fitness: f64,
    pub evolved_components: usize,
    pub archive_size: usize,
    pub populations: usize,
    pub genomes_evaluated: u64,
    pub improvements: u64,
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Linear congruential generator
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Generate a random weight in [-2, 2]
fn random_weight(seed: u64) -> f64 {
    (seed as f64 / u64::MAX as f64) * 4.0 - 2.0
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_functions() {
        assert!((ActivationFunction::Sigmoid.apply(0.0) - 0.5).abs() < 1e-10);
        assert!((ActivationFunction::Tanh.apply(0.0)).abs() < 1e-10);
        assert_eq!(ActivationFunction::ReLU.apply(-1.0), 0.0);
        assert_eq!(ActivationFunction::ReLU.apply(1.0), 1.0);
        assert_eq!(ActivationFunction::Step.apply(0.5), 1.0);
        assert_eq!(ActivationFunction::Step.apply(-0.5), 0.0);
    }

    #[test]
    fn test_node_gene_creation() {
        let input = NodeGene::input(1);
        assert_eq!(input.node_type, NodeType::Input);
        assert_eq!(input.layer, -1);

        let hidden = NodeGene::hidden(2, ActivationFunction::Tanh);
        assert_eq!(hidden.node_type, NodeType::Hidden);
        assert_eq!(hidden.activation, ActivationFunction::Tanh);

        let output = NodeGene::output(3, ActivationFunction::Sigmoid);
        assert_eq!(output.node_type, NodeType::Output);
    }

    #[test]
    fn test_innovation_tracker() {
        let mut tracker = InnovationTracker::new();

        let innov1 = tracker.get_or_create(1, 2);
        let innov2 = tracker.get_or_create(1, 3);
        let innov1_again = tracker.get_or_create(1, 2);

        assert_eq!(innov1, 1);
        assert_eq!(innov2, 2);
        assert_eq!(innov1_again, innov1); // Should return same innovation

        let node1 = tracker.new_node_id();
        let node2 = tracker.new_node_id();
        assert_ne!(node1, node2);
    }

    #[test]
    fn test_minimal_genome() {
        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        assert_eq!(genome.num_inputs, 2);
        assert_eq!(genome.num_outputs, 1);
        // Should have: 1 bias + 2 inputs + 1 output = 4 nodes
        assert_eq!(genome.nodes.len(), 4);
        // Should have connections from each input + bias to output = 3 connections
        assert_eq!(genome.connections.len(), 3);
    }

    #[test]
    fn test_network_activation() {
        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let mut network = NeatNetwork::from_genome(&genome);
        let output = network.activate(&[0.5, 0.5]);

        assert_eq!(output.len(), 1);
        // Output should be in [-1, 1] due to tanh
        assert!(output[0] >= -1.0 && output[0] <= 1.0);
    }

    #[test]
    fn test_weight_mutation() {
        let mut tracker = InnovationTracker::new();
        let mut genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let original_weights: Vec<f64> = genome.connections.iter().map(|c| c.weight).collect();
        genome.mutate_weights(1.0, 0.5, 12345); // 100% mutation rate

        let mutated_weights: Vec<f64> = genome.connections.iter().map(|c| c.weight).collect();

        // At least some weights should have changed
        let changed = original_weights
            .iter()
            .zip(mutated_weights.iter())
            .any(|(a, b)| (a - b).abs() > 1e-10);
        assert!(changed);
    }

    #[test]
    fn test_add_node_mutation() {
        let mut tracker = InnovationTracker::new();
        let mut genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let original_nodes = genome.nodes.len();
        let original_connections = genome.connections.len();

        genome.mutate_add_node(&mut tracker, 12345);

        // Should have added 1 hidden node
        assert_eq!(genome.nodes.len(), original_nodes + 1);
        // Should have added 2 new connections (disabled one old)
        assert_eq!(genome.connections.len(), original_connections + 2);

        // One connection should be disabled
        let disabled = genome.connections.iter().filter(|c| !c.enabled).count();
        assert_eq!(disabled, 1);
    }

    #[test]
    fn test_compatibility_distance() {
        let mut tracker = InnovationTracker::new();
        let genome1 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);
        let genome2 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let distance = genome1.compatibility_distance(&genome2, 1.0, 1.0, 0.4);

        // Similar genomes should have low distance
        assert!(distance < 5.0);
    }

    #[test]
    fn test_crossover() {
        let mut tracker = InnovationTracker::new();
        let mut parent1 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);
        let mut parent2 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        parent1.fitness = 0.8;
        parent2.fitness = 0.6;

        let child = NeatGenome::crossover(&parent1, &parent2, 12345);

        assert_eq!(child.num_inputs, 2);
        assert_eq!(child.num_outputs, 1);
        assert_eq!(child.generation, 1);
    }

    #[test]
    fn test_species() {
        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);
        let species = Species::new(0, genome.clone());

        assert!(species.is_compatible(&genome, 5.0));
    }

    #[test]
    fn test_neat_population() {
        let config = NeatConfig {
            population_size: 10,
            num_inputs: 2,
            num_outputs: 1,
            ..Default::default()
        };

        let mut population = NeatPopulation::new(config, 12345);

        assert_eq!(population.genomes.len(), 10);

        // Set random fitness and evolve
        for (i, genome) in population.genomes.iter_mut().enumerate() {
            genome.fitness = i as f64 / 10.0;
        }

        population.speciate();
        population.calculate_adjusted_fitness();
        population.evolve();

        assert_eq!(population.genomes.len(), 10);
        assert_eq!(population.generation, 1);
    }

    #[test]
    fn test_behavior_vector() {
        let b1 = BehaviorVector::new(vec![0.0, 0.0, 0.0]);
        let b2 = BehaviorVector::new(vec![1.0, 0.0, 0.0]);
        let b3 = BehaviorVector::new(vec![1.0, 1.0, 1.0]);

        assert!((b1.distance(&b2) - 1.0).abs() < 1e-10);
        assert!((b1.distance(&b3) - libm::sqrt(3.0)).abs() < 1e-10);
        assert!((b1.distance(&b1)).abs() < 1e-10);
    }

    #[test]
    fn test_novelty_search() {
        let mut novelty = NoveltySearch::new(0.5, 5, 100);

        let b1 = BehaviorVector::new(vec![0.0, 0.0]);
        let score1 = novelty.novelty_score(&b1);
        assert_eq!(score1, f64::INFINITY); // First behavior is maximally novel

        novelty.add_to_archive(b1);

        let b2 = BehaviorVector::new(vec![0.0, 0.0]);
        let score2 = novelty.novelty_score(&b2);
        assert!(score2 < f64::INFINITY); // Now there's something to compare to
    }

    #[test]
    fn test_map_elites() {
        let config = MapElitesConfig {
            behavior_dims: 2,
            bins_per_dim: 10,
            behavior_bounds: vec![(0.0, 1.0), (0.0, 1.0)],
            batch_size: 10,
        };

        let mut map_elites = MapElites::new(config, 12345);

        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let added = map_elites.try_add(genome.clone(), vec![0.5, 0.5], 1.0);
        assert!(added);
        assert_eq!(map_elites.coverage, 1);

        // Same cell, lower fitness - should not add
        let not_added = map_elites.try_add(genome.clone(), vec![0.5, 0.5], 0.5);
        assert!(!not_added);

        // Same cell, higher fitness - should add
        let replaced = map_elites.try_add(genome, vec![0.5, 0.5], 2.0);
        assert!(replaced);
        assert_eq!(map_elites.coverage, 1); // Still 1 cell
    }

    #[test]
    fn test_substrate_point() {
        let p1 = SubstratePoint::new(0.0, 0.0);
        let p2 = SubstratePoint::new(3.0, 4.0);

        assert!((p1.distance(&p2) - 5.0).abs() < 1e-10); // 3-4-5 triangle
    }

    #[test]
    fn test_substrate_config() {
        let substrate = SubstrateConfig::grid_2d(4, 8, 2);

        assert_eq!(substrate.inputs.len(), 4);
        assert_eq!(substrate.hidden.len(), 8);
        assert_eq!(substrate.outputs.len(), 2);
    }

    #[test]
    fn test_module_blueprint() {
        let mut module = ModuleBlueprint::new(1, 12345);

        assert!(module.num_layers >= 1 && module.num_layers <= 5);
        assert!(!module.layers.is_empty());

        module.mutate(67890);
        // Mutation should not break the module
        assert!(!module.layers.is_empty());
    }

    #[test]
    fn test_network_blueprint() {
        let mut network = NetworkBlueprint::new(1, vec![0, 1, 2]);

        assert_eq!(network.modules.len(), 3);
        assert_eq!(network.connections.len(), 2); // Sequential connections

        network.mutate(&[0, 1, 2, 3, 4], 12345);
        // Should still have valid structure
        assert!(!network.modules.is_empty());
    }

    #[test]
    fn test_codeepneat() {
        let mut codeepneat = CoDeepNeat::new(5, 5, 12345);

        assert_eq!(codeepneat.modules.len(), 5);
        assert_eq!(codeepneat.networks.len(), 5);

        // Set fitness
        for module in &mut codeepneat.modules {
            module.fitness = 1.0;
        }
        for network in &mut codeepneat.networks {
            network.fitness = 1.0;
        }

        codeepneat.evolve();

        assert_eq!(codeepneat.modules.len(), 5);
        assert_eq!(codeepneat.networks.len(), 5);
        assert_eq!(codeepneat.generation, 1);
    }

    #[test]
    fn test_hyperneat() {
        let substrate = SubstrateConfig::grid_2d(4, 8, 2);
        let hyperneat = HyperNeat::new(substrate, 12345);

        assert_eq!(hyperneat.cppn_population.genomes.len(), 100);

        // Generate a network
        let network = hyperneat.generate_network(0);
        assert_eq!(network.num_inputs, 4);
        assert_eq!(network.num_hidden, 8);
        assert_eq!(network.num_outputs, 2);
    }

    #[test]
    fn test_kernel_neuroevo_manager() {
        let mut manager = KernelNeuroevoManager::new(12345);

        manager.initialize_target(KernelEvolutionTarget::Scheduler, 8, 4, 12345);

        // Set fitness for genomes
        if let Some(pop) = manager
            .populations
            .get_mut(&KernelEvolutionTarget::Scheduler)
        {
            for (i, genome) in pop.genomes.iter_mut().enumerate() {
                genome.fitness = i as f64 / 100.0;
            }
        }

        manager.evolve_target(KernelEvolutionTarget::Scheduler);

        let summary = manager.get_summary();
        assert_eq!(summary.total_generations, 1);
        assert!(summary.genomes_evaluated > 0);
    }

    #[test]
    fn test_generated_network() {
        let mut network = GeneratedNetwork::new(2, 4, 1);

        // Add some connections manually
        network.add_connection(0, 0, 2, 0.5);
        network.add_connection(0, 1, 3, 0.5);
        network.add_connection(1, 2, 6, 0.5);
        network.add_connection(1, 3, 6, 0.5);

        let output = network.activate(&[1.0, 1.0]);
        assert_eq!(output.len(), 1);
    }
}
