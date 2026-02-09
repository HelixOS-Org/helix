//! # GPT-Style Models
//!
//! Decoder-only language models (GPT, LLaMA, etc.)

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::vec::Vec;

use super::layers::{GatedFFN, GroupedQueryAttention, MultiHeadSelfAttention, PreNormBlock};
use super::types::{
    ActivationType, Dropout, Embedding, LayerNorm, Linear, PositionalEmbedding, RMSNorm, Tensor2,
    TransformerConfig,
};

// ============================================================================
// GPT MODEL
// ============================================================================

/// GPT-style decoder-only transformer
pub struct GPT {
    /// Token embedding
    pub token_embed: Embedding,
    /// Position embedding
    pub pos_embed: PositionalEmbedding,
    /// Dropout
    pub dropout: Dropout,
    /// Transformer blocks
    pub blocks: Vec<PreNormBlock>,
    /// Final layer norm
    pub final_norm: LayerNorm,
    /// LM head (output projection)
    pub lm_head: Linear,
    /// Config
    pub config: TransformerConfig,
}

impl GPT {
    /// Create new GPT model
    pub fn new(config: TransformerConfig, seed: u64) -> Self {
        let mut blocks = Vec::with_capacity(config.n_layers);

        for i in 0..config.n_layers {
            blocks.push(PreNormBlock::new(
                &config,
                seed.wrapping_add(i as u64 * 1000),
            ));
        }

        Self {
            token_embed: Embedding::new(config.vocab_size, config.d_model, seed),
            pos_embed: PositionalEmbedding::learnable(
                config.max_seq_len,
                config.d_model,
                seed.wrapping_add(1),
            ),
            dropout: Dropout::new(config.dropout),
            blocks,
            final_norm: LayerNorm::new(config.d_model, config.layer_norm_eps),
            lm_head: Linear::new(
                config.d_model,
                config.vocab_size,
                false,
                seed.wrapping_add(2),
            ),
            config,
        }
    }

    /// Create GPT-2 small
    #[inline(always)]
    pub fn gpt2_small(seed: u64) -> Self {
        Self::new(TransformerConfig::gpt_small(), seed)
    }

    /// Forward pass
    pub fn forward(&mut self, input_ids: &[usize], training: bool) -> Tensor2 {
        let seq_len = input_ids.len();

        // Embeddings
        let token_embeddings = self.token_embed.forward(input_ids);
        let pos_embeddings = self.pos_embed.forward(seq_len);
        let mut hidden = token_embeddings
            .add(&pos_embeddings)
            .unwrap_or(token_embeddings);

        hidden = self.dropout.forward(&hidden, training);

        // Causal mask
        let causal_mask = MultiHeadSelfAttention::causal_mask(seq_len);

        // Transformer blocks
        for block in &mut self.blocks {
            hidden = block.forward(&hidden, Some(&causal_mask), training);
        }

        // Final norm and LM head
        hidden = self.final_norm.forward(&hidden);
        self.lm_head.forward(&hidden)
    }

    /// Get hidden states without LM head
    pub fn forward_hidden(&mut self, input_ids: &[usize], training: bool) -> Tensor2 {
        let seq_len = input_ids.len();

        let token_embeddings = self.token_embed.forward(input_ids);
        let pos_embeddings = self.pos_embed.forward(seq_len);
        let mut hidden = token_embeddings
            .add(&pos_embeddings)
            .unwrap_or(token_embeddings);

        hidden = self.dropout.forward(&hidden, training);

        let causal_mask = MultiHeadSelfAttention::causal_mask(seq_len);

        for block in &mut self.blocks {
            hidden = block.forward(&hidden, Some(&causal_mask), training);
        }

        self.final_norm.forward(&hidden)
    }

