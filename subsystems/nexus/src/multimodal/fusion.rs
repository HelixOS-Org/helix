//! Fusion strategies for combining multiple modalities.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::multimodal::encoder::ModalityEncoder;
use crate::multimodal::utils::{create_layer, lcg_next};

/// Early fusion: concatenate then process
#[derive(Debug, Clone)]
pub struct EarlyFusion {
    /// Total input dimension
    pub total_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Projection layers
    pub layers: Vec<(Vec<Vec<f64>>, Vec<f64>)>,
}

impl EarlyFusion {
    /// Create a new early fusion module
    pub fn new(modality_dims: &[usize], output_dim: usize, hidden_dim: usize, seed: u64) -> Self {
        let total_dim: usize = modality_dims.iter().sum();

        let mut layers = Vec::new();
        let mut rng = seed;

        // First layer: concat -> hidden
        let (w1, b1, rng2) = create_layer(total_dim, hidden_dim, rng);
        layers.push((w1, b1));
        rng = rng2;

        // Second layer: hidden -> output
        let (w2, b2, _) = create_layer(hidden_dim, output_dim, rng);
        layers.push((w2, b2));

        Self {
            total_dim,
            output_dim,
            layers,
        }
    }

    /// Fuse modalities
    pub fn fuse(&self, inputs: &[&[f64]]) -> Vec<f64> {
        // Concatenate all inputs
        let mut concat = Vec::with_capacity(self.total_dim);
        for input in inputs {
            concat.extend_from_slice(input);
        }

        // Forward through layers
        let mut x = concat;

        for (i, (weight, bias)) in self.layers.iter().enumerate() {
            let mut y = bias.clone();

            for (j, out) in y.iter_mut().enumerate() {
                for (k, &inp) in x.iter().enumerate() {
                    if k < weight[j].len() {
                        *out += weight[j][k] * inp;
                    }
                }
            }

            // ReLU for all but last layer
            if i < self.layers.len() - 1 {
                for v in &mut y {
                    *v = v.max(0.0);
                }
            }

            x = y;
        }

        x
    }
}

/// Late fusion: process separately then combine
#[derive(Debug, Clone)]
pub struct LateFusion {
    /// Modality encoders
    pub encoders: Vec<ModalityEncoder>,
    /// Fusion weights
    pub fusion_weights: Vec<f64>,
    /// Learnable weights
    pub learnable: bool,
}

impl LateFusion {
    /// Create a new late fusion module
    pub fn new(modality_dims: &[usize], hidden_dim: usize, seed: u64) -> Self {
        let mut encoders = Vec::new();
        let mut rng = seed;

        for &dim in modality_dims {
            encoders.push(ModalityEncoder::new(dim, hidden_dim, rng));
            rng = lcg_next(rng);
        }

        let num_modalities = modality_dims.len();
        let fusion_weights = vec![1.0 / num_modalities as f64; num_modalities];

        Self {
            encoders,
            fusion_weights,
            learnable: true,
        }
    }

    /// Fuse modalities
    pub fn fuse(&self, inputs: &[&[f64]], present: &[bool]) -> Vec<f64> {
        if self.encoders.is_empty() {
            return Vec::new();
        }

        let hidden_dim = self.encoders[0].output_dim;
        let mut fused = vec![0.0; hidden_dim];
        let mut total_weight = 0.0;

        for (i, (encoder, (&input, &is_present))) in self
            .encoders
            .iter()
            .zip(inputs.iter().zip(present.iter()))
            .enumerate()
        {
            if is_present {
                let encoded = encoder.encode(input);
                let weight = self.fusion_weights.get(i).copied().unwrap_or(1.0);

                for (f, &e) in fused.iter_mut().zip(encoded.iter()) {
                    *f += weight * e;
                }

                total_weight += weight;
            }
        }

        // Normalize
        if total_weight > 0.0 {
            for f in &mut fused {
                *f /= total_weight;
            }
        }

        fused
    }

    /// Update fusion weights based on importance
    pub fn update_weights(&mut self, importance_scores: &[f64]) {
        if importance_scores.len() != self.fusion_weights.len() {
            return;
        }

        // Softmax normalization
        let max_score = importance_scores
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| f64::max(a, b));
        let exp_scores: Vec<f64> = importance_scores
            .iter()
            .map(|&s| libm::exp(s - max_score))
            .collect();
        let sum: f64 = exp_scores.iter().sum();

        for (w, e) in self.fusion_weights.iter_mut().zip(exp_scores.iter()) {
            *w = e / sum;
        }
    }
}
