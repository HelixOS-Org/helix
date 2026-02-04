//! HyperNEAT - indirect encoding using compositional pattern-producing networks.

use alloc::vec;
use alloc::vec::Vec;

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::network::NeatNetwork;
use crate::neuroevo::population::{NeatConfig, NeatPopulation};

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
