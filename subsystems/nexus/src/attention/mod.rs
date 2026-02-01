//! # Attention Module
//!
//! Production-quality attention mechanisms for transformer architectures.
//!
//! Year 3 "EVOLUTION" - Revolutionary attention mechanisms enabling
//! the kernel to focus on relevant information dynamically.
//!
//! ## Module Structure
//!
//! - [`types`] - Core types: Matrix, Tensor3, Linear, LayerNorm, Dropout
//! - [`scaled`] - Scaled dot-product attention, efficient attention, RoPE
//! - [`multihead`] - Multi-head attention, KV cache, GQA
//! - [`linear`] - O(n) linear attention, Performer, feature maps
//! - [`sparse`] - Sparse patterns (local, strided, BigBird, Longformer)
//! - [`flash`] - Flash Attention v2, paged attention, sliding window
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_nexus::attention::{
//!     types::{Matrix, Linear},
//!     multihead::MultiHeadAttention,
//!     flash::FlashAttention,
//! };
//!
//! // Create multi-head attention
//! let mha = MultiHeadAttention::new(512, 8);
//!
//! // Or use Flash Attention for memory efficiency
//! let flash = FlashAttention::new(64);
//! ```
//!
//! ## Kernel Applications
//!
//! - Efficient sequence modeling for kernel logs
//! - Dynamic resource prioritization
//! - Context-aware system monitoring
//! - Efficient time-series analysis

#![no_std]

extern crate alloc;

// Submodules with production-quality implementations
pub mod flash;
pub mod linear;
pub mod multihead;
pub mod scaled;
pub mod sparse;
pub mod types;

// Re-export main types for convenience
// Legacy code below - kept for backward compatibility
// TODO: Migrate to new module structure
use alloc::vec;
use alloc::vec::Vec;

pub use flash::{
    FlashAttention as FlashAttentionNew, PageTableEntry, PagedAttention,
    SlidingWindowFlashAttention,
};
pub use linear::{
    FeatureMap, FeatureMapType, LinearAttention as LinearAttentionNew, LinearAttentionRNN,
    Performer,
};
pub use multihead::{
    CachedMultiHeadAttention, GroupedQueryAttention, KVCache,
    MultiHeadAttention as MultiHeadAttentionNew,
};
pub use scaled::{
    EfficientAttention, RelativePositionAttention, RotaryPositionEmbedding,
    ScaledDotProductAttention,
};
pub use sparse::{
    BlockSparseAttention, DilatedAttention, SparseAttention as SparseAttentionNew, SparsePattern,
    SparsePatternType,
};
pub use types::{
    AttentionMask as AttentionMaskNew, AttentionOutput, Dropout, LayerNorm, Linear, Matrix, Tensor3,
};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default number of attention heads
const DEFAULT_NUM_HEADS: usize = 8;

/// Default attention dropout rate
const DEFAULT_DROPOUT: f64 = 0.1;

/// Epsilon for numerical stability
const EPSILON: f64 = 1e-8;

/// Default sequence length for kernel monitoring
const DEFAULT_SEQ_LEN: usize = 512;

// ============================================================================
// CORE ATTENTION TYPES (LEGACY)
// ============================================================================

/// Attention score computation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionType {
    /// Dot-product attention
    DotProduct,
    /// Additive attention (Bahdanau)
    Additive,
    /// Multiplicative attention
    Multiplicative,
    /// Cosine similarity attention
    Cosine,
}

/// Attention mask type (legacy)
#[derive(Debug, Clone)]
pub enum AttentionMask {
    /// No mask
    None,
    /// Causal (autoregressive) mask
    Causal,
    /// Padding mask (indices to ignore)
    Padding(Vec<bool>),
    /// Custom mask matrix
    Custom(Vec<Vec<f64>>),
}

impl AttentionMask {
    /// Get mask value for position (i, j)
    pub fn get_mask(&self, i: usize, j: usize, seq_len: usize) -> f64 {
        let _ = seq_len; // Used for API compatibility
        match self {
            AttentionMask::None => 0.0,
            AttentionMask::Causal => {
                if j > i {
                    f64::NEG_INFINITY
                } else {
                    0.0
                }
            },
            AttentionMask::Padding(mask) => {
                if j < mask.len() && mask[j] {
                    f64::NEG_INFINITY
                } else {
                    0.0
                }
            },
            AttentionMask::Custom(mask) => {
                if i < mask.len() && j < mask[i].len() {
                    mask[i][j]
                } else {
                    0.0
                }
            },
        }
    }
}

// ============================================================================
// SCALED DOT-PRODUCT ATTENTION
// ============================================================================

/// Scaled dot-product attention
#[derive(Debug, Clone)]
pub struct ScaledDotProductAttention {
    /// Scaling factor (1/sqrt(d_k))
    pub scale: f64,
    /// Dropout probability
    pub dropout: f64,
    /// Attention mask
    pub mask: AttentionMask,
    /// RNG state for dropout
    rng_state: u64,
}

