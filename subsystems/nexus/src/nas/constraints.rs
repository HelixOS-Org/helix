//! Architecture constraints for NAS.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;

use super::types::OperationType;

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
