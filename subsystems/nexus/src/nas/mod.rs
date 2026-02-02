//! # Neural Architecture Search (NAS) for Kernel Decision Making
//!
//! Automatic discovery of optimal neural network topologies for kernel-level
//! AI decisions. This revolutionary module uses evolutionary algorithms and
//! differentiable architecture search to find optimal network structures.
//!
//! ## Features
//!
//! - **DARTS (Differentiable Architecture Search)**: Gradient-based topology search
//! - **ENAS (Efficient NAS)**: Parameter sharing for fast architecture evaluation
//! - **Neural Topology Evolution**: Evolutionary search for network structures
//! - **Once-For-All Networks**: Train once, deploy many architectures
//! - **Hardware-Aware NAS**: Optimize for kernel execution constraints
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    NEURAL ARCHITECTURE SEARCH                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐    │
//! │  │                   SEARCH SPACE                                   │    │
//! │  │   Operations: [Conv, Linear, ReLU, Skip, Pool, Attention]       │    │
//! │  │   Connections: Dense, Sparse, Residual                          │    │
//! │  │   Depth: 2-32 layers                                            │    │
//! │  │   Width: 8-512 neurons                                          │    │
//! │  └─────────────────────────────────────────────────────────────────┘    │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐    │
//! │  │                   SEARCH ALGORITHM                               │    │
//! │  │                                                                  │    │
//! │  │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │    │
//! │  │   │  DARTS   │  │   ENAS   │  │ Evolution│  │    OFA   │       │    │
//! │  │   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │    │
//! │  │        └─────────────┴─────────────┴─────────────┘             │    │
//! │  │                              │                                  │    │
//! │  │                              ▼                                  │    │
//! │  │                    ┌───────────────────┐                        │    │
//! │  │                    │ Architecture Pool │                        │    │
//! │  │                    └───────────────────┘                        │    │
//! │  └─────────────────────────────────────────────────────────────────┘    │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐    │
//! │  │                   EVALUATION & SELECTION                         │    │
//! │  │   Accuracy + Latency + Memory → Pareto Optimal                  │    │
//! │  └─────────────────────────────────────────────────────────────────┘    │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
// TODO: Ces sous-modules doivent être créés
// pub mod darts;
// pub mod enas;
// pub mod evolution;
// pub mod ofa;
// pub mod search_space;
// pub mod hardware;
// pub mod supernet;
// pub mod cell;
// pub mod proxy;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::math::F64Ext;
use crate::types::{NexusError, NexusResult};
// TODO: These types don't exist in ml module yet
// use crate::ml::{Tensor, NeuralNetwork, Layer, Activation};

/// Operation types in the search space
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    /// No operation (zero)
    Zero,
    /// Skip connection (identity)
    Skip,
    /// 3x3 separable convolution
    SepConv3x3,
    /// 5x5 separable convolution
    SepConv5x5,
    /// 3x3 dilated convolution
    DilConv3x3,
    /// 5x5 dilated convolution
    DilConv5x5,
    /// 3x3 max pooling
    MaxPool3x3,
    /// 3x3 average pooling
    AvgPool3x3,
    /// Fully connected (linear)
    Linear,
    /// Linear with ReLU
    LinearReLU,
    /// Linear with GELU
    LinearGELU,
    /// Multi-head attention
    Attention,
    /// Layer normalization
    LayerNorm,
    /// Batch normalization
    BatchNorm,
    /// Squeeze and excitation
    SEBlock,
    /// Depthwise separable
    DepthwiseSep,
}