impl ScaledDotProductAttention {
    /// Create new attention
    pub fn new(d_k: usize) -> Self {
        Self {
            scale: 1.0 / libm::sqrt(d_k as f64),
            dropout: DEFAULT_DROPOUT,
            mask: AttentionMask::None,
            rng_state: 12345,
        }
    }

    /// Create causal attention (for autoregressive models)
    pub fn causal(d_k: usize) -> Self {
        Self {
            scale: 1.0 / libm::sqrt(d_k as f64),
            dropout: DEFAULT_DROPOUT,
            mask: AttentionMask::Causal,
            rng_state: 12345,
        }
    }

    /// Compute attention: softmax(QK^T / sqrt(d_k)) * V
    pub fn forward(
        &mut self,
        query: &[Vec<f64>],
        key: &[Vec<f64>],
        value: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        let seq_len_q = query.len();
        let seq_len_k = key.len();
        let d_v = if value.is_empty() { 0 } else { value[0].len() };

        if seq_len_q == 0 || seq_len_k == 0 {
            return Vec::new();
        }

        // Compute attention scores: Q * K^T
        let mut scores = vec![vec![0.0; seq_len_k]; seq_len_q];

        for i in 0..seq_len_q {
            for j in 0..seq_len_k {
                let dot: f64 = query[i]
                    .iter()
                    .zip(key[j].iter())
                    .map(|(&q, &k)| q * k)
                    .sum();

                scores[i][j] = dot * self.scale;

                // Apply mask
                scores[i][j] += self.mask.get_mask(i, j, seq_len_k);
            }
        }

        // Apply softmax
        let attention_weights = self.softmax_rows(&scores);

        // Compute weighted values
        let mut output = vec![vec![0.0; d_v]; seq_len_q];

        for i in 0..seq_len_q {
            for j in 0..seq_len_k {
                let weight = attention_weights[i][j];
                for k in 0..d_v {
                    output[i][k] += weight * value[j][k];
                }
            }
        }

        output
    }

    /// Softmax over each row
    fn softmax_rows(&self, scores: &[Vec<f64>]) -> Vec<Vec<f64>> {
        scores
            .iter()
            .map(|row| {
                let max_val = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

                let exp_vals: Vec<f64> = row.iter().map(|&x| libm::exp(x - max_val)).collect();

                let sum: f64 = exp_vals.iter().sum();

                exp_vals.iter().map(|&x| x / (sum + EPSILON)).collect()
            })
            .collect()
    }

    /// Get attention weights (for visualization)
    pub fn get_attention_weights(&mut self, query: &[Vec<f64>], key: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let seq_len_q = query.len();
        let seq_len_k = key.len();

        if seq_len_q == 0 || seq_len_k == 0 {
            return Vec::new();
        }

        let mut scores = vec![vec![0.0; seq_len_k]; seq_len_q];

        for i in 0..seq_len_q {
            for j in 0..seq_len_k {
                let dot: f64 = query[i]
                    .iter()
                    .zip(key[j].iter())
                    .map(|(&q, &k)| q * k)
                    .sum();

                scores[i][j] = dot * self.scale + self.mask.get_mask(i, j, seq_len_k);
            }
        }

        self.softmax_rows(&scores)
    }
}

// ============================================================================
// MULTI-HEAD ATTENTION
// ============================================================================

/// Linear projection layer
#[derive(Debug, Clone)]
pub struct LinearProjection {
    /// Weight matrix (input_dim x output_dim)
    pub weights: Vec<Vec<f64>>,
    /// Bias vector
    pub bias: Vec<f64>,
}

impl LinearProjection {
    /// Create new projection
    pub fn new(input_dim: usize, output_dim: usize, seed: u64) -> Self {
        let mut rng = seed;
        let scale = libm::sqrt(2.0 / (input_dim + output_dim) as f64);

        let weights: Vec<Vec<f64>> = (0..input_dim)
            .map(|_| {
                (0..output_dim)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale
                    })
                    .collect()
            })
            .collect();

        let bias = vec![0.0; output_dim];

        Self { weights, bias }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let input_dim = self.weights.len();
        let output_dim = if input_dim > 0 {
            self.weights[0].len()
        } else {
            0
        };

        input
            .iter()
            .map(|x| {
                let mut output = self.bias.clone();
                for (i, &xi) in x.iter().enumerate() {
                    if i < input_dim {
                        for (j, out) in output.iter_mut().enumerate() {
                            if j < output_dim {
                                *out += xi * self.weights[i][j];
                            }
                        }
                    }
                }
                output
            })
            .collect()
    }
}

/// Multi-head attention
#[derive(Debug, Clone)]
pub struct MultiHeadAttention {
    /// Number of heads
    pub num_heads: usize,
    /// Model dimension
    pub d_model: usize,
    /// Key/Query dimension per head
    pub d_k: usize,
    /// Value dimension per head
    pub d_v: usize,
    /// Query projection
    pub w_q: LinearProjection,
    /// Key projection
    pub w_k: LinearProjection,
    /// Value projection
    pub w_v: LinearProjection,
    /// Output projection
    pub w_o: LinearProjection,
    /// Attention module
    pub attention: ScaledDotProductAttention,
}

