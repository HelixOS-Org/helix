//! # Transformer Architecture for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Complete transformer implementations optimized
//! for kernel-level AI operations.
//!
//! ## Module Structure
//!
//! - [`types`] - Core types: Tensor2, Tensor3, TransformerConfig, Linear, LayerNorm, Embedding
//! - [`layers`] - Building blocks: FFN, attention blocks, transformer blocks
//! - [`encoder`] - BERT-style encoders, Vision Transformer
//! - [`decoder`] - Decoder with cross-attention, seq2seq transformer
//! - [`gpt`] - Decoder-only models: GPT, LLaMA, Mistral
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_nexus::transformer::{
//!     types::TransformerConfig,
//!     gpt::{GPT, Llama, LlamaConfig},
//!     encoder::BertEncoder,
//! };
//!
//! // Create GPT model
//! let config = TransformerConfig::tiny();
//! let mut gpt = GPT::new(config, 42);
//!
//! // Generate text
//! let input = vec![1, 2, 3];
//! let output = gpt.generate_greedy(&input, 10);
//! ```
//!
//! ## Kernel Applications
//!
//! - Log sequence analysis and prediction
//! - System state understanding
//! - Predictive maintenance
//! - Kernel configuration generation

#![no_std]

extern crate alloc;

// Submodules with production-quality implementations
pub mod types;
pub mod layers;
pub mod encoder;
pub mod decoder;
pub mod gpt;

// Re-export main types for convenience
pub use types::{Tensor2, Tensor3, TransformerConfig, ActivationType, Linear, LayerNorm, RMSNorm, Dropout, Embedding, PositionalEmbedding};
pub use layers::{FeedForward, GatedFFN, MoEFFN, MultiHeadSelfAttention, GroupedQueryAttention, PreNormBlock, PostNormBlock, LlamaBlock};
pub use encoder::{Encoder, EncoderLayer, EncoderOutput, BertEncoder, VisionEncoder, VisionEncoderOutput};
pub use decoder::{Decoder, DecoderLayer, DecoderOnly, MultiHeadCrossAttention, Seq2SeqTransformer, KVCache};
pub use gpt::{GPT, Llama, LlamaConfig, LlamaLayer, Mistral, MistralConfig, MistralLayer};

// Legacy code below - kept for backward compatibility
// TODO: Migrate to new module structure

use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS (LEGACY)
// ============================================================================

/// Default model dimension
const DEFAULT_D_MODEL: usize = 256;

/// Default FFN dimension
const DEFAULT_D_FF: usize = 1024;

/// Default number of layers
const DEFAULT_NUM_LAYERS: usize = 6;

/// Default number of heads
const DEFAULT_NUM_HEADS: usize = 8;

/// Epsilon for layer normalization
const LAYER_NORM_EPS: f64 = 1e-6;

/// Default dropout rate
const DEFAULT_DROPOUT: f64 = 0.1;

// ============================================================================
// ACTIVATION FUNCTIONS (LEGACY)
// ============================================================================

/// Activation function type (legacy)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// ReLU
    Relu,
    /// GELU
    Gelu,
    /// SiLU / Swish
    Silu,
    /// Tanh
    Tanh,
    /// Sigmoid
    Sigmoid,
}

impl Activation {
    /// Apply activation
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Activation::Relu => x.max(0.0),
            Activation::Gelu => {
                // GELU approximation: 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
                let c = libm::sqrt(2.0 / core::f64::consts::PI);
                0.5 * x * (1.0 + libm::tanh(c * (x + 0.044715 * x.powi(3))))
            },
            Activation::Silu => x * sigmoid(x),
            Activation::Tanh => libm::tanh(x),
            Activation::Sigmoid => sigmoid(x),
        }
    }

    /// Apply to vector
    pub fn apply_vec(&self, v: &mut [f64]) {
        for x in v {
            *x = self.apply(*x);
        }
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + libm::exp(-x))
}

// ============================================================================
// LAYER NORMALIZATION
// ============================================================================

/// Layer normalization
#[derive(Debug, Clone)]
pub struct LayerNorm {
    /// Dimension
    pub dim: usize,
    /// Gamma (scale)
    pub gamma: Vec<f64>,
    /// Beta (shift)
    pub beta: Vec<f64>,
    /// Epsilon
    pub eps: f64,
}

impl LayerNorm {
    /// Create new layer norm
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            gamma: vec![1.0; dim],
            beta: vec![0.0; dim],
            eps: LAYER_NORM_EPS,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        let n = input.len();
        if n == 0 {
            return Vec::new();
        }

        // Mean
        let mean = input.iter().sum::<f64>() / n as f64;

        // Variance
        let var = input.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n as f64;

        // Normalize
        let std = libm::sqrt(var + self.eps);

        input
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let normalized = (x - mean) / std;
                self.gamma[i.min(self.dim - 1)] * normalized + self.beta[i.min(self.dim - 1)]
            })
            .collect()
    }

    /// Forward for batch
    pub fn forward_batch(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        input.iter().map(|x| self.forward(x)).collect()
    }
}

