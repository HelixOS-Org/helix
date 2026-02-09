//! # Transformer Layers
//!
//! Core building blocks: FFN, attention blocks, residual connections.

#![allow(dead_code)]

extern crate alloc;

use super::types::{
    ActivationType, Dropout, LayerNorm, Linear, RMSNorm, Tensor2, TransformerConfig,
};

// ============================================================================
// FEEDFORWARD NETWORK
// ============================================================================

/// Standard feedforward network (FFN)
pub struct FeedForward {
    /// First linear layer (d_model -> d_ff)
    pub fc1: Linear,
    /// Second linear layer (d_ff -> d_model)
    pub fc2: Linear,
    /// Activation function
    pub activation: ActivationType,
    /// Dropout
    pub dropout: Dropout,
}

impl FeedForward {
    /// Create new FFN
    pub fn new(
        d_model: usize,
        d_ff: usize,
        activation: ActivationType,
        dropout: f64,
        seed: u64,
    ) -> Self {
        Self {
            fc1: Linear::new(d_model, d_ff, true, seed),
            fc2: Linear::new(d_ff, d_model, true, seed.wrapping_add(1)),
            activation,
            dropout: Dropout::new(dropout),
        }
    }

    /// Create from config
    #[inline]
    pub fn from_config(config: &TransformerConfig, seed: u64) -> Self {
        Self::new(
            config.d_model,
            config.d_ff,
            config.activation,
            config.dropout,
            seed,
        )
    }

    /// Forward pass
    #[inline]
    pub fn forward(&mut self, input: &Tensor2, training: bool) -> Tensor2 {
        // input: (seq_len, d_model)
        let hidden = self.fc1.forward(input);

        // Apply activation
        let activated = hidden.apply(|x| self.activation.apply(x));

        // Dropout and project back
        let dropped = self.dropout.forward(&activated, training);
        self.fc2.forward(&dropped)
    }
}

/// Gated Linear Unit FFN (used in LLaMA, PaLM)
pub struct GatedFFN {
    /// Gate projection
    pub gate_proj: Linear,
    /// Up projection
    pub up_proj: Linear,
    /// Down projection
    pub down_proj: Linear,
    /// Activation
    pub activation: ActivationType,
}

impl GatedFFN {
    /// Create new gated FFN
    pub fn new(d_model: usize, d_ff: usize, activation: ActivationType, seed: u64) -> Self {
        Self {
            gate_proj: Linear::new(d_model, d_ff, false, seed),
            up_proj: Linear::new(d_model, d_ff, false, seed.wrapping_add(1)),
            down_proj: Linear::new(d_ff, d_model, false, seed.wrapping_add(2)),
            activation,
        }
    }

    /// Forward pass: down(activation(gate(x)) * up(x))
    pub fn forward(&self, input: &Tensor2) -> Tensor2 {
        let gate = self.gate_proj.forward(input);
        let gate_activated = gate.apply(|x| self.activation.apply(x));

        let up = self.up_proj.forward(input);

        // Element-wise multiplication
        let mut gated = Tensor2::new(gate_activated.rows, gate_activated.cols);
        for i in 0..gated.rows {
            for j in 0..gated.cols {
                gated.set(i, j, gate_activated.get(i, j) * up.get(i, j));
            }
        }

        self.down_proj.forward(&gated)
    }
}

/// Mixture of Experts FFN
pub struct MoEFFN {
    /// Expert FFNs
    pub experts: alloc::vec::Vec<FeedForward>,
    /// Router/gate network
    pub router: Linear,
    /// Number of experts to activate per token
    pub top_k: usize,
    /// Number of experts
    pub num_experts: usize,
}

impl MoEFFN {
    /// Create new MoE layer
    pub fn new(
        d_model: usize,
        d_ff: usize,
        num_experts: usize,
        top_k: usize,
        activation: ActivationType,
        seed: u64,
    ) -> Self {
        let mut experts = alloc::vec::Vec::with_capacity(num_experts);
        for i in 0..num_experts {
            experts.push(FeedForward::new(
                d_model,
                d_ff,
                activation,
                0.0,
                seed.wrapping_add(i as u64 * 10),
            ));
        }

        Self {
            experts,
            router: Linear::new(d_model, num_experts, false, seed.wrapping_add(1000)),
            top_k,
            num_experts,
        }
    }

