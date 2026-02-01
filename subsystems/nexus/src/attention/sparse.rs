//! # Sparse Attention
//!
//! Sparse attention patterns for efficient long-range modeling.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::types::Matrix;

// ============================================================================
// SPARSE PATTERN TYPES
// ============================================================================

/// Type of sparse attention pattern
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SparsePatternType {
    /// Local window attention
    Local,
    /// Strided attention (every k positions)
    Strided,
    /// Block sparse
    BlockSparse,
    /// Random sparse
    Random,
    /// Longformer-style (local + global)
    Longformer,
    /// BigBird-style (local + global + random)
    BigBird,
    /// Star-shaped (all attend to first/last)
    Star,
}

/// Sparse attention pattern
pub struct SparsePattern {
    /// Pattern type
    pub pattern_type: SparsePatternType,
    /// Sequence length
    pub seq_len: usize,
    /// Adjacency list: for each query, list of keys to attend to
    pub adjacency: Vec<Vec<usize>>,
}

impl SparsePattern {
    /// Create local (sliding window) pattern
    pub fn local(seq_len: usize, window_size: usize) -> Self {
        let mut adjacency = Vec::with_capacity(seq_len);

        for q in 0..seq_len {
            let start = q.saturating_sub(window_size / 2);
            let end = (q + window_size / 2 + 1).min(seq_len);

            let keys: Vec<usize> = (start..end).collect();
            adjacency.push(keys);
        }

        Self {
            pattern_type: SparsePatternType::Local,
            seq_len,
            adjacency,
        }
    }

    /// Create strided pattern
    pub fn strided(seq_len: usize, stride: usize) -> Self {
        let mut adjacency = Vec::with_capacity(seq_len);

        for q in 0..seq_len {
            let keys: Vec<usize> = (0..seq_len).filter(|&k| k % stride == q % stride).collect();
            adjacency.push(keys);
        }

        Self {
            pattern_type: SparsePatternType::Strided,
            seq_len,
            adjacency,
        }
    }

    /// Create block sparse pattern
    pub fn block_sparse(seq_len: usize, block_size: usize) -> Self {
        let num_blocks = (seq_len + block_size - 1) / block_size;
        let mut adjacency = Vec::with_capacity(seq_len);

        for q in 0..seq_len {
            let q_block = q / block_size;
            let mut keys = Vec::new();

            // Same block
            let block_start = q_block * block_size;
            let block_end = ((q_block + 1) * block_size).min(seq_len);
            keys.extend(block_start..block_end);

            // Previous and next blocks
            if q_block > 0 {
                let prev_start = (q_block - 1) * block_size;
                let prev_end = q_block * block_size;
                keys.extend(prev_start..prev_end);
            }
            if q_block + 1 < num_blocks {
                let next_start = (q_block + 1) * block_size;
                let next_end = ((q_block + 2) * block_size).min(seq_len);
                keys.extend(next_start..next_end);
            }

            keys.sort_unstable();
            keys.dedup();
            adjacency.push(keys);
        }

        Self {
            pattern_type: SparsePatternType::BlockSparse,
            seq_len,
            adjacency,
        }
    }

    /// Create random sparse pattern
    pub fn random(seq_len: usize, num_connections: usize, seed: u64) -> Self {
        let mut adjacency = Vec::with_capacity(seq_len);
        let mut rng_state = seed;

        for _ in 0..seq_len {
            let mut keys = Vec::with_capacity(num_connections);

            for _ in 0..num_connections {
                // Simple LCG random
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let k = (rng_state >> 33) as usize % seq_len;
                keys.push(k);
            }

            keys.sort_unstable();
            keys.dedup();
            adjacency.push(keys);
        }

        Self {
            pattern_type: SparsePatternType::Random,
            seq_len,
            adjacency,
        }
    }