/// RMS Normalization (from LLaMA)
#[derive(Debug, Clone)]
pub struct RMSNorm {
    /// Dimension
    pub dim: usize,
    /// Gamma (scale)
    pub gamma: Vec<f64>,
    /// Epsilon
    pub eps: f64,
}

impl RMSNorm {
    /// Create new RMS norm
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            gamma: vec![1.0; dim],
            eps: LAYER_NORM_EPS,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        let n = input.len();
        if n == 0 {
            return Vec::new();
        }

        // RMS
        let rms = libm::sqrt(input.iter().map(|&x| x * x).sum::<f64>() / n as f64 + self.eps);

        // Normalize
        input
            .iter()
            .enumerate()
            .map(|(i, &x)| self.gamma[i.min(self.dim - 1)] * x / rms)
            .collect()
    }
}

// ============================================================================
// FEED-FORWARD NETWORK
// ============================================================================

/// Feed-forward network
#[derive(Debug, Clone)]
pub struct FeedForward {
    /// Input dimension
    pub d_model: usize,
    /// Hidden dimension
    pub d_ff: usize,
    /// First linear (d_model -> d_ff)
    pub w1: Vec<Vec<f64>>,
    /// Second linear (d_ff -> d_model)
    pub w2: Vec<Vec<f64>>,
    /// First bias
    pub b1: Vec<f64>,
    /// Second bias
    pub b2: Vec<f64>,
    /// Activation
    pub activation: Activation,
}

impl FeedForward {
    /// Create new FFN
    pub fn new(d_model: usize, d_ff: usize, seed: u64) -> Self {
        let mut rng = seed;

        let scale1 = libm::sqrt(2.0 / (d_model + d_ff) as f64);
        let scale2 = libm::sqrt(2.0 / (d_ff + d_model) as f64);

        let w1: Vec<Vec<f64>> = (0..d_model)
            .map(|_| {
                (0..d_ff)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale1
                    })
                    .collect()
            })
            .collect();

        let w2: Vec<Vec<f64>> = (0..d_ff)
            .map(|_| {
                (0..d_model)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale2
                    })
                    .collect()
            })
            .collect();

        Self {
            d_model,
            d_ff,
            w1,
            w2,
            b1: vec![0.0; d_ff],
            b2: vec![0.0; d_model],
            activation: Activation::Gelu,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        // First linear + activation
        let mut hidden = self.b1.clone();
        for (i, &x) in input.iter().enumerate() {
            if i < self.w1.len() {
                for (j, h) in hidden.iter_mut().enumerate() {
                    if j < self.w1[i].len() {
                        *h += x * self.w1[i][j];
                    }
                }
            }
        }

        self.activation.apply_vec(&mut hidden);

        // Second linear
        let mut output = self.b2.clone();
        for (i, &h) in hidden.iter().enumerate() {
            if i < self.w2.len() {
                for (j, o) in output.iter_mut().enumerate() {
                    if j < self.w2[i].len() {
                        *o += h * self.w2[i][j];
                    }
                }
            }
        }

        output
    }

    /// Forward for batch
    pub fn forward_batch(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        input.iter().map(|x| self.forward(x)).collect()
    }
}

/// Gated Linear Unit (GLU) FFN
#[derive(Debug, Clone)]
pub struct GatedFFN {
    /// Input dimension
    pub d_model: usize,
    /// Hidden dimension
    pub d_ff: usize,
    /// Gate linear
    pub w_gate: Vec<Vec<f64>>,
    /// Up projection
    pub w_up: Vec<Vec<f64>>,
    /// Down projection
    pub w_down: Vec<Vec<f64>>,
    /// Activation
    pub activation: Activation,
}

