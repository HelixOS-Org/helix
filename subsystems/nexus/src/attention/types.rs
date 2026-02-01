//! # Attention Mechanism Types
//!
//! Core types for attention mechanisms in kernel-space.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// TENSOR TYPE
// ============================================================================

/// Simple 2D tensor (matrix)
#[derive(Debug, Clone)]
pub struct Matrix {
    /// Data storage (row-major)
    pub data: Vec<f64>,
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub cols: usize,
}

impl Matrix {
    /// Create new matrix
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            data: alloc::vec![0.0; rows * cols],
            rows,
            cols,
        }
    }

    /// Create from data
    pub fn from_data(rows: usize, cols: usize, data: Vec<f64>) -> Option<Self> {
        if data.len() == rows * cols {
            Some(Self { data, rows, cols })
        } else {
            None
        }
    }

    /// Create identity matrix
    pub fn identity(n: usize) -> Self {
        let mut m = Self::new(n, n);
        for i in 0..n {
            m.set(i, i, 1.0);
        }
        m
    }

    /// Create random matrix
    pub fn random(rows: usize, cols: usize, seed: u64) -> Self {
        let mut data = Vec::with_capacity(rows * cols);
        let mut rng = seed;

        for _ in 0..rows * cols {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let val = (rng as f64 / u64::MAX as f64) * 2.0 - 1.0;
            data.push(val);
        }

        Self { data, rows, cols }
    }

    /// Get element
    pub fn get(&self, row: usize, col: usize) -> f64 {
        if row < self.rows && col < self.cols {
            self.data[row * self.cols + col]
        } else {
            0.0
        }
    }

    /// Set element
    pub fn set(&mut self, row: usize, col: usize, val: f64) {
        if row < self.rows && col < self.cols {
            self.data[row * self.cols + col] = val;
        }
    }

    /// Get row as slice
    pub fn row(&self, idx: usize) -> &[f64] {
        let start = idx * self.cols;
        &self.data[start..start + self.cols]
    }

    /// Get mutable row
    pub fn row_mut(&mut self, idx: usize) -> &mut [f64] {
        let start = idx * self.cols;
        &mut self.data[start..start + self.cols]
    }

    /// Transpose
    pub fn transpose(&self) -> Self {
        let mut result = Matrix::new(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                result.set(j, i, self.get(i, j));
            }
        }
        result
    }

    /// Matrix multiplication
    pub fn matmul(&self, other: &Matrix) -> Option<Matrix> {
        if self.cols != other.rows {
            return None;
        }

        let mut result = Matrix::new(self.rows, other.cols);

        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    sum += self.get(i, k) * other.get(k, j);
                }
                result.set(i, j, sum);
            }
        }

        Some(result)
    }

    /// Element-wise addition
    pub fn add(&self, other: &Matrix) -> Option<Matrix> {
        if self.rows != other.rows || self.cols != other.cols {
            return None;
        }

        let data: Vec<f64> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(&a, &b)| a + b)
            .collect();

        Some(Matrix {
            data,
            rows: self.rows,
            cols: self.cols,
        })
    }

    /// Scale by factor
    pub fn scale(&self, factor: f64) -> Matrix {
        Matrix {
            data: self.data.iter().map(|&x| x * factor).collect(),
            rows: self.rows,
            cols: self.cols,
        }
    }

    /// Apply function element-wise
    pub fn map<F>(&self, f: F) -> Matrix
    where
        F: Fn(f64) -> f64,
    {
        Matrix {
            data: self.data.iter().map(|&x| f(x)).collect(),
            rows: self.rows,
            cols: self.cols,
        }
    }

    /// Softmax along rows
    pub fn softmax_rows(&self) -> Matrix {
        let mut result = Matrix::new(self.rows, self.cols);

        for i in 0..self.rows {
            // Find max for numerical stability
            let mut max_val = f64::NEG_INFINITY;
            for j in 0..self.cols {
                max_val = max_val.max(self.get(i, j));
            }

            // Compute exp and sum
            let mut sum = 0.0;
            for j in 0..self.cols {
                let exp_val = libm::exp(self.get(i, j) - max_val);
                result.set(i, j, exp_val);
                sum += exp_val;
            }

            // Normalize
            if sum > 1e-10 {
                for j in 0..self.cols {
                    result.set(i, j, result.get(i, j) / sum);
                }
            }
        }

        result
    }

    /// L2 norm of entire matrix
    pub fn norm(&self) -> f64 {
        let sq_sum: f64 = self.data.iter().map(|&x| x * x).sum();
        libm::sqrt(sq_sum)
    }

    /// Frobenius norm
    pub fn frobenius_norm(&self) -> f64 {
        self.norm()
    }
}