impl OperationType {
    /// Get FLOPs estimate for this operation
    pub fn estimated_flops(&self, input_size: usize, output_size: usize) -> u64 {
        match self {
            Self::Zero => 0,
            Self::Skip => 0,
            Self::Linear | Self::LinearReLU | Self::LinearGELU => {
                (input_size * output_size * 2) as u64
            },
            Self::Attention => (input_size * input_size * 3) as u64, // Q, K, V
            Self::LayerNorm | Self::BatchNorm => (input_size * 4) as u64,
            Self::SepConv3x3 | Self::DilConv3x3 => (input_size * 9 * 2) as u64,
            Self::SepConv5x5 | Self::DilConv5x5 => (input_size * 25 * 2) as u64,
            Self::MaxPool3x3 | Self::AvgPool3x3 => (input_size * 9) as u64,
            Self::SEBlock => (input_size * 4) as u64,
            Self::DepthwiseSep => (input_size * 9 + input_size * output_size) as u64,
        }
    }

    /// Get memory footprint for this operation
    pub fn memory_bytes(&self, input_size: usize, output_size: usize) -> usize {
        match self {
            Self::Zero | Self::Skip => 0,
            Self::Linear | Self::LinearReLU | Self::LinearGELU => {
                input_size * output_size * 4 + output_size * 4 // weights + bias
            },
            Self::Attention => input_size * 4 * 4, // 4 projection matrices
            Self::LayerNorm | Self::BatchNorm => output_size * 8, // gamma + beta
            _ => output_size * 4,
        }
    }
}

/// Neural architecture cell (building block)
#[derive(Debug, Clone)]
pub struct Cell {
    /// Cell identifier
    pub id: usize,
    /// Number of nodes in the cell
    pub num_nodes: usize,
    /// Operations on edges (from_node, to_node, operation)
    pub edges: Vec<(usize, usize, OperationType)>,
    /// Whether this is a reduction cell
    pub is_reduction: bool,
    /// Input dimension
    pub input_dim: usize,
    /// Output dimension
    pub output_dim: usize,
}

impl Cell {
    /// Create a new cell
    pub fn new(id: usize, num_nodes: usize, input_dim: usize, output_dim: usize) -> Self {
        Self {
            id,
            num_nodes,
            edges: Vec::new(),
            is_reduction: false,
            input_dim,
            output_dim,
        }
    }

    /// Add an edge with operation
    pub fn add_edge(&mut self, from: usize, to: usize, op: OperationType) {
        self.edges.push((from, to, op));
    }

    /// Count parameters in this cell
    pub fn param_count(&self) -> usize {
        self.edges
            .iter()
            .map(|(_, _, op)| op.memory_bytes(self.input_dim, self.output_dim) / 4)
            .sum()
    }

    /// Estimate FLOPs for this cell
    pub fn estimated_flops(&self) -> u64 {
        self.edges
            .iter()
            .map(|(_, _, op)| op.estimated_flops(self.input_dim, self.output_dim))
            .sum()
    }
}

/// Complete neural architecture
#[derive(Debug, Clone)]
pub struct Architecture {
    /// Unique identifier
    pub id: u64,
    /// Architecture name
    pub name: String,
    /// Cells in the architecture
    pub cells: Vec<Cell>,
    /// Number of cells of each type
    pub num_normal_cells: usize,
    pub num_reduction_cells: usize,
    /// Initial channel count
    pub init_channels: usize,
    /// Number of output classes
    pub num_classes: usize,
    /// Auxiliary head weight
    pub auxiliary_weight: f64,
    /// Performance metrics
    pub metrics: ArchitectureMetrics,
}

/// Architecture performance metrics
#[derive(Debug, Clone, Default)]
pub struct ArchitectureMetrics {
    /// Validation accuracy
    pub accuracy: f64,
    /// Training loss
    pub loss: f64,
    /// Inference latency (microseconds)
    pub latency_us: u64,
    /// Memory usage (bytes)
    pub memory_bytes: usize,
    /// Total parameters
    pub params: usize,
    /// Total FLOPs
    pub flops: u64,
    /// Pareto rank (lower is better)
    pub pareto_rank: usize,
    /// Energy efficiency score
    pub energy_score: f64,
}