impl GatedFFN {
    /// Create new gated FFN
    pub fn new(d_model: usize, d_ff: usize, seed: u64) -> Self {
        let mut rng = seed;
        let scale = libm::sqrt(2.0 / (d_model + d_ff) as f64);

        let init_matrix = |dim_in: usize, dim_out: usize, rng: &mut u64| -> Vec<Vec<f64>> {
            (0..dim_in)
                .map(|_| {
                    (0..dim_out)
                        .map(|_| {
                            *rng = lcg_next(*rng);
                            ((*rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale
                        })
                        .collect()
                })
                .collect()
        };

        Self {
            d_model,
            d_ff,
            w_gate: init_matrix(d_model, d_ff, &mut rng),
            w_up: init_matrix(d_model, d_ff, &mut rng),
            w_down: init_matrix(d_ff, d_model, &mut rng),
            activation: Activation::Silu,
        }
    }

    /// Forward pass: down(activation(gate(x)) * up(x))
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        // Gate
        let mut gate = vec![0.0; self.d_ff];
        for (i, &x) in input.iter().enumerate() {
            if i < self.w_gate.len() {
                for (j, g) in gate.iter_mut().enumerate() {
                    if j < self.w_gate[i].len() {
                        *g += x * self.w_gate[i][j];
                    }
                }
            }
        }
        self.activation.apply_vec(&mut gate);

        // Up
        let mut up = vec![0.0; self.d_ff];
        for (i, &x) in input.iter().enumerate() {
            if i < self.w_up.len() {
                for (j, u) in up.iter_mut().enumerate() {
                    if j < self.w_up[i].len() {
                        *u += x * self.w_up[i][j];
                    }
                }
            }
        }

        // Element-wise product
        for (g, u) in gate.iter_mut().zip(up.iter()) {
            *g *= u;
        }

        // Down
        let mut output = vec![0.0; self.d_model];
        for (i, &g) in gate.iter().enumerate() {
            if i < self.w_down.len() {
                for (j, o) in output.iter_mut().enumerate() {
                    if j < self.w_down[i].len() {
                        *o += g * self.w_down[i][j];
                    }
                }
            }
        }

        output
    }
}

// ============================================================================
// POSITIONAL ENCODING
// ============================================================================

/// Positional encoding type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosEncodingType {
    /// Sinusoidal (original transformer)
    Sinusoidal,
    /// Learned embeddings
    Learned,
    /// Rotary (RoPE)
    Rotary,
    /// ALiBi (Attention with Linear Biases)
    ALiBi,
}

/// Sinusoidal positional encoding
#[derive(Debug, Clone)]
pub struct SinusoidalPosEncoding {
    /// Maximum sequence length
    pub max_len: usize,
    /// Model dimension
    pub d_model: usize,
    /// Precomputed encodings
    pub encodings: Vec<Vec<f64>>,
}

impl SinusoidalPosEncoding {
    /// Create new positional encoding
    pub fn new(max_len: usize, d_model: usize) -> Self {
        let mut encodings = vec![vec![0.0; d_model]; max_len];

        for pos in 0..max_len {
            for i in 0..d_model / 2 {
                let angle = pos as f64 / libm::pow(10000.0, 2.0 * i as f64 / d_model as f64);
                encodings[pos][2 * i] = libm::sin(angle);
                encodings[pos][2 * i + 1] = libm::cos(angle);
            }
        }

        Self {
            max_len,
            d_model,
            encodings,
        }
    }

    /// Get encoding for position
    pub fn get(&self, position: usize) -> &[f64] {
        &self.encodings[position.min(self.max_len - 1)]
    }

    /// Add positional encoding to input
    pub fn encode(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        input
            .iter()
            .enumerate()
            .map(|(pos, vec)| {
                let enc = self.get(pos);
                vec.iter().zip(enc.iter()).map(|(&x, &e)| x + e).collect()
            })
            .collect()
    }
}

/// Learned positional embedding
#[derive(Debug, Clone)]
pub struct LearnedPosEmbedding {
    /// Maximum length
    pub max_len: usize,
    /// Embedding dimension
    pub d_model: usize,
    /// Position embeddings
    pub embeddings: Vec<Vec<f64>>,
}

impl LearnedPosEmbedding {
    /// Create new learned embeddings
    pub fn new(max_len: usize, d_model: usize, seed: u64) -> Self {
        let mut rng = seed;
        let scale = libm::sqrt(1.0 / d_model as f64);

        let embeddings: Vec<Vec<f64>> = (0..max_len)
            .map(|_| {
                (0..d_model)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale
                    })
                    .collect()
            })
            .collect();

        Self {
            max_len,
            d_model,
            embeddings,
        }
    }

    /// Encode input
    pub fn encode(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        input
            .iter()
            .enumerate()
            .map(|(pos, vec)| {
                let emb = &self.embeddings[pos.min(self.max_len - 1)];
                vec.iter().zip(emb.iter()).map(|(&x, &e)| x + e).collect()
            })
            .collect()
    }
}

// ============================================================================
// MULTI-HEAD ATTENTION (Simplified for Transformer)
// ============================================================================

/// Simplified multi-head attention for transformer
#[derive(Debug, Clone)]
pub struct TransformerAttention {
    /// Number of heads
    pub num_heads: usize,
    /// Model dimension
    pub d_model: usize,
    /// Head dimension
    pub d_head: usize,
    /// QKV projection
    pub w_qkv: Vec<Vec<f64>>,
    /// Output projection
    pub w_o: Vec<Vec<f64>>,
    /// Causal mask
    pub causal: bool,
}

