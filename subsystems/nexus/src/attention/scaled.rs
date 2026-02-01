//! # Scaled Dot-Product Attention
//!
//! Core attention mechanism implementation.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::vec::Vec;

use super::types::{AttentionMask, AttentionOutput, Matrix};

// ============================================================================
// SCALED DOT-PRODUCT ATTENTION
// ============================================================================

/// Scaled dot-product attention
///
/// Computes: Attention(Q, K, V) = softmax(QK^T / √d_k) V
pub struct ScaledDotProductAttention {
    /// Scale factor (1/√d_k)
    scale: f64,
    /// Whether to store attention weights
    store_weights: bool,
}

impl ScaledDotProductAttention {
    /// Create new attention with dimension
    pub fn new(d_k: usize) -> Self {
        Self {
            scale: 1.0 / libm::sqrt(d_k as f64),
            store_weights: false,
        }
    }

    /// Enable weight storage
    pub fn with_weight_storage(mut self) -> Self {
        self.store_weights = true;
        self
    }

    /// Forward pass
    ///
    /// Q: [seq_len, d_k]
    /// K: [seq_len, d_k]
    /// V: [seq_len, d_v]
    pub fn forward(
        &self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        mask: &AttentionMask,
    ) -> AttentionOutput {
        // Compute attention scores: Q @ K^T
        let k_t = key.transpose();
        let mut scores = query
            .matmul(&k_t)
            .unwrap_or_else(|| Matrix::new(query.rows, key.rows));

        // Scale
        scores = scores.scale(self.scale);

        // Apply mask
        mask.apply(&mut scores);

        // Softmax
        let attention_weights = scores.softmax_rows();

        // Apply attention to values
        let output = attention_weights
            .matmul(value)
            .unwrap_or_else(|| Matrix::new(query.rows, value.cols));

        if self.store_weights {
            AttentionOutput::with_weights(output, attention_weights)
        } else {
            AttentionOutput::new(output)
        }
    }

    /// Compute only attention scores (for analysis)
    pub fn attention_scores(&self, query: &Matrix, key: &Matrix, mask: &AttentionMask) -> Matrix {
        let k_t = key.transpose();
        let mut scores = query
            .matmul(&k_t)
            .unwrap_or_else(|| Matrix::new(query.rows, key.rows));

        scores = scores.scale(self.scale);
        mask.apply(&mut scores);
        scores.softmax_rows()
    }
}

// ============================================================================
// EFFICIENT ATTENTION
// ============================================================================

/// Memory-efficient attention computation
/// Uses chunking to reduce peak memory usage
pub struct EfficientAttention {
    /// Chunk size for queries
    query_chunk_size: usize,
    /// Chunk size for keys/values
    key_chunk_size: usize,
    /// Scale factor
    scale: f64,
}

impl EfficientAttention {
    /// Create new efficient attention
    pub fn new(d_k: usize, query_chunk_size: usize, key_chunk_size: usize) -> Self {
        Self {
            query_chunk_size,
            key_chunk_size,
            scale: 1.0 / libm::sqrt(d_k as f64),
        }
    }