    /// Generate tokens autoregressively
    pub fn generate(
        &mut self,
        input_ids: &[usize],
        max_new_tokens: usize,
        temperature: f64,
    ) -> Vec<usize> {
        let mut generated = input_ids.to_vec();
        let mut rng_state = 42u64;

        for _ in 0..max_new_tokens {
            if generated.len() >= self.config.max_seq_len {
                break;
            }

            // Forward pass
            let logits = self.forward(&generated, false);

            // Get last token logits
            let last_idx = logits.rows - 1;
            let mut probs = Vec::with_capacity(self.config.vocab_size);

            // Apply temperature and compute probabilities
            let mut max_logit = f64::NEG_INFINITY;
            for j in 0..logits.cols {
                let logit = logits.get(last_idx, j) / temperature;
                max_logit = max_logit.max(logit);
            }

            let mut sum_exp = 0.0;
            for j in 0..logits.cols {
                let logit = logits.get(last_idx, j) / temperature;
                let exp_val = libm::exp(logit - max_logit);
                probs.push(exp_val);
                sum_exp += exp_val;
            }

            for p in &mut probs {
                *p /= sum_exp;
            }

            // Sample from distribution
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let r = (rng_state >> 33) as f64 / (1u64 << 31) as f64;

            let mut cumsum = 0.0;
            let mut next_token = 0;
            for (idx, &p) in probs.iter().enumerate() {
                cumsum += p;
                if r < cumsum {
                    next_token = idx;
                    break;
                }
            }

            generated.push(next_token);
        }

        generated
    }

    /// Greedy generation
    pub fn generate_greedy(&mut self, input_ids: &[usize], max_new_tokens: usize) -> Vec<usize> {
        let mut generated = input_ids.to_vec();

        for _ in 0..max_new_tokens {
            if generated.len() >= self.config.max_seq_len {
                break;
            }

            let logits = self.forward(&generated, false);
            let last_idx = logits.rows - 1;

            // Find argmax
            let mut max_idx = 0;
            let mut max_val = f64::NEG_INFINITY;
            for j in 0..logits.cols {
                if logits.get(last_idx, j) > max_val {
                    max_val = logits.get(last_idx, j);
                    max_idx = j;
                }
            }

            generated.push(max_idx);
        }

        generated
    }
}

// ============================================================================
// LLAMA MODEL
// ============================================================================

/// LLaMA block with RMSNorm, GQA, and SwiGLU
pub struct LlamaLayer {
    /// Attention norm
    pub attn_norm: RMSNorm,
    /// Grouped query attention
    pub attn: GroupedQueryAttention,
    /// FFN norm
    pub ffn_norm: RMSNorm,
    /// Gated FFN with SiLU
    pub ffn: GatedFFN,
}

impl LlamaLayer {
    /// Create new LLaMA layer
    pub fn new(d_model: usize, n_heads: usize, n_kv_heads: usize, d_ff: usize, seed: u64) -> Self {
        Self {
            attn_norm: RMSNorm::new(d_model, 1e-5),
            attn: GroupedQueryAttention::new(d_model, n_heads, n_kv_heads, seed),
            ffn_norm: RMSNorm::new(d_model, 1e-5),
            ffn: GatedFFN::new(d_model, d_ff, ActivationType::Silu, seed.wrapping_add(100)),
        }
    }

    /// Forward pass
    pub fn forward(&self, x: &Tensor2) -> Tensor2 {
        // Attention with pre-norm
        let normed1 = self.attn_norm.forward(x);
        let attn_out = self.attn.forward(&normed1, true);
        let residual1 = x.add(&attn_out).unwrap_or_else(|| x.clone());

        // FFN with pre-norm
        let normed2 = self.ffn_norm.forward(&residual1);
        let ffn_out = self.ffn.forward(&normed2);

        residual1.add(&ffn_out).unwrap_or(residual1)
    }
}

/// LLaMA-style model
pub struct Llama {
    /// Token embedding
    pub embed: Embedding,
    /// Layers
    pub layers: Vec<LlamaLayer>,
    /// Final RMS norm
    pub norm: RMSNorm,
    /// Output projection
    pub lm_head: Linear,
    /// Model dimension
    pub d_model: usize,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Max sequence length
    pub max_seq_len: usize,
}

/// LLaMA configuration
#[derive(Debug, Clone)]
pub struct LlamaConfig {
    /// Model dimension
    pub d_model: usize,
    /// Number of attention heads
    pub n_heads: usize,
    /// Number of KV heads (for GQA)
    pub n_kv_heads: usize,
    /// FFN dimension (typically 8/3 * d_model)
    pub d_ff: usize,
    /// Number of layers
    pub n_layers: usize,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Maximum sequence length
    pub max_seq_len: usize,
}

