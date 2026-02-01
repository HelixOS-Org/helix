//! # Flash Attention
//!
//! Memory-efficient attention with IO-aware computation.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use super::types::{Matrix, AttentionMask};

// ============================================================================
// FLASH ATTENTION V2
// ============================================================================

/// Flash Attention v2 implementation
/// Optimized for memory efficiency through tiling
pub struct FlashAttention {
    /// Block size for queries
    block_size_q: usize,
    /// Block size for keys/values
    block_size_kv: usize,
    /// Attention scale
    scale: f64,
}

impl FlashAttention {
    /// Create new Flash Attention
    pub fn new(head_dim: usize) -> Self {
        // Optimal block sizes depend on available memory
        // These are reasonable defaults
        Self {
            block_size_q: 64,
            block_size_kv: 64,
            scale: 1.0 / libm::sqrt(head_dim as f64),
        }
    }
    
    /// Set block sizes
    pub fn with_block_size(mut self, q_size: usize, kv_size: usize) -> Self {
        self.block_size_q = q_size;
        self.block_size_kv = kv_size;
        self
    }
    
    /// Forward pass using Flash Attention algorithm
    /// 
    /// This implementation uses the online softmax trick to avoid
    /// materializing the full attention matrix.
    pub fn forward(
        &self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        mask: &AttentionMask,
    ) -> Matrix {
        let seq_len_q = query.rows;
        let seq_len_kv = key.rows;
        let head_dim = query.cols;
        let value_dim = value.cols;
        
        // Output accumulator
        let mut output = Matrix::new(seq_len_q, value_dim);
        
        // Running max and sum for online softmax
        let mut row_max = alloc::vec![f64::NEG_INFINITY; seq_len_q];
        let mut row_sum = alloc::vec![0.0f64; seq_len_q];
        
        // Process KV blocks
        for kv_start in (0..seq_len_kv).step_by(self.block_size_kv) {
            let kv_end = (kv_start + self.block_size_kv).min(seq_len_kv);
            let kv_block_size = kv_end - kv_start;
            
            // Extract KV block
            let mut k_block = Matrix::new(kv_block_size, head_dim);
            let mut v_block = Matrix::new(kv_block_size, value_dim);
            
            for i in 0..kv_block_size {
                for j in 0..head_dim {
                    k_block.set(i, j, key.get(kv_start + i, j));
                }
                for j in 0..value_dim {
                    v_block.set(i, j, value.get(kv_start + i, j));
                }
            }
            
            // Process query blocks
            for q_start in (0..seq_len_q).step_by(self.block_size_q) {
                let q_end = (q_start + self.block_size_q).min(seq_len_q);
                let q_block_size = q_end - q_start;
                
                // Extract Q block
                let mut q_block = Matrix::new(q_block_size, head_dim);
                for i in 0..q_block_size {
                    for j in 0..head_dim {
                        q_block.set(i, j, query.get(q_start + i, j));
                    }
                }
                
                // Compute attention scores: Q_block @ K_block^T
                let k_t = k_block.transpose();
                let mut scores = q_block.matmul(&k_t)
                    .unwrap_or_else(|| Matrix::new(q_block_size, kv_block_size));
                scores = scores.scale(self.scale);
                
                // Apply mask
                self.apply_block_mask(
                    &mut scores,
                    mask,
                    q_start,
                    kv_start,
                );
                
                // Online softmax update
                for qi in 0..q_block_size {
                    let global_qi = q_start + qi;
                    let old_max = row_max[global_qi];
                    
                    // Find block max
                    let mut block_max = f64::NEG_INFINITY;
                    for ki in 0..kv_block_size {
                        block_max = block_max.max(scores.get(qi, ki));
                    }
                    
                    let new_max = old_max.max(block_max);
                    row_max[global_qi] = new_max;
                    
                    // Compute correction factor
                    let old_correction = if old_max > f64::NEG_INFINITY {
                        libm::exp(old_max - new_max)
                    } else {
                        0.0
                    };
                    
                    // Rescale existing output and sum
                    row_sum[global_qi] *= old_correction;
                    for j in 0..value_dim {
                        output.set(global_qi, j, output.get(global_qi, j) * old_correction);
                    }
                    
                    // Accumulate new block contribution
                    for ki in 0..kv_block_size {
                        let score = scores.get(qi, ki);
                        if score > f64::NEG_INFINITY {
                            let exp_score = libm::exp(score - new_max);
                            row_sum[global_qi] += exp_score;
                            
                            for j in 0..value_dim {
                                let val = output.get(global_qi, j) + exp_score * v_block.get(ki, j);
                                output.set(global_qi, j, val);
                            }
                        }
                    }
                }
            }
        }
        
        // Final normalization
        for i in 0..seq_len_q {
            if row_sum[i] > 1e-10 {
                for j in 0..value_dim {
                    output.set(i, j, output.get(i, j) / row_sum[i]);
                }
            }
        }
        
        output
    }
    
