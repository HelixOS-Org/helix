//! Semantic encoder for zero-shot learning.

extern crate alloc;

use alloc::vec::Vec;

use crate::math::F64Ext;
use crate::zeroshot::types::{EmbeddingVector, FeatureVector};

/// Semantic embedding layer
#[derive(Debug, Clone)]
pub struct SemanticEncoder {
    /// Input dimension
    input_dim: usize,
    /// Hidden dimension
    hidden_dim: usize,
    /// Output (embedding) dimension
    output_dim: usize,
    /// First layer weights
    weights_1: Vec<f64>,
    /// Second layer weights
    weights_2: Vec<f64>,
    /// First layer biases
    biases_1: Vec<f64>,
    /// Second layer biases
    biases_2: Vec<f64>,
}

impl SemanticEncoder {
    /// Create a new semantic encoder
    pub fn new(input_dim: usize, hidden_dim: usize, output_dim: usize) -> Self {
        let mut rng = 42u64;

        let weights_1 = Self::init_weights(input_dim * hidden_dim, &mut rng);
        let weights_2 = Self::init_weights(hidden_dim * output_dim, &mut rng);
        let biases_1 = alloc::vec![0.0; hidden_dim];
        let biases_2 = alloc::vec![0.0; output_dim];

        Self {
            input_dim,
            hidden_dim,
            output_dim,
            weights_1,
            weights_2,
            biases_1,
            biases_2,
        }
    }

    /// Xavier initialization
    fn init_weights(size: usize, rng: &mut u64) -> Vec<f64> {
        let scale = libm::sqrt(2.0 / size as f64);
        let mut weights = Vec::with_capacity(size);

        for _ in 0..size {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let r = (*rng as f64 / u64::MAX as f64) * 2.0 - 1.0;
            weights.push(r * scale);
        }

        weights
    }

    /// Encode input features to embedding space
    pub fn encode(&self, features: &FeatureVector) -> EmbeddingVector {
        assert_eq!(features.len(), self.input_dim);

        // First layer
        let mut hidden = Vec::with_capacity(self.hidden_dim);
        for (j, bias) in self.biases_1.iter().enumerate().take(self.hidden_dim) {
            let mut sum = *bias;
            for (i, feature) in features.iter().enumerate().take(self.input_dim) {
                sum += feature * self.weights_1[i * self.hidden_dim + j];
            }
            // ReLU activation
            hidden.push(sum.max(0.0));
        }

        // Second layer
        let mut output = Vec::with_capacity(self.output_dim);
        for (j, bias) in self.biases_2.iter().enumerate().take(self.output_dim) {
            let mut sum = *bias;
            for (i, h) in hidden.iter().enumerate() {
                sum += h * self.weights_2[i * self.output_dim + j];
            }
            output.push(sum);
        }

        // L2 normalize
        let norm: f64 = output.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-8 {
            for x in &mut output {
                *x /= norm;
            }
        }

        output
    }

    /// Update weights (for training)
    pub fn update_weights(&mut self, gradients: &[f64], learning_rate: f64) {
        let mut idx = 0;
        for w in &mut self.weights_1 {
            if idx < gradients.len() {
                *w -= learning_rate * gradients[idx];
                idx += 1;
            }
        }
        for w in &mut self.weights_2 {
            if idx < gradients.len() {
                *w -= learning_rate * gradients[idx];
                idx += 1;
            }
        }
    }
}
