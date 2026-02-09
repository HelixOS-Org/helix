//! # Multi-Head Attention
//!
//! Multi-head attention mechanism for parallel attention computation.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::scaled::ScaledDotProductAttention;
use super::types::{AttentionMask, AttentionOutput, Dropout, LayerNorm, Linear, Matrix};

// ============================================================================
// MULTI-HEAD ATTENTION
// ============================================================================

/// Multi-head attention layer
pub struct MultiHeadAttention {
    /// Number of heads
    pub n_heads: usize,
    /// Model dimension
    pub d_model: usize,
    /// Key/Query dimension per head
    pub d_k: usize,
    /// Value dimension per head
    pub d_v: usize,
    /// Query projection
    pub w_q: Linear,
    /// Key projection
    pub w_k: Linear,
    /// Value projection
    pub w_v: Linear,
    /// Output projection
    pub w_o: Linear,
    /// Attention module
    attention: ScaledDotProductAttention,
    /// Dropout
    dropout: Dropout,
    /// Store attention weights
    store_weights: bool,
}

impl MultiHeadAttention {
    /// Create new multi-head attention
    pub fn new(d_model: usize, n_heads: usize, dropout_p: f64) -> Self {
        assert!(
            d_model % n_heads == 0,
            "d_model must be divisible by n_heads"
        );

        let d_k = d_model / n_heads;
        let d_v = d_k;

        Self {
            n_heads,
            d_model,
            d_k,
            d_v,
            w_q: Linear::new(d_model, d_model),
            w_k: Linear::new(d_model, d_model),
            w_v: Linear::new(d_model, d_model),
            w_o: Linear::new(d_model, d_model),
            attention: ScaledDotProductAttention::new(d_k),
            dropout: Dropout::new(dropout_p),
            store_weights: false,
        }
    }

    /// Enable weight storage
    #[inline]
    pub fn with_weight_storage(mut self) -> Self {
        self.store_weights = true;
        self.attention = self.attention.with_weight_storage();
        self
    }

    /// Set training mode
    #[inline(always)]
    pub fn train(&mut self, training: bool) {
        self.dropout.train(training);
    }

    /// Split heads
    fn split_heads(&self, x: &Matrix) -> Vec<Matrix> {
        let seq_len = x.rows;
        let mut heads = Vec::with_capacity(self.n_heads);

        for h in 0..self.n_heads {
            let mut head = Matrix::new(seq_len, self.d_k);
            let offset = h * self.d_k;

            for i in 0..seq_len {
                for j in 0..self.d_k {
                    head.set(i, j, x.get(i, offset + j));
                }
            }

            heads.push(head);
        }

        heads
    }

    /// Combine heads
    fn combine_heads(&self, heads: &[Matrix]) -> Matrix {
        if heads.is_empty() {
            return Matrix::new(0, 0);
        }

        let seq_len = heads[0].rows;
        let mut output = Matrix::new(seq_len, self.d_model);

        for (h, head) in heads.iter().enumerate() {
            let offset = h * self.d_k;

            for i in 0..seq_len {
                for j in 0..self.d_k {
                    output.set(i, offset + j, head.get(i, j));
                }
            }
        }

        output
    }

    /// Forward pass
    pub fn forward(
        &mut self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        mask: &AttentionMask,
    ) -> AttentionOutput {
        // Linear projections
        let q = self.w_q.forward(query);
        let k = self.w_k.forward(key);
        let v = self.w_v.forward(value);

        // Split into heads
        let q_heads = self.split_heads(&q);
        let k_heads = self.split_heads(&k);
        let v_heads = self.split_heads(&v);

        // Attention per head
        let mut head_outputs = Vec::with_capacity(self.n_heads);
        let mut all_weights = if self.store_weights {
            Some(Vec::with_capacity(self.n_heads))
        } else {
            None
        };

        for h in 0..self.n_heads {
            let head_out = self
                .attention
                .forward(&q_heads[h], &k_heads[h], &v_heads[h], mask);

            head_outputs.push(head_out.output);

            if let Some(ref mut weights) = all_weights {
                if let Some(w) = head_out.weights {
                    weights.push(w);
                }
            }
        }

        // Combine heads
        let combined = self.combine_heads(&head_outputs);

        // Output projection
        let output = self.w_o.forward(&combined);

        // Dropout
        let output = self.dropout.forward(&output);

        // Combine attention weights if stored
        let weights = all_weights.map(|heads| {
            // Average weights across heads for visualization
            if heads.is_empty() {
                return Matrix::new(0, 0);
            }

            let seq_len = heads[0].rows;
            let key_len = heads[0].cols;
            let mut avg = Matrix::new(seq_len, key_len);

            for head in &heads {
                for i in 0..seq_len {
                    for j in 0..key_len {
                        avg.set(i, j, avg.get(i, j) + head.get(i, j) / heads.len() as f64);
                    }
                }
            }

            avg
        });

        AttentionOutput { output, weights }
    }

    /// Self-attention forward
    #[inline(always)]
    pub fn self_attention(&mut self, x: &Matrix, mask: &AttentionMask) -> AttentionOutput {
        self.forward(x, x, x, mask)
    }
}