impl TransformerAttention {
    /// Create new attention
    pub fn new(d_model: usize, num_heads: usize, causal: bool, seed: u64) -> Self {
        let mut rng = seed;
        let d_head = d_model / num_heads;
        let scale = libm::sqrt(2.0 / (d_model * 2) as f64);

        // QKV combined projection
        let w_qkv: Vec<Vec<f64>> = (0..d_model)
            .map(|_| {
                (0..d_model * 3)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale
                    })
                    .collect()
            })
            .collect();

        let w_o: Vec<Vec<f64>> = (0..d_model)
            .map(|_| {
                (0..d_model)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale
                    })
                    .collect()
            })
            .collect();

        Self {
            num_heads,
            d_model,
            d_head,
            w_qkv,
            w_o,
            causal,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>], kv_cache: Option<&[Vec<f64>]>) -> Vec<Vec<f64>> {
        let seq_len = input.len();
        if seq_len == 0 {
            return Vec::new();
        }

        // Project QKV
        let mut qkv: Vec<Vec<f64>> = vec![vec![0.0; self.d_model * 3]; seq_len];

        for (i, x) in input.iter().enumerate() {
            for (j, &xj) in x.iter().enumerate() {
                if j < self.w_qkv.len() {
                    for (k, qkv_val) in qkv[i].iter_mut().enumerate() {
                        if k < self.w_qkv[j].len() {
                            *qkv_val += xj * self.w_qkv[j][k];
                        }
                    }
                }
            }
        }

        // Split into Q, K, V
        let q: Vec<Vec<f64>> = qkv.iter().map(|row| row[..self.d_model].to_vec()).collect();
        let k: Vec<Vec<f64>> = qkv
            .iter()
            .map(|row| row[self.d_model..self.d_model * 2].to_vec())
            .collect();
        let v: Vec<Vec<f64>> = qkv
            .iter()
            .map(|row| row[self.d_model * 2..].to_vec())
            .collect();

        // Compute attention per head
        let scale = 1.0 / libm::sqrt(self.d_head as f64);
        let mut head_outputs = vec![vec![0.0; self.d_model]; seq_len];

        for h in 0..self.num_heads {
            let start = h * self.d_head;
            let end = start + self.d_head;

            // Extract head slices
            let q_h: Vec<Vec<f64>> = q
                .iter()
                .map(|row| row[start..end.min(row.len())].to_vec())
                .collect();
            let k_h: Vec<Vec<f64>> = k
                .iter()
                .map(|row| row[start..end.min(row.len())].to_vec())
                .collect();
            let v_h: Vec<Vec<f64>> = v
                .iter()
                .map(|row| row[start..end.min(row.len())].to_vec())
                .collect();

            // Attention scores
            let mut scores = vec![vec![0.0; seq_len]; seq_len];
            for i in 0..seq_len {
                for j in 0..seq_len {
                    if self.causal && j > i {
                        scores[i][j] = f64::NEG_INFINITY;
                    } else {
                        let dot: f64 = q_h[i]
                            .iter()
                            .zip(k_h[j].iter())
                            .map(|(&qi, &kj)| qi * kj)
                            .sum();
                        scores[i][j] = dot * scale;
                    }
                }
            }

            // Softmax
            for row in &mut scores {
                let max_val = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let sum: f64 = row.iter().map(|&s| libm::exp(s - max_val)).sum();
                for s in row {
                    *s = libm::exp(*s - max_val) / (sum + 1e-10);
                }
            }

            // Weighted sum
            for i in 0..seq_len {
                for j in 0..seq_len {
                    for (k, &vh) in v_h[j].iter().enumerate() {
                        let out_idx = start + k;
                        if out_idx < self.d_model {
                            head_outputs[i][out_idx] += scores[i][j] * vh;
                        }
                    }
                }
            }
        }

        // Output projection
        let mut output = vec![vec![0.0; self.d_model]; seq_len];
        for (i, ho) in head_outputs.iter().enumerate() {
            for (j, &h) in ho.iter().enumerate() {
                if j < self.w_o.len() {
                    for (k, o) in output[i].iter_mut().enumerate() {
                        if k < self.w_o[j].len() {
                            *o += h * self.w_o[j][k];
                        }
                    }
                }
            }
        }

        output
    }
}

// ============================================================================
// TRANSFORMER ENCODER
// ============================================================================

/// Encoder layer
#[derive(Debug, Clone)]
pub struct EncoderLayer {
    /// Self-attention
    pub attention: TransformerAttention,
    /// Feed-forward network
    pub ffn: FeedForward,
    /// Pre-attention layer norm
    pub norm1: LayerNorm,
    /// Pre-FFN layer norm
    pub norm2: LayerNorm,
    /// Pre-norm or post-norm
    pub pre_norm: bool,
}