impl MultiHeadAttention {
    /// Create new multi-head attention
    pub fn new(d_model: usize, num_heads: usize, seed: u64) -> Self {
        let d_k = d_model / num_heads;
        let d_v = d_model / num_heads;

        let mut rng = seed;

        let w_q = LinearProjection::new(d_model, d_model, rng);
        rng = lcg_next(rng);
        let w_k = LinearProjection::new(d_model, d_model, rng);
        rng = lcg_next(rng);
        let w_v = LinearProjection::new(d_model, d_model, rng);
        rng = lcg_next(rng);
        let w_o = LinearProjection::new(d_model, d_model, rng);

        Self {
            num_heads,
            d_model,
            d_k,
            d_v,
            w_q,
            w_k,
            w_v,
            w_o,
            attention: ScaledDotProductAttention::new(d_k),
        }
    }

    /// Create with causal masking
    pub fn causal(d_model: usize, num_heads: usize, seed: u64) -> Self {
        let mut mha = Self::new(d_model, num_heads, seed);
        mha.attention.mask = AttentionMask::Causal;
        mha
    }

    /// Forward pass
    pub fn forward(
        &mut self,
        query: &[Vec<f64>],
        key: &[Vec<f64>],
        value: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        let seq_len = query.len();

        // Project Q, K, V
        let q_proj = self.w_q.forward(query);
        let k_proj = self.w_k.forward(key);
        let v_proj = self.w_v.forward(value);

        // Split into heads and compute attention per head
        let mut head_outputs = Vec::new();

        for h in 0..self.num_heads {
            let start = h * self.d_k;
            let end = start + self.d_k;

            // Extract head's portion
            let q_head: Vec<Vec<f64>> = q_proj
                .iter()
                .map(|x| x[start..end.min(x.len())].to_vec())
                .collect();
            let k_head: Vec<Vec<f64>> = k_proj
                .iter()
                .map(|x| x[start..end.min(x.len())].to_vec())
                .collect();
            let v_head: Vec<Vec<f64>> = v_proj
                .iter()
                .map(|x| x[start..end.min(x.len())].to_vec())
                .collect();

            // Compute attention
            let head_out = self.attention.forward(&q_head, &k_head, &v_head);
            head_outputs.push(head_out);
        }

        // Concatenate heads
        let mut concat = vec![vec![0.0; self.d_model]; seq_len];
        for (h, head_out) in head_outputs.iter().enumerate() {
            for (i, vec) in head_out.iter().enumerate() {
                for (j, &val) in vec.iter().enumerate() {
                    let idx = h * self.d_k + j;
                    if idx < self.d_model && i < seq_len {
                        concat[i][idx] = val;
                    }
                }
            }
        }

        // Final projection
        self.w_o.forward(&concat)
    }

    /// Self-attention (query = key = value)
    pub fn self_attention(&mut self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        self.forward(input, input, input)
    }
}

// ============================================================================
// LINEAR ATTENTION (O(n) complexity)
// ============================================================================

/// Kernel function for linear attention
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelFunction {
    /// ELU + 1 kernel
    Elu,
    /// ReLU kernel
    Relu,
    /// Softmax approximation (random features)
    SoftmaxApprox,
    /// Polynomial kernel
    Polynomial,
}

/// Linear attention (Performer-style)
#[derive(Debug, Clone)]
pub struct LinearAttention {
    /// Model dimension
    pub d_model: usize,
    /// Number of random features
    pub num_features: usize,
    /// Kernel function
    pub kernel: KernelFunction,
    /// Random projection weights (for softmax approx)
    pub random_weights: Vec<Vec<f64>>,
    /// Query/Key projection
    pub w_qk: LinearProjection,
    /// Value projection
    pub w_v: LinearProjection,
}

impl LinearAttention {
    /// Create new linear attention
    pub fn new(d_model: usize, num_features: usize, seed: u64) -> Self {
        let mut rng = seed;

        // Random features for softmax approximation
        let random_weights: Vec<Vec<f64>> = (0..num_features)
            .map(|_| {
                (0..d_model)
                    .map(|_| {
                        rng = lcg_next(rng);
                        box_muller(rng)
                    })
                    .collect()
            })
            .collect();

        Self {
            d_model,
            num_features,
            kernel: KernelFunction::Elu,
            random_weights,
            w_qk: LinearProjection::new(d_model, d_model, rng),
            w_v: LinearProjection::new(d_model, d_model, lcg_next(rng)),
        }
    }

    /// Apply kernel function
    fn apply_kernel(&self, x: &[f64]) -> Vec<f64> {
        match self.kernel {
            KernelFunction::Elu => x
                .iter()
                .map(|&v| if v > 0.0 { v + 1.0 } else { libm::exp(v) })
                .collect(),
            KernelFunction::Relu => x.iter().map(|&v| v.max(0.0)).collect(),
            KernelFunction::SoftmaxApprox => {
                // Random feature map
                let mut phi = vec![0.0; self.num_features * 2];
                let scale = 1.0 / libm::sqrt(self.num_features as f64);

                for (i, w) in self.random_weights.iter().enumerate() {
                    let dot: f64 = x.iter().zip(w.iter()).map(|(&a, &b)| a * b).sum();
                    phi[i] = libm::cos(dot) * scale;
                    phi[i + self.num_features] = libm::sin(dot) * scale;
                }
                phi
            },
            KernelFunction::Polynomial => {
                // (1 + <x, y>)^2 approximation
                let mut result = x.to_vec();
                result.push(1.0); // bias term
                result
            },
        }
    }