// ============================================================================
// MULTI-HEAD ATTENTION WITH CACHE
// ============================================================================

/// KV Cache for incremental decoding
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KVCache {
    /// Cached keys per layer
    pub keys: Vec<Matrix>,
    /// Cached values per layer
    pub values: Vec<Matrix>,
    /// Maximum cache length
    pub max_len: usize,
}

impl KVCache {
    /// Create new cache
    pub fn new(max_len: usize) -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
            max_len,
        }
    }

    /// Update cache for layer
    pub fn update(&mut self, layer: usize, new_k: &Matrix, new_v: &Matrix) {
        // Ensure vectors are large enough
        while self.keys.len() <= layer {
            self.keys.push(Matrix::new(0, 0));
            self.values.push(Matrix::new(0, 0));
        }

        // Append or set
        if self.keys[layer].rows == 0 {
            self.keys[layer] = new_k.clone();
            self.values[layer] = new_v.clone();
        } else {
            // Concatenate
            let old_k = &self.keys[layer];
            let old_v = &self.values[layer];

            let total_len = old_k.rows + new_k.rows;
            let d_k = old_k.cols;
            let d_v = old_v.cols;

            let mut concat_k = Matrix::new(total_len, d_k);
            let mut concat_v = Matrix::new(total_len, d_v);

            // Copy old
            for i in 0..old_k.rows {
                for j in 0..d_k {
                    concat_k.set(i, j, old_k.get(i, j));
                }
                for j in 0..d_v {
                    concat_v.set(i, j, old_v.get(i, j));
                }
            }

            // Copy new
            for i in 0..new_k.rows {
                for j in 0..d_k {
                    concat_k.set(old_k.rows + i, j, new_k.get(i, j));
                }
                for j in 0..d_v {
                    concat_v.set(old_v.rows + i, j, new_v.get(i, j));
                }
            }

            // Trim if exceeding max length
            if concat_k.rows > self.max_len {
                let trim = concat_k.rows - self.max_len;
                let mut trimmed_k = Matrix::new(self.max_len, d_k);
                let mut trimmed_v = Matrix::new(self.max_len, d_v);

                for i in 0..self.max_len {
                    for j in 0..d_k {
                        trimmed_k.set(i, j, concat_k.get(trim + i, j));
                    }
                    for j in 0..d_v {
                        trimmed_v.set(i, j, concat_v.get(trim + i, j));
                    }
                }

                concat_k = trimmed_k;
                concat_v = trimmed_v;
            }

            self.keys[layer] = concat_k;
            self.values[layer] = concat_v;
        }
    }

    /// Get cached KV for layer
    #[inline]
    pub fn get(&self, layer: usize) -> Option<(&Matrix, &Matrix)> {
        if layer < self.keys.len() && self.keys[layer].rows > 0 {
            Some((&self.keys[layer], &self.values[layer]))
        } else {
            None
        }
    }

    /// Clear cache
    #[inline(always)]
    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
    }

    /// Get current length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.keys.first().map(|k| k.rows).unwrap_or(0)
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() || self.keys[0].rows == 0
    }
}

/// Multi-head attention with KV cache
#[repr(align(64))]
pub struct CachedMultiHeadAttention {
    /// Base attention
    inner: MultiHeadAttention,
    /// Layer index
    layer_idx: usize,
}

impl CachedMultiHeadAttention {
    /// Create cached attention
    pub fn new(d_model: usize, n_heads: usize, dropout_p: f64, layer_idx: usize) -> Self {
        Self {
            inner: MultiHeadAttention::new(d_model, n_heads, dropout_p),
            layer_idx,
        }
    }

    /// Forward with cache
    pub fn forward(
        &mut self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        mask: &AttentionMask,
        cache: &mut KVCache,
    ) -> AttentionOutput {
        // Project current K, V
        let k = self.inner.w_k.forward(key);
        let v = self.inner.w_v.forward(value);

        // Update cache
        cache.update(self.layer_idx, &k, &v);

        // Get full K, V from cache
        let (full_k, full_v) = cache.get(self.layer_idx).unwrap_or((&k, &v));

        // Project query
        let q = self.inner.w_q.forward(query);

        // Split heads
        let q_heads = self.inner.split_heads(&q);
        let k_heads = self.inner.split_heads(full_k);
        let v_heads = self.inner.split_heads(full_v);

        // Attention per head
        let mut head_outputs = Vec::with_capacity(self.inner.n_heads);

        for h in 0..self.inner.n_heads {
            let head_out =
                self.inner
                    .attention
                    .forward(&q_heads[h], &k_heads[h], &v_heads[h], mask);
            head_outputs.push(head_out.output);
        }

        // Combine and project
        let combined = self.inner.combine_heads(&head_outputs);
        let output = self.inner.w_o.forward(&combined);
        let output = self.inner.dropout.forward(&output);

        AttentionOutput::new(output)
    }
}

// ============================================================================
// GROUPED QUERY ATTENTION
// ============================================================================