impl EncoderLayer {
    /// Create new encoder layer
    pub fn new(d_model: usize, d_ff: usize, num_heads: usize, seed: u64) -> Self {
        Self {
            attention: TransformerAttention::new(d_model, num_heads, false, seed),
            ffn: FeedForward::new(d_model, d_ff, lcg_next(seed)),
            norm1: LayerNorm::new(d_model),
            norm2: LayerNorm::new(d_model),
            pre_norm: true,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        if self.pre_norm {
            // Pre-norm: norm -> attention -> residual -> norm -> ffn -> residual
            let normed1 = self.norm1.forward_batch(input);
            let attn_out = self.attention.forward(&normed1, None);

            // Residual
            let mut hidden: Vec<Vec<f64>> = input
                .iter()
                .zip(attn_out.iter())
                .map(|(x, a)| x.iter().zip(a.iter()).map(|(&xi, &ai)| xi + ai).collect())
                .collect();

            let normed2 = self.norm2.forward_batch(&hidden);
            let ffn_out = self.ffn.forward_batch(&normed2);

            // Residual
            for (h, f) in hidden.iter_mut().zip(ffn_out.iter()) {
                for (hi, &fi) in h.iter_mut().zip(f.iter()) {
                    *hi += fi;
                }
            }

            hidden
        } else {
            // Post-norm: attention -> residual -> norm -> ffn -> residual -> norm
            let attn_out = self.attention.forward(input, None);

            let mut hidden: Vec<Vec<f64>> = input
                .iter()
                .zip(attn_out.iter())
                .map(|(x, a)| x.iter().zip(a.iter()).map(|(&xi, &ai)| xi + ai).collect())
                .collect();

            hidden = self.norm1.forward_batch(&hidden);

            let ffn_out = self.ffn.forward_batch(&hidden);

            for (h, f) in hidden.iter_mut().zip(ffn_out.iter()) {
                for (hi, &fi) in h.iter_mut().zip(f.iter()) {
                    *hi += fi;
                }
            }

            self.norm2.forward_batch(&hidden)
        }
    }
}

/// Transformer Encoder
#[derive(Debug, Clone)]
pub struct TransformerEncoder {
    /// Layers
    pub layers: Vec<EncoderLayer>,
    /// Final layer norm
    pub final_norm: LayerNorm,
    /// Positional encoding
    pub pos_encoding: SinusoidalPosEncoding,
    /// Model dimension
    pub d_model: usize,
}

impl TransformerEncoder {
    /// Create new encoder
    pub fn new(
        d_model: usize,
        d_ff: usize,
        num_heads: usize,
        num_layers: usize,
        max_len: usize,
        seed: u64,
    ) -> Self {
        let mut rng = seed;

        let layers: Vec<EncoderLayer> = (0..num_layers)
            .map(|_| {
                rng = lcg_next(rng);
                EncoderLayer::new(d_model, d_ff, num_heads, rng)
            })
            .collect();

        Self {
            layers,
            final_norm: LayerNorm::new(d_model),
            pos_encoding: SinusoidalPosEncoding::new(max_len, d_model),
            d_model,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        // Add positional encoding
        let mut hidden = self.pos_encoding.encode(input);

        // Process through layers
        for layer in &self.layers {
            hidden = layer.forward(&hidden);
        }

        // Final normalization
        self.final_norm.forward_batch(&hidden)
    }
}

// ============================================================================
// TRANSFORMER DECODER
// ============================================================================

/// Decoder layer
#[derive(Debug, Clone)]
pub struct DecoderLayer {
    /// Masked self-attention
    pub self_attention: TransformerAttention,
    /// Cross-attention
    pub cross_attention: TransformerAttention,
    /// Feed-forward network
    pub ffn: FeedForward,
    /// Layer norms
    pub norm1: LayerNorm,
    pub norm2: LayerNorm,
    pub norm3: LayerNorm,
    /// Pre-norm
    pub pre_norm: bool,
}

impl DecoderLayer {
    /// Create new decoder layer
    pub fn new(d_model: usize, d_ff: usize, num_heads: usize, seed: u64) -> Self {
        let mut rng = seed;

        Self {
            self_attention: TransformerAttention::new(d_model, num_heads, true, rng),
            cross_attention: TransformerAttention::new(d_model, num_heads, false, lcg_next(rng)),
            ffn: FeedForward::new(d_model, d_ff, lcg_next(lcg_next(rng))),
            norm1: LayerNorm::new(d_model),
            norm2: LayerNorm::new(d_model),
            norm3: LayerNorm::new(d_model),
            pre_norm: true,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>], encoder_output: &[Vec<f64>]) -> Vec<Vec<f64>> {
        // Self-attention with causal mask
        let normed1 = self.norm1.forward_batch(input);
        let self_attn = self.self_attention.forward(&normed1, None);

        // Residual
        let mut hidden: Vec<Vec<f64>> = input
            .iter()
            .zip(self_attn.iter())
            .map(|(x, a)| x.iter().zip(a.iter()).map(|(&xi, &ai)| xi + ai).collect())
            .collect();

        // Cross-attention (simplified - using encoder output as KV)
        let normed2 = self.norm2.forward_batch(&hidden);
        let cross_attn = self.cross_attention.forward(&normed2, None);

        // Residual
        for (h, c) in hidden.iter_mut().zip(cross_attn.iter()) {
            for (hi, &ci) in h.iter_mut().zip(c.iter()) {
                *hi += ci;
            }
        }

        // FFN
        let normed3 = self.norm3.forward_batch(&hidden);
        let ffn_out = self.ffn.forward_batch(&normed3);

        // Residual
        for (h, f) in hidden.iter_mut().zip(ffn_out.iter()) {
            for (hi, &fi) in h.iter_mut().zip(f.iter()) {
                *hi += fi;
            }
        }

        hidden
    }
}

/// Transformer Decoder
#[derive(Debug, Clone)]
pub struct TransformerDecoder {
    /// Layers
    pub layers: Vec<DecoderLayer>,
    /// Final norm
    pub final_norm: LayerNorm,
    /// Positional encoding
    pub pos_encoding: SinusoidalPosEncoding,
    /// Model dimension
    pub d_model: usize,
}

impl TransformerDecoder {
    /// Create new decoder
    pub fn new(
        d_model: usize,
        d_ff: usize,
        num_heads: usize,
        num_layers: usize,
        max_len: usize,
        seed: u64,
    ) -> Self {
        let mut rng = seed;

        let layers: Vec<DecoderLayer> = (0..num_layers)
            .map(|_| {
                rng = lcg_next(rng);
                DecoderLayer::new(d_model, d_ff, num_heads, rng)
            })
            .collect();

        Self {
            layers,
            final_norm: LayerNorm::new(d_model),
            pos_encoding: SinusoidalPosEncoding::new(max_len, d_model),
            d_model,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>], encoder_output: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut hidden = self.pos_encoding.encode(input);

        for layer in &self.layers {
            hidden = layer.forward(&hidden, encoder_output);
        }

        self.final_norm.forward_batch(&hidden)
    }
}

// ============================================================================
// FULL TRANSFORMER (Encoder-Decoder)
// ============================================================================

/// Full Encoder-Decoder Transformer
#[derive(Debug, Clone)]
pub struct Transformer {
    /// Encoder
    pub encoder: TransformerEncoder,
    /// Decoder
    pub decoder: TransformerDecoder,
    /// Model dimension
    pub d_model: usize,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Output projection
    pub output_proj: Vec<Vec<f64>>,
}

impl Transformer {
    /// Create new transformer
    pub fn new(
        d_model: usize,
        d_ff: usize,
        num_heads: usize,
        num_layers: usize,
        vocab_size: usize,
        max_len: usize,
        seed: u64,
    ) -> Self {
        let mut rng = seed;

        let encoder = TransformerEncoder::new(d_model, d_ff, num_heads, num_layers, max_len, rng);
        rng = lcg_next(rng);
        let decoder = TransformerDecoder::new(d_model, d_ff, num_heads, num_layers, max_len, rng);

        // Output projection to vocabulary
        rng = lcg_next(rng);
        let scale = libm::sqrt(2.0 / (d_model + vocab_size) as f64);
        let output_proj: Vec<Vec<f64>> = (0..d_model)
            .map(|_| {
                (0..vocab_size)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale
                    })
                    .collect()
            })
            .collect();