    /// Forward pass with chunking
    pub fn forward(
        &self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        mask: &AttentionMask,
    ) -> Matrix {
        let seq_len_q = query.rows;
        let seq_len_k = key.rows;
        let d_v = value.cols;

        let mut output = Matrix::new(seq_len_q, d_v);

        // Process query chunks
        for q_start in (0..seq_len_q).step_by(self.query_chunk_size) {
            let q_end = (q_start + self.query_chunk_size).min(seq_len_q);
            let q_chunk_len = q_end - q_start;

            // Extract query chunk
            let mut q_chunk = Matrix::new(q_chunk_len, query.cols);
            for i in 0..q_chunk_len {
                for j in 0..query.cols {
                    q_chunk.set(i, j, query.get(q_start + i, j));
                }
            }

            // Accumulate weighted values for this query chunk
            let mut chunk_output = Matrix::new(q_chunk_len, d_v);
            let mut max_scores = alloc::vec![f64::NEG_INFINITY; q_chunk_len];
            let mut sum_exp = alloc::vec![0.0; q_chunk_len];

            // Process key chunks
            for k_start in (0..seq_len_k).step_by(self.key_chunk_size) {
                let k_end = (k_start + self.key_chunk_size).min(seq_len_k);
                let k_chunk_len = k_end - k_start;

                // Extract key and value chunks
                let mut k_chunk = Matrix::new(k_chunk_len, key.cols);
                let mut v_chunk = Matrix::new(k_chunk_len, value.cols);

                for i in 0..k_chunk_len {
                    for j in 0..key.cols {
                        k_chunk.set(i, j, key.get(k_start + i, j));
                    }
                    for j in 0..value.cols {
                        v_chunk.set(i, j, value.get(k_start + i, j));
                    }
                }

                // Compute scores for this chunk
                let k_t = k_chunk.transpose();
                let mut scores = q_chunk
                    .matmul(&k_t)
                    .unwrap_or_else(|| Matrix::new(q_chunk_len, k_chunk_len));
                scores = scores.scale(self.scale);

                // Apply mask (simplified for chunked case)
                if let AttentionMask::Causal(_) = mask {
                    for i in 0..q_chunk_len {
                        for j in 0..k_chunk_len {
                            if k_start + j > q_start + i {
                                scores.set(i, j, f64::NEG_INFINITY);
                            }
                        }
                    }
                }

                // Online softmax update
                for i in 0..q_chunk_len {
                    let old_max = max_scores[i];

                    // Find new max
                    for j in 0..k_chunk_len {
                        max_scores[i] = max_scores[i].max(scores.get(i, j));
                    }

                    let new_max = max_scores[i];

                    // Rescale old accumulator
                    if old_max > f64::NEG_INFINITY {
                        let correction = libm::exp(old_max - new_max);
                        sum_exp[i] *= correction;
                        for j in 0..d_v {
                            chunk_output.set(i, j, chunk_output.get(i, j) * correction);
                        }
                    }

                    // Add new contributions
                    for j in 0..k_chunk_len {
                        let exp_score = libm::exp(scores.get(i, j) - new_max);
                        sum_exp[i] += exp_score;

                        for d in 0..d_v {
                            let val = chunk_output.get(i, d) + exp_score * v_chunk.get(j, d);
                            chunk_output.set(i, d, val);
                        }
                    }
                }
            }

            // Normalize and store
            for i in 0..q_chunk_len {
                if sum_exp[i] > 1e-10 {
                    for j in 0..d_v {
                        let normalized = chunk_output.get(i, j) / sum_exp[i];
                        output.set(q_start + i, j, normalized);
                    }
                }
            }
        }

        output
    }
}

// ============================================================================
// RELATIVE POSITION ATTENTION
// ============================================================================

/// Attention with relative position encoding
pub struct RelativePositionAttention {
    /// Base attention
    base: ScaledDotProductAttention,
    /// Maximum relative distance
    max_distance: usize,
    /// Position embedding dimension
    embed_dim: usize,
    /// Position embeddings
    position_embeddings: Matrix,
}

impl RelativePositionAttention {
    /// Create new relative position attention
    pub fn new(d_k: usize, max_distance: usize) -> Self {
        // Initialize position embeddings
        let embed_dim = d_k;
        let n_positions = 2 * max_distance + 1;
        let position_embeddings = Matrix::random(n_positions, embed_dim, 42).scale(0.1);

        Self {
            base: ScaledDotProductAttention::new(d_k),
            max_distance,
            embed_dim,
            position_embeddings,
        }
    }

    /// Get relative position embedding
    fn get_position_embedding(&self, rel_pos: i32) -> Vec<f64> {
        let clamped = rel_pos.clamp(-(self.max_distance as i32), self.max_distance as i32);
        let idx = (clamped + self.max_distance as i32) as usize;

        let row = self.position_embeddings.row(idx);
        row.to_vec()
    }

    /// Compute relative position scores
    fn relative_scores(&self, query: &Matrix, seq_len: usize) -> Matrix {
        let mut rel_scores = Matrix::new(seq_len, seq_len);

        for i in 0..seq_len {
            for j in 0..seq_len {
                let rel_pos = j as i32 - i as i32;
                let pos_embed = self.get_position_embedding(rel_pos);

                // Dot product of query[i] with position embedding
                let mut score = 0.0;
                for k in 0..query.cols.min(pos_embed.len()) {
                    score += query.get(i, k) * pos_embed[k];
                }

                rel_scores.set(i, j, score);
            }
        }

        rel_scores.scale(self.base.scale)
    }

    /// Forward pass
    pub fn forward(
        &self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        mask: &AttentionMask,
    ) -> AttentionOutput {
        let seq_len = query.rows;

        // Standard QK^T scores
        let k_t = key.transpose();
        let mut content_scores = query
            .matmul(&k_t)
            .unwrap_or_else(|| Matrix::new(seq_len, seq_len));
        content_scores = content_scores.scale(self.base.scale);

        // Relative position scores
        let rel_scores = self.relative_scores(query, seq_len);

        // Combine
        let mut scores = content_scores
            .add(&rel_scores)
            .unwrap_or_else(|| Matrix::new(seq_len, seq_len));

        // Apply mask and softmax
        mask.apply(&mut scores);
        let weights = scores.softmax_rows();

        // Apply to values
        let output = weights
            .matmul(value)
            .unwrap_or_else(|| Matrix::new(seq_len, value.cols));

        AttentionOutput::with_weights(output, weights)
    }
}

