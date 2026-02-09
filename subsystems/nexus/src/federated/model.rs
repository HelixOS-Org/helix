//! Federated model representation.

use alloc::string::String;
use alloc::vec::Vec;

use crate::federated::types::lcg_next;

/// A federated model (parameter vector)
#[derive(Debug, Clone)]
pub struct FederatedModel {
    /// Model parameters (flattened)
    pub parameters: Vec<f64>,
    /// Parameter shapes for reconstruction
    pub shapes: Vec<(usize, usize)>,
    /// Model version
    pub version: u64,
    /// Model name/id
    pub name: String,
    /// Creation timestamp
    pub timestamp: u64,
}

impl FederatedModel {
    /// Create a new model
    pub fn new(parameters: Vec<f64>, shapes: Vec<(usize, usize)>) -> Self {
        Self {
            parameters,
            shapes,
            version: 0,
            name: String::from("federated_model"),
            timestamp: 0,
        }
    }

    /// Create from layer dimensions
    pub fn from_layers(layer_dims: &[(usize, usize)], seed: u64) -> Self {
        let mut parameters = Vec::new();
        let mut shapes = Vec::new();
        let mut rng = seed;

        for &(in_dim, out_dim) in layer_dims {
            // Xavier initialization
            let scale = libm::sqrt(2.0 / (in_dim + out_dim) as f64);

            for _ in 0..(in_dim * out_dim) {
                rng = lcg_next(rng);
                let val = ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                parameters.push(val);
            }

            // Bias
            for _ in 0..out_dim {
                parameters.push(0.0);
            }

            shapes.push((in_dim, out_dim));
        }

        Self {
            parameters,
            shapes,
            version: 0,
            name: String::from("federated_model"),
            timestamp: 0,
        }
    }

    /// Number of parameters
    #[inline(always)]
    pub fn num_parameters(&self) -> usize {
        self.parameters.len()
    }

    /// Clone parameters
    #[inline(always)]
    pub fn get_parameters(&self) -> Vec<f64> {
        self.parameters.clone()
    }

    /// Set parameters
    #[inline(always)]
    pub fn set_parameters(&mut self, parameters: Vec<f64>) {
        self.parameters = parameters;
        self.version += 1;
    }

    /// Compute model norm
    #[inline(always)]
    pub fn norm(&self) -> f64 {
        libm::sqrt(self.parameters.iter().map(|x| x * x).sum())
    }

    /// Distance to another model
    #[inline]
    pub fn distance(&self, other: &FederatedModel) -> f64 {
        let sum_sq: f64 = self
            .parameters
            .iter()
            .zip(other.parameters.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum();

        libm::sqrt(sum_sq)
    }
}
