//! # Linear Attention
//!
//! O(n) complexity attention mechanisms using kernel feature maps.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::vec::Vec;

use super::types::Matrix;

// ============================================================================
// KERNEL FEATURE MAPS
// ============================================================================

/// Feature map types for linearizing attention
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FeatureMapType {
    /// ELU + 1 feature map
    Elu,
    /// Random Fourier Features
    RandomFourier,
    /// Positive Random Features (Performer)
    PositiveRandom,
    /// Taylor expansion
    Taylor,
    /// Favor+ (Fast Attention Via positive Orthogonal Random features)
    FavorPlus,
}

/// Feature map for kernel approximation
pub struct FeatureMap {
    /// Feature map type
    pub map_type: FeatureMapType,
    /// Input dimension
    pub input_dim: usize,
    /// Output (feature) dimension
    pub feature_dim: usize,
    /// Random projection matrix (if needed)
    random_matrix: Option<Matrix>,
}

impl FeatureMap {
    /// Create ELU feature map
    #[inline]
    pub fn elu(dim: usize) -> Self {
        Self {
            map_type: FeatureMapType::Elu,
            input_dim: dim,
            feature_dim: dim,
            random_matrix: None,
        }
    }

    /// Create random Fourier features
    #[inline]
    pub fn random_fourier(input_dim: usize, feature_dim: usize, seed: u64) -> Self {
        let random_matrix = Matrix::random(feature_dim, input_dim, seed);
        Self {
            map_type: FeatureMapType::RandomFourier,
            input_dim,
            feature_dim: feature_dim * 2, // cos and sin
            random_matrix: Some(random_matrix),
        }
    }

    /// Create positive random features (Performer)
    #[inline]
    pub fn positive_random(input_dim: usize, feature_dim: usize, seed: u64) -> Self {
        let random_matrix = Matrix::random(feature_dim, input_dim, seed);
        Self {
            map_type: FeatureMapType::PositiveRandom,
            input_dim,
            feature_dim,
            random_matrix: Some(random_matrix),
        }
    }

    /// Apply feature map to input
    #[inline]
    pub fn apply(&self, x: &Matrix) -> Matrix {
        match self.map_type {
            FeatureMapType::Elu => self.apply_elu(x),
            FeatureMapType::RandomFourier => self.apply_rff(x),
            FeatureMapType::PositiveRandom => self.apply_positive_random(x),
            FeatureMapType::Taylor => self.apply_taylor(x),
            FeatureMapType::FavorPlus => self.apply_favor_plus(x),
        }
    }

    /// ELU + 1 feature map
    fn apply_elu(&self, x: &Matrix) -> Matrix {
        let mut result = Matrix::new(x.rows, x.cols);

        for i in 0..x.rows {
            for j in 0..x.cols {
                let v = x.get(i, j);
                // elu(x) + 1 = max(0, x) + 1 for positive x, exp(x) for negative
                let mapped = if v >= 0.0 { v + 1.0 } else { libm::exp(v) };
                result.set(i, j, mapped);
            }
        }

        result
    }

    /// Random Fourier Features
    fn apply_rff(&self, x: &Matrix) -> Matrix {
        let proj = self.random_matrix.as_ref().unwrap();
        let base_features = proj.rows;

        let mut result = Matrix::new(x.rows, base_features * 2);

        for i in 0..x.rows {
            for f in 0..base_features {
                // Compute projection
                let mut z = 0.0;
                for j in 0..x.cols.min(proj.cols) {
                    z += x.get(i, j) * proj.get(f, j);
                }

                // cos and sin features
                let scale = libm::sqrt(1.0 / base_features as f64);
                result.set(i, f, libm::cos(z) * scale);
                result.set(i, f + base_features, libm::sin(z) * scale);
            }
        }

        result
    }

    /// Positive random features (Performer)
    fn apply_positive_random(&self, x: &Matrix) -> Matrix {
        let proj = self.random_matrix.as_ref().unwrap();

        let mut result = Matrix::new(x.rows, proj.rows);

        for i in 0..x.rows {
            // Compute x norm squared
            let mut x_norm_sq = 0.0;
            for j in 0..x.cols {
                x_norm_sq += x.get(i, j) * x.get(i, j);
            }

            for f in 0..proj.rows {
                // Compute projection
                let mut z = 0.0;
                for j in 0..x.cols.min(proj.cols) {
                    z += x.get(i, j) * proj.get(f, j);
                }

                // Positive random feature: exp(wTx - x^2/2)
                let feature = libm::exp(z - x_norm_sq / 2.0);
                let scale = libm::sqrt(1.0 / proj.rows as f64);
                result.set(i, f, feature * scale);
            }
        }

        result
    }

