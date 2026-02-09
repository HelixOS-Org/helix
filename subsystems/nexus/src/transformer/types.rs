//! # Transformer Core Types
//!
//! Foundational types for transformer architectures.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::vec::Vec;

// ============================================================================
// TENSOR TYPES
// ============================================================================

/// 2D Tensor (Matrix)
#[derive(Debug, Clone)]
pub struct Tensor2 {
    /// Data in row-major order
    pub data: Vec<f64>,
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub cols: usize,
}

impl Tensor2 {
    /// Create zero tensor
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            data: alloc::vec![0.0; rows * cols],
            rows,
            cols,
        }
    }

    /// Create from data
    #[inline(always)]
    pub fn from_data(data: Vec<f64>, rows: usize, cols: usize) -> Self {
        assert_eq!(data.len(), rows * cols);
        Self { data, rows, cols }
    }

    /// Create with random values
    pub fn random(rows: usize, cols: usize, seed: u64) -> Self {
        let mut tensor = Self::new(rows, cols);
        let mut state = seed;

        for i in 0..rows * cols {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let u = (state >> 33) as f64 / (1u64 << 31) as f64;
            tensor.data[i] = (u - 0.5) * 2.0 * libm::sqrt(6.0 / (rows + cols) as f64);
        }

        tensor
    }

    /// Get element
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row * self.cols + col]
    }

    /// Set element
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        self.data[row * self.cols + col] = value;
    }

    /// Matrix multiplication
    pub fn matmul(&self, other: &Tensor2) -> Option<Tensor2> {
        if self.cols != other.rows {
            return None;
        }

        let mut result = Tensor2::new(self.rows, other.cols);

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

    /// Transpose
    #[inline]
    pub fn transpose(&self) -> Tensor2 {
        let mut result = Tensor2::new(self.cols, self.rows);

        for i in 0..self.rows {
            for j in 0..self.cols {
                result.set(j, i, self.get(i, j));
            }
        }

        result
    }

    /// Add tensors
    #[inline]
    pub fn add(&self, other: &Tensor2) -> Option<Tensor2> {
        if self.rows != other.rows || self.cols != other.cols {
            return None;
        }

        let mut result = self.clone();
        for i in 0..self.data.len() {
            result.data[i] += other.data[i];
        }
        Some(result)
    }

    /// Scale by scalar
    #[inline]
    pub fn scale(&self, factor: f64) -> Tensor2 {
        let mut result = self.clone();
        for v in &mut result.data {
            *v *= factor;
        }
        result
    }

    /// Apply function element-wise
    #[inline]
    pub fn apply<F: Fn(f64) -> f64>(&self, f: F) -> Tensor2 {
        let mut result = self.clone();
        for v in &mut result.data {
            *v = f(*v);
        }
        result
    }

    /// Row-wise softmax
    pub fn softmax(&self) -> Tensor2 {
        let mut result = Tensor2::new(self.rows, self.cols);

        for i in 0..self.rows {
            let mut max_val = f64::NEG_INFINITY;
            for j in 0..self.cols {
                max_val = max_val.max(self.get(i, j));
            }

            let mut sum = 0.0;
            for j in 0..self.cols {
                let exp_val = libm::exp(self.get(i, j) - max_val);
                result.set(i, j, exp_val);
                sum += exp_val;
            }

            if sum > 1e-10 {
                for j in 0..self.cols {
                    result.set(i, j, result.get(i, j) / sum);
                }
            }
        }

        result
    }
}

/// 3D Tensor for batched operations
#[derive(Debug, Clone)]
pub struct Tensor3 {
    /// Data in batch-major, row-major order
    pub data: Vec<f64>,
    /// Batch size
    pub batch: usize,
    /// Sequence length
    pub seq_len: usize,
    /// Feature dimension
    pub dim: usize,
}

impl Tensor3 {
    /// Create zero tensor
    pub fn new(batch: usize, seq_len: usize, dim: usize) -> Self {
        Self {
            data: alloc::vec![0.0; batch * seq_len * dim],
            batch,
            seq_len,
            dim,
        }
    }

    /// Get element
    #[inline]
    pub fn get(&self, b: usize, s: usize, d: usize) -> f64 {
        self.data[b * self.seq_len * self.dim + s * self.dim + d]
    }