    /// Create Longformer-style pattern (local + global tokens)
    pub fn longformer(seq_len: usize, window_size: usize, global_indices: &[usize]) -> Self {
        let mut adjacency = Vec::with_capacity(seq_len);

        for q in 0..seq_len {
            let mut keys = Vec::new();

            // Local window
            let start = q.saturating_sub(window_size / 2);
            let end = (q + window_size / 2 + 1).min(seq_len);
            keys.extend(start..end);

            // Global tokens
            keys.extend(global_indices.iter().copied());

            // If query is global, attend to all
            if global_indices.contains(&q) {
                keys = (0..seq_len).collect();
            }

            keys.sort_unstable();
            keys.dedup();
            adjacency.push(keys);
        }

        Self {
            pattern_type: SparsePatternType::Longformer,
            seq_len,
            adjacency,
        }
    }

    /// Create BigBird-style pattern (local + global + random)
    pub fn bigbird(
        seq_len: usize,
        window_size: usize,
        num_global: usize,
        num_random: usize,
        seed: u64,
    ) -> Self {
        let mut adjacency = Vec::with_capacity(seq_len);
        let mut rng_state = seed;

        // Global tokens are first num_global positions
        let global_indices: Vec<usize> = (0..num_global.min(seq_len)).collect();

        for q in 0..seq_len {
            let mut keys = Vec::new();

            // Local window
            let start = q.saturating_sub(window_size / 2);
            let end = (q + window_size / 2 + 1).min(seq_len);
            keys.extend(start..end);

            // Global tokens
            keys.extend(global_indices.iter().copied());

            // Random tokens
            for _ in 0..num_random {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let k = (rng_state >> 33) as usize % seq_len;
                keys.push(k);
            }

            // If query is global, attend to all
            if global_indices.contains(&q) {
                keys = (0..seq_len).collect();
            }

            keys.sort_unstable();
            keys.dedup();
            adjacency.push(keys);
        }

        Self {
            pattern_type: SparsePatternType::BigBird,
            seq_len,
            adjacency,
        }
    }

    /// Create star pattern (all attend to first and last)
    pub fn star(seq_len: usize, window_size: usize) -> Self {
        let mut adjacency = Vec::with_capacity(seq_len);

        for q in 0..seq_len {
            let mut keys = Vec::new();

            // Always attend to first and last
            keys.push(0);
            if seq_len > 1 {
                keys.push(seq_len - 1);
            }

            // Local window
            let start = q.saturating_sub(window_size / 2);
            let end = (q + window_size / 2 + 1).min(seq_len);
            keys.extend(start..end);

            // First and last attend to all
            if q == 0 || q == seq_len - 1 {
                keys = (0..seq_len).collect();
            }

            keys.sort_unstable();
            keys.dedup();
            adjacency.push(keys);
        }

        Self {
            pattern_type: SparsePatternType::Star,
            seq_len,
            adjacency,
        }
    }

    /// Combine two patterns (union)
    pub fn combine(&self, other: &SparsePattern) -> SparsePattern {
        assert_eq!(self.seq_len, other.seq_len);

        let mut adjacency = Vec::with_capacity(self.seq_len);

        for q in 0..self.seq_len {
            let mut keys: Vec<usize> = self.adjacency[q]
                .iter()
                .chain(other.adjacency[q].iter())
                .copied()
                .collect();

            keys.sort_unstable();
            keys.dedup();
            adjacency.push(keys);
        }

        SparsePattern {
            pattern_type: SparsePatternType::Random, // Combined pattern
            seq_len: self.seq_len,
            adjacency,
        }
    }

    /// Get sparsity ratio
    pub fn sparsity(&self) -> f64 {
        let total_edges: usize = self.adjacency.iter().map(|k| k.len()).sum();
        let dense_edges = self.seq_len * self.seq_len;

        1.0 - (total_edges as f64 / dense_edges as f64)
    }

