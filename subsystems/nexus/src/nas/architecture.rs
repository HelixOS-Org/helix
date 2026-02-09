//! Architecture and ArchitectureMetrics structs.

#![allow(dead_code)]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use super::cell::Cell;
use super::constraints::ArchitectureConstraints;

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
#[repr(align(64))]
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
    #[inline(always)]
    pub fn total_params(&self) -> usize {
        self.cells.iter().map(|c| c.param_count()).sum()
    }

    /// Total FLOPs
    #[inline(always)]
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