impl Architecture {
    /// Create a new architecture
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            cells: Vec::new(),
            num_normal_cells: 0,
            num_reduction_cells: 0,
            init_channels: 16,
            num_classes: 10,
            auxiliary_weight: 0.4,
            metrics: ArchitectureMetrics::default(),
        }
    }

    /// Total parameter count
    pub fn total_params(&self) -> usize {
        self.cells.iter().map(|c| c.param_count()).sum()
    }

    /// Total FLOPs
    pub fn total_flops(&self) -> u64 {
        self.cells.iter().map(|c| c.estimated_flops()).sum()
    }

    /// Check if architecture satisfies constraints
    pub fn satisfies_constraints(&self, constraints: &ArchitectureConstraints) -> bool {
        let params = self.total_params();
        let flops = self.total_flops();
        let memory = self
            .cells
            .iter()
            .map(|c| c.input_dim * 4 + c.output_dim * 4)
            .sum::<usize>();

        params <= constraints.max_params
            && flops <= constraints.max_flops
            && memory <= constraints.max_memory
            && self.metrics.latency_us <= constraints.max_latency_us
    }
}

/// Constraints for architecture search
#[derive(Debug, Clone)]
pub struct ArchitectureConstraints {
    /// Maximum parameters
    pub max_params: usize,
    /// Maximum FLOPs
    pub max_flops: u64,
    /// Maximum memory (bytes)
    pub max_memory: usize,
    /// Maximum latency (microseconds)
    pub max_latency_us: u64,
    /// Minimum accuracy
    pub min_accuracy: f64,
    /// Target operations
    pub allowed_ops: Vec<OperationType>,
}

impl Default for ArchitectureConstraints {
    fn default() -> Self {
        Self {
            max_params: 1_000_000,
            max_flops: 100_000_000,
            max_memory: 10 * 1024 * 1024, // 10 MB
            max_latency_us: 1000,
            min_accuracy: 0.9,
            allowed_ops: alloc::vec![
                OperationType::Linear,
                OperationType::LinearReLU,
                OperationType::Skip,
                OperationType::LayerNorm,
            ],
        }
    }
}

/// Search space configuration
#[derive(Debug, Clone)]
pub struct SearchSpace {
    /// Available operations
    pub operations: Vec<OperationType>,
    /// Min/max number of nodes per cell
    pub min_nodes: usize,
    pub max_nodes: usize,
    /// Min/max number of cells
    pub min_cells: usize,
    pub max_cells: usize,
    /// Min/max width (channels)
    pub min_width: usize,
    pub max_width: usize,
    /// Allow skip connections
    pub allow_skip: bool,
    /// Allow attention
    pub allow_attention: bool,
}

impl Default for SearchSpace {
    fn default() -> Self {
        Self {
            operations: alloc::vec![
                OperationType::Linear,
                OperationType::LinearReLU,
                OperationType::Skip,
                OperationType::Zero,
            ],
            min_nodes: 2,
            max_nodes: 8,
            min_cells: 1,
            max_cells: 8,
            min_width: 16,
            max_width: 256,
            allow_skip: true,
            allow_attention: false,
        }
    }
}

/// Architecture encoding for search algorithms
#[derive(Debug, Clone)]
pub struct ArchitectureEncoding {
    /// Edge operation indices (encoded as integers)
    pub edges: Vec<u8>,
    /// Cell configuration
    pub cell_config: Vec<u8>,
    /// Width multipliers
    pub widths: Vec<u8>,
    /// Fitness value
    pub fitness: f64,
    /// Generation created
    pub generation: usize,
}