    /// Apply causal mask
    pub fn make_causal(&mut self) {
        for q in 0..self.seq_len {
            self.adjacency[q].retain(|&k| k <= q);
        }
    }
}

// ============================================================================
// SPARSE ATTENTION
// ============================================================================

/// Sparse attention mechanism
pub struct SparseAttention {
    /// Scale factor
    scale: f64,
    /// Pattern
    pattern: SparsePattern,
}

impl SparseAttention {
    /// Create sparse attention with pattern
    pub fn new(head_dim: usize, pattern: SparsePattern) -> Self {
        Self {
            scale: 1.0 / libm::sqrt(head_dim as f64),
            pattern,
        }
    }

    /// Create with local pattern
    pub fn local(head_dim: usize, seq_len: usize, window_size: usize) -> Self {
        let pattern = SparsePattern::local(seq_len, window_size);
        Self::new(head_dim, pattern)
    }

    /// Create with BigBird pattern
    pub fn bigbird(
        head_dim: usize,
        seq_len: usize,
        window_size: usize,
        num_global: usize,
        num_random: usize,
    ) -> Self {
        let pattern = SparsePattern::bigbird(seq_len, window_size, num_global, num_random, 42);
        Self::new(head_dim, pattern)
    }

    /// Forward pass
    pub fn forward(&self, query: &Matrix, key: &Matrix, value: &Matrix) -> Matrix {
        let seq_len = query.rows;
        let head_dim = query.cols;
        let value_dim = value.cols;

        let mut output = Matrix::new(seq_len, value_dim);

        for q in 0..seq_len {
            let keys_to_attend = &self.pattern.adjacency[q];

            if keys_to_attend.is_empty() {
                continue;
            }

            // Compute scores for sparse positions
            let mut scores: Vec<f64> = Vec::with_capacity(keys_to_attend.len());
            let mut max_score = f64::NEG_INFINITY;

            for &k in keys_to_attend {
                let mut score = 0.0;
                for d in 0..head_dim {
                    score += query.get(q, d) * key.get(k, d);
                }
                score *= self.scale;
                max_score = max_score.max(score);
                scores.push(score);
            }

            // Softmax
            let mut sum_exp = 0.0;
            for score in &mut scores {
                *score = libm::exp(*score - max_score);
                sum_exp += *score;
            }

            if sum_exp > 1e-10 {
                for score in &mut scores {
                    *score /= sum_exp;
                }
            }

            // Weighted sum of values
            for (i, &k) in keys_to_attend.iter().enumerate() {
                let weight = scores[i];
                for d in 0..value_dim {
                    let val = output.get(q, d) + weight * value.get(k, d);
                    output.set(q, d, val);
                }
            }
        }

        output
    }

    /// Get sparsity ratio
    pub fn sparsity(&self) -> f64 {
        self.pattern.sparsity()
    }
}

// ============================================================================
// BLOCK SPARSE ATTENTION
// ============================================================================

/// Block-based sparse attention
pub struct BlockSparseAttention {
    /// Block size
    block_size: usize,
    /// Scale factor
    scale: f64,
    /// Block pattern (which blocks attend to which)
    block_pattern: Vec<Vec<usize>>,
}

impl BlockSparseAttention {
    /// Create block sparse attention
    pub fn new(head_dim: usize, seq_len: usize, block_size: usize) -> Self {
        let num_blocks = (seq_len + block_size - 1) / block_size;

        // Default: each block attends to itself and adjacent blocks
        let mut block_pattern = Vec::with_capacity(num_blocks);
        for b in 0..num_blocks {
            let mut attending = Vec::new();
            if b > 0 {
                attending.push(b - 1);
            }
            attending.push(b);
            if b + 1 < num_blocks {
                attending.push(b + 1);
            }
            block_pattern.push(attending);
        }

        Self {
            block_size,
            scale: 1.0 / libm::sqrt(head_dim as f64),
            block_pattern,
        }
    }