    /// Taylor expansion
    fn apply_taylor(&self, x: &Matrix) -> Matrix {
        // Second-order Taylor: [1, x, x^2/sqrt(2)]
        let mut result = Matrix::new(x.rows, 1 + x.cols + x.cols);

        for i in 0..x.rows {
            result.set(i, 0, 1.0);

            for j in 0..x.cols {
                let v = x.get(i, j);
                result.set(i, 1 + j, v);
                result.set(i, 1 + x.cols + j, v * v / libm::sqrt(2.0));
            }
        }

        result
    }

    /// Favor+ features
    fn apply_favor_plus(&self, x: &Matrix) -> Matrix {
        // Similar to positive random but with orthogonal features
        self.apply_positive_random(x)
    }
}

// ============================================================================
// LINEAR ATTENTION
// ============================================================================

/// Linear attention with O(n) complexity
pub struct LinearAttention {
    /// Feature map for queries
    query_feature_map: FeatureMap,
    /// Feature map for keys
    key_feature_map: FeatureMap,
    /// Epsilon for numerical stability
    eps: f64,
}

impl LinearAttention {
    /// Create linear attention with ELU features
    pub fn new(dim: usize) -> Self {
        Self {
            query_feature_map: FeatureMap::elu(dim),
            key_feature_map: FeatureMap::elu(dim),
            eps: 1e-6,
        }
    }

    /// Create with custom feature maps
    #[inline]
    pub fn with_feature_maps(query_map: FeatureMap, key_map: FeatureMap) -> Self {
        Self {
            query_feature_map: query_map,
            key_feature_map: key_map,
            eps: 1e-6,
        }
    }

    /// Forward pass
    ///
    /// Instead of computing softmax(QK^T)V which is O(n^2),
    /// we compute φ(Q)(φ(K)^T V) which is O(n).
    pub fn forward(&self, query: &Matrix, key: &Matrix, value: &Matrix) -> Matrix {
        // Apply feature maps
        let q_features = self.query_feature_map.apply(query);
        let k_features = self.key_feature_map.apply(key);

        // Compute K^T V: (feature_dim, seq_len) @ (seq_len, value_dim) = (feature_dim, value_dim)
        let k_t = k_features.transpose();
        let kv = k_t
            .matmul(value)
            .unwrap_or_else(|| Matrix::new(k_t.rows, value.cols));

        // Compute Q(KV): (seq_len, feature_dim) @ (feature_dim, value_dim) = (seq_len, value_dim)
        let qkv = q_features
            .matmul(&kv)
            .unwrap_or_else(|| Matrix::new(query.rows, value.cols));

        // Compute normalizer: sum of K features per position
        let mut k_sum = alloc::vec![0.0; k_features.cols];
        for i in 0..k_features.rows {
            for j in 0..k_features.cols {
                k_sum[j] += k_features.get(i, j);
            }
        }

        // Normalize output
        let mut output = Matrix::new(query.rows, value.cols);
        for i in 0..query.rows {
            // Compute normalizer for this query
            let mut norm = 0.0;
            for j in 0..q_features.cols.min(k_sum.len()) {
                norm += q_features.get(i, j) * k_sum[j];
            }
            norm = norm.max(self.eps);

            for j in 0..value.cols {
                output.set(i, j, qkv.get(i, j) / norm);
            }
        }

        output
    }

    /// Causal forward pass
    ///
    /// For causal attention, we use cumulative sums instead of full sums.
    pub fn forward_causal(&self, query: &Matrix, key: &Matrix, value: &Matrix) -> Matrix {
        let q_features = self.query_feature_map.apply(query);
        let k_features = self.key_feature_map.apply(key);

        let seq_len = query.rows;
        let feature_dim = q_features.cols;
        let value_dim = value.cols;

        // Cumulative KV and K sum
        let mut kv_cumsum = Matrix::new(feature_dim, value_dim);
        let mut k_cumsum = alloc::vec![0.0; feature_dim];

        let mut output = Matrix::new(seq_len, value_dim);

        for i in 0..seq_len {
            // Update cumulative sums
            for f in 0..feature_dim {
                let k_f = k_features.get(i, f);
                k_cumsum[f] += k_f;

                for v in 0..value_dim {
                    let kv_val = kv_cumsum.get(f, v) + k_f * value.get(i, v);
                    kv_cumsum.set(f, v, kv_val);
                }
            }

            // Compute output for position i
            let mut norm = 0.0;
            for f in 0..feature_dim {
                norm += q_features.get(i, f) * k_cumsum[f];
            }
            norm = norm.max(self.eps);

            for v in 0..value_dim {
                let mut out = 0.0;
                for f in 0..feature_dim {
                    out += q_features.get(i, f) * kv_cumsum.get(f, v);
                }
                output.set(i, v, out / norm);
            }
        }

        output
    }
}