    /// Set element
    #[inline]
    pub fn set(&mut self, b: usize, s: usize, d: usize, value: f64) {
        self.data[b * self.seq_len * self.dim + s * self.dim + d] = value;
    }

    /// Get batch as Tensor2
    #[inline]
    pub fn get_batch(&self, b: usize) -> Tensor2 {
        let mut result = Tensor2::new(self.seq_len, self.dim);
        for s in 0..self.seq_len {
            for d in 0..self.dim {
                result.set(s, d, self.get(b, s, d));
            }
        }
        result
    }

    /// Set batch from Tensor2
    #[inline]
    pub fn set_batch(&mut self, b: usize, tensor: &Tensor2) {
        for s in 0..self.seq_len.min(tensor.rows) {
            for d in 0..self.dim.min(tensor.cols) {
                self.set(b, s, d, tensor.get(s, d));
            }
        }
    }
}

// ============================================================================
// CONFIGURATION TYPES
// ============================================================================

/// Transformer model configuration
#[derive(Debug, Clone)]
pub struct TransformerConfig {
    /// Model dimension (d_model)
    pub d_model: usize,
    /// Number of attention heads
    pub n_heads: usize,
    /// Feedforward dimension
    pub d_ff: usize,
    /// Number of layers
    pub n_layers: usize,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Maximum sequence length
    pub max_seq_len: usize,
    /// Dropout rate
    pub dropout: f64,
    /// Layer norm epsilon
    pub layer_norm_eps: f64,
    /// Whether to use pre-norm (vs post-norm)
    pub pre_norm: bool,
    /// Activation function
    pub activation: ActivationType,
}

impl Default for TransformerConfig {
    fn default() -> Self {
        Self {
            d_model: 512,
            n_heads: 8,
            d_ff: 2048,
            n_layers: 6,
            vocab_size: 32000,
            max_seq_len: 2048,
            dropout: 0.1,
            layer_norm_eps: 1e-5,
            pre_norm: true,
            activation: ActivationType::Gelu,
        }
    }
}

impl TransformerConfig {
    /// Create GPT-style config
    pub fn gpt_small() -> Self {
        Self {
            d_model: 768,
            n_heads: 12,
            d_ff: 3072,
            n_layers: 12,
            vocab_size: 50257,
            max_seq_len: 1024,
            dropout: 0.1,
            layer_norm_eps: 1e-5,
            pre_norm: true,
            activation: ActivationType::Gelu,
        }
    }

    /// Create BERT-style config
    pub fn bert_base() -> Self {
        Self {
            d_model: 768,
            n_heads: 12,
            d_ff: 3072,
            n_layers: 12,
            vocab_size: 30522,
            max_seq_len: 512,
            dropout: 0.1,
            layer_norm_eps: 1e-12,
            pre_norm: false,
            activation: ActivationType::Gelu,
        }
    }

    /// Create small config for testing
    pub fn tiny() -> Self {
        Self {
            d_model: 64,
            n_heads: 2,
            d_ff: 128,
            n_layers: 2,
            vocab_size: 1000,
            max_seq_len: 128,
            dropout: 0.0,
            layer_norm_eps: 1e-5,
            pre_norm: true,
            activation: ActivationType::Relu,
        }
    }

    /// Head dimension
    #[inline(always)]
    pub fn head_dim(&self) -> usize {
        self.d_model / self.n_heads
    }
}

/// Activation function type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivationType {
    /// ReLU
    Relu,
    /// GELU (Gaussian Error Linear Unit)
    Gelu,
    /// SiLU / Swish
    Silu,
    /// Squared ReLU
    ReluSquared,
    /// GLU (Gated Linear Unit)
    Glu,
}

impl ActivationType {
    /// Apply activation function
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            ActivationType::Relu => x.max(0.0),
            ActivationType::Gelu => {
                // GELU approximation
                0.5 * x
                    * (1.0
                        + libm::tanh(
                            libm::sqrt(2.0 / core::f64::consts::PI) * (x + 0.044715 * x * x * x),
                        ))
            },
            ActivationType::Silu => x / (1.0 + libm::exp(-x)),
            ActivationType::ReluSquared => {
                let relu = x.max(0.0);
                relu * relu
            },
            ActivationType::Glu => x, // GLU needs paired input
        }
    }
}

// ============================================================================
// LAYER TYPES
// ============================================================================