        Self {
            encoder,
            decoder,
            d_model,
            vocab_size,
            output_proj,
        }
    }

    /// Encode source sequence
    pub fn encode(&self, source: &[Vec<f64>]) -> Vec<Vec<f64>> {
        self.encoder.forward(source)
    }

    /// Decode given encoder output
    pub fn decode(&self, target: &[Vec<f64>], encoder_output: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let hidden = self.decoder.forward(target, encoder_output);

        // Project to vocabulary
        hidden
            .iter()
            .map(|h| {
                let mut logits = vec![0.0; self.vocab_size];
                for (i, &hi) in h.iter().enumerate() {
                    if i < self.output_proj.len() {
                        for (j, l) in logits.iter_mut().enumerate() {
                            if j < self.output_proj[i].len() {
                                *l += hi * self.output_proj[i][j];
                            }
                        }
                    }
                }
                logits
            })
            .collect()
    }

    /// Full forward pass
    pub fn forward(&self, source: &[Vec<f64>], target: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let encoder_output = self.encode(source);
        self.decode(target, &encoder_output)
    }
}

// ============================================================================
// DECODER-ONLY TRANSFORMER (GPT-style)
// ============================================================================

/// Decoder-only transformer (GPT-style)
#[derive(Debug, Clone)]
pub struct DecoderOnlyTransformer {
    /// Layers
    pub layers: Vec<DecoderOnlyLayer>,
    /// Final norm
    pub final_norm: LayerNorm,
    /// Positional encoding
    pub pos_encoding: SinusoidalPosEncoding,
    /// Model dimension
    pub d_model: usize,
    /// Output projection
    pub output_proj: Vec<Vec<f64>>,
    /// Vocab size
    pub vocab_size: usize,
}

/// Decoder-only layer
#[derive(Debug, Clone)]
pub struct DecoderOnlyLayer {
    /// Causal self-attention
    pub attention: TransformerAttention,
    /// FFN
    pub ffn: GatedFFN,
    /// Norms
    pub norm1: RMSNorm,
    pub norm2: RMSNorm,
}

