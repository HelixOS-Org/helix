//! # Transformer Decoder
//!
//! Autoregressive decoder architecture with cross-attention.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::vec::Vec;

use super::layers::{FeedForward, MultiHeadSelfAttention, PreNormBlock};
use super::types::{
    Dropout, Embedding, LayerNorm, Linear, PositionalEmbedding, Tensor2, TransformerConfig,
};

// ============================================================================
// CROSS-ATTENTION
// ============================================================================

/// Multi-head cross-attention
pub struct MultiHeadCrossAttention {
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
    /// Head dimension
    pub head_dim: usize,
    /// Model dimension
    pub d_model: usize,
    /// Scale factor
    pub scale: f64,
    /// Dropout
    pub dropout: Dropout,
}

impl MultiHeadCrossAttention {
    /// Create new cross-attention
    pub fn new(d_model: usize, n_heads: usize, dropout: f64, seed: u64) -> Self {
        let head_dim = d_model / n_heads;

        Self {
            wq: Linear::new(d_model, d_model, false, seed),
            wk: Linear::new(d_model, d_model, false, seed.wrapping_add(1)),
            wv: Linear::new(d_model, d_model, false, seed.wrapping_add(2)),
            wo: Linear::new(d_model, d_model, false, seed.wrapping_add(3)),
            n_heads,
            head_dim,
            d_model,
            scale: 1.0 / libm::sqrt(head_dim as f64),
            dropout: Dropout::new(dropout),
        }
    }

    /// Forward pass
    ///
    /// - `x`: decoder hidden state (query source)
    /// - `encoder_output`: encoder hidden state (key/value source)
    pub fn forward(
        &mut self,
        x: &Tensor2,
        encoder_output: &Tensor2,
        encoder_mask: Option<&Tensor2>,
        training: bool,
    ) -> Tensor2 {
        let tgt_len = x.rows;
        let src_len = encoder_output.rows;

        // Q from decoder, K/V from encoder
        let q = self.wq.forward(x);
        let k = self.wk.forward(encoder_output);
        let v = self.wv.forward(encoder_output);

        let mut head_outputs = Vec::with_capacity(self.n_heads);

        for h in 0..self.n_heads {
            // Extract head
            let mut q_h = Tensor2::new(tgt_len, self.head_dim);
            let mut k_h = Tensor2::new(src_len, self.head_dim);
            let mut v_h = Tensor2::new(src_len, self.head_dim);

            for i in 0..tgt_len {
                for j in 0..self.head_dim {
                    q_h.set(i, j, q.get(i, h * self.head_dim + j));
                }
            }
            for i in 0..src_len {
                for j in 0..self.head_dim {
                    k_h.set(i, j, k.get(i, h * self.head_dim + j));
                    v_h.set(i, j, v.get(i, h * self.head_dim + j));
                }
            }

            // Attention scores
            let k_t = k_h.transpose();
            let mut scores = q_h
                .matmul(&k_t)
                .unwrap_or_else(|| Tensor2::new(tgt_len, src_len));
            scores = scores.scale(self.scale);

            // Apply encoder mask
            if let Some(mask) = encoder_mask {
                for i in 0..tgt_len {
                    for j in 0..src_len {
                        if mask.get(0, j.min(mask.cols.saturating_sub(1))) == 0.0 {
                            scores.set(i, j, f64::NEG_INFINITY);
                        }
                    }
                }
            }

            let attn_weights = scores.softmax();
            let attn_weights = self.dropout.forward(&attn_weights, training);

            let head_out = attn_weights
                .matmul(&v_h)
                .unwrap_or_else(|| Tensor2::new(tgt_len, self.head_dim));

            head_outputs.push(head_out);
        }

        // Concatenate
        let mut concat = Tensor2::new(tgt_len, self.d_model);
        for h in 0..self.n_heads {
            for i in 0..tgt_len {
                for j in 0..self.head_dim {
                    concat.set(i, h * self.head_dim + j, head_outputs[h].get(i, j));
                }
            }
        }

        self.wo.forward(&concat)
    }
}