// ============================================================================
// ROTARY POSITION EMBEDDING (RoPE)
// ============================================================================

/// Rotary Position Embedding
pub struct RotaryPositionEmbedding {
    /// Dimension
    dim: usize,
    /// Base frequency
    base: f64,
    /// Precomputed sin/cos
    sin_cache: Vec<f64>,
    cos_cache: Vec<f64>,
}

impl RotaryPositionEmbedding {
    /// Create new RoPE
    pub fn new(dim: usize, max_seq_len: usize) -> Self {
        let base = 10000.0;
        let half_dim = dim / 2;

        let mut sin_cache = Vec::with_capacity(max_seq_len * half_dim);
        let mut cos_cache = Vec::with_capacity(max_seq_len * half_dim);

        for pos in 0..max_seq_len {
            for i in 0..half_dim {
                let theta = (pos as f64) / libm::pow(base, (2 * i) as f64 / dim as f64);
                sin_cache.push(libm::sin(theta));
                cos_cache.push(libm::cos(theta));
            }
        }

        Self {
            dim,
            base,
            sin_cache,
            cos_cache,
        }
    }

    /// Apply rotary embedding to query or key
    pub fn apply(&self, x: &Matrix) -> Matrix {
        let seq_len = x.rows;
        let half_dim = self.dim / 2;

        let mut output = Matrix::new(seq_len, x.cols);

        for pos in 0..seq_len {
            for i in 0..half_dim.min(x.cols / 2) {
                let cache_idx = pos * half_dim + i;

                if cache_idx >= self.sin_cache.len() {
                    continue;
                }

                let sin_val = self.sin_cache[cache_idx];
                let cos_val = self.cos_cache[cache_idx];

                let x1 = x.get(pos, 2 * i);
                let x2 = x.get(pos, 2 * i + 1);

                // Rotate
                output.set(pos, 2 * i, x1 * cos_val - x2 * sin_val);
                output.set(pos, 2 * i + 1, x1 * sin_val + x2 * cos_val);
            }

            // Copy remaining dimensions unchanged
            for j in (2 * half_dim)..x.cols {
                output.set(pos, j, x.get(pos, j));
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
    fn test_scaled_attention() {
        let attn = ScaledDotProductAttention::new(4).with_weight_storage();

        let q = Matrix::random(3, 4, 42);
        let k = Matrix::random(3, 4, 43);
        let v = Matrix::random(3, 6, 44);

        let output = attn.forward(&q, &k, &v, &AttentionMask::None);

        assert_eq!(output.output.rows, 3);
        assert_eq!(output.output.cols, 6);
        assert!(output.weights.is_some());

        // Check attention weights sum to 1
        let weights = output.weights.unwrap();
        for i in 0..weights.rows {
            let sum: f64 = (0..weights.cols).map(|j| weights.get(i, j)).sum();
            assert!((sum - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_causal_attention() {
        let attn = ScaledDotProductAttention::new(4).with_weight_storage();

        let q = Matrix::random(4, 4, 42);
        let k = Matrix::random(4, 4, 43);
        let v = Matrix::random(4, 4, 44);

        let output = attn.forward(&q, &k, &v, &AttentionMask::causal(4));

        // Check causal pattern in weights
        let weights = output.weights.unwrap();

        // Upper triangle should be 0
        assert!(weights.get(0, 1) < 1e-10);
        assert!(weights.get(0, 2) < 1e-10);
        assert!(weights.get(1, 2) < 1e-10);
    }

    #[test]
    fn test_efficient_attention() {
        let attn = EfficientAttention::new(4, 2, 2);

        let q = Matrix::random(5, 4, 42);
        let k = Matrix::random(5, 4, 43);
        let v = Matrix::random(5, 6, 44);

        let output = attn.forward(&q, &k, &v, &AttentionMask::None);

        assert_eq!(output.rows, 5);
        assert_eq!(output.cols, 6);
    }

    #[test]
    fn test_rope() {
        let rope = RotaryPositionEmbedding::new(8, 10);
        let x = Matrix::random(5, 8, 42);

        let rotated = rope.apply(&x);

        assert_eq!(rotated.rows, 5);
        assert_eq!(rotated.cols, 8);

        // Values should be modified
        assert!(
            (rotated.get(0, 0) - x.get(0, 0)).abs() > 1e-10
                || (rotated.get(0, 1) - x.get(0, 1)).abs() > 1e-10
        );
    }
}