// ============================================================================
// 3D TENSOR
// ============================================================================

/// 3D tensor for batched operations
#[derive(Debug, Clone)]
pub struct Tensor3 {
    /// Data storage
    pub data: Vec<f64>,
    /// Batch size
    pub batch: usize,
    /// Sequence length (rows)
    pub seq_len: usize,
    /// Hidden dimension (cols)
    pub hidden: usize,
}

impl Tensor3 {
    /// Create new tensor
    pub fn new(batch: usize, seq_len: usize, hidden: usize) -> Self {
        Self {
            data: alloc::vec![0.0; batch * seq_len * hidden],
            batch,
            seq_len,
            hidden,
        }
    }

    /// Get element
    pub fn get(&self, b: usize, s: usize, h: usize) -> f64 {
        if b < self.batch && s < self.seq_len && h < self.hidden {
            self.data[(b * self.seq_len + s) * self.hidden + h]
        } else {
            0.0
        }
    }

    /// Set element
    pub fn set(&mut self, b: usize, s: usize, h: usize, val: f64) {
        if b < self.batch && s < self.seq_len && h < self.hidden {
            self.data[(b * self.seq_len + s) * self.hidden + h] = val;
        }
    }

    /// Get batch slice as matrix
    pub fn batch_matrix(&self, b: usize) -> Matrix {
        let mut m = Matrix::new(self.seq_len, self.hidden);
        for s in 0..self.seq_len {
            for h in 0..self.hidden {
                m.set(s, h, self.get(b, s, h));
            }
        }
        m
    }

    /// Set batch from matrix
    pub fn set_batch(&mut self, b: usize, matrix: &Matrix) {
        for s in 0..self.seq_len.min(matrix.rows) {
            for h in 0..self.hidden.min(matrix.cols) {
                self.set(b, s, h, matrix.get(s, h));
            }
        }
    }
}

// ============================================================================
// ATTENTION MASK
// ============================================================================

/// Attention mask type
#[derive(Debug, Clone)]
pub enum AttentionMask {
    /// No mask
    None,
    /// Padding mask (true = valid, false = padding)
    Padding(Vec<bool>),
    /// Causal mask (lower triangular)
    Causal(usize),
    /// Custom mask matrix
    Custom(Matrix),
}

impl AttentionMask {
    /// Create causal mask
    pub fn causal(seq_len: usize) -> Self {
        AttentionMask::Causal(seq_len)
    }

    /// Create padding mask from lengths
    pub fn from_lengths(lengths: &[usize], max_len: usize) -> Self {
        let mut mask = Vec::with_capacity(lengths.len() * max_len);
        for &len in lengths {
            for i in 0..max_len {
                mask.push(i < len);
            }
        }
        AttentionMask::Padding(mask)
    }

    /// Apply mask to attention scores
    pub fn apply(&self, scores: &mut Matrix) {
        match self {
            AttentionMask::None => {},
            AttentionMask::Padding(mask) => {
                for i in 0..scores.rows {
                    for j in 0..scores.cols {
                        if j < mask.len() && !mask[j] {
                            scores.set(i, j, f64::NEG_INFINITY);
                        }
                    }
                }
            },
            AttentionMask::Causal(len) => {
                for i in 0..scores.rows {
                    for j in 0..scores.cols {
                        if j > i || j >= *len || i >= *len {
                            scores.set(i, j, f64::NEG_INFINITY);
                        }
                    }
                }
            },
            AttentionMask::Custom(mask_matrix) => {
                for i in 0..scores.rows.min(mask_matrix.rows) {
                    for j in 0..scores.cols.min(mask_matrix.cols) {
                        if mask_matrix.get(i, j) == 0.0 {
                            scores.set(i, j, f64::NEG_INFINITY);
                        }
                    }
                }
            },
        }
    }
}