impl Default for LlamaConfig {
    fn default() -> Self {
        Self {
            d_model: 4096,
            n_heads: 32,
            n_kv_heads: 8,
            d_ff: 11008,
            n_layers: 32,
            vocab_size: 32000,
            max_seq_len: 4096,
        }
    }
}

impl LlamaConfig {
    /// LLaMA 7B config
    #[inline(always)]
    pub fn llama_7b() -> Self {
        Self::default()
    }

    /// LLaMA 13B config
    #[inline]
    pub fn llama_13b() -> Self {
        Self {
            d_model: 5120,
            n_heads: 40,
            n_kv_heads: 8,
            d_ff: 13824,
            n_layers: 40,
            vocab_size: 32000,
            max_seq_len: 4096,
        }
    }

    /// Tiny config for testing
    #[inline]
    pub fn tiny() -> Self {
        Self {
            d_model: 64,
            n_heads: 4,
            n_kv_heads: 2,
            d_ff: 128,
            n_layers: 2,
            vocab_size: 1000,
            max_seq_len: 128,
        }
    }
}

impl Llama {
    /// Create new LLaMA model
    pub fn new(config: LlamaConfig, seed: u64) -> Self {
        let mut layers = Vec::with_capacity(config.n_layers);

        for i in 0..config.n_layers {
            layers.push(LlamaLayer::new(
                config.d_model,
                config.n_heads,
                config.n_kv_heads,
                config.d_ff,
                seed.wrapping_add(i as u64 * 1000),
            ));
        }

        Self {
            embed: Embedding::new(config.vocab_size, config.d_model, seed),
            layers,
            norm: RMSNorm::new(config.d_model, 1e-5),
            lm_head: Linear::new(
                config.d_model,
                config.vocab_size,
                false,
                seed.wrapping_add(1),
            ),
            d_model: config.d_model,
            vocab_size: config.vocab_size,
            max_seq_len: config.max_seq_len,
        }
    }

    /// Forward pass
    #[inline]
    pub fn forward(&self, input_ids: &[usize]) -> Tensor2 {
        let mut hidden = self.embed.forward(input_ids);

        for layer in &self.layers {
            hidden = layer.forward(&hidden);
        }

        hidden = self.norm.forward(&hidden);
        self.lm_head.forward(&hidden)
    }

    /// Generate tokens
    pub fn generate(&self, input_ids: &[usize], max_new_tokens: usize) -> Vec<usize> {
        let mut generated = input_ids.to_vec();

        for _ in 0..max_new_tokens {
            if generated.len() >= self.max_seq_len {
                break;
            }

            let logits = self.forward(&generated);
            let last_idx = logits.rows - 1;

            // Greedy decode
            let mut max_idx = 0;
            let mut max_val = f64::NEG_INFINITY;
            for j in 0..logits.cols {
                if logits.get(last_idx, j) > max_val {
                    max_val = logits.get(last_idx, j);
                    max_idx = j;
                }
            }

            generated.push(max_idx);
        }

        generated
    }
}

// ============================================================================
// MISTRAL MODEL
// ============================================================================

/// Mistral-style model with sliding window attention
pub struct Mistral {
    /// Token embedding
    pub embed: Embedding,
    /// Layers
    pub layers: Vec<MistralLayer>,
    /// Final norm
    pub norm: RMSNorm,
    /// LM head
    pub lm_head: Linear,
    /// Sliding window size
    pub window_size: usize,
    /// Model dimension
    pub d_model: usize,
    /// Vocab size
    pub vocab_size: usize,
}

/// Mistral layer with sliding window attention
pub struct MistralLayer {
    /// Attention norm
    pub attn_norm: RMSNorm,
    /// Sliding window attention
    pub attn: SlidingWindowAttention,
    /// FFN norm
    pub ffn_norm: RMSNorm,
    /// Gated FFN
    pub ffn: GatedFFN,
}