/// Grouped Query Attention (GQA)
/// Uses fewer KV heads than query heads
pub struct GroupedQueryAttention {
    /// Number of query heads
    pub n_q_heads: usize,
    /// Number of KV heads
    pub n_kv_heads: usize,
    /// Model dimension
    pub d_model: usize,
    /// Head dimension
    pub head_dim: usize,
    /// Query projection
    pub w_q: Linear,
    /// Key projection
    pub w_k: Linear,
    /// Value projection
    pub w_v: Linear,
    /// Output projection
    pub w_o: Linear,
    /// Attention scale
    scale: f64,
}

impl GroupedQueryAttention {
    /// Create GQA
    pub fn new(d_model: usize, n_q_heads: usize, n_kv_heads: usize) -> Self {
        assert!(
            n_q_heads % n_kv_heads == 0,
            "n_q_heads must be divisible by n_kv_heads"
        );

        let head_dim = d_model / n_q_heads;
        let kv_dim = head_dim * n_kv_heads;

        Self {
            n_q_heads,
            n_kv_heads,
            d_model,
            head_dim,
            w_q: Linear::new(d_model, d_model),
            w_k: Linear::new(d_model, kv_dim),
            w_v: Linear::new(d_model, kv_dim),
            w_o: Linear::new(d_model, d_model),
            scale: 1.0 / libm::sqrt(head_dim as f64),
        }
    }

    /// Forward pass
    pub fn forward(
        &self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        mask: &AttentionMask,
    ) -> Matrix {
        let seq_len = query.rows;
        let key_len = key.rows;
        let group_size = self.n_q_heads / self.n_kv_heads;

        // Project
        let q = self.w_q.forward(query);
        let k = self.w_k.forward(key);
        let v = self.w_v.forward(value);

        // Compute attention for each query head
        let mut head_outputs = Vec::with_capacity(self.n_q_heads);

        for qh in 0..self.n_q_heads {
            let kv_idx = qh / group_size;

            // Extract query head
            let mut q_head = Matrix::new(seq_len, self.head_dim);
            for i in 0..seq_len {
                for j in 0..self.head_dim {
                    q_head.set(i, j, q.get(i, qh * self.head_dim + j));
                }
            }

            // Extract corresponding KV head
            let mut k_head = Matrix::new(key_len, self.head_dim);
            let mut v_head = Matrix::new(key_len, self.head_dim);
            for i in 0..key_len {
                for j in 0..self.head_dim {
                    k_head.set(i, j, k.get(i, kv_idx * self.head_dim + j));
                    v_head.set(i, j, v.get(i, kv_idx * self.head_dim + j));
                }
            }

            // Attention
            let k_t = k_head.transpose();
            let mut scores = q_head
                .matmul(&k_t)
                .unwrap_or_else(|| Matrix::new(seq_len, key_len));
            scores = scores.scale(self.scale);

            mask.apply(&mut scores);
            let weights = scores.softmax_rows();

            let output = weights
                .matmul(&v_head)
                .unwrap_or_else(|| Matrix::new(seq_len, self.head_dim));

            head_outputs.push(output);
        }

        // Concatenate heads
        let mut combined = Matrix::new(seq_len, self.d_model);
        for (h, head) in head_outputs.iter().enumerate() {
            let offset = h * self.head_dim;
            for i in 0..seq_len {
                for j in 0..self.head_dim {
                    combined.set(i, offset + j, head.get(i, j));
                }
            }
        }

        // Output projection
        self.w_o.forward(&combined)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_head_attention() {
        let mut mha = MultiHeadAttention::new(64, 8, 0.0);

        let x = Matrix::random(10, 64, 42);
        let output = mha.self_attention(&x, &AttentionMask::None);

        assert_eq!(output.output.rows, 10);
        assert_eq!(output.output.cols, 64);
    }

    #[test]
    fn test_causal_multi_head() {
        let mut mha = MultiHeadAttention::new(32, 4, 0.0).with_weight_storage();

        let x = Matrix::random(8, 32, 42);
        let output = mha.self_attention(&x, &AttentionMask::causal(8));

        assert!(output.weights.is_some());
        let weights = output.weights.unwrap();

        // Check causal property
        assert!(weights.get(0, 1) < 1e-6);
    }

    #[test]
    fn test_kv_cache() {
        let mut cache = KVCache::new(100);

        let k1 = Matrix::random(5, 16, 42);
        let v1 = Matrix::random(5, 16, 43);

        cache.update(0, &k1, &v1);
        assert_eq!(cache.len(), 5);

        let k2 = Matrix::random(3, 16, 44);
        let v2 = Matrix::random(3, 16, 45);

        cache.update(0, &k2, &v2);
        assert_eq!(cache.len(), 8);
    }

    #[test]
    fn test_grouped_query_attention() {
        let gqa = GroupedQueryAttention::new(64, 8, 2);

        let q = Matrix::random(10, 64, 42);
        let k = Matrix::random(10, 64, 43);
        let v = Matrix::random(10, 64, 44);

        let output = gqa.forward(&q, &k, &v, &AttentionMask::None);

        assert_eq!(output.rows, 10);
        assert_eq!(output.cols, 64);
    }
}