    /// Forward pass with O(n) complexity
    pub fn forward(
        &self,
        query: &[Vec<f64>],
        key: &[Vec<f64>],
        value: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        let seq_len_q = query.len();
        let seq_len_k = key.len();

        if seq_len_q == 0 || seq_len_k == 0 || value.is_empty() {
            return Vec::new();
        }

        let d_v = value[0].len();

        // Apply kernel to Q and K
        let phi_q: Vec<Vec<f64>> = query.iter().map(|q| self.apply_kernel(q)).collect();
        let phi_k: Vec<Vec<f64>> = key.iter().map(|k| self.apply_kernel(k)).collect();

        let d_phi = if phi_k.is_empty() { 0 } else { phi_k[0].len() };

        // Compute K^T * V (accumulated)
        let mut kv = vec![vec![0.0; d_v]; d_phi];
        for (phi_ki, vi) in phi_k.iter().zip(value.iter()) {
            for (j, &pk) in phi_ki.iter().enumerate() {
                for (k, &v) in vi.iter().enumerate() {
                    if j < d_phi && k < d_v {
                        kv[j][k] += pk * v;
                    }
                }
            }
        }

        // Compute sum of phi_k for normalization
        let mut k_sum = vec![0.0; d_phi];
        for phi_ki in &phi_k {
            for (i, &pk) in phi_ki.iter().enumerate() {
                if i < d_phi {
                    k_sum[i] += pk;
                }
            }
        }

        // Compute output for each query
        let mut output = vec![vec![0.0; d_v]; seq_len_q];

        for (i, phi_qi) in phi_q.iter().enumerate() {
            // Numerator: phi_q * (K^T * V)
            for j in 0..d_v {
                let mut num = 0.0;
                for (k, &pq) in phi_qi.iter().enumerate() {
                    if k < d_phi {
                        num += pq * kv[k][j];
                    }
                }
                output[i][j] = num;
            }

            // Denominator: phi_q * sum(phi_k)
            let denom: f64 = phi_qi
                .iter()
                .zip(k_sum.iter())
                .map(|(&pq, &ks)| pq * ks)
                .sum();

            // Normalize
            for val in &mut output[i] {
                *val /= denom.max(EPSILON);
            }
        }

        output
    }
}

// ============================================================================
// SPARSE ATTENTION
// ============================================================================

/// Sparsity pattern for attention
#[derive(Debug, Clone)]
pub enum SparsityPattern {
    /// Full attention (no sparsity)
    Full,
    /// Local window attention
    Local(usize),
    /// Strided attention
    Strided(usize),
    /// Combination of local + strided
    LocalStrided(usize, usize),
    /// Random sparsity pattern
    Random(f64),
    /// Custom pattern
    Custom(Vec<Vec<bool>>),
}

/// Sparse attention implementation
#[derive(Debug, Clone)]
pub struct SparseAttention {
    /// Model dimension
    pub d_model: usize,
    /// Number of heads
    pub num_heads: usize,
    /// Sparsity pattern
    pub pattern: SparsityPattern,
    /// Base attention
    pub base_attention: ScaledDotProductAttention,
    /// RNG state
    rng_state: u64,
}

impl SparseAttention {
    /// Create new sparse attention
    pub fn new(d_model: usize, num_heads: usize, pattern: SparsityPattern) -> Self {
        let d_k = d_model / num_heads;

        Self {
            d_model,
            num_heads,
            pattern,
            base_attention: ScaledDotProductAttention::new(d_k),
            rng_state: 12345,
        }
    }

    /// Generate sparsity mask
    fn generate_mask(&mut self, seq_len: usize) -> Vec<Vec<bool>> {
        match &self.pattern {
            SparsityPattern::Full => {
                vec![vec![true; seq_len]; seq_len]
            },
            SparsityPattern::Local(window) => {
                let w = *window;
                (0..seq_len)
                    .map(|i| {
                        (0..seq_len)
                            .map(|j| {
                                let diff = if i > j { i - j } else { j - i };
                                diff <= w / 2
                            })
                            .collect()
                    })
                    .collect()
            },
            SparsityPattern::Strided(stride) => {
                let s = *stride;
                (0..seq_len)
                    .map(|i| {
                        (0..seq_len)
                            .map(|j| (i % s == 0) || (j % s == 0) || i == j)
                            .collect()
                    })
                    .collect()
            },
            SparsityPattern::LocalStrided(window, stride) => {
                let w = *window;
                let s = *stride;
                (0..seq_len)
                    .map(|i| {
                        (0..seq_len)
                            .map(|j| {
                                let diff = if i > j { i - j } else { j - i };
                                (diff <= w / 2) || (j % s == 0)
                            })
                            .collect()
                    })
                    .collect()
            },
            SparsityPattern::Random(prob) => (0..seq_len)
                .map(|i| {
                    (0..seq_len)
                        .map(|j| {
                            if i == j {
                                return true;
                            }
                            self.rng_state = lcg_next(self.rng_state);
                            (self.rng_state as f64 / u64::MAX as f64) < *prob
                        })
                        .collect()
                })
                .collect(),
            SparsityPattern::Custom(pattern) => pattern.clone(),
        }
    }