/// Sliding window attention
pub struct SlidingWindowAttention {
    /// Query projection
    pub wq: Linear,
    /// Key projection
    pub wk: Linear,
    /// Value projection
    pub wv: Linear,
    /// Output projection
    pub wo: Linear,
    /// Number of heads
    pub n_heads: usize,
    /// Number of KV heads
    pub n_kv_heads: usize,
    /// Head dimension
    pub head_dim: usize,
    /// Window size
    pub window_size: usize,
    /// Scale
    pub scale: f64,
}

impl SlidingWindowAttention {
    /// Create sliding window attention
    pub fn new(
        d_model: usize,
        n_heads: usize,
        n_kv_heads: usize,
        window_size: usize,
        seed: u64,
    ) -> Self {
        let head_dim = d_model / n_heads;
        let kv_dim = n_kv_heads * head_dim;

        Self {
            wq: Linear::new(d_model, d_model, false, seed),
            wk: Linear::new(d_model, kv_dim, false, seed.wrapping_add(1)),
            wv: Linear::new(d_model, kv_dim, false, seed.wrapping_add(2)),
            wo: Linear::new(d_model, d_model, false, seed.wrapping_add(3)),
            n_heads,
            n_kv_heads,
            head_dim,
            window_size,
            scale: 1.0 / libm::sqrt(head_dim as f64),
        }
    }

    /// Forward with sliding window
    pub fn forward(&self, x: &Tensor2) -> Tensor2 {
        let seq_len = x.rows;
        let d_model = self.n_heads * self.head_dim;

        let q = self.wq.forward(x);
        let k = self.wk.forward(x);
        let v = self.wv.forward(x);

        let heads_per_kv = self.n_heads / self.n_kv_heads;
        let mut output = Tensor2::new(seq_len, d_model);

        for h in 0..self.n_heads {
            let kv_h = h / heads_per_kv;

            for qi in 0..seq_len {
                // Sliding window range
                let window_start = qi.saturating_sub(self.window_size);
                let window_end = qi + 1; // Causal

                let mut max_score = f64::NEG_INFINITY;
                let mut scores = Vec::with_capacity(window_end - window_start);

                // Compute attention scores within window
                for ki in window_start..window_end {
                    let mut score = 0.0;
                    for d in 0..self.head_dim {
                        score +=
                            q.get(qi, h * self.head_dim + d) * k.get(ki, kv_h * self.head_dim + d);
                    }
                    score *= self.scale;
                    max_score = max_score.max(score);
                    scores.push((ki, score));
                }

                // Softmax
                let mut sum_exp = 0.0;
                for (_, score) in &mut scores {
                    *score = libm::exp(*score - max_score);
                    sum_exp += *score;
                }

                // Weighted sum
                for d in 0..self.head_dim {
                    let mut acc = 0.0;
                    for (ki, score) in &scores {
                        acc += (*score / sum_exp.max(1e-10)) * v.get(*ki, kv_h * self.head_dim + d);
                    }
                    output.set(qi, h * self.head_dim + d, acc);
                }
            }
        }

        self.wo.forward(&output)
    }
}

impl MistralLayer {
    /// Create Mistral layer
    pub fn new(
        d_model: usize,
        n_heads: usize,
        n_kv_heads: usize,
        d_ff: usize,
        window_size: usize,
        seed: u64,
    ) -> Self {
        Self {
            attn_norm: RMSNorm::new(d_model, 1e-5),
            attn: SlidingWindowAttention::new(d_model, n_heads, n_kv_heads, window_size, seed),
            ffn_norm: RMSNorm::new(d_model, 1e-5),
            ffn: GatedFFN::new(d_model, d_ff, ActivationType::Silu, seed.wrapping_add(100)),
        }
    }

    /// Forward pass
    #[inline]
    pub fn forward(&self, x: &Tensor2) -> Tensor2 {
        let normed1 = self.attn_norm.forward(x);
        let attn_out = self.attn.forward(&normed1);
        let residual1 = x.add(&attn_out).unwrap_or_else(|| x.clone());

        let normed2 = self.ffn_norm.forward(&residual1);
        let ffn_out = self.ffn.forward(&normed2);

        residual1.add(&ffn_out).unwrap_or(residual1)
    }
}