    /// Forward pass
    pub fn forward(&mut self, input: &Tensor2, training: bool) -> Tensor2 {
        let seq_len = input.rows;
        let d_model = input.cols;

        // Compute router scores
        let router_logits = self.router.forward(input);
        let router_probs = router_logits.softmax();

        let mut output = Tensor2::new(seq_len, d_model);

        for i in 0..seq_len {
            // Get input row
            let mut token_input = Tensor2::new(1, d_model);
            for j in 0..d_model {
                token_input.set(0, j, input.get(i, j));
            }

            // Find top-k experts
            let mut expert_scores: alloc::vec::Vec<(usize, f64)> = (0..self.num_experts)
                .map(|e| (e, router_probs.get(i, e)))
                .collect();

            expert_scores
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

            // Normalize top-k weights
            let top_k_sum: f64 = expert_scores.iter().take(self.top_k).map(|(_, w)| *w).sum();

            // Compute weighted sum of expert outputs
            for (expert_idx, weight) in expert_scores.iter().take(self.top_k) {
                let normalized_weight = weight / top_k_sum.max(1e-10);
                let expert_output = self.experts[*expert_idx].forward(&token_input, training);

                for j in 0..d_model {
                    let val = output.get(i, j) + normalized_weight * expert_output.get(0, j);
                    output.set(i, j, val);
                }
            }
        }

        output
    }
}

// ============================================================================
// ATTENTION BLOCKS
// ============================================================================

/// Multi-head self-attention
pub struct MultiHeadSelfAttention {
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

impl MultiHeadSelfAttention {
    /// Create new multi-head attention
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

    /// Create from config
    #[inline(always)]
    pub fn from_config(config: &TransformerConfig, seed: u64) -> Self {
        Self::new(config.d_model, config.n_heads, config.dropout, seed)
    }

    /// Forward pass
    pub fn forward(&mut self, x: &Tensor2, mask: Option<&Tensor2>, training: bool) -> Tensor2 {
        let seq_len = x.rows;

        // Project to Q, K, V
        let q = self.wq.forward(x);
        let k = self.wk.forward(x);
        let v = self.wv.forward(x);

        // Reshape to (n_heads, seq_len, head_dim) and compute attention per head
        let mut head_outputs = alloc::vec::Vec::with_capacity(self.n_heads);

        for h in 0..self.n_heads {
            // Extract head
            let mut q_h = Tensor2::new(seq_len, self.head_dim);
            let mut k_h = Tensor2::new(seq_len, self.head_dim);
            let mut v_h = Tensor2::new(seq_len, self.head_dim);

            for i in 0..seq_len {
                for j in 0..self.head_dim {
                    q_h.set(i, j, q.get(i, h * self.head_dim + j));
                    k_h.set(i, j, k.get(i, h * self.head_dim + j));
                    v_h.set(i, j, v.get(i, h * self.head_dim + j));
                }
            }

            // Compute attention scores
            let k_t = k_h.transpose();
            let mut scores = q_h
                .matmul(&k_t)
                .unwrap_or_else(|| Tensor2::new(seq_len, seq_len));
            scores = scores.scale(self.scale);

            // Apply mask
            if let Some(m) = mask {
                for i in 0..seq_len {
                    for j in 0..seq_len {
                        if m.get(i, j) == 0.0 {
                            scores.set(i, j, f64::NEG_INFINITY);
                        }
                    }
                }
            }

            // Softmax and attention
            let attn_weights = scores.softmax();
            let attn_weights = self.dropout.forward(&attn_weights, training);

            let head_out = attn_weights
                .matmul(&v_h)
                .unwrap_or_else(|| Tensor2::new(seq_len, self.head_dim));

            head_outputs.push(head_out);
        }

        // Concatenate heads
        let mut concat = Tensor2::new(seq_len, self.d_model);
        for h in 0..self.n_heads {
            for i in 0..seq_len {
                for j in 0..self.head_dim {
                    concat.set(i, h * self.head_dim + j, head_outputs[h].get(i, j));
                }
            }
        }

        // Output projection
        self.wo.forward(&concat)
    }

    /// Create causal mask
    #[inline]
    pub fn causal_mask(seq_len: usize) -> Tensor2 {
        let mut mask = Tensor2::new(seq_len, seq_len);
        for i in 0..seq_len {
            for j in 0..=i {
                mask.set(i, j, 1.0);
            }
        }
        mask
    }
}

/// Grouped Query Attention (used in LLaMA 2)
pub struct GroupedQueryAttention {
    /// Query projection
    pub wq: Linear,
    /// Key projection (fewer heads)
    pub wk: Linear,
    /// Value projection (fewer heads)
    pub wv: Linear,
    /// Output projection
    pub wo: Linear,
    /// Number of query heads
    pub n_heads: usize,
    /// Number of KV heads
    pub n_kv_heads: usize,
    /// Head dimension
    pub head_dim: usize,
    /// Scale
    pub scale: f64,
}

impl GroupedQueryAttention {
    /// Create new GQA
    pub fn new(d_model: usize, n_heads: usize, n_kv_heads: usize, seed: u64) -> Self {
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
            scale: 1.0 / libm::sqrt(head_dim as f64),
        }
    }