// ============================================================================
// ATTENTION OUTPUT
// ============================================================================

/// Output of attention computation
#[derive(Debug, Clone)]
pub struct AttentionOutput {
    /// Attention output
    pub output: Matrix,
    /// Attention weights (if stored)
    pub weights: Option<Matrix>,
}

impl AttentionOutput {
    /// Create new output
    pub fn new(output: Matrix) -> Self {
        Self {
            output,
            weights: None,
        }
    }

    /// Create with weights
    pub fn with_weights(output: Matrix, weights: Matrix) -> Self {
        Self {
            output,
            weights: Some(weights),
        }
    }
}

// ============================================================================
// LINEAR PROJECTION
// ============================================================================

/// Linear projection layer
#[derive(Debug, Clone)]
pub struct Linear {
    /// Weight matrix
    pub weight: Matrix,
    /// Bias vector (optional)
    pub bias: Option<Vec<f64>>,
}

impl Linear {
    /// Create new linear layer
    pub fn new(in_features: usize, out_features: usize) -> Self {
        // Xavier initialization
        let scale = libm::sqrt(2.0 / (in_features + out_features) as f64);
        let mut weight = Matrix::random(out_features, in_features, 42);
        weight = weight.scale(scale);

        Self { weight, bias: None }
    }

    /// Create with bias
    pub fn with_bias(mut self) -> Self {
        self.bias = Some(alloc::vec![0.0; self.weight.rows]);
        self
    }

    /// Forward pass
    pub fn forward(&self, input: &Matrix) -> Matrix {
        // input: [batch, in_features]
        // weight: [out_features, in_features]
        // output: [batch, out_features]

        let weight_t = self.weight.transpose();
        let mut output = input.matmul(&weight_t).unwrap_or_else(|| Matrix::new(0, 0));

        // Add bias
        if let Some(ref bias) = self.bias {
            for i in 0..output.rows {
                for j in 0..output.cols.min(bias.len()) {
                    output.set(i, j, output.get(i, j) + bias[j]);
                }
            }
        }

        output
    }
}

// ============================================================================
// LAYER NORMALIZATION
// ============================================================================

/// Layer normalization
#[derive(Debug, Clone)]
pub struct LayerNorm {
    /// Normalized shape (last dimension)
    pub normalized_shape: usize,
    /// Epsilon for numerical stability
    pub eps: f64,
    /// Learnable scale (gamma)
    pub gamma: Vec<f64>,
    /// Learnable shift (beta)
    pub beta: Vec<f64>,
}

impl LayerNorm {
    /// Create new layer norm
    pub fn new(normalized_shape: usize) -> Self {
        Self {
            normalized_shape,
            eps: 1e-5,
            gamma: alloc::vec![1.0; normalized_shape],
            beta: alloc::vec![0.0; normalized_shape],
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &Matrix) -> Matrix {
        let mut output = Matrix::new(input.rows, input.cols);

        for i in 0..input.rows {
            // Compute mean
            let mut mean = 0.0;
            for j in 0..input.cols {
                mean += input.get(i, j);
            }
            mean /= input.cols as f64;

            // Compute variance
            let mut var = 0.0;
            for j in 0..input.cols {
                let diff = input.get(i, j) - mean;
                var += diff * diff;
            }
            var /= input.cols as f64;

            // Normalize
            let std = libm::sqrt(var + self.eps);
            for j in 0..input.cols {
                let normalized = (input.get(i, j) - mean) / std;
                let scaled = normalized * self.gamma[j % self.normalized_shape]
                    + self.beta[j % self.normalized_shape];
                output.set(i, j, scaled);
            }
        }

        output
    }
}

// ============================================================================
// DROPOUT
// ============================================================================

/// Dropout layer
#[derive(Debug, Clone)]
pub struct Dropout {
    /// Dropout probability
    pub p: f64,
    /// Training mode
    pub training: bool,
    /// RNG state
    rng_state: u64,
}

impl Dropout {
    /// Create new dropout
    pub fn new(p: f64) -> Self {
        Self {
            p,
            training: true,
            rng_state: 12345,
        }
    }