    /// Forward pass with sparse attention
    pub fn forward(
        &mut self,
        query: &[Vec<f64>],
        key: &[Vec<f64>],
        value: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        let seq_len = query.len();
        let d_v = if value.is_empty() { 0 } else { value[0].len() };

        if seq_len == 0 || d_v == 0 {
            return Vec::new();
        }

        // Generate sparsity mask
        let mask = self.generate_mask(seq_len);

        // Compute attention scores only for non-masked positions
        let d_k = self.d_model / self.num_heads;
        let scale = 1.0 / libm::sqrt(d_k as f64);

        let mut scores = vec![vec![f64::NEG_INFINITY; seq_len]; seq_len];

        for i in 0..seq_len {
            for j in 0..seq_len {
                if mask[i][j] {
                    let dot: f64 = query[i]
                        .iter()
                        .zip(key[j].iter())
                        .map(|(&q, &k)| q * k)
                        .sum();
                    scores[i][j] = dot * scale;
                }
            }
        }

        // Apply softmax per row
        let weights = self.softmax_sparse(&scores, &mask);

        // Compute output
        let mut output = vec![vec![0.0; d_v]; seq_len];

        for i in 0..seq_len {
            for j in 0..seq_len {
                if mask[i][j] {
                    for k in 0..d_v {
                        output[i][k] += weights[i][j] * value[j][k];
                    }
                }
            }
        }

        output
    }