// ============================================================================
// DECODER LAYER
// ============================================================================

/// Decoder layer with self-attention and cross-attention
pub struct DecoderLayer {
    /// Self-attention
    pub self_attn: MultiHeadSelfAttention,
    /// Layer norm after self-attention
    pub ln1: LayerNorm,
    /// Cross-attention
    pub cross_attn: MultiHeadCrossAttention,
    /// Layer norm after cross-attention
    pub ln2: LayerNorm,
    /// FFN
    pub ffn: FeedForward,
    /// Layer norm after FFN
    pub ln3: LayerNorm,
    /// Dropout
    pub dropout: Dropout,
    /// Use pre-norm
    pub pre_norm: bool,
}

impl DecoderLayer {
    /// Create new decoder layer
    pub fn new(config: &TransformerConfig, seed: u64) -> Self {
        Self {
            self_attn: MultiHeadSelfAttention::from_config(config, seed),
            ln1: LayerNorm::new(config.d_model, config.layer_norm_eps),
            cross_attn: MultiHeadCrossAttention::new(
                config.d_model,
                config.n_heads,
                config.dropout,
                seed.wrapping_add(100),
            ),
            ln2: LayerNorm::new(config.d_model, config.layer_norm_eps),
            ffn: FeedForward::from_config(config, seed.wrapping_add(200)),
            ln3: LayerNorm::new(config.d_model, config.layer_norm_eps),
            dropout: Dropout::new(config.dropout),
            pre_norm: config.pre_norm,
        }
    }

    /// Forward pass
    #[inline]
    pub fn forward(
        &mut self,
        x: &Tensor2,
        encoder_output: &Tensor2,
        self_attn_mask: Option<&Tensor2>,
        cross_attn_mask: Option<&Tensor2>,
        training: bool,
    ) -> Tensor2 {
        if self.pre_norm {
            self.forward_prenorm(x, encoder_output, self_attn_mask, cross_attn_mask, training)
        } else {
            self.forward_postnorm(x, encoder_output, self_attn_mask, cross_attn_mask, training)
        }
    }

    fn forward_prenorm(
        &mut self,
        x: &Tensor2,
        encoder_output: &Tensor2,
        self_attn_mask: Option<&Tensor2>,
        cross_attn_mask: Option<&Tensor2>,
        training: bool,
    ) -> Tensor2 {
        // Self-attention
        let normed1 = self.ln1.forward(x);
        let self_attn_out = self.self_attn.forward(&normed1, self_attn_mask, training);
        let self_attn_out = self.dropout.forward(&self_attn_out, training);
        let residual1 = x.add(&self_attn_out).unwrap_or_else(|| x.clone());

        // Cross-attention
        let normed2 = self.ln2.forward(&residual1);
        let cross_attn_out =
            self.cross_attn
                .forward(&normed2, encoder_output, cross_attn_mask, training);
        let cross_attn_out = self.dropout.forward(&cross_attn_out, training);
        let residual2 = residual1.add(&cross_attn_out).unwrap_or(residual1);

        // FFN
        let normed3 = self.ln3.forward(&residual2);
        let ffn_out = self.ffn.forward(&normed3, training);
        let ffn_out = self.dropout.forward(&ffn_out, training);

        residual2.add(&ffn_out).unwrap_or(residual2)
    }

    fn forward_postnorm(
        &mut self,
        x: &Tensor2,
        encoder_output: &Tensor2,
        self_attn_mask: Option<&Tensor2>,
        cross_attn_mask: Option<&Tensor2>,
        training: bool,
    ) -> Tensor2 {
        // Self-attention
        let self_attn_out = self.self_attn.forward(x, self_attn_mask, training);
        let self_attn_out = self.dropout.forward(&self_attn_out, training);
        let residual1 = x.add(&self_attn_out).unwrap_or_else(|| x.clone());
        let normed1 = self.ln1.forward(&residual1);

        // Cross-attention
        let cross_attn_out =
            self.cross_attn
                .forward(&normed1, encoder_output, cross_attn_mask, training);
        let cross_attn_out = self.dropout.forward(&cross_attn_out, training);
        let residual2 = normed1.add(&cross_attn_out).unwrap_or(normed1);
        let normed2 = self.ln2.forward(&residual2);

        // FFN
        let ffn_out = self.ffn.forward(&normed2, training);
        let ffn_out = self.dropout.forward(&ffn_out, training);
        let residual3 = normed2.add(&ffn_out).unwrap_or(normed2);

        self.ln3.forward(&residual3)
    }
}