/// Mistral configuration
#[derive(Debug, Clone)]
pub struct MistralConfig {
    pub d_model: usize,
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub d_ff: usize,
    pub n_layers: usize,
    pub vocab_size: usize,
    pub window_size: usize,
}

impl Default for MistralConfig {
    fn default() -> Self {
        Self {
            d_model: 4096,
            n_heads: 32,
            n_kv_heads: 8,
            d_ff: 14336,
            n_layers: 32,
            vocab_size: 32000,
            window_size: 4096,
        }
    }
}

impl MistralConfig {
    /// Tiny config for testing
    #[inline]
    pub fn tiny() -> Self {
        Self {
            d_model: 64,
            n_heads: 4,
            n_kv_heads: 2,
            d_ff: 128,
            n_layers: 2,
            vocab_size: 1000,
            window_size: 32,
        }
    }
}

impl Mistral {
    /// Create new Mistral model
    pub fn new(config: MistralConfig, seed: u64) -> Self {
        let mut layers = Vec::with_capacity(config.n_layers);

        for i in 0..config.n_layers {
            layers.push(MistralLayer::new(
                config.d_model,
                config.n_heads,
                config.n_kv_heads,
                config.d_ff,
                config.window_size,
                seed.wrapping_add(i as u64 * 1000),
            ));
        }

        Self {
            embed: Embedding::new(config.vocab_size, config.d_model, seed),
            layers,
            norm: RMSNorm::new(config.d_model, 1e-5),
            lm_head: Linear::new(
                config.d_model,
                config.vocab_size,
                false,
                seed.wrapping_add(1),
            ),
            window_size: config.window_size,
            d_model: config.d_model,
            vocab_size: config.vocab_size,
        }
    }

    /// Forward pass
    #[inline]
    pub fn forward(&self, input_ids: &[usize]) -> Tensor2 {
        let mut hidden = self.embed.forward(input_ids);

        for layer in &self.layers {
            hidden = layer.forward(&hidden);
        }

        hidden = self.norm.forward(&hidden);
        self.lm_head.forward(&hidden)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpt() {
        let config = TransformerConfig::tiny();
        let mut gpt = GPT::new(config, 42);

        let input_ids = alloc::vec![1, 2, 3, 4, 5];
        let logits = gpt.forward(&input_ids, false);

        assert_eq!(logits.rows, 5);
        assert_eq!(logits.cols, 1000); // vocab_size
    }

    #[test]
    fn test_gpt_generate() {
        let config = TransformerConfig::tiny();
        let mut gpt = GPT::new(config, 42);

        let input_ids = alloc::vec![1, 2, 3];
        let generated = gpt.generate_greedy(&input_ids, 5);

        assert_eq!(generated.len(), 8); // 3 input + 5 generated
    }

    #[test]
    fn test_llama() {
        let config = LlamaConfig::tiny();
        let llama = Llama::new(config, 42);

        let input_ids = alloc::vec![1, 2, 3, 4, 5];
        let logits = llama.forward(&input_ids);

        assert_eq!(logits.rows, 5);
        assert_eq!(logits.cols, 1000);
    }

    #[test]
    fn test_llama_generate() {
        let config = LlamaConfig::tiny();
        let llama = Llama::new(config, 42);

        let input_ids = alloc::vec![1, 2, 3];
        let generated = llama.generate(&input_ids, 5);

        assert_eq!(generated.len(), 8);
    }

    #[test]
    fn test_sliding_window_attention() {
        let attn = SlidingWindowAttention::new(64, 4, 2, 8, 42);

        let input = Tensor2::random(20, 64, 43);
        let output = attn.forward(&input);

        assert_eq!(output.rows, 20);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_mistral() {
        let config = MistralConfig::tiny();
        let mistral = Mistral::new(config, 42);

        let input_ids = alloc::vec![1, 2, 3, 4, 5];
        let logits = mistral.forward(&input_ids);

        assert_eq!(logits.rows, 5);
        assert_eq!(logits.cols, 1000);
    }
}
