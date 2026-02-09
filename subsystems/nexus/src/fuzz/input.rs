//! Fuzz input representation

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// FUZZ INPUT
// ============================================================================

/// A fuzz input
#[derive(Debug, Clone)]
pub struct FuzzInput {
    /// Unique ID
    pub id: u64,
    /// Raw data
    pub data: Vec<u8>,
    /// Generation number
    pub generation: u32,
    /// Parent input ID (if mutated)
    pub parent: Option<u64>,
    /// Coverage hash (for deduplication)
    pub coverage_hash: u64,
    /// Times this input was used
    pub use_count: u64,
    /// Interesting score
    pub score: f64,
}

impl FuzzInput {
    /// Create a new input
    pub fn new(data: Vec<u8>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            data,
            generation: 0,
            parent: None,
            coverage_hash: 0,
            use_count: 0,
            score: 0.0,
        }
    }

    /// Create a mutated child
    pub fn mutate(&self, data: Vec<u8>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            data,
            generation: self.generation + 1,
            parent: Some(self.id),
            coverage_hash: 0,
            use_count: 0,
            score: 0.0,
        }
    }

    /// Get length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Is empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Set coverage hash
    #[inline(always)]
    pub fn with_coverage(mut self, hash: u64) -> Self {
        self.coverage_hash = hash;
        self
    }

    /// Set score
    #[inline(always)]
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }

    /// Increment use count
    #[inline(always)]
    pub fn use_once(&mut self) {
        self.use_count += 1;
    }
}