    /// Create with custom block pattern
    pub fn with_pattern(head_dim: usize, block_size: usize, pattern: Vec<Vec<usize>>) -> Self {
        Self {
            block_size,
            scale: 1.0 / libm::sqrt(head_dim as f64),
            block_pattern: pattern,
        }
    }

    /// Forward pass
    pub fn forward(&self, query: &Matrix, key: &Matrix, value: &Matrix) -> Matrix {
        let seq_len = query.rows;
        let head_dim = query.cols;
        let value_dim = value.cols;

        let mut output = Matrix::new(seq_len, value_dim);

        let num_blocks = (seq_len + self.block_size - 1) / self.block_size;

        for q_block in 0..num_blocks {
            let q_start = q_block * self.block_size;
            let q_end = ((q_block + 1) * self.block_size).min(seq_len);

            let attending_blocks = &self.block_pattern[q_block];

            // Gather all key positions for this query block
            let mut key_positions: Vec<usize> = Vec::new();
            for &k_block in attending_blocks {
                let k_start = k_block * self.block_size;
                let k_end = ((k_block + 1) * self.block_size).min(seq_len);
                key_positions.extend(k_start..k_end);
            }

            // Compute attention for each query in block
            for q in q_start..q_end {
                let mut scores: Vec<f64> = Vec::with_capacity(key_positions.len());
                let mut max_score = f64::NEG_INFINITY;

                for &k in &key_positions {
                    let mut score = 0.0;
                    for d in 0..head_dim {
                        score += query.get(q, d) * key.get(k, d);
                    }
                    score *= self.scale;
                    max_score = max_score.max(score);
                    scores.push(score);
                }

                // Softmax
                let mut sum_exp = 0.0;
                for score in &mut scores {
                    *score = libm::exp(*score - max_score);
                    sum_exp += *score;
                }

                if sum_exp > 1e-10 {
                    for score in &mut scores {
                        *score /= sum_exp;
                    }
                }

                // Weighted sum
                for (i, &k) in key_positions.iter().enumerate() {
                    let weight = scores[i];
                    for d in 0..value_dim {
                        let val = output.get(q, d) + weight * value.get(k, d);
                        output.set(q, d, val);
                    }
                }
            }
        }

        output
    }

    /// Get number of blocks
    pub fn num_blocks(&self) -> usize {
        self.block_pattern.len()
    }
}

// ============================================================================
// DILATED ATTENTION
// ============================================================================

/// Dilated attention with multiple dilation rates
pub struct DilatedAttention {
    /// Dilation rates per head
    dilation_rates: Vec<usize>,
    /// Scale factor
    scale: f64,
    /// Segment length for each dilation
    segment_len: usize,
}

impl DilatedAttention {
    /// Create dilated attention
    pub fn new(head_dim: usize, dilation_rates: Vec<usize>, segment_len: usize) -> Self {
        Self {
            dilation_rates,
            scale: 1.0 / libm::sqrt(head_dim as f64),
            segment_len,
        }
    }

    /// Create with exponentially increasing rates
    pub fn exponential(head_dim: usize, num_heads: usize, segment_len: usize) -> Self {
        let rates: Vec<usize> = (0..num_heads)
            .map(|h| 1 << h)  // 1, 2, 4, 8, ...
            .collect();

        Self::new(head_dim, rates, segment_len)
    }

