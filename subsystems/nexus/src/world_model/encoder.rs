//! Observation encoder for the world model.

use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::latent::LatentState;

/// Observation encoder
#[derive(Debug, Clone)]
pub struct Encoder {
    /// Input dimension
    pub input_dim: usize,
    /// Latent dimension
    pub latent_dim: usize,
    /// Weight matrices
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
    /// Layer sizes
    pub layer_sizes: Vec<usize>,
}

impl Encoder {
    /// Create a new encoder
    pub fn new(input_dim: usize, latent_dim: usize, hidden_sizes: &[usize]) -> Self {
        let mut layer_sizes = vec![input_dim];
        layer_sizes.extend_from_slice(hidden_sizes);
        layer_sizes.push(latent_dim * 2); // Mean and log variance

        let mut weights = Vec::new();
        let mut biases = Vec::new();

        for i in 0..layer_sizes.len() - 1 {
            let in_size = layer_sizes[i];
            let out_size = layer_sizes[i + 1];

            // Xavier initialization
            let scale = libm::sqrt(2.0 / (in_size + out_size) as f64);
            let mut w = Vec::with_capacity(out_size);

            for j in 0..out_size {
                let mut row = Vec::with_capacity(in_size);
                for k in 0..in_size {
                    let seed = ((i * 1000 + j * 100 + k) as u64).wrapping_mul(6364136223846793005);
                    let val = ((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                    row.push(val);
                }
                w.push(row);
            }

            weights.push(w);
            biases.push(vec![0.0; out_size]);
        }

        Self {
            input_dim,
            latent_dim,
            weights,
            biases,
            layer_sizes,
        }
    }

    /// Forward pass
    pub fn encode(&self, observation: &[f64]) -> LatentState {
        let mut x = observation.to_vec();

        // Forward through layers
        for i in 0..self.weights.len() {
            let mut y = self.biases[i].clone();

            for (j, bias_j) in y.iter_mut().enumerate() {
                for (k, &xk) in x.iter().enumerate() {
                    *bias_j += self.weights[i][j][k] * xk;
                }
            }

            // ReLU (except last layer)
            if i < self.weights.len() - 1 {
                for v in &mut y {
                    *v = v.max(0.0);
                }
            }

            x = y;
        }

        // Split into mean and log variance
        let mean: Vec<f64> = x[..self.latent_dim].to_vec();
        let log_var: Vec<f64> = x[self.latent_dim..].to_vec();

        let variance: Vec<f64> = log_var
            .iter()
            .map(|&lv| libm::exp(lv.clamp(-10.0, 10.0)))
            .collect();

        LatentState {
            z: mean.clone(),
            uncertainty: variance,
            timestamp: 0,
            h: mean[..mean.len() / 2].to_vec(),
            s: mean[mean.len() / 2..].to_vec(),
        }
    }
}