impl DecoderOnlyLayer {
    /// Create new layer
    pub fn new(d_model: usize, d_ff: usize, num_heads: usize, seed: u64) -> Self {
        Self {
            attention: TransformerAttention::new(d_model, num_heads, true, seed),
            ffn: GatedFFN::new(d_model, d_ff, lcg_next(seed)),
            norm1: RMSNorm::new(d_model),
            norm2: RMSNorm::new(d_model),
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        // Pre-norm self-attention
        let normed: Vec<Vec<f64>> = input.iter().map(|x| self.norm1.forward(x)).collect();
        let attn_out = self.attention.forward(&normed, None);

        // Residual
        let mut hidden: Vec<Vec<f64>> = input
            .iter()
            .zip(attn_out.iter())
            .map(|(x, a)| x.iter().zip(a.iter()).map(|(&xi, &ai)| xi + ai).collect())
            .collect();

        // Pre-norm FFN
        let normed2: Vec<Vec<f64>> = hidden.iter().map(|x| self.norm2.forward(x)).collect();
        let ffn_out: Vec<Vec<f64>> = normed2.iter().map(|x| self.ffn.forward(x)).collect();

        // Residual
        for (h, f) in hidden.iter_mut().zip(ffn_out.iter()) {
            for (hi, &fi) in h.iter_mut().zip(f.iter()) {
                *hi += fi;
            }
        }

        hidden
    }
}

impl DecoderOnlyTransformer {
    /// Create new decoder-only transformer
    pub fn new(
        d_model: usize,
        d_ff: usize,
        num_heads: usize,
        num_layers: usize,
        vocab_size: usize,
        max_len: usize,
        seed: u64,
    ) -> Self {
        let mut rng = seed;

        let layers: Vec<DecoderOnlyLayer> = (0..num_layers)
            .map(|_| {
                rng = lcg_next(rng);
                DecoderOnlyLayer::new(d_model, d_ff, num_heads, rng)
            })
            .collect();

        let scale = libm::sqrt(2.0 / (d_model + vocab_size) as f64);
        let output_proj: Vec<Vec<f64>> = (0..d_model)
            .map(|_| {
                (0..vocab_size)
                    .map(|_| {
                        rng = lcg_next(rng);
                        ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale
                    })
                    .collect()
            })
            .collect();

        Self {
            layers,
            final_norm: LayerNorm::new(d_model),
            pos_encoding: SinusoidalPosEncoding::new(max_len, d_model),
            d_model,
            output_proj,
            vocab_size,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut hidden = self.pos_encoding.encode(input);

        for layer in &self.layers {
            hidden = layer.forward(&hidden);
        }

        hidden = self.final_norm.forward_batch(&hidden);

        // Project to vocabulary
        hidden
            .iter()
            .map(|h| {
                let mut logits = vec![0.0; self.vocab_size];
                for (i, &hi) in h.iter().enumerate() {
                    if i < self.output_proj.len() {
                        for (j, l) in logits.iter_mut().enumerate() {
                            if j < self.output_proj[i].len() {
                                *l += hi * self.output_proj[i][j];
                            }
                        }
                    }
                }
                logits
            })
            .collect()
    }
}

// ============================================================================
// KERNEL TRANSFORMER MANAGER
// ============================================================================

/// Kernel transformer manager
pub struct KernelTransformerManager {
    /// Encoder for understanding
    pub encoder: TransformerEncoder,
    /// Decoder for generation
    pub decoder: DecoderOnlyTransformer,
    /// Model dimension
    pub d_model: usize,
    /// Forward passes
    pub forward_count: u64,
}

impl KernelTransformerManager {
    /// Create new manager
    pub fn new(
        d_model: usize,
        d_ff: usize,
        num_heads: usize,
        num_layers: usize,
        vocab_size: usize,
    ) -> Self {
        Self {
            encoder: TransformerEncoder::new(d_model, d_ff, num_heads, num_layers, 512, 12345),
            decoder: DecoderOnlyTransformer::new(
                d_model, d_ff, num_heads, num_layers, vocab_size, 512, 54321,
            ),
            d_model,
            forward_count: 0,
        }
    }

    /// Encode sequence
    pub fn encode(&mut self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        self.forward_count += 1;
        self.encoder.forward(input)
    }

    /// Generate logits
    pub fn generate(&mut self, input: &[Vec<f64>]) -> Vec<Vec<f64>> {
        self.forward_count += 1;
        self.decoder.forward(input)
    }