    /// Apply mask to score block
    fn apply_block_mask(
        &self,
        scores: &mut Matrix,
        mask: &AttentionMask,
        q_offset: usize,
        kv_offset: usize,
    ) {
        match mask {
            AttentionMask::None => {}
            AttentionMask::Causal(max_len) => {
                for qi in 0..scores.rows {
                    for ki in 0..scores.cols {
                        let global_q = q_offset + qi;
                        let global_k = kv_offset + ki;
                        
                        if global_k > global_q || global_q >= *max_len || global_k >= *max_len {
                            scores.set(qi, ki, f64::NEG_INFINITY);
                        }
                    }
                }
            }
            AttentionMask::Padding(mask_vec) => {
                for qi in 0..scores.rows {
                    for ki in 0..scores.cols {
                        let global_k = kv_offset + ki;
                        if global_k < mask_vec.len() && !mask_vec[global_k] {
                            scores.set(qi, ki, f64::NEG_INFINITY);
                        }
                    }
                }
            }
            AttentionMask::Custom(custom_mask) => {
                for qi in 0..scores.rows {
                    for ki in 0..scores.cols {
                        let global_q = q_offset + qi;
                        let global_k = kv_offset + ki;
                        
                        if custom_mask.get(global_q, global_k) == 0.0 {
                            scores.set(qi, ki, f64::NEG_INFINITY);
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// FLASH ATTENTION WITH SLIDING WINDOW
// ============================================================================

/// Flash Attention with sliding window for very long sequences
pub struct SlidingWindowFlashAttention {
    /// Base flash attention
    base: FlashAttention,
    /// Window size
    window_size: usize,
}

impl SlidingWindowFlashAttention {
    /// Create sliding window attention
    pub fn new(head_dim: usize, window_size: usize) -> Self {
        Self {
            base: FlashAttention::new(head_dim),
            window_size,
        }
    }
    
    /// Forward with sliding window
    pub fn forward(
        &self,
        query: &Matrix,
        key: &Matrix,
        value: &Matrix,
        causal: bool,
    ) -> Matrix {
        let seq_len_q = query.rows;
        let seq_len_kv = key.rows;
        let head_dim = query.cols;
        let value_dim = value.cols;
        
        let mut output = Matrix::new(seq_len_q, value_dim);
        let mut row_max = alloc::vec![f64::NEG_INFINITY; seq_len_q];
        let mut row_sum = alloc::vec![0.0f64; seq_len_q];
        
        // For each query position, only attend to window
        for qi in 0..seq_len_q {
            let window_start = if qi >= self.window_size {
                qi - self.window_size + 1
            } else {
                0
            };
            let window_end = if causal {
                (qi + 1).min(seq_len_kv)
            } else {
                (qi + self.window_size).min(seq_len_kv)
            };
            
            // Compute attention within window
            let mut max_score = f64::NEG_INFINITY;
            let mut scores = Vec::with_capacity(window_end - window_start);
            
            for ki in window_start..window_end {
                let mut score = 0.0;
                for d in 0..head_dim {
                    score += query.get(qi, d) * key.get(ki, d);
                }
                score *= self.base.scale;
                max_score = max_score.max(score);
                scores.push((ki, score));
            }
            
            row_max[qi] = max_score;
            
            // Softmax and accumulate
            for (ki, score) in &scores {
                let exp_score = libm::exp(*score - max_score);
                row_sum[qi] += exp_score;
                
                for j in 0..value_dim {
                    let val = output.get(qi, j) + exp_score * value.get(*ki, j);
                    output.set(qi, j, val);
                }
            }
            
            // Normalize
            if row_sum[qi] > 1e-10 {
                for j in 0..value_dim {
                    output.set(qi, j, output.get(qi, j) / row_sum[qi]);
                }
            }
        }
        
        output
    }
}

// ============================================================================
// PAGED ATTENTION
// ============================================================================

/// Page table entry for paged attention
#[derive(Debug, Clone)]
pub struct PageTableEntry {
    /// Physical page index
    pub physical_page: usize,
    /// Valid flag
    pub valid: bool,
}

/// Paged attention for efficient KV cache management
pub struct PagedAttention {
    /// Page size (number of tokens per page)
    pub page_size: usize,
    /// Physical pages for keys
    pub key_pages: Vec<Matrix>,
    /// Physical pages for values
    pub value_pages: Vec<Matrix>,
    /// Page tables per sequence
    pub page_tables: Vec<Vec<PageTableEntry>>,
    /// Free pages
    free_pages: Vec<usize>,
    /// Head dimension
    head_dim: usize,
}

impl PagedAttention {
    /// Create paged attention
    pub fn new(page_size: usize, head_dim: usize, initial_pages: usize) -> Self {
        let mut key_pages = Vec::with_capacity(initial_pages);
        let mut value_pages = Vec::with_capacity(initial_pages);
        let mut free_pages = Vec::with_capacity(initial_pages);
        
        for i in 0..initial_pages {
            key_pages.push(Matrix::new(page_size, head_dim));
            value_pages.push(Matrix::new(page_size, head_dim));
            free_pages.push(i);
        }
        
        Self {
            page_size,
            key_pages,
            value_pages,
            page_tables: Vec::new(),
            free_pages,
            head_dim,
        }
    }
    
    /// Allocate page for sequence
    pub fn allocate_page(&mut self, seq_id: usize) -> Option<usize> {
        let physical_page = self.free_pages.pop()?;
        
        // Ensure page table exists
        while self.page_tables.len() <= seq_id {
            self.page_tables.push(Vec::new());
        }
        
        let logical_page = self.page_tables[seq_id].len();
        self.page_tables[seq_id].push(PageTableEntry {
            physical_page,
            valid: true,
        });
        
        Some(logical_page)
    }
    
    /// Free all pages for sequence
    pub fn free_sequence(&mut self, seq_id: usize) {
        if seq_id < self.page_tables.len() {
            for entry in &self.page_tables[seq_id] {
                if entry.valid {
                    self.free_pages.push(entry.physical_page);
                }
            }
            self.page_tables[seq_id].clear();
        }
    }
    
    /// Write to cache
    pub fn write(
        &mut self,
        seq_id: usize,
        position: usize,
        key: &[f64],
        value: &[f64],
    ) -> bool {
        let logical_page = position / self.page_size;
        let offset = position % self.page_size;
        
        // Ensure we have enough pages
        while self.page_tables.get(seq_id).map(|pt| pt.len()).unwrap_or(0) <= logical_page {
            if self.allocate_page(seq_id).is_none() {
                return false;  // Out of pages
            }
        }
        
        let physical_page = self.page_tables[seq_id][logical_page].physical_page;
        
        // Write key
        for (j, &v) in key.iter().enumerate().take(self.head_dim) {
            self.key_pages[physical_page].set(offset, j, v);
        }
        
        // Write value
        for (j, &v) in value.iter().enumerate().take(self.head_dim) {
            self.value_pages[physical_page].set(offset, j, v);
        }
        
        true
    }
    
    /// Read from cache
    pub fn read(
        &self,
        seq_id: usize,
        position: usize,
    ) -> Option<(Vec<f64>, Vec<f64>)> {
        let logical_page = position / self.page_size;
        let offset = position % self.page_size;
        
        let page_table = self.page_tables.get(seq_id)?;
        let entry = page_table.get(logical_page)?;
        
        if !entry.valid {
            return None;
        }
        
        let physical_page = entry.physical_page;
        
        let key: Vec<f64> = (0..self.head_dim)
            .map(|j| self.key_pages[physical_page].get(offset, j))
            .collect();
        
        let value: Vec<f64> = (0..self.head_dim)
            .map(|j| self.value_pages[physical_page].get(offset, j))
            .collect();
        
        Some((key, value))
    }
    
    /// Get sequence length
    pub fn sequence_length(&self, seq_id: usize) -> usize {
        self.page_tables.get(seq_id)
            .map(|pt| pt.len() * self.page_size)
            .unwrap_or(0)
    }
    
    /// Compute attention for query against paged cache
    pub fn attention(
        &self,
        seq_id: usize,
        query: &Matrix,
        context_len: usize,
        causal: bool,
    ) -> Matrix {
        let query_len = query.rows;
        let scale = 1.0 / libm::sqrt(self.head_dim as f64);
        
        let mut output = Matrix::new(query_len, self.head_dim);
        
        for qi in 0..query_len {
            let max_ki = if causal {
                (context_len + qi + 1).min(context_len)
            } else {
                context_len
            };
            
            // Compute scores
            let mut max_score = f64::NEG_INFINITY;
            let mut scores = Vec::with_capacity(max_ki);
            
            for ki in 0..max_ki {
                if let Some((key, _)) = self.read(seq_id, ki) {
                    let mut score = 0.0;
                    for d in 0..self.head_dim {
                        score += query.get(qi, d) * key[d];
                    }
                    score *= scale;
                    max_score = max_score.max(score);
                    scores.push((ki, score));
                }
            }
            
            // Softmax and accumulate
            let mut sum_exp = 0.0;
            let mut acc = alloc::vec![0.0; self.head_dim];
            
            for (ki, score) in &scores {
                let exp_score = libm::exp(*score - max_score);
                sum_exp += exp_score;
                
                if let Some((_, value)) = self.read(seq_id, *ki) {
                    for (d, &v) in value.iter().enumerate() {
                        acc[d] += exp_score * v;
                    }
                }
            }
            
            // Normalize and store
            if sum_exp > 1e-10 {
                for (d, &v) in acc.iter().enumerate() {
                    output.set(qi, d, v / sum_exp);
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
    fn test_flash_attention() {
        let flash = FlashAttention::new(64)
            .with_block_size(16, 16);
        
        let q = Matrix::random(32, 64, 42);
        let k = Matrix::random(32, 64, 43);
        let v = Matrix::random(32, 64, 44);
        
        let output = flash.forward(&q, &k, &v, &AttentionMask::None);
        
        assert_eq!(output.rows, 32);
        assert_eq!(output.cols, 64);
    }
    
    #[test]
    fn test_flash_attention_causal() {
        let flash = FlashAttention::new(32)
            .with_block_size(8, 8);
        
        let q = Matrix::random(16, 32, 42);
        let k = Matrix::random(16, 32, 43);
        let v = Matrix::random(16, 32, 44);
        
        let output = flash.forward(&q, &k, &v, &AttentionMask::causal(16));
        
        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 32);
    }
    
    #[test]
    fn test_sliding_window() {
        let sw = SlidingWindowFlashAttention::new(32, 8);
        
        let q = Matrix::random(20, 32, 42);
        let k = Matrix::random(20, 32, 43);
        let v = Matrix::random(20, 32, 44);
        
        let output = sw.forward(&q, &k, &v, true);
        
        assert_eq!(output.rows, 20);
        assert_eq!(output.cols, 32);
    }
    
    #[test]
    fn test_paged_attention() {
        let mut paged = PagedAttention::new(16, 32, 10);
        
        // Allocate pages for sequence 0
        paged.allocate_page(0);
        paged.allocate_page(0);
        
        // Write some data
        let key = alloc::vec![1.0; 32];
        let value = alloc::vec![2.0; 32];
        
        assert!(paged.write(0, 0, &key, &value));
        
        // Read back
        let (read_key, read_value) = paged.read(0, 0).unwrap();
        
        assert!((read_key[0] - 1.0).abs() < 1e-10);
        assert!((read_value[0] - 2.0).abs() < 1e-10);
    }
}