// ============================================================================
// PERFORMER
// ============================================================================

/// Performer attention using FAVOR+ mechanism
pub struct Performer {
    /// Feature dimension
    feature_dim: usize,
    /// Head dimension
    head_dim: usize,
    /// Feature map
    feature_map: FeatureMap,
}

impl Performer {
    /// Create new Performer
    pub fn new(head_dim: usize, feature_dim: usize, seed: u64) -> Self {
        Self {
            feature_dim,
            head_dim,
            feature_map: FeatureMap::positive_random(head_dim, feature_dim, seed),
        }
    }

    /// Forward pass with FAVOR+
    pub fn forward(&self, query: &Matrix, key: &Matrix, value: &Matrix) -> Matrix {
        let q_prime = self.feature_map.apply(query);
        let k_prime = self.feature_map.apply(key);

        // D^-1 (Q' (K'^T V))
        // where D = diag(Q' K'^T 1)

        let k_t = k_prime.transpose();
        let kv = k_t
            .matmul(value)
            .unwrap_or_else(|| Matrix::new(self.feature_dim, value.cols));

        let qkv = q_prime
            .matmul(&kv)
            .unwrap_or_else(|| Matrix::new(query.rows, value.cols));

        // Normalizer
        let mut k_sum = alloc::vec![0.0; self.feature_dim];
        for i in 0..k_prime.rows {
            for j in 0..self.feature_dim {
                k_sum[j] += k_prime.get(i, j);
            }
        }

        let mut output = Matrix::new(query.rows, value.cols);
        for i in 0..query.rows {
            let mut d = 0.0;
            for j in 0..self.feature_dim {
                d += q_prime.get(i, j) * k_sum[j];
            }
            d = d.max(1e-6);

            for j in 0..value.cols {
                output.set(i, j, qkv.get(i, j) / d);
            }
        }

        output
    }

    /// Causal forward pass
    pub fn forward_causal(&self, query: &Matrix, key: &Matrix, value: &Matrix) -> Matrix {
        let q_prime = self.feature_map.apply(query);
        let k_prime = self.feature_map.apply(key);

        let seq_len = query.rows;
        let value_dim = value.cols;

        let mut s_matrix = Matrix::new(self.feature_dim, value_dim);
        let mut z_vec = alloc::vec![0.0; self.feature_dim];

        let mut output = Matrix::new(seq_len, value_dim);

        for i in 0..seq_len {
            // Update S and z
            for f in 0..self.feature_dim {
                let k_f = k_prime.get(i, f);
                z_vec[f] += k_f;

                for v in 0..value_dim {
                    let s_val = s_matrix.get(f, v) + k_f * value.get(i, v);
                    s_matrix.set(f, v, s_val);
                }
            }

            // Compute output
            let mut d = 0.0;
            for f in 0..self.feature_dim {
                d += q_prime.get(i, f) * z_vec[f];
            }
            d = d.max(1e-6);

            for v in 0..value_dim {
                let mut out = 0.0;
                for f in 0..self.feature_dim {
                    out += q_prime.get(i, f) * s_matrix.get(f, v);
                }
                output.set(i, v, out / d);
            }
        }

        output
    }
}

// ============================================================================
// RNN-LIKE LINEAR ATTENTION
// ============================================================================

/// Linear attention as RNN state
pub struct LinearAttentionRNN {
    /// Feature map
    feature_map: FeatureMap,
    /// Hidden state: S matrix
    s_matrix: Matrix,
    /// Normalizer: z vector
    z_vec: Vec<f64>,
    /// Feature dimension
    feature_dim: usize,
    /// Value dimension
    value_dim: usize,
}

