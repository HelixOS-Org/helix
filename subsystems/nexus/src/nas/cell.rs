//! Neural architecture cell (building block).

#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;

use super::types::OperationType;

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
    #[inline(always)]
    pub fn add_edge(&mut self, from: usize, to: usize, op: OperationType) {
        self.edges.push((from, to, op));
    }

    /// Count parameters in this cell
    #[inline]
    pub fn param_count(&self) -> usize {
        self.edges
            .iter()
            .map(|(_, _, op)| op.memory_bytes(self.input_dim, self.output_dim) / 4)
            .sum()
    }

    /// Estimate FLOPs for this cell
    #[inline]
    pub fn estimated_flops(&self) -> u64 {
        self.edges
            .iter()
            .map(|(_, _, op)| op.estimated_flops(self.input_dim, self.output_dim))
            .sum()
    }
}