/// Linear layer
#[derive(Debug, Clone)]
pub struct Linear {
    /// Weight matrix (out_features x in_features)
    pub weight: Tensor2,
    /// Bias vector (out_features)
    pub bias: Option<Vec<f64>>,
}

impl Linear {
    /// Create new linear layer
    pub fn new(in_features: usize, out_features: usize, use_bias: bool, seed: u64) -> Self {
        let weight = Tensor2::random(out_features, in_features, seed);
        let bias = if use_bias {
            Some(alloc::vec![0.0; out_features])
        } else {
            None
        };

        Self { weight, bias }
    }

    /// Forward pass
    pub fn forward(&self, input: &Tensor2) -> Tensor2 {
        // input: (seq_len, in_features)
        // weight: (out_features, in_features)
        // output: (seq_len, out_features)

        let weight_t = self.weight.transpose();
        let mut output = input
            .matmul(&weight_t)
            .unwrap_or_else(|| Tensor2::new(input.rows, self.weight.rows));

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

/// Layer normalization
#[derive(Debug, Clone)]
pub struct LayerNorm {
    /// Normalized shape (feature dimension)
    pub normalized_shape: usize,
    /// Gamma (scale) parameter
    pub gamma: Vec<f64>,
    /// Beta (shift) parameter
    pub beta: Vec<f64>,
    /// Epsilon for numerical stability
    pub eps: f64,
}

impl LayerNorm {
    /// Create new layer norm
    pub fn new(normalized_shape: usize, eps: f64) -> Self {
        Self {
            normalized_shape,
            gamma: alloc::vec![1.0; normalized_shape],
            beta: alloc::vec![0.0; normalized_shape],
            eps,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &Tensor2) -> Tensor2 {
        let mut output = Tensor2::new(input.rows, input.cols);

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

            // Normalize and scale
            let std = libm::sqrt(var + self.eps);
            for j in 0..input.cols.min(self.normalized_shape) {
                let normalized = (input.get(i, j) - mean) / std;
                output.set(i, j, normalized * self.gamma[j] + self.beta[j]);
            }
        }

        output
    }
}

/// RMS normalization (used in LLaMA)
#[derive(Debug, Clone)]
pub struct RMSNorm {
    /// Feature dimension
    pub dim: usize,
    /// Scale parameter
    pub weight: Vec<f64>,
    /// Epsilon
    pub eps: f64,
}

impl RMSNorm {
    /// Create new RMS norm
    pub fn new(dim: usize, eps: f64) -> Self {
        Self {
            dim,
            weight: alloc::vec![1.0; dim],
            eps,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &Tensor2) -> Tensor2 {
        let mut output = Tensor2::new(input.rows, input.cols);

        for i in 0..input.rows {
            // Compute RMS
            let mut sum_sq = 0.0;
            for j in 0..input.cols {
                sum_sq += input.get(i, j) * input.get(i, j);
            }
            let rms = libm::sqrt(sum_sq / input.cols as f64 + self.eps);

            // Normalize and scale
            for j in 0..input.cols.min(self.dim) {
                output.set(i, j, input.get(i, j) / rms * self.weight[j]);
            }
        }

        output
    }
}

/// Dropout layer
#[derive(Debug, Clone)]
pub struct Dropout {
    /// Dropout probability
    pub p: f64,
    /// Random state
    pub seed: u64,
}

impl Dropout {
    /// Create new dropout
    pub fn new(p: f64) -> Self {
        Self { p, seed: 42 }
    }

    /// Forward pass (training mode)
    pub fn forward(&mut self, input: &Tensor2, training: bool) -> Tensor2 {
        if !training || self.p == 0.0 {
            return input.clone();
        }

        let scale = 1.0 / (1.0 - self.p);
        let mut output = Tensor2::new(input.rows, input.cols);

        for i in 0..input.rows {
            for j in 0..input.cols {
                self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let r = (self.seed >> 33) as f64 / (1u64 << 31) as f64;

                if r > self.p {
                    output.set(i, j, input.get(i, j) * scale);
                }
            }
        }

        output
    }
}

// ============================================================================
// EMBEDDING TYPES
// ============================================================================

/// Token embedding layer
#[derive(Debug, Clone)]
pub struct Embedding {
    /// Embedding weight (vocab_size x d_model)
    pub weight: Tensor2,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Embedding dimension
    pub d_model: usize,
}

impl Embedding {
    /// Create new embedding
    pub fn new(vocab_size: usize, d_model: usize, seed: u64) -> Self {
        let weight = Tensor2::random(vocab_size, d_model, seed);
        Self {
            weight,
            vocab_size,
            d_model,
        }
    }

    /// Forward pass
    pub fn forward(&self, indices: &[usize]) -> Tensor2 {
        let seq_len = indices.len();
        let mut output = Tensor2::new(seq_len, self.d_model);

        for (i, &idx) in indices.iter().enumerate() {
            if idx < self.vocab_size {
                for j in 0..self.d_model {
                    output.set(i, j, self.weight.get(idx, j));
                }
            }
        }

        output
    }
}

/// Positional embedding
#[derive(Debug, Clone)]
pub struct PositionalEmbedding {
    /// Positional encoding matrix
    pub encoding: Tensor2,
    /// Maximum sequence length
    pub max_seq_len: usize,
    /// Embedding dimension
    pub d_model: usize,
}

impl PositionalEmbedding {
    /// Create sinusoidal positional embedding
    pub fn sinusoidal(max_seq_len: usize, d_model: usize) -> Self {
        let mut encoding = Tensor2::new(max_seq_len, d_model);

        for pos in 0..max_seq_len {
            for i in 0..d_model / 2 {
                let angle = pos as f64 / libm::pow(10000.0, 2.0 * i as f64 / d_model as f64);
                encoding.set(pos, 2 * i, libm::sin(angle));
                encoding.set(pos, 2 * i + 1, libm::cos(angle));
            }
        }

        Self {
            encoding,
            max_seq_len,
            d_model,
        }
    }

    /// Create learnable positional embedding
    #[inline]
    pub fn learnable(max_seq_len: usize, d_model: usize, seed: u64) -> Self {
        let encoding = Tensor2::random(max_seq_len, d_model, seed);
        Self {
            encoding,
            max_seq_len,
            d_model,
        }
    }

    /// Forward pass
    pub fn forward(&self, seq_len: usize) -> Tensor2 {
        let actual_len = seq_len.min(self.max_seq_len);
        let mut output = Tensor2::new(actual_len, self.d_model);

        for i in 0..actual_len {
            for j in 0..self.d_model {
                output.set(i, j, self.encoding.get(i, j));
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
    fn test_tensor2() {
        let t = Tensor2::new(3, 4);
        assert_eq!(t.rows, 3);
        assert_eq!(t.cols, 4);
        assert_eq!(t.data.len(), 12);
    }

    #[test]
    fn test_tensor2_matmul() {
        let a = Tensor2::random(3, 4, 42);
        let b = Tensor2::random(4, 5, 43);

        let c = a.matmul(&b).unwrap();

        assert_eq!(c.rows, 3);
        assert_eq!(c.cols, 5);
    }

    #[test]
    fn test_config() {
        let config = TransformerConfig::default();

        assert_eq!(config.d_model, 512);
        assert_eq!(config.head_dim(), 64);
    }

    #[test]
    fn test_activation() {
        assert!(ActivationType::Relu.apply(-1.0) == 0.0);
        assert!(ActivationType::Relu.apply(1.0) == 1.0);

        let gelu_0 = ActivationType::Gelu.apply(0.0);
        assert!(gelu_0.abs() < 0.01);
    }

    #[test]
    fn test_linear() {
        let linear = Linear::new(64, 128, true, 42);
        let input = Tensor2::random(10, 64, 43);

        let output = linear.forward(&input);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 128);
    }

    #[test]
    fn test_layer_norm() {
        let ln = LayerNorm::new(64, 1e-5);
        let input = Tensor2::random(10, 64, 42);

        let output = ln.forward(&input);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 64);

        // Check that output is roughly normalized
        let mut sum = 0.0;
        for j in 0..64 {
            sum += output.get(0, j);
        }
        let mean = sum / 64.0;
        assert!(mean.abs() < 0.1);
    }

    #[test]
    fn test_embedding() {
        let embed = Embedding::new(1000, 64, 42);
        let indices = alloc::vec![0, 5, 10, 15];

        let output = embed.forward(&indices);

        assert_eq!(output.rows, 4);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_positional_embedding() {
        let pos = PositionalEmbedding::sinusoidal(128, 64);

        let output = pos.forward(32);

        assert_eq!(output.rows, 32);
        assert_eq!(output.cols, 64);
    }
}