    /// Set training mode
    pub fn train(&mut self, training: bool) {
        self.training = training;
    }

    /// Forward pass
    pub fn forward(&mut self, input: &Matrix) -> Matrix {
        if !self.training || self.p == 0.0 {
            return input.clone();
        }

        let scale = 1.0 / (1.0 - self.p);
        let mut output = Matrix::new(input.rows, input.cols);

        for i in 0..input.data.len() {
            self.rng_state ^= self.rng_state << 13;
            self.rng_state ^= self.rng_state >> 7;
            self.rng_state ^= self.rng_state << 17;

            let rand = (self.rng_state as f64) / (u64::MAX as f64);

            if rand >= self.p {
                output.data[i] = input.data[i] * scale;
            }
        }

        output
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_basics() {
        let m = Matrix::identity(3);

        assert!((m.get(0, 0) - 1.0).abs() < 1e-10);
        assert!((m.get(0, 1) - 0.0).abs() < 1e-10);
        assert!((m.get(1, 1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_matrix_matmul() {
        let a = Matrix::from_data(2, 3, alloc::vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        let b = Matrix::from_data(3, 2, alloc::vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();

        let c = a.matmul(&b).unwrap();

        assert_eq!(c.rows, 2);
        assert_eq!(c.cols, 2);
        assert!((c.get(0, 0) - 22.0).abs() < 1e-10); // 1*1 + 2*3 + 3*5
    }

    #[test]
    fn test_softmax() {
        let m = Matrix::from_data(1, 3, alloc::vec![1.0, 2.0, 3.0]).unwrap();
        let s = m.softmax_rows();

        // Sum should be 1
        let sum: f64 = (0..3).map(|i| s.get(0, i)).sum();
        assert!((sum - 1.0).abs() < 1e-10);

        // Should be increasing
        assert!(s.get(0, 2) > s.get(0, 1));
        assert!(s.get(0, 1) > s.get(0, 0));
    }

    #[test]
    fn test_linear() {
        let linear = Linear::new(4, 3).with_bias();
        let input = Matrix::random(2, 4, 42);

        let output = linear.forward(&input);

        assert_eq!(output.rows, 2);
        assert_eq!(output.cols, 3);
    }

    #[test]
    fn test_layer_norm() {
        let ln = LayerNorm::new(4);
        let input = Matrix::random(2, 4, 42);

        let output = ln.forward(&input);

        assert_eq!(output.rows, 2);
        assert_eq!(output.cols, 4);

        // Check normalized (mean ≈ 0, std ≈ 1)
        let mut mean = 0.0;
        for j in 0..output.cols {
            mean += output.get(0, j);
        }
        mean /= output.cols as f64;

        assert!(mean.abs() < 1e-10);
    }

    #[test]
    fn test_causal_mask() {
        let mut scores = Matrix::from_data(3, 3, alloc::vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
        ])
        .unwrap();

        let mask = AttentionMask::causal(3);
        mask.apply(&mut scores);

        // Upper triangle should be -inf
        assert!(scores.get(0, 1) == f64::NEG_INFINITY);
        assert!(scores.get(0, 2) == f64::NEG_INFINITY);
        assert!(scores.get(1, 2) == f64::NEG_INFINITY);

        // Lower triangle + diagonal should be unchanged
        assert!((scores.get(0, 0) - 1.0).abs() < 1e-10);
        assert!((scores.get(1, 0) - 4.0).abs() < 1e-10);
        assert!((scores.get(1, 1) - 5.0).abs() < 1e-10);
    }
}