    /// Forward pass
    pub fn forward(&self, x: &Tensor2, causal: bool) -> Tensor2 {
        let seq_len = x.rows;
        let d_model = self.n_heads * self.head_dim;

        let q = self.wq.forward(x);
        let k = self.wk.forward(x);
        let v = self.wv.forward(x);

        let heads_per_kv = self.n_heads / self.n_kv_heads;
        let mut head_outputs = alloc::vec::Vec::with_capacity(self.n_heads);

        for h in 0..self.n_heads {
            let kv_h = h / heads_per_kv;

            // Extract Q head
            let mut q_h = Tensor2::new(seq_len, self.head_dim);
            for i in 0..seq_len {
                for j in 0..self.head_dim {
                    q_h.set(i, j, q.get(i, h * self.head_dim + j));
                }
            }

            // Extract KV head (shared)
            let mut k_h = Tensor2::new(seq_len, self.head_dim);
            let mut v_h = Tensor2::new(seq_len, self.head_dim);
            for i in 0..seq_len {
                for j in 0..self.head_dim {
                    k_h.set(i, j, k.get(i, kv_h * self.head_dim + j));
                    v_h.set(i, j, v.get(i, kv_h * self.head_dim + j));
                }
            }

            // Attention
            let k_t = k_h.transpose();
            let mut scores = q_h
                .matmul(&k_t)
                .unwrap_or_else(|| Tensor2::new(seq_len, seq_len));
            scores = scores.scale(self.scale);

            // Causal mask
            if causal {
                for i in 0..seq_len {
                    for j in (i + 1)..seq_len {
                        scores.set(i, j, f64::NEG_INFINITY);
                    }
                }
            }

            let attn_weights = scores.softmax();
            let head_out = attn_weights
                .matmul(&v_h)
                .unwrap_or_else(|| Tensor2::new(seq_len, self.head_dim));

            head_outputs.push(head_out);
        }

        // Concatenate
        let mut concat = Tensor2::new(seq_len, d_model);
        for h in 0..self.n_heads {
            for i in 0..seq_len {
                for j in 0..self.head_dim {
                    concat.set(i, h * self.head_dim + j, head_outputs[h].get(i, j));
                }
            }
        }

        self.wo.forward(&concat)
    }
}

// ============================================================================
// TRANSFORMER BLOCKS
// ============================================================================

/// Pre-norm transformer block (GPT-style)
pub struct PreNormBlock {
    /// Layer norm before attention
    pub ln1: LayerNorm,
    /// Self-attention
    pub attn: MultiHeadSelfAttention,
    /// Layer norm before FFN
    pub ln2: LayerNorm,
    /// Feed-forward network
    pub ffn: FeedForward,
    /// Dropout for residual
    pub dropout: Dropout,
}

impl PreNormBlock {
    /// Create new pre-norm block
    pub fn new(config: &TransformerConfig, seed: u64) -> Self {
        Self {
            ln1: LayerNorm::new(config.d_model, config.layer_norm_eps),
            attn: MultiHeadSelfAttention::from_config(config, seed),
            ln2: LayerNorm::new(config.d_model, config.layer_norm_eps),
            ffn: FeedForward::from_config(config, seed.wrapping_add(100)),
            dropout: Dropout::new(config.dropout),
        }
    }

    /// Forward pass
    pub fn forward(&mut self, x: &Tensor2, mask: Option<&Tensor2>, training: bool) -> Tensor2 {
        // Pre-norm: x + Attn(LN(x))
        let normed1 = self.ln1.forward(x);
        let attn_out = self.attn.forward(&normed1, mask, training);
        let attn_out = self.dropout.forward(&attn_out, training);
        let residual1 = x.add(&attn_out).unwrap_or_else(|| x.clone());

        // Pre-norm: x + FFN(LN(x))
        let normed2 = self.ln2.forward(&residual1);
        let ffn_out = self.ffn.forward(&normed2, training);
        let ffn_out = self.dropout.forward(&ffn_out, training);

        residual1.add(&ffn_out).unwrap_or(residual1)
    }
}

/// Post-norm transformer block (BERT-style)
pub struct PostNormBlock {
    /// Self-attention
    pub attn: MultiHeadSelfAttention,
    /// Layer norm after attention
    pub ln1: LayerNorm,
    /// Feed-forward network
    pub ffn: FeedForward,
    /// Layer norm after FFN
    pub ln2: LayerNorm,
    /// Dropout
    pub dropout: Dropout,
}