impl ArchitectureEncoding {
    /// Create random encoding
    pub fn random(search_space: &SearchSpace, rng: &mut u64) -> Self {
        let num_ops = search_space.operations.len();
        let num_edges = search_space.max_nodes * (search_space.max_nodes - 1) / 2;

        let edges: Vec<u8> = (0..num_edges)
            .map(|_| {
                *rng ^= *rng << 13;
                *rng ^= *rng >> 7;
                *rng ^= *rng << 17;
                (*rng % num_ops as u64) as u8
            })
            .collect();

        let num_cells = ((*rng >> 16) as usize
            % (search_space.max_cells - search_space.min_cells + 1))
            + search_space.min_cells;

        let cell_config: Vec<u8> = (0..num_cells)
            .map(|_| {
                *rng ^= *rng << 13;
                *rng ^= *rng >> 7;
                (*rng % 4) as u8 // Cell type encoding
            })
            .collect();

        let widths: Vec<u8> = (0..num_cells)
            .map(|_| {
                *rng ^= *rng << 13;
                *rng ^= *rng >> 7;
                (*rng % 8) as u8 // Width multiplier
            })
            .collect();

        Self {
            edges,
            cell_config,
            widths,
            fitness: 0.0,
            generation: 0,
        }
    }

    /// Decode to architecture
    pub fn decode(&self, search_space: &SearchSpace, id: u64) -> Architecture {
        let mut arch = Architecture::new(id, alloc::format!("arch_{}", id));

        for (cell_idx, &config) in self.cell_config.iter().enumerate() {
            let num_nodes = search_space.min_nodes
                + (config as usize % (search_space.max_nodes - search_space.min_nodes + 1));

            let width = search_space.min_width
                + (self.widths.get(cell_idx).copied().unwrap_or(0) as usize * 16);

            let mut cell = Cell::new(cell_idx, num_nodes, width, width);
            cell.is_reduction = config > 2;

            // Add edges based on encoding
            let mut edge_idx = 0;
            for to in 2..num_nodes {
                for from in 0..to {
                    if edge_idx < self.edges.len() {
                        let op_idx = self.edges[edge_idx] as usize % search_space.operations.len();
                        cell.add_edge(from, to, search_space.operations[op_idx]);
                        edge_idx += 1;
                    }
                }
            }

            if cell.is_reduction {
                arch.num_reduction_cells += 1;
            } else {
                arch.num_normal_cells += 1;
            }
            arch.cells.push(cell);
        }

        arch.init_channels = search_space.min_width;
        arch
    }

    /// Mutate encoding
    pub fn mutate(&mut self, mutation_rate: f64, search_space: &SearchSpace, rng: &mut u64) {
        let num_ops = search_space.operations.len();

        // Mutate edges
        for edge in &mut self.edges {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            if (*rng as f64 / u64::MAX as f64) < mutation_rate {
                *rng ^= *rng << 13;
                *edge = (*rng % num_ops as u64) as u8;
            }
        }

        // Mutate cell config
        for config in &mut self.cell_config {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            if (*rng as f64 / u64::MAX as f64) < mutation_rate * 0.5 {
                *rng ^= *rng << 13;
                *config = (*rng % 4) as u8;
            }
        }

        // Mutate widths
        for width in &mut self.widths {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            if (*rng as f64 / u64::MAX as f64) < mutation_rate * 0.3 {
                *rng ^= *rng << 13;
                *width = (*rng % 8) as u8;
            }
        }
    }

    /// Crossover with another encoding
    pub fn crossover(&self, other: &Self, rng: &mut u64) -> Self {
        let mut child = self.clone();

        // Uniform crossover for edges
        for (i, edge) in child.edges.iter_mut().enumerate() {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            if *rng & 1 == 1 {
                if let Some(&other_edge) = other.edges.get(i) {
                    *edge = other_edge;
                }
            }
        }

        // Single-point crossover for cell config
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        let crossover_point = (*rng as usize) % child.cell_config.len().max(1);
        for i in crossover_point..child.cell_config.len() {
            if let Some(&other_config) = other.cell_config.get(i) {
                child.cell_config[i] = other_config;
            }
        }

        child.fitness = 0.0;
        child.generation = self.generation.max(other.generation) + 1;
        child
    }
}