/// Decoder-only layer (no cross-attention)
pub struct DecoderOnlyLayer {
    /// Block
    block: PreNormBlock,
}

impl DecoderOnlyLayer {
    /// Create decoder-only layer
    pub fn new(config: &TransformerConfig, seed: u64) -> Self {
        Self {
            block: PreNormBlock::new(config, seed),
        }
    }

    /// Forward with causal mask
    #[inline(always)]
    pub fn forward(&mut self, x: &Tensor2, training: bool) -> Tensor2 {
        let causal_mask = MultiHeadSelfAttention::causal_mask(x.rows);
        self.block.forward(x, Some(&causal_mask), training)
    }
}

// ============================================================================
// DECODER STACK
// ============================================================================

/// Encoder-Decoder transformer decoder
pub struct Decoder {
    /// Layers
    pub layers: Vec<DecoderLayer>,
    /// Final norm
    pub final_norm: Option<LayerNorm>,
    /// Config
    pub config: TransformerConfig,
}

impl Decoder {
    /// Create new decoder
    pub fn new(config: TransformerConfig, seed: u64) -> Self {
        let mut layers = Vec::with_capacity(config.n_layers);

        for i in 0..config.n_layers {
            layers.push(DecoderLayer::new(
                &config,
                seed.wrapping_add(i as u64 * 1000),
            ));
        }

        let final_norm = if config.pre_norm {
            Some(LayerNorm::new(config.d_model, config.layer_norm_eps))
        } else {
            None
        };

        Self {
            layers,
            final_norm,
            config,
        }
    }

    /// Forward pass
    pub fn forward(
        &mut self,
        x: &Tensor2,
        encoder_output: &Tensor2,
        self_attn_mask: Option<&Tensor2>,
        cross_attn_mask: Option<&Tensor2>,
        training: bool,
    ) -> Tensor2 {
        let mut hidden = x.clone();

        for layer in &mut self.layers {
            hidden = layer.forward(
                &hidden,
                encoder_output,
                self_attn_mask,
                cross_attn_mask,
                training,
            );
        }

        if let Some(ref norm) = self.final_norm {
            hidden = norm.forward(&hidden);
        }

        hidden
    }
}

/// Decoder-only model stack
pub struct DecoderOnly {
    /// Layers
    pub layers: Vec<DecoderOnlyLayer>,
    /// Final norm
    pub final_norm: LayerNorm,
    /// Config
    pub config: TransformerConfig,
}

impl DecoderOnly {
    /// Create decoder-only stack
    pub fn new(config: TransformerConfig, seed: u64) -> Self {
        let mut layers = Vec::with_capacity(config.n_layers);

        for i in 0..config.n_layers {
            layers.push(DecoderOnlyLayer::new(
                &config,
                seed.wrapping_add(i as u64 * 1000),
            ));
        }

        Self {
            layers,
            final_norm: LayerNorm::new(config.d_model, config.layer_norm_eps),
            config,
        }
    }

    /// Forward pass
    #[inline]
    pub fn forward(&mut self, x: &Tensor2, training: bool) -> Tensor2 {
        let mut hidden = x.clone();

        for layer in &mut self.layers {
            hidden = layer.forward(&hidden, training);
        }

        self.final_norm.forward(&hidden)
    }
}

// ============================================================================
// FULL ENCODER-DECODER MODEL
// ============================================================================

