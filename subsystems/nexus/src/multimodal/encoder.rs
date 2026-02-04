//! Modality encoder for projecting input features to hidden representations.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::multimodal::utils::lcg_next;

/// Linear encoder for a modality
#[derive(Debug, Clone)]
pub struct ModalityEncoder {
    /// Input dimension
    pub input_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Weight matrix
    pub weight: Vec<Vec<f64>>,
    /// Bias
    pub bias: Vec<f64>,
    /// Layer normalization scale
    pub ln_scale: Vec<f64>,
    /// Layer normalization bias
    pub ln_bias: Vec<f64>,
}

impl ModalityEncoder {
    /// Create a new encoder
    pub fn new(input_dim: usize, output_dim: usize, seed: u64) -> Self {
        let scale = libm::sqrt(2.0 / (input_dim + output_dim) as f64);
        let mut rng = seed;

        let mut weight = Vec::with_capacity(output_dim);
        for _ in 0..output_dim {
            let mut row = Vec::with_capacity(input_dim);
            for _ in 0..input_dim {
                rng = lcg_next(rng);
                row.push(((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
            }
            weight.push(row);
        }

        Self {
            input_dim,
            output_dim,
            weight,
            bias: vec![0.0; output_dim],
            ln_scale: vec![1.0; output_dim],
            ln_bias: vec![0.0; output_dim],
        }
    }

    /// Encode modality
    pub fn encode(&self, input: &[f64]) -> Vec<f64> {
        let mut output = self.bias.clone();

        for (i, out) in output.iter_mut().enumerate() {
            for (j, &inp) in input.iter().enumerate() {
                if j < self.weight[i].len() {
                    *out += self.weight[i][j] * inp;
                }
            }
        }

        // ReLU
        for v in &mut output {
            *v = v.max(0.0);
        }

        // Layer normalization
        self.layer_norm(&mut output);

        output
    }

    /// Apply layer normalization
    fn layer_norm(&self, x: &mut Vec<f64>) {
        if x.is_empty() {
            return;
        }

        let mean: f64 = x.iter().sum::<f64>() / x.len() as f64;
        let var: f64 = x.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64;
        let std = libm::sqrt(var + 1e-5);

        for (i, v) in x.iter_mut().enumerate() {
            *v = (*v - mean) / std * self.ln_scale[i] + self.ln_bias[i];
        }
    }
}