impl LinearAttentionRNN {
    /// Create new RNN state
    pub fn new(head_dim: usize, feature_dim: usize, value_dim: usize) -> Self {
        Self {
            feature_map: FeatureMap::elu(head_dim),
            s_matrix: Matrix::new(feature_dim, value_dim),
            z_vec: alloc::vec![0.0; feature_dim],
            feature_dim,
            value_dim,
        }
    }

    /// Process single step
    pub fn step(&mut self, query: &[f64], key: &[f64], value: &[f64]) -> Vec<f64> {
        // Create single-row matrices for feature mapping
        let mut q_mat = Matrix::new(1, query.len());
        let mut k_mat = Matrix::new(1, key.len());

        for (j, &v) in query.iter().enumerate() {
            q_mat.set(0, j, v);
        }
        for (j, &v) in key.iter().enumerate() {
            k_mat.set(0, j, v);
        }

        let q_features = self.feature_map.apply(&q_mat);
        let k_features = self.feature_map.apply(&k_mat);

        // Update state
        for f in 0..self.feature_dim.min(k_features.cols) {
            let k_f = k_features.get(0, f);
            self.z_vec[f] += k_f;

            for v in 0..self.value_dim.min(value.len()) {
                let s_val = self.s_matrix.get(f, v) + k_f * value[v];
                self.s_matrix.set(f, v, s_val);
            }
        }

        // Compute output
        let mut d = 0.0;
        for f in 0..self.feature_dim.min(q_features.cols) {
            d += q_features.get(0, f) * self.z_vec[f];
        }
        d = d.max(1e-6);

        let mut output = alloc::vec![0.0; self.value_dim];
        for v in 0..self.value_dim {
            let mut out = 0.0;
            for f in 0..self.feature_dim.min(q_features.cols) {
                out += q_features.get(0, f) * self.s_matrix.get(f, v);
            }
            output[v] = out / d;
        }

        output
    }

    /// Reset state
    #[inline(always)]
    pub fn reset(&mut self) {
        self.s_matrix = Matrix::new(self.feature_dim, self.value_dim);
        self.z_vec = alloc::vec![0.0; self.feature_dim];
    }

    /// Get current S matrix
    #[inline(always)]
    pub fn get_s_matrix(&self) -> &Matrix {
        &self.s_matrix
    }

    /// Get current z vector
    #[inline(always)]
    pub fn get_z_vec(&self) -> &[f64] {
        &self.z_vec
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elu_feature_map() {
        let fm = FeatureMap::elu(32);

        let x = Matrix::random(10, 32, 42);
        let features = fm.apply(&x);

        assert_eq!(features.rows, 10);
        assert_eq!(features.cols, 32);

        // ELU + 1 is always positive
        for i in 0..features.rows {
            for j in 0..features.cols {
                assert!(features.get(i, j) > 0.0);
            }
        }
    }

    #[test]
    fn test_linear_attention() {
        let attn = LinearAttention::new(32);

        let q = Matrix::random(16, 32, 42);
        let k = Matrix::random(16, 32, 43);
        let v = Matrix::random(16, 32, 44);

        let output = attn.forward(&q, &k, &v);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 32);
    }

    #[test]
    fn test_linear_attention_causal() {
        let attn = LinearAttention::new(32);

        let q = Matrix::random(16, 32, 42);
        let k = Matrix::random(16, 32, 43);
        let v = Matrix::random(16, 32, 44);

        let output = attn.forward_causal(&q, &k, &v);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 32);
    }

    #[test]
    fn test_performer() {
        let performer = Performer::new(32, 64, 42);

        let q = Matrix::random(16, 32, 42);
        let k = Matrix::random(16, 32, 43);
        let v = Matrix::random(16, 32, 44);

        let output = performer.forward(&q, &k, &v);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 32);
    }

    #[test]
    fn test_performer_causal() {
        let performer = Performer::new(32, 64, 42);

        let q = Matrix::random(16, 32, 42);
        let k = Matrix::random(16, 32, 43);
        let v = Matrix::random(16, 32, 44);

        let output = performer.forward_causal(&q, &k, &v);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 32);
    }

    #[test]
    fn test_linear_attention_rnn() {
        let mut rnn = LinearAttentionRNN::new(32, 32, 32);

        let query = alloc::vec![0.5; 32];
        let key = alloc::vec![0.3; 32];
        let value = alloc::vec![1.0; 32];

        // Process multiple steps
        let _out1 = rnn.step(&query, &key, &value);
        let out2 = rnn.step(&query, &key, &value);

        assert_eq!(out2.len(), 32);
    }
}