/// Full sequence-to-sequence transformer
pub struct Seq2SeqTransformer {
    /// Source embedding
    pub src_embed: Embedding,
    /// Target embedding
    pub tgt_embed: Embedding,
    /// Source positional embedding
    pub src_pos: PositionalEmbedding,
    /// Target positional embedding
    pub tgt_pos: PositionalEmbedding,
    /// Encoder
    pub encoder: super::encoder::Encoder,
    /// Decoder
    pub decoder: Decoder,
    /// Output projection
    pub output_proj: Linear,
    /// Embedding dropout
    pub dropout: Dropout,
    /// Config
    pub config: TransformerConfig,
}

impl Seq2SeqTransformer {
    /// Create new seq2seq model
    pub fn new(
        src_vocab_size: usize,
        tgt_vocab_size: usize,
        config: TransformerConfig,
        seed: u64,
    ) -> Self {
        Self {
            src_embed: Embedding::new(src_vocab_size, config.d_model, seed),
            tgt_embed: Embedding::new(tgt_vocab_size, config.d_model, seed.wrapping_add(1)),
            src_pos: PositionalEmbedding::sinusoidal(config.max_seq_len, config.d_model),
            tgt_pos: PositionalEmbedding::sinusoidal(config.max_seq_len, config.d_model),
            encoder: super::encoder::Encoder::new(config.clone(), seed.wrapping_add(100)),
            decoder: Decoder::new(config.clone(), seed.wrapping_add(200)),
            output_proj: Linear::new(
                config.d_model,
                tgt_vocab_size,
                false,
                seed.wrapping_add(300),
            ),
            dropout: Dropout::new(config.dropout),
            config,
        }
    }

    /// Encode source sequence
    pub fn encode(
        &mut self,
        src_ids: &[usize],
        src_mask: Option<&Tensor2>,
        training: bool,
    ) -> Tensor2 {
        let seq_len = src_ids.len();

        // Embed
        let embeddings = self.src_embed.forward(src_ids);
        let pos = self.src_pos.forward(seq_len);
        let mut hidden = embeddings.add(&pos).unwrap_or(embeddings);

        hidden = self.dropout.forward(&hidden, training);

        // Encode
        self.encoder.forward(&hidden, src_mask, training)
    }

    /// Decode target sequence
    pub fn decode(
        &mut self,
        tgt_ids: &[usize],
        encoder_output: &Tensor2,
        tgt_mask: Option<&Tensor2>,
        src_mask: Option<&Tensor2>,
        training: bool,
    ) -> Tensor2 {
        let seq_len = tgt_ids.len();

        // Embed
        let embeddings = self.tgt_embed.forward(tgt_ids);
        let pos = self.tgt_pos.forward(seq_len);
        let mut hidden = embeddings.add(&pos).unwrap_or(embeddings);

        hidden = self.dropout.forward(&hidden, training);

        // Decode
        let decoded = self
            .decoder
            .forward(&hidden, encoder_output, tgt_mask, src_mask, training);

        // Project to vocabulary
        self.output_proj.forward(&decoded)
    }

    /// Full forward pass
    pub fn forward(&mut self, src_ids: &[usize], tgt_ids: &[usize], training: bool) -> Tensor2 {
        let tgt_len = tgt_ids.len();

        // Encode
        let encoder_output = self.encode(src_ids, None, training);

        // Create causal mask for decoder
        let causal_mask = MultiHeadSelfAttention::causal_mask(tgt_len);

        // Decode
        self.decode(tgt_ids, &encoder_output, Some(&causal_mask), None, training)
    }
}

// ============================================================================
// KV CACHE FOR EFFICIENT GENERATION
// ============================================================================

/// KV cache for autoregressive generation
#[repr(align(64))]
pub struct KVCache {
    /// Cached keys per layer (n_layers, seq_len, d_model)
    pub keys: Vec<Tensor2>,
    /// Cached values per layer
    pub values: Vec<Tensor2>,
    /// Current length
    pub length: usize,
    /// Maximum length
    pub max_length: usize,
}

impl KVCache {
    /// Create new cache
    pub fn new(n_layers: usize, max_length: usize, d_model: usize) -> Self {
        let mut keys = Vec::with_capacity(n_layers);
        let mut values = Vec::with_capacity(n_layers);

        for _ in 0..n_layers {
            keys.push(Tensor2::new(max_length, d_model));
            values.push(Tensor2::new(max_length, d_model));
        }

        Self {
            keys,
            values,
            length: 0,
            max_length,
        }
    }