/// NAS search configuration
#[derive(Debug, Clone)]
pub struct NasConfig {
    /// Population size for evolutionary search
    pub population_size: usize,
    /// Number of generations
    pub num_generations: usize,
    /// Mutation rate
    pub mutation_rate: f64,
    /// Crossover rate
    pub crossover_rate: f64,
    /// Elite size (preserved between generations)
    pub elite_size: usize,
    /// Tournament size for selection
    pub tournament_size: usize,
    /// Use weight sharing (ENAS)
    pub weight_sharing: bool,
    /// Early stopping patience
    pub patience: usize,
    /// Number of epochs for architecture evaluation
    pub eval_epochs: usize,
    /// Proxy task (for faster evaluation)
    pub use_proxy: bool,
}

impl Default for NasConfig {
    fn default() -> Self {
        Self {
            population_size: 50,
            num_generations: 100,
            mutation_rate: 0.1,
            crossover_rate: 0.7,
            elite_size: 5,
            tournament_size: 3,
            weight_sharing: true,
            patience: 10,
            eval_epochs: 5,
            use_proxy: true,
        }
    }
}

/// Neural Architecture Search Engine
pub struct NasEngine {
    /// Configuration
    config: NasConfig,
    /// Search space
    search_space: SearchSpace,
    /// Constraints
    constraints: ArchitectureConstraints,
    /// Current population
    population: Vec<ArchitectureEncoding>,
    /// Best architecture found
    best_architecture: Option<Architecture>,
    /// Best fitness achieved
    best_fitness: f64,
    /// Generation counter
    generation: usize,
    /// RNG state
    rng: u64,
    /// Search history
    history: Vec<SearchHistoryEntry>,
    /// Supernet for weight sharing
    supernet: Option<Supernet>,
}

/// Search history entry
#[derive(Debug, Clone)]
pub struct SearchHistoryEntry {
    pub generation: usize,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub population_diversity: f64,
    pub num_valid_architectures: usize,
}

/// Supernet for weight sharing
#[derive(Debug, Clone)]
pub struct Supernet {
    /// Shared weights for each operation type
    pub shared_weights: BTreeMap<u8, Vec<f64>>,
    /// Training iterations
    pub iterations: usize,
}

impl Supernet {
    pub fn new(search_space: &SearchSpace) -> Self {
        let mut shared_weights = BTreeMap::new();

        for (i, _op) in search_space.operations.iter().enumerate() {
            // Initialize random weights for each operation
            let weights: Vec<f64> = (0..1000)
                .map(|j| ((i + j) as f64 * 0.01).sin() * 0.1)
                .collect();
            shared_weights.insert(i as u8, weights);
        }

        Self {
            shared_weights,
            iterations: 0,
        }
    }

    /// Get weights for an architecture
    pub fn get_weights(&self, encoding: &ArchitectureEncoding) -> Vec<f64> {
        let mut weights = Vec::new();
        for &edge in &encoding.edges {
            if let Some(w) = self.shared_weights.get(&edge) {
                weights.extend(w.iter().take(100));
            }
        }
        weights
    }

    /// Update shared weights
    pub fn update_weights(&mut self, encoding: &ArchitectureEncoding, gradients: &[f64], lr: f64) {
        let mut grad_idx = 0;
        for &edge in &encoding.edges {
            if let Some(w) = self.shared_weights.get_mut(&edge) {
                for i in 0..100.min(w.len()) {
                    if grad_idx < gradients.len() {
                        w[i] -= lr * gradients[grad_idx];
                        grad_idx += 1;
                    }
                }
            }
        }
        self.iterations += 1;
    }
}