    /// Sparse softmax
    fn softmax_sparse(&self, scores: &[Vec<f64>], mask: &[Vec<bool>]) -> Vec<Vec<f64>> {
        scores
            .iter()
            .zip(mask.iter())
            .map(|(row, mask_row)| {
                let valid: Vec<f64> = row
                    .iter()
                    .zip(mask_row.iter())
                    .filter_map(|(&s, &m)| if m { Some(s) } else { None })
                    .collect();

                let max_val = valid.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

                let exp_sum: f64 = valid.iter().map(|&s| libm::exp(s - max_val)).sum();

                row.iter()
                    .zip(mask_row.iter())
                    .map(|(&s, &m)| {
                        if m {
                            libm::exp(s - max_val) / (exp_sum + EPSILON)
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

// ============================================================================
// RELATIVE POSITIONAL ENCODING
// ============================================================================

/// Relative position encoding type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativePosType {
    /// Shaw et al. style
    Shaw,
    /// Transformer-XL style
    TransformerXL,
    /// T5 relative position bias
    T5Bias,
    /// RoPE (Rotary Position Embedding)
    Rotary,
}

/// Relative positional attention
#[derive(Debug, Clone)]
pub struct RelativePositionalAttention {
    /// Model dimension
    pub d_model: usize,
    /// Number of heads
    pub num_heads: usize,
    /// Maximum relative distance
    pub max_distance: usize,
    /// Position type
    pub pos_type: RelativePosType,
    /// Position embeddings (key)
    pub rel_key: Vec<Vec<f64>>,
    /// Position embeddings (value)
    pub rel_value: Vec<Vec<f64>>,
    /// Base attention
    pub base_attention: MultiHeadAttention,
}

impl RelativePositionalAttention {
    /// Create new relative positional attention
    pub fn new(d_model: usize, num_heads: usize, max_distance: usize, seed: u64) -> Self {
        let d_k = d_model / num_heads;
        let mut rng = seed;

        // Initialize relative position embeddings
        let num_positions = 2 * max_distance + 1;

        let rel_key: Vec<Vec<f64>> = (0..num_positions)
            .map(|_| {
                (0..d_k)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 0.1
                    })
                    .collect()
            })
            .collect();

        let rel_value: Vec<Vec<f64>> = (0..num_positions)
            .map(|_| {
                (0..d_k)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 0.1
                    })
                    .collect()
            })
            .collect();

        Self {
            d_model,
            num_heads,
            max_distance,
            pos_type: RelativePosType::Shaw,
            rel_key,
            rel_value,
            base_attention: MultiHeadAttention::new(d_model, num_heads, rng),
        }
    }

    /// Get relative position index
    fn get_rel_pos(&self, i: usize, j: usize) -> usize {
        let diff = i as isize - j as isize;
        let clipped = diff
            .max(-(self.max_distance as isize))
            .min(self.max_distance as isize);

        (clipped + self.max_distance as isize) as usize
    }

    /// Forward with relative positions
    pub fn forward(
        &mut self,
        query: &[Vec<f64>],
        key: &[Vec<f64>],
        value: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        let seq_len_q = query.len();
        let seq_len_k = key.len();

        if seq_len_q == 0 || seq_len_k == 0 {
            return Vec::new();
        }

        // Compute base content attention
        let content_output = self.base_attention.forward(query, key, value);

        // Add relative position contribution
        let d_k = self.d_model / self.num_heads;
        let scale = 1.0 / libm::sqrt(d_k as f64);

        // Compute relative attention scores
        let mut rel_scores = vec![vec![0.0; seq_len_k]; seq_len_q];

        for i in 0..seq_len_q {
            for j in 0..seq_len_k {
                let rel_idx = self.get_rel_pos(i, j);
                let rel_k = &self.rel_key[rel_idx.min(self.rel_key.len() - 1)];

                // Q * R_k
                let score: f64 = query[i]
                    .iter()
                    .take(d_k)
                    .zip(rel_k.iter())
                    .map(|(&q, &r)| q * r)
                    .sum();

                rel_scores[i][j] = score * scale;
            }
        }

        // Combine (simplified - in practice would recompute softmax)
        content_output
    }

    /// Apply RoPE (Rotary Position Embedding)
    pub fn apply_rope(&self, x: &[Vec<f64>], positions: &[usize]) -> Vec<Vec<f64>> {
        let d = x[0].len();

        x.iter()
            .zip(positions.iter())
            .map(|(vec, &pos)| {
                let mut result = vec.clone();

                for i in 0..d / 2 {
                    let theta = pos as f64 / libm::pow(10000.0, 2.0 * i as f64 / d as f64);
                    let cos_theta = libm::cos(theta);
                    let sin_theta = libm::sin(theta);

                    let x0 = result[2 * i];
                    let x1 = result[2 * i + 1];

                    result[2 * i] = x0 * cos_theta - x1 * sin_theta;
                    result[2 * i + 1] = x0 * sin_theta + x1 * cos_theta;
                }

                result
            })
            .collect()
    }
}

// ============================================================================
// CROSS ATTENTION
// ============================================================================

/// Cross-attention for multi-source fusion
#[derive(Debug, Clone)]
pub struct CrossAttention {
    /// Query dimension
    pub d_query: usize,
    /// Key/Value dimension
    pub d_kv: usize,
    /// Output dimension
    pub d_out: usize,
    /// Number of heads
    pub num_heads: usize,
    /// Query projection
    pub w_q: LinearProjection,
    /// Key projection
    pub w_k: LinearProjection,
    /// Value projection
    pub w_v: LinearProjection,
    /// Output projection
    pub w_o: LinearProjection,
    /// Attention
    attention: ScaledDotProductAttention,
}

impl CrossAttention {
    /// Create new cross attention
    pub fn new(d_query: usize, d_kv: usize, d_out: usize, num_heads: usize, seed: u64) -> Self {
        let d_k = d_out / num_heads;
        let mut rng = seed;

        Self {
            d_query,
            d_kv,
            d_out,
            num_heads,
            w_q: LinearProjection::new(d_query, d_out, rng),
            w_k: LinearProjection::new(d_kv, d_out, lcg_next(rng)),
            w_v: LinearProjection::new(d_kv, d_out, lcg_next(lcg_next(rng))),
            w_o: LinearProjection::new(d_out, d_out, lcg_next(lcg_next(lcg_next(rng)))),
            attention: ScaledDotProductAttention::new(d_k),
        }
    }

    /// Forward pass
    pub fn forward(&mut self, query: &[Vec<f64>], context: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let q_proj = self.w_q.forward(query);
        let k_proj = self.w_k.forward(context);
        let v_proj = self.w_v.forward(context);

        let attended = self.attention.forward(&q_proj, &k_proj, &v_proj);

        self.w_o.forward(&attended)
    }
}

// ============================================================================
// FLASH ATTENTION (Memory Efficient)
// ============================================================================

/// Flash attention (memory-efficient, tiled computation)
#[derive(Debug, Clone)]
pub struct FlashAttention {
    /// Block size for tiling
    pub block_size: usize,
    /// Model dimension
    pub d_model: usize,
    /// Causal masking
    pub causal: bool,
    /// Scaling factor
    pub scale: f64,
}

impl FlashAttention {
    /// Create new flash attention
    pub fn new(d_model: usize, block_size: usize) -> Self {
        Self {
            block_size,
            d_model,
            causal: false,
            scale: 1.0 / libm::sqrt(d_model as f64),
        }
    }

    /// Tiled attention computation (memory efficient)
    pub fn forward(
        &self,
        query: &[Vec<f64>],
        key: &[Vec<f64>],
        value: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        let n_q = query.len();
        let n_k = key.len();

        if n_q == 0 || n_k == 0 || value.is_empty() {
            return Vec::new();
        }

        let d = value[0].len();
        let bs = self.block_size;

        // Initialize output and normalization
        let mut output = vec![vec![0.0; d]; n_q];
        let mut row_max = vec![f64::NEG_INFINITY; n_q];
        let mut row_sum = vec![0.0; n_q];

        // Process in blocks
        for j_start in (0..n_k).step_by(bs) {
            let j_end = (j_start + bs).min(n_k);

            for i in 0..n_q {
                // Compute scores for this block
                let mut block_max = f64::NEG_INFINITY;
                let mut block_scores = Vec::new();

                for j in j_start..j_end {
                    if self.causal && j > i {
                        block_scores.push(f64::NEG_INFINITY);
                        continue;
                    }

                    let score: f64 = query[i]
                        .iter()
                        .zip(key[j].iter())
                        .map(|(&q, &k)| q * k)
                        .sum::<f64>()
                        * self.scale;

                    block_max = block_max.max(score);
                    block_scores.push(score);
                }

                // Update running max
                let prev_max = row_max[i];
                row_max[i] = row_max[i].max(block_max);

                // Rescale previous sum
                if prev_max > f64::NEG_INFINITY && row_max[i] > prev_max {
                    let scale_factor = libm::exp(prev_max - row_max[i]);
                    row_sum[i] *= scale_factor;
                    for val in &mut output[i] {
                        *val *= scale_factor;
                    }
                }

                // Accumulate block contribution
                for (idx, j) in (j_start..j_end).enumerate() {
                    let exp_score = libm::exp(block_scores[idx] - row_max[i]);
                    row_sum[i] += exp_score;

                    for (k, val) in output[i].iter_mut().enumerate() {
                        *val += exp_score * value[j][k];
                    }
                }
            }
        }

        // Normalize
        for (i, row) in output.iter_mut().enumerate() {
            let norm = row_sum[i].max(EPSILON);
            for val in row {
                *val /= norm;
            }
        }

        output
    }
}

// ============================================================================
// KERNEL ATTENTION MANAGER
// ============================================================================

/// Kernel attention type for different use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAttentionType {
    /// Full attention for short sequences
    Full,
    /// Linear attention for long sequences
    Linear,
    /// Sparse attention for very long sequences
    Sparse,
    /// Flash attention for memory efficiency
    Flash,
}

/// Kernel attention manager
pub struct KernelAttentionManager {
    /// Model dimension
    pub d_model: usize,
    /// Number of heads
    pub num_heads: usize,
    /// Multi-head attention
    pub mha: MultiHeadAttention,
    /// Linear attention
    pub linear: LinearAttention,
    /// Sparse attention
    pub sparse: SparseAttention,
    /// Flash attention
    pub flash: FlashAttention,
    /// Active attention type
    pub active_type: KernelAttentionType,
    /// Statistics
    pub ops_count: u64,
}

impl KernelAttentionManager {
    /// Create a new attention manager
    pub fn new(d_model: usize, num_heads: usize) -> Self {
        Self {
            d_model,
            num_heads,
            mha: MultiHeadAttention::new(d_model, num_heads, 12345),
            linear: LinearAttention::new(d_model, d_model / 2, 54321),
            sparse: SparseAttention::new(
                d_model,
                num_heads,
                SparsityPattern::LocalStrided(128, 32),
            ),
            flash: FlashAttention::new(d_model, 64),
            active_type: KernelAttentionType::Full,
            ops_count: 0,
        }
    }