impl PostNormBlock {
    /// Create new post-norm block
    pub fn new(config: &TransformerConfig, seed: u64) -> Self {
        Self {
            attn: MultiHeadSelfAttention::from_config(config, seed),
            ln1: LayerNorm::new(config.d_model, config.layer_norm_eps),
            ffn: FeedForward::from_config(config, seed.wrapping_add(100)),
            ln2: LayerNorm::new(config.d_model, config.layer_norm_eps),
            dropout: Dropout::new(config.dropout),
        }
    }

    /// Forward pass
    pub fn forward(&mut self, x: &Tensor2, mask: Option<&Tensor2>, training: bool) -> Tensor2 {
        // Post-norm: LN(x + Attn(x))
        let attn_out = self.attn.forward(x, mask, training);
        let attn_out = self.dropout.forward(&attn_out, training);
        let residual1 = x.add(&attn_out).unwrap_or_else(|| x.clone());
        let normed1 = self.ln1.forward(&residual1);

        // Post-norm: LN(x + FFN(x))
        let ffn_out = self.ffn.forward(&normed1, training);
        let ffn_out = self.dropout.forward(&ffn_out, training);
        let residual2 = normed1.add(&ffn_out).unwrap_or(normed1);

        self.ln2.forward(&residual2)
    }
}

/// LLaMA-style block (RMSNorm + GatedFFN + RoPE)
pub struct LlamaBlock {
    /// RMS norm before attention
    pub attn_norm: RMSNorm,
    /// Grouped query attention
    pub attn: GroupedQueryAttention,
    /// RMS norm before FFN
    pub ffn_norm: RMSNorm,
    /// Gated FFN
    pub ffn: GatedFFN,
}

impl LlamaBlock {
    /// Create new LLaMA block
    pub fn new(d_model: usize, n_heads: usize, n_kv_heads: usize, d_ff: usize, seed: u64) -> Self {
        Self {
            attn_norm: RMSNorm::new(d_model, 1e-5),
            attn: GroupedQueryAttention::new(d_model, n_heads, n_kv_heads, seed),
            ffn_norm: RMSNorm::new(d_model, 1e-5),
            ffn: GatedFFN::new(d_model, d_ff, ActivationType::Silu, seed.wrapping_add(100)),
        }
    }

    /// Forward pass
    pub fn forward(&self, x: &Tensor2, causal: bool) -> Tensor2 {
        // Pre-norm with RMS + GQA
        let normed1 = self.attn_norm.forward(x);
        let attn_out = self.attn.forward(&normed1, causal);
        let residual1 = x.add(&attn_out).unwrap_or_else(|| x.clone());

        // Pre-norm with RMS + Gated FFN
        let normed2 = self.ffn_norm.forward(&residual1);
        let ffn_out = self.ffn.forward(&normed2);

        residual1.add(&ffn_out).unwrap_or(residual1)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedforward() {
        let config = TransformerConfig::tiny();
        let mut ffn = FeedForward::from_config(&config, 42);

        let input = Tensor2::random(10, 64, 43);
        let output = ffn.forward(&input, false);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_gated_ffn() {
        let ffn = GatedFFN::new(64, 128, ActivationType::Silu, 42);

        let input = Tensor2::random(10, 64, 43);
        let output = ffn.forward(&input);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_mhsa() {
        let config = TransformerConfig::tiny();
        let mut attn = MultiHeadSelfAttention::from_config(&config, 42);

        let input = Tensor2::random(16, 64, 43);
        let output = attn.forward(&input, None, false);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_mhsa_causal() {
        let config = TransformerConfig::tiny();
        let mut attn = MultiHeadSelfAttention::from_config(&config, 42);

        let input = Tensor2::random(16, 64, 43);
        let mask = MultiHeadSelfAttention::causal_mask(16);
        let output = attn.forward(&input, Some(&mask), false);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_gqa() {
        let gqa = GroupedQueryAttention::new(64, 8, 2, 42);

        let input = Tensor2::random(16, 64, 43);
        let output = gqa.forward(&input, true);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_pre_norm_block() {
        let config = TransformerConfig::tiny();
        let mut block = PreNormBlock::new(&config, 42);

        let input = Tensor2::random(16, 64, 43);
        let output = block.forward(&input, None, false);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_llama_block() {
        let block = LlamaBlock::new(64, 8, 2, 128, 42);

        let input = Tensor2::random(16, 64, 43);
        let output = block.forward(&input, true);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }
}