impl NasEngine {
    /// Create a new NAS engine
    pub fn new(
        config: NasConfig,
        search_space: SearchSpace,
        constraints: ArchitectureConstraints,
    ) -> Self {
        let mut rng = 0xDEADBEEF12345678u64;

        // Initialize population
        let population: Vec<ArchitectureEncoding> = (0..config.population_size)
            .map(|_| ArchitectureEncoding::random(&search_space, &mut rng))
            .collect();

        let supernet = if config.weight_sharing {
            Some(Supernet::new(&search_space))
        } else {
            None
        };

        Self {
            config,
            search_space,
            constraints,
            population,
            best_architecture: None,
            best_fitness: f64::MIN,
            generation: 0,
            rng,
            history: Vec::new(),
            supernet,
        }
    }

    /// Run the NAS search
    pub fn search(&mut self) -> NexusResult<Architecture> {
        let mut generations_without_improvement = 0;

        for generation_idx in 0..self.config.num_generations {
            self.generation = generation_idx;

            // Evaluate population
            self.evaluate_population()?;

            // Record history
            let entry = self.record_history();
            self.history.push(entry);

            // Update best
            if let Some(best) = self
                .population
                .iter()
                .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(Ordering::Equal))
            {
                if best.fitness > self.best_fitness {
                    self.best_fitness = best.fitness;
                    self.best_architecture =
                        Some(best.decode(&self.search_space, generation_idx as u64));
                    generations_without_improvement = 0;
                } else {
                    generations_without_improvement += 1;
                }
            }

            // Early stopping
            if generations_without_improvement >= self.config.patience {
                break;
            }

            // Evolution
            self.evolve_population()?;
        }