    /// Get statistics
    pub fn get_stats(&self) -> TransformerStats {
        TransformerStats {
            d_model: self.d_model,
            encoder_layers: self.encoder.layers.len(),
            decoder_layers: self.decoder.layers.len(),
            forward_count: self.forward_count,
        }
    }
}

/// Transformer statistics
#[derive(Debug, Clone)]
pub struct TransformerStats {
    pub d_model: usize,
    pub encoder_layers: usize,
    pub decoder_layers: usize,
    pub forward_count: u64,
}

// ============================================================================
// UTILITIES
// ============================================================================

fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_input(seq_len: usize, d_model: usize, seed: u64) -> Vec<Vec<f64>> {
        let mut rng = seed;
        (0..seq_len)
            .map(|_| {
                (0..d_model)
                    .map(|_| {
                        rng = lcg_next(rng);
                        (rng as f64 / u64::MAX as f64) - 0.5
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn test_layer_norm() {
        let ln = LayerNorm::new(64);
        let input: Vec<f64> = (0..64).map(|i| i as f64 / 10.0).collect();

        let output = ln.forward(&input);

        assert_eq!(output.len(), 64);

        // Check mean ≈ 0
        let mean: f64 = output.iter().sum::<f64>() / output.len() as f64;
        assert!(mean.abs() < 1e-5);
    }

    #[test]
    fn test_rms_norm() {
        let rn = RMSNorm::new(64);
        let input: Vec<f64> = (0..64).map(|i| i as f64 / 10.0).collect();

        let output = rn.forward(&input);

        assert_eq!(output.len(), 64);
    }

    #[test]
    fn test_feed_forward() {
        let ffn = FeedForward::new(64, 256, 12345);
        let input: Vec<f64> = (0..64).map(|i| i as f64 / 100.0).collect();

        let output = ffn.forward(&input);

        assert_eq!(output.len(), 64);
    }

    #[test]
    fn test_gated_ffn() {
        let gffn = GatedFFN::new(64, 256, 12345);
        let input: Vec<f64> = (0..64).map(|i| i as f64 / 100.0).collect();

        let output = gffn.forward(&input);

        assert_eq!(output.len(), 64);
    }

    #[test]
    fn test_sinusoidal_encoding() {
        let pe = SinusoidalPosEncoding::new(100, 64);
        let input = create_test_input(10, 64, 12345);

        let encoded = pe.encode(&input);

        assert_eq!(encoded.len(), 10);
        assert_eq!(encoded[0].len(), 64);
    }

    #[test]
    fn test_transformer_attention() {
        let attn = TransformerAttention::new(64, 4, false, 12345);
        let input = create_test_input(8, 64, 12345);

        let output = attn.forward(&input, None);

        assert_eq!(output.len(), 8);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_causal_attention() {
        let attn = TransformerAttention::new(64, 4, true, 12345);
        let input = create_test_input(8, 64, 12345);

        let output = attn.forward(&input, None);

        assert_eq!(output.len(), 8);
    }

    #[test]
    fn test_encoder_layer() {
        let layer = EncoderLayer::new(64, 256, 4, 12345);
        let input = create_test_input(8, 64, 12345);

        let output = layer.forward(&input);

        assert_eq!(output.len(), 8);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_transformer_encoder() {
        let encoder = TransformerEncoder::new(64, 256, 4, 2, 100, 12345);
        let input = create_test_input(8, 64, 12345);

        let output = encoder.forward(&input);

        assert_eq!(output.len(), 8);
        assert_eq!(output[0].len(), 64);
    }

    #[test]
    fn test_decoder_layer() {
        let layer = DecoderLayer::new(64, 256, 4, 12345);
        let input = create_test_input(8, 64, 12345);
        let encoder_out = create_test_input(10, 64, 54321);

        let output = layer.forward(&input, &encoder_out);

        assert_eq!(output.len(), 8);
    }

    #[test]
    fn test_transformer_decoder() {
        let decoder = TransformerDecoder::new(64, 256, 4, 2, 100, 12345);
        let input = create_test_input(8, 64, 12345);
        let encoder_out = create_test_input(10, 64, 54321);

        let output = decoder.forward(&input, &encoder_out);

        assert_eq!(output.len(), 8);
    }

    #[test]
    fn test_full_transformer() {
        let transformer = Transformer::new(64, 256, 4, 2, 1000, 100, 12345);
        let source = create_test_input(10, 64, 12345);
        let target = create_test_input(8, 64, 54321);

        let logits = transformer.forward(&source, &target);

        assert_eq!(logits.len(), 8);
        assert_eq!(logits[0].len(), 1000);
    }

    #[test]
    fn test_decoder_only() {
        let decoder = DecoderOnlyTransformer::new(64, 256, 4, 2, 1000, 100, 12345);
        let input = create_test_input(8, 64, 12345);

        let logits = decoder.forward(&input);

        assert_eq!(logits.len(), 8);
        assert_eq!(logits[0].len(), 1000);
    }

    #[test]
    fn test_kernel_transformer_manager() {
        let mut manager = KernelTransformerManager::new(64, 256, 4, 2, 1000);
        let input = create_test_input(8, 64, 12345);

        let _ = manager.encode(&input);
        let _ = manager.generate(&input);

        assert_eq!(manager.forward_count, 2);
    }

    #[test]
    fn test_activation_functions() {
        let x = 0.5;

        assert!(Activation::Relu.apply(x) > 0.0);
        assert!(Activation::Gelu.apply(x) > 0.0);
        assert!(Activation::Silu.apply(x) > 0.0);
        assert!(Activation::Tanh.apply(x) > 0.0);
        assert!(Activation::Sigmoid.apply(x) > 0.0);

        assert_eq!(Activation::Relu.apply(-1.0), 0.0);
    }
}
