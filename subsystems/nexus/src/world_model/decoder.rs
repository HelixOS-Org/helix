//! Observation decoder for the world model.

use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::latent::LatentState;

/// Observation decoder
#[derive(Debug, Clone)]
pub struct Decoder {
    /// Latent dimension
    pub latent_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Weight matrices
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
    /// Layer sizes
    pub layer_sizes: Vec<usize>,
}

impl Decoder {
    /// Create a new decoder
    pub fn new(latent_dim: usize, output_dim: usize, hidden_sizes: &[usize]) -> Self {
        let mut layer_sizes = vec![latent_dim];
        layer_sizes.extend_from_slice(hidden_sizes);
        layer_sizes.push(output_dim);

        let mut weights = Vec::new();
        let mut biases = Vec::new();

        for i in 0..layer_sizes.len() - 1 {
            let in_size = layer_sizes[i];
            let out_size = layer_sizes[i + 1];

            let scale = libm::sqrt(2.0 / (in_size + out_size) as f64);
            let mut w = Vec::with_capacity(out_size);

            for j in 0..out_size {
                let mut row = Vec::with_capacity(in_size);
                for k in 0..in_size {
                    let seed =
                        ((i * 1000 + j * 100 + k + 50000) as u64).wrapping_mul(6364136223846793005);
                    let val = ((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                    row.push(val);
                }
                w.push(row);
            }

            weights.push(w);
            biases.push(vec![0.0; out_size]);
        }

        Self {
            latent_dim,
            output_dim,
            weights,
            biases,
            layer_sizes,
        }
    }

    /// Forward pass
    pub fn decode(&self, state: &LatentState) -> Vec<f64> {
        let mut x = state.z.clone();

        for i in 0..self.weights.len() {
            let mut y = self.biases[i].clone();

            for (j, bias_j) in y.iter_mut().enumerate() {
                for (k, &xk) in x.iter().enumerate() {
                    if k < self.weights[i][j].len() {
                        *bias_j += self.weights[i][j][k] * xk;
                    }
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

        x
    }
}