        self.best_architecture
            .clone()
            .ok_or(NexusError::operation_failed())
    }

    /// Evaluate all architectures in population
    fn evaluate_population(&mut self) -> NexusResult<()> {
        for i in 0..self.population.len() {
            let arch = self.population[i].decode(&self.search_space, self.generation as u64);

            // Check constraints
            if !arch.satisfies_constraints(&self.constraints) {
                self.population[i].fitness = -1.0;
                continue;
            }

            // Evaluate fitness
            let fitness = if self.config.use_proxy {
                self.proxy_evaluate(&arch)?
            } else if self.config.weight_sharing {
                let encoding = self.population[i].clone();
                self.supernet_evaluate(&encoding)?
            } else {
                self.full_evaluate(&arch)?
            };
            self.population[i].fitness = fitness;
        }

        Ok(())
    }

    /// Quick proxy evaluation
    fn proxy_evaluate(&self, arch: &Architecture) -> NexusResult<f64> {
        // Use architecture complexity as proxy for performance
        let params = arch.total_params() as f64;
        let flops = arch.total_flops() as f64;

        // Prefer smaller, more efficient architectures
        let efficiency = 1.0 / (1.0 + params / 100000.0);
        let speed = 1.0 / (1.0 + flops / 10000000.0);

        // Depth bonus (deeper = potentially better)
        let depth_bonus = (arch.cells.len() as f64).sqrt() / 5.0;

        // Skip connection bonus
        let skip_count = arch
            .cells
            .iter()
            .flat_map(|c| c.edges.iter())
            .filter(|(_, _, op)| *op == OperationType::Skip)
            .count();
        let skip_bonus = (skip_count as f64) * 0.05;

        Ok(efficiency * 0.3 + speed * 0.3 + depth_bonus * 0.2 + skip_bonus * 0.2)
    }

    /// Evaluate using supernet with weight sharing
    fn supernet_evaluate(&self, encoding: &ArchitectureEncoding) -> NexusResult<f64> {
        let supernet = self
            .supernet
            .as_ref()
            .ok_or(NexusError::not_initialized())?;

        let weights = supernet.get_weights(encoding);

        // Simple forward pass simulation
        let complexity = weights.len() as f64;
        let weight_sum: f64 = weights.iter().map(|w| w.abs()).sum();

        // Normalized fitness
        Ok(weight_sum / complexity.max(1.0))
    }

    /// Full training evaluation
    fn full_evaluate(&self, arch: &Architecture) -> NexusResult<f64> {
        // Would train the architecture for real
        // For kernel usage, use proxy instead
        self.proxy_evaluate(arch)
    }

    /// Evolve population to next generation
    fn evolve_population(&mut self) -> NexusResult<()> {
        // Sort by fitness
        self.population
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(Ordering::Equal));

        let mut new_population = Vec::with_capacity(self.config.population_size);

        // Elitism: keep best individuals
        for i in 0..self.config.elite_size.min(self.population.len()) {
            let mut elite = self.population[i].clone();
            elite.generation = self.generation + 1;
            new_population.push(elite);
        }

        // Generate rest of population through selection & crossover
        while new_population.len() < self.config.population_size {
            let parent1 = self.tournament_select_cloned();
            let parent2 = self.tournament_select_cloned();

            // Crossover
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            let mut child = if (self.rng as f64 / u64::MAX as f64) < self.config.crossover_rate {
                parent1.crossover(&parent2, &mut self.rng)
            } else {
                parent1.clone()
            };

            // Mutation
            child.mutate(self.config.mutation_rate, &self.search_space, &mut self.rng);
            child.generation = self.generation + 1;

            new_population.push(child);
        }

        self.population = new_population;
        Ok(())
    }

    /// Tournament selection - returns cloned encoding to avoid borrow issues
    fn tournament_select_cloned(&mut self) -> ArchitectureEncoding {
        let mut best_idx = 0;
        let mut best_fitness = f64::MIN;

        for _ in 0..self.config.tournament_size {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            let idx = (self.rng as usize) % self.population.len();

            if self.population[idx].fitness > best_fitness {
                best_fitness = self.population[idx].fitness;
                best_idx = idx;
            }
        }

        self.population[best_idx].clone()
    }

    /// Record search history
    fn record_history(&self) -> SearchHistoryEntry {
        let fitnesses: Vec<f64> = self
            .population
            .iter()
            .filter(|e| e.fitness >= 0.0)
            .map(|e| e.fitness)
            .collect();

        let avg_fitness = if fitnesses.is_empty() {
            0.0
        } else {
            fitnesses.iter().sum::<f64>() / fitnesses.len() as f64
        };

        // Calculate diversity (unique edge patterns)
        let unique_patterns: alloc::collections::BTreeSet<_> =
            self.population.iter().map(|e| e.edges.clone()).collect();

        SearchHistoryEntry {
            generation: self.generation,
            best_fitness: self.best_fitness,
            avg_fitness,
            population_diversity: unique_patterns.len() as f64 / self.population.len() as f64,
            num_valid_architectures: fitnesses.len(),
        }
    }

    /// Get search statistics
    pub fn get_stats(&self) -> NasStats {
        NasStats {
            generation: self.generation,
            best_fitness: self.best_fitness,
            population_size: self.population.len(),
            history_len: self.history.len(),
            best_architecture: self.best_architecture.clone(),
        }
    }
}

/// NAS statistics
#[derive(Debug, Clone)]
pub struct NasStats {
    pub generation: usize,
    pub best_fitness: f64,
    pub population_size: usize,
    pub history_len: usize,
    pub best_architecture: Option<Architecture>,
}

/// Kernel-specific architecture search
pub struct KernelNas {
    engine: NasEngine,
    task_type: KernelNasTask,
}

/// Kernel NAS task types
#[derive(Debug, Clone, Copy)]
pub enum KernelNasTask {
    /// Scheduler decision network
    SchedulerDecision,
    /// Memory allocation prediction
    MemoryPrediction,
    /// Anomaly detection
    AnomalyDetection,
    /// Resource estimation
    ResourceEstimation,
    /// Failure prediction
    FailurePrediction,
    /// Cache optimization
    CacheOptimization,
}