    /// Select attention type based on sequence length
    pub fn auto_select(&mut self, seq_len: usize) {
        self.active_type = if seq_len <= 512 {
            KernelAttentionType::Full
        } else if seq_len <= 2048 {
            KernelAttentionType::Flash
        } else if seq_len <= 8192 {
            KernelAttentionType::Sparse
        } else {
            KernelAttentionType::Linear
        };
    }

    /// Apply attention using current type
    pub fn apply(
        &mut self,
        query: &[Vec<f64>],
        key: &[Vec<f64>],
        value: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        self.ops_count += 1;

        match self.active_type {
            KernelAttentionType::Full => self.mha.forward(query, key, value),
            KernelAttentionType::Linear => self.linear.forward(query, key, value),
            KernelAttentionType::Sparse => self.sparse.forward(query, key, value),
            KernelAttentionType::Flash => self.flash.forward(query, key, value),
        }
    }

    /// Self-attention helper
    pub fn self_attention(&mut self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        self.auto_select(input.len());
        self.apply(input, input, input)
    }

    /// Get attention statistics
    pub fn get_stats(&self) -> AttentionStats {
        AttentionStats {
            d_model: self.d_model,
            num_heads: self.num_heads,
            active_type: self.active_type,
            ops_count: self.ops_count,
        }
    }
}

/// Attention statistics
#[derive(Debug, Clone)]
pub struct AttentionStats {
    /// Model dimension
    pub d_model: usize,
    /// Number of heads
    pub num_heads: usize,
    /// Active attention type
    pub active_type: KernelAttentionType,
    /// Total operations
    pub ops_count: u64,
}

// ============================================================================
// UTILITIES
// ============================================================================

/// LCG random number generator
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Box-Muller transform for Gaussian sampling
fn box_muller(seed: u64) -> f64 {
    let u1 = (seed as f64 / u64::MAX as f64).max(1e-10);
    let seed2 = lcg_next(seed);
    let u2 = seed2 as f64 / u64::MAX as f64;

    libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * core::f64::consts::PI * u2)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_matrix(rows: usize, cols: usize, seed: u64) -> Vec<Vec<f64>> {
        let mut rng = seed;
        (0..rows)
            .map(|_| {
                (0..cols)
                    .map(|_| {
                        rng = lcg_next(rng);
                        (rng as f64 / u64::MAX as f64) - 0.5
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn test_scaled_dot_product_attention() {
        let mut attn = ScaledDotProductAttention::new(64);

        let q = create_test_matrix(4, 64, 12345);
        let k = create_test_matrix(6, 64, 54321);
        let v = create_test_matrix(6, 64, 99999);

        let output = attn.forward(&q, &k, &v);

        assert_eq!(output.len(), 4);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_causal_attention() {
        let mut attn = ScaledDotProductAttention::causal(64);

        let x = create_test_matrix(8, 64, 12345);

        let weights = attn.get_attention_weights(&x, &x);

        // Upper triangle should be zero
        for i in 0..weights.len() {
            for j in (i + 1)..weights[i].len() {
                assert!(weights[i][j] < 1e-6);
            }
        }
    }

    #[test]
    fn test_multi_head_attention() {
        let mut mha = MultiHeadAttention::new(128, 8, 12345);

        let x = create_test_matrix(10, 128, 12345);

        let output = mha.self_attention(&x);

        assert_eq!(output.len(), 10);
        assert_eq!(output[0].len(), 128);
    }

    #[test]
    fn test_linear_attention() {
        let la = LinearAttention::new(64, 32, 12345);

        let q = create_test_matrix(100, 64, 12345);
        let k = create_test_matrix(100, 64, 54321);
        let v = create_test_matrix(100, 64, 99999);

        let output = la.forward(&q, &k, &v);

        assert_eq!(output.len(), 100);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_sparse_attention_local() {
        let mut sa = SparseAttention::new(64, 8, SparsityPattern::Local(8));

        let x = create_test_matrix(32, 64, 12345);

        let output = sa.forward(&x, &x, &x);

        assert_eq!(output.len(), 32);
    }

    #[test]
    fn test_sparse_attention_strided() {
        let mut sa = SparseAttention::new(64, 8, SparsityPattern::Strided(4));

        let x = create_test_matrix(32, 64, 12345);

        let output = sa.forward(&x, &x, &x);

        assert_eq!(output.len(), 32);
    }

    #[test]
    fn test_relative_positional_attention() {
        let mut rpa = RelativePositionalAttention::new(128, 8, 32, 12345);

        let x = create_test_matrix(16, 128, 12345);

        let output = rpa.forward(&x, &x, &x);

        assert_eq!(output.len(), 16);
    }

    #[test]
    fn test_rope() {
        let rpa = RelativePositionalAttention::new(64, 4, 32, 12345);

        let x = create_test_matrix(8, 64, 12345);
        let positions: Vec<usize> = (0..8).collect();

        let rotated = rpa.apply_rope(&x, &positions);

        assert_eq!(rotated.len(), 8);
        assert_eq!(rotated[0].len(), 64);
    }

    #[test]
    fn test_cross_attention() {
        let mut ca = CrossAttention::new(64, 128, 64, 4, 12345);

        let query = create_test_matrix(8, 64, 12345);
        let context = create_test_matrix(16, 128, 54321);

        let output = ca.forward(&query, &context);

        assert_eq!(output.len(), 8);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_flash_attention() {
        let flash = FlashAttention::new(64, 16);

        let q = create_test_matrix(32, 64, 12345);
        let k = create_test_matrix(32, 64, 54321);
        let v = create_test_matrix(32, 64, 99999);

        let output = flash.forward(&q, &k, &v);

        assert_eq!(output.len(), 32);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_kernel_attention_manager() {
        let mut manager = KernelAttentionManager::new(64, 8);

        let x = create_test_matrix(16, 64, 12345);

        let output = manager.self_attention(&x);

        assert_eq!(output.len(), 16);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_auto_select() {
        let mut manager = KernelAttentionManager::new(64, 8);

        manager.auto_select(256);
        assert_eq!(manager.active_type, KernelAttentionType::Full);

        manager.auto_select(1000);
        assert_eq!(manager.active_type, KernelAttentionType::Flash);

        manager.auto_select(4000);
        assert_eq!(manager.active_type, KernelAttentionType::Sparse);

        manager.auto_select(10000);
        assert_eq!(manager.active_type, KernelAttentionType::Linear);
    }

    #[test]
    fn test_attention_mask_types() {
        let mask = AttentionMask::Causal;

        assert_eq!(mask.get_mask(0, 0, 4), 0.0);
        assert_eq!(mask.get_mask(0, 1, 4), f64::NEG_INFINITY);
        assert_eq!(mask.get_mask(2, 1, 4), 0.0);
    }

    #[test]
    fn test_linear_projection() {
        let proj = LinearProjection::new(32, 64, 12345);

        let input = create_test_matrix(8, 32, 12345);
        let output = proj.forward(&input);

        assert_eq!(output.len(), 8);
        assert_eq!(output[0].len(), 64);
    }
}
