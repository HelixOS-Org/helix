//! Cross-modal attention mechanisms for multimodal fusion.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::multimodal::utils::create_layer;

/// Cross-modal attention mechanism
#[derive(Debug, Clone)]
pub struct CrossModalAttention {
    /// Hidden dimension
    pub hidden_dim: usize,
    /// Number of attention heads
    pub num_heads: usize,
    /// Query projection
    pub w_q: Vec<Vec<f64>>,
    /// Key projection
    pub w_k: Vec<Vec<f64>>,
    /// Value projection
    pub w_v: Vec<Vec<f64>>,
    /// Output projection
    pub w_o: Vec<Vec<f64>>,
}

impl CrossModalAttention {
    /// Create a new cross-modal attention
    pub fn new(hidden_dim: usize, num_heads: usize, seed: u64) -> Self {
        let (w_q, _, rng2) = create_layer(hidden_dim, hidden_dim, seed);
        let (w_k, _, rng3) = create_layer(hidden_dim, hidden_dim, rng2);
        let (w_v, _, rng4) = create_layer(hidden_dim, hidden_dim, rng3);
        let (w_o, _, _) = create_layer(hidden_dim, hidden_dim, rng4);

        Self {
            hidden_dim,
            num_heads,
            w_q,
            w_k,
            w_v,
            w_o,
        }
    }

    /// Compute attention from query modality to key/value modality
    pub fn attend(&self, query: &[f64], key: &[f64], value: &[f64]) -> Vec<f64> {
        // Project to Q, K, V
        let q = self.project(&self.w_q, query);
        let k = self.project(&self.w_k, key);
        let v = self.project(&self.w_v, value);

        // Compute attention score
        let scale = libm::sqrt(self.hidden_dim as f64);
        let score: f64 = q
            .iter()
            .zip(k.iter())
            .map(|(&qi, &ki)| qi * ki)
            .sum::<f64>()
            / scale;

        // Softmax (single key, so just sigmoid-like)
        let attention = 1.0 / (1.0 + libm::exp(-score));

        // Apply attention to values
        let attended: Vec<f64> = v.iter().map(|&vi| attention * vi).collect();

        // Output projection
        self.project(&self.w_o, &attended)
    }

    /// Multi-head attention over multiple modalities
    pub fn multi_attend(&self, query: &[f64], keys: &[&[f64]], values: &[&[f64]]) -> Vec<f64> {
        if keys.is_empty() || values.is_empty() {
            return query.to_vec();
        }

        let q = self.project(&self.w_q, query);
        let scale = libm::sqrt(self.hidden_dim as f64);

        // Compute attention scores
        let mut scores: Vec<f64> = Vec::new();
        let mut projected_values: Vec<Vec<f64>> = Vec::new();

        for (key, value) in keys.iter().zip(values.iter()) {
            let k = self.project(&self.w_k, key);
            let v = self.project(&self.w_v, value);

            let score: f64 = q
                .iter()
                .zip(k.iter())
                .map(|(&qi, &ki)| qi * ki)
                .sum::<f64>()
                / scale;
            scores.push(score);
            projected_values.push(v);
        }

        // Softmax
        let max_score = scores
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| f64::max(a, b));
        let exp_scores: Vec<f64> = scores.iter().map(|&s| libm::exp(s - max_score)).collect();
        let sum: f64 = exp_scores.iter().sum();
        let attention_weights: Vec<f64> = exp_scores.iter().map(|&e| e / sum).collect();

        // Weighted sum of values
        let mut attended = vec![0.0; self.hidden_dim];
        for (weight, value) in attention_weights.iter().zip(projected_values.iter()) {
            for (a, &v) in attended.iter_mut().zip(value.iter()) {
                *a += weight * v;
            }
        }

        // Output projection
        self.project(&self.w_o, &attended)
    }

    /// Project vector
    fn project(&self, weight: &[Vec<f64>], input: &[f64]) -> Vec<f64> {
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
}

/// Bidirectional cross-modal attention
#[derive(Debug, Clone)]
pub struct BiCrossModalAttention {
    /// Forward attention (A -> B)
    pub forward_attention: CrossModalAttention,
    /// Backward attention (B -> A)
    pub backward_attention: CrossModalAttention,
    /// Gated combination
    pub gate_weight: Vec<f64>,
}

impl BiCrossModalAttention {
    /// Create a new bidirectional attention
    pub fn new(hidden_dim: usize, num_heads: usize, seed: u64) -> Self {
        Self {
            forward_attention: CrossModalAttention::new(hidden_dim, num_heads, seed),
            backward_attention: CrossModalAttention::new(hidden_dim, num_heads, seed + 1000),
            gate_weight: vec![0.5; hidden_dim],
        }
    }

    /// Bidirectional attention between two modalities
    pub fn attend(&self, modality_a: &[f64], modality_b: &[f64]) -> (Vec<f64>, Vec<f64>) {
        // A attends to B
        let a_updated = self
            .forward_attention
            .attend(modality_a, modality_b, modality_b);

        // B attends to A
        let b_updated = self
            .backward_attention
            .attend(modality_b, modality_a, modality_a);

        // Gate combination
        let a_final: Vec<f64> = modality_a
            .iter()
            .zip(a_updated.iter())
            .zip(self.gate_weight.iter())
            .map(|((&a, &a_up), &g)| g * a_up + (1.0 - g) * a)
            .collect();

        let b_final: Vec<f64> = modality_b
            .iter()
            .zip(b_updated.iter())
            .zip(self.gate_weight.iter())
            .map(|((&b, &b_up), &g)| g * b_up + (1.0 - g) * b)
            .collect();

        (a_final, b_final)
    }
}