    /// Update cache for layer
    pub fn update(&mut self, layer_idx: usize, new_key: &Tensor2, new_value: &Tensor2) {
        if layer_idx >= self.keys.len() {
            return;
        }

        let d_model = new_key.cols.min(self.keys[layer_idx].cols);

        for i in 0..new_key.rows {
            let pos = self.length + i;
            if pos >= self.max_length {
                break;
            }

            for j in 0..d_model {
                self.keys[layer_idx].set(pos, j, new_key.get(i, j));
                self.values[layer_idx].set(pos, j, new_value.get(i, j));
            }
        }
    }

    /// Increment length
    #[inline(always)]
    pub fn increment(&mut self, amount: usize) {
        self.length = (self.length + amount).min(self.max_length);
    }

    /// Get cached K for layer
    pub fn get_keys(&self, layer_idx: usize) -> Tensor2 {
        let cache = &self.keys[layer_idx];
        let d_model = cache.cols;

        let mut result = Tensor2::new(self.length, d_model);
        for i in 0..self.length {
            for j in 0..d_model {
                result.set(i, j, cache.get(i, j));
            }
        }
        result
    }

    /// Get cached V for layer
    pub fn get_values(&self, layer_idx: usize) -> Tensor2 {
        let cache = &self.values[layer_idx];
        let d_model = cache.cols;

        let mut result = Tensor2::new(self.length, d_model);
        for i in 0..self.length {
            for j in 0..d_model {
                result.set(i, j, cache.get(i, j));
            }
        }
        result
    }

    /// Reset cache
    pub fn reset(&mut self) {
        self.length = 0;
        for k in &mut self.keys {
            for v in &mut k.data {
                *v = 0.0;
            }
        }
        for v in &mut self.values {
            for val in &mut v.data {
                *val = 0.0;
            }
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_attention() {
        let mut cross_attn = MultiHeadCrossAttention::new(64, 4, 0.0, 42);

        let decoder_hidden = Tensor2::random(10, 64, 43);
        let encoder_output = Tensor2::random(20, 64, 44);

        let output = cross_attn.forward(&decoder_hidden, &encoder_output, None, false);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_decoder_layer() {
        let config = TransformerConfig::tiny();
        let mut layer = DecoderLayer::new(&config, 42);

        let decoder_hidden = Tensor2::random(10, 64, 43);
        let encoder_output = Tensor2::random(20, 64, 44);

        let output = layer.forward(&decoder_hidden, &encoder_output, None, None, false);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_decoder_stack() {
        let config = TransformerConfig::tiny();
        let mut decoder = Decoder::new(config, 42);

        let decoder_hidden = Tensor2::random(10, 64, 43);
        let encoder_output = Tensor2::random(20, 64, 44);

        let output = decoder.forward(&decoder_hidden, &encoder_output, None, None, false);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_decoder_only() {
        let config = TransformerConfig::tiny();
        let mut decoder = DecoderOnly::new(config, 42);

        let input = Tensor2::random(16, 64, 43);
        let output = decoder.forward(&input, false);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_seq2seq() {
        let config = TransformerConfig::tiny();
        let mut model = Seq2SeqTransformer::new(1000, 1000, config, 42);

        let src_ids = alloc::vec![1, 2, 3, 4, 5];
        let tgt_ids = alloc::vec![6, 7, 8];

        let output = model.forward(&src_ids, &tgt_ids, false);

        assert_eq!(output.rows, 3);
        assert_eq!(output.cols, 1000); // vocab size
    }

    #[test]
    fn test_kv_cache() {
        let mut cache = KVCache::new(2, 32, 64);

        let key = Tensor2::random(1, 64, 42);
        let value = Tensor2::random(1, 64, 43);

        cache.update(0, &key, &value);
        cache.increment(1);

        assert_eq!(cache.length, 1);

        let cached_k = cache.get_keys(0);
        assert_eq!(cached_k.rows, 1);
    }
}
