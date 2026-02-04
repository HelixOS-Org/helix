//! Utility functions for multimodal fusion.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

/// LCG random number generator
pub fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Create a random layer
pub fn create_layer(in_dim: usize, out_dim: usize, seed: u64) -> (Vec<Vec<f64>>, Vec<f64>, u64) {
    let scale = libm::sqrt(2.0 / (in_dim + out_dim) as f64);
    let mut rng = seed;

    let mut weight = Vec::with_capacity(out_dim);
    for _ in 0..out_dim {
        let mut row = Vec::with_capacity(in_dim);
        for _ in 0..in_dim {
            rng = lcg_next(rng);
            row.push(((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
        }
        weight.push(row);
    }

    let bias = vec![0.0; out_dim];

    (weight, bias, rng)
}

/// Project vector through weight matrix
pub fn project_vec(weight: &[Vec<f64>], input: &[f64]) -> Vec<f64> {
    let mut output = vec![0.0; weight.len()];

    for (i, out) in output.iter_mut().enumerate() {
        for (j, &inp) in input.iter().enumerate() {
            if j < weight[i].len() {
                *out += weight[i][j] * inp;
            }
        }
    }

    output
}

/// Layer normalization
pub fn layer_norm(x: &[f64], scale: &[f64], bias: &[f64]) -> Vec<f64> {
    if x.is_empty() {
        return Vec::new();
    }

    let mean: f64 = x.iter().sum::<f64>() / x.len() as f64;
    let var: f64 = x.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64;
    let std = libm::sqrt(var + 1e-5);

    x.iter()
        .zip(scale.iter())
        .zip(bias.iter())
        .map(|((&v, &s), &b)| (v - mean) / std * s + b)
        .collect()
}