    /// Forward for single head with dilation
    pub fn forward_head(
        &self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        head_idx: usize,
    ) -> Matrix {
        let dilation = self.dilation_rates.get(head_idx).copied().unwrap_or(1);
        let seq_len = query.rows;
        let value_dim = value.cols;
        let head_dim = query.cols;

        let mut output = Matrix::new(seq_len, value_dim);

        for q in 0..seq_len {
            // Gather dilated positions within segment
            let segment_start = (q / self.segment_len) * self.segment_len;
            let segment_end = (segment_start + self.segment_len).min(seq_len);

            let mut key_positions: Vec<usize> = Vec::new();
            let mut pos = segment_start;
            while pos < segment_end {
                key_positions.push(pos);
                pos += dilation;
            }

            if key_positions.is_empty() {
                continue;
            }

            // Compute attention
            let mut scores: Vec<f64> = Vec::with_capacity(key_positions.len());
            let mut max_score = f64::NEG_INFINITY;

            for &k in &key_positions {
                let mut score = 0.0;
                for d in 0..head_dim {
                    score += query.get(q, d) * key.get(k, d);
                }
                score *= self.scale;
                max_score = max_score.max(score);
                scores.push(score);
            }

            // Softmax
            let mut sum_exp = 0.0;
            for score in &mut scores {
                *score = libm::exp(*score - max_score);
                sum_exp += *score;
            }

            if sum_exp > 1e-10 {
                for score in &mut scores {
                    *score /= sum_exp;
                }
            }

            // Weighted sum
            for (i, &k) in key_positions.iter().enumerate() {
                let weight = scores[i];
                for d in 0..value_dim {
                    let val = output.get(q, d) + weight * value.get(k, d);
                    output.set(q, d, val);
                }
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
    fn test_local_pattern() {
        let pattern = SparsePattern::local(32, 8);

        assert_eq!(pattern.seq_len, 32);
        assert!(pattern.sparsity() > 0.0);

        // Check local window
        assert!(pattern.adjacency[16].contains(&12));
        assert!(pattern.adjacency[16].contains(&20));
        assert!(!pattern.adjacency[16].contains(&0));
    }

    #[test]
    fn test_strided_pattern() {
        let pattern = SparsePattern::strided(32, 4);

        // Position 0 should attend to 0, 4, 8, 12, ...
        assert!(pattern.adjacency[0].contains(&0));
        assert!(pattern.adjacency[0].contains(&4));
        assert!(pattern.adjacency[0].contains(&8));
        assert!(!pattern.adjacency[0].contains(&1));
    }

    #[test]
    fn test_bigbird_pattern() {
        let pattern = SparsePattern::bigbird(64, 8, 4, 3, 42);

        // First 4 positions should attend to all (global)
        assert_eq!(pattern.adjacency[0].len(), 64);

        // Non-global should have limited attention
        assert!(pattern.adjacency[32].len() < 64);
    }

    #[test]
    fn test_sparse_attention() {
        let attn = SparseAttention::local(32, 16, 4);

        let q = Matrix::random(16, 32, 42);
        let k = Matrix::random(16, 32, 43);
        let v = Matrix::random(16, 32, 44);

        let output = attn.forward(&q, &k, &v);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 32);
    }

    #[test]
    fn test_block_sparse_attention() {
        let attn = BlockSparseAttention::new(32, 32, 8);

        let q = Matrix::random(32, 32, 42);
        let k = Matrix::random(32, 32, 43);
        let v = Matrix::random(32, 32, 44);

        let output = attn.forward(&q, &k, &v);

        assert_eq!(output.rows, 32);
        assert_eq!(output.cols, 32);
    }

    #[test]
    fn test_dilated_attention() {
        let attn = DilatedAttention::exponential(32, 4, 16);

        let q = Matrix::random(32, 32, 42);
        let k = Matrix::random(32, 32, 43);
        let v = Matrix::random(32, 32, 44);

        let output = attn.forward_head(&q, &k, &v, 0);

        assert_eq!(output.rows, 32);
        assert_eq!(output.cols, 32);
    }

    #[test]
    fn test_pattern_combination() {
        let local = SparsePattern::local(32, 4);
        let strided = SparsePattern::strided(32, 8);

        let combined = local.combine(&strided);

        // Combined should have more connections
        for q in 0..32 {
            assert!(combined.adjacency[q].len() >= local.adjacency[q].len());
        }
    }
}