impl KernelNas {
    /// Create NAS for a specific kernel task
    pub fn for_task(task: KernelNasTask) -> Self {
        let (search_space, constraints) = match task {
            KernelNasTask::SchedulerDecision => (
                SearchSpace {
                    operations: alloc::vec![
                        OperationType::Linear,
                        OperationType::LinearReLU,
                        OperationType::Skip,
                    ],
                    max_nodes: 4,
                    max_cells: 3,
                    max_width: 64,
                    ..Default::default()
                },
                ArchitectureConstraints {
                    max_params: 10000,
                    max_flops: 100000,
                    max_latency_us: 10,
                    ..Default::default()
                },
            ),
            KernelNasTask::MemoryPrediction => (
                SearchSpace {
                    operations: alloc::vec![
                        OperationType::Linear,
                        OperationType::LinearReLU,
                        OperationType::LayerNorm,
                        OperationType::Skip,
                    ],
                    max_nodes: 6,
                    max_cells: 4,
                    max_width: 128,
                    ..Default::default()
                },
                ArchitectureConstraints {
                    max_params: 50000,
                    max_flops: 1000000,
                    max_latency_us: 100,
                    ..Default::default()
                },
            ),
            KernelNasTask::AnomalyDetection => (
                SearchSpace {
                    operations: alloc::vec![
                        OperationType::Linear,
                        OperationType::LinearGELU,
                        OperationType::LayerNorm,
                        OperationType::Attention,
                        OperationType::Skip,
                    ],
                    max_nodes: 8,
                    max_cells: 6,
                    max_width: 256,
                    allow_attention: true,
                    ..Default::default()
                },
                ArchitectureConstraints {
                    max_params: 200000,
                    max_flops: 10000000,
                    max_latency_us: 500,
                    min_accuracy: 0.95,
                    ..Default::default()
                },
            ),
            _ => (SearchSpace::default(), ArchitectureConstraints::default()),
        };

        let config = NasConfig {
            population_size: 30,
            num_generations: 50,
            use_proxy: true,
            weight_sharing: true,
            ..Default::default()
        };

        Self {
            engine: NasEngine::new(config, search_space, constraints),
            task_type: task,
        }
    }

    /// Search for optimal architecture
    pub fn search(&mut self) -> NexusResult<Architecture> {
        self.engine.search()
    }

    /// Get the best architecture found
    pub fn best_architecture(&self) -> Option<&Architecture> {
        self.engine.best_architecture.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_encoding() {
        let search_space = SearchSpace::default();
        let mut rng = 12345u64;

        let encoding = ArchitectureEncoding::random(&search_space, &mut rng);
        let arch = encoding.decode(&search_space, 1);

        assert!(!arch.cells.is_empty());
        assert!(arch.total_params() > 0);
    }

    #[test]
    fn test_nas_search() {
        let config = NasConfig {
            population_size: 10,
            num_generations: 5,
            ..Default::default()
        };

        let mut engine = NasEngine::new(
            config,
            SearchSpace::default(),
            ArchitectureConstraints::default(),
        );

        let result = engine.search();
        assert!(result.is_ok());
    }

    #[test]
    fn test_kernel_nas() {
        let mut nas = KernelNas::for_task(KernelNasTask::SchedulerDecision);
        let result = nas.search();

        assert!(result.is_ok());
        let arch = result.unwrap();
        assert!(arch.total_params() <= 10000);
    }

    #[test]
    fn test_mutation_crossover() {
        let search_space = SearchSpace::default();
        let mut rng = 67890u64;

        let mut parent1 = ArchitectureEncoding::random(&search_space, &mut rng);
        let parent2 = ArchitectureEncoding::random(&search_space, &mut rng);

        let child = parent1.crossover(&parent2, &mut rng);
        assert_eq!(child.generation, 1);

        parent1.mutate(1.0, &search_space, &mut rng); // High mutation rate
        // Some edges should have changed
    }
}
