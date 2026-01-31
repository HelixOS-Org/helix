//! Corpus management

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};

use super::input::FuzzInput;

// ============================================================================
// CORPUS
// ============================================================================

/// Corpus of inputs
pub struct Corpus {
    /// Inputs by ID
    inputs: BTreeMap<u64, FuzzInput>,
    /// Inputs by coverage hash (for deduplication)
    coverage_set: BTreeSet<u64>,
    /// Maximum size
    max_size: usize,
}

impl Corpus {
    /// Create a new corpus
    pub fn new(max_size: usize) -> Self {
        Self {
            inputs: BTreeMap::new(),
            coverage_set: BTreeSet::new(),
            max_size,
        }
    }

    /// Add input if it provides new coverage
    pub fn add(&mut self, input: FuzzInput) -> bool {
        // Check if this coverage is new
        if input.coverage_hash != 0 && self.coverage_set.contains(&input.coverage_hash) {
            return false;
        }

        // Enforce max size
        if self.inputs.len() >= self.max_size {
            // Remove lowest scoring input
            if let Some((&id, _)) = self.inputs.iter().min_by(|(_, a), (_, b)| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(core::cmp::Ordering::Equal)
            }) {
                self.inputs.remove(&id);
            }
        }

        if input.coverage_hash != 0 {
            self.coverage_set.insert(input.coverage_hash);
        }
        self.inputs.insert(input.id, input);
        true
    }

    /// Get random input
    pub fn random(&self, seed: &mut u64) -> Option<&FuzzInput> {
        if self.inputs.is_empty() {
            return None;
        }

        // Simple xorshift
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;

        let idx = (*seed as usize) % self.inputs.len();
        self.inputs.values().nth(idx)
    }

    /// Get best input
    pub fn best(&self) -> Option<&FuzzInput> {
        self.inputs.values().max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
    }

    /// Get input by ID
    pub fn get(&self, id: u64) -> Option<&FuzzInput> {
        self.inputs.get(&id)
    }

    /// Get mutable input
    pub fn get_mut(&mut self, id: u64) -> Option<&mut FuzzInput> {
        self.inputs.get_mut(&id)
    }

    /// Get all inputs
    pub fn all(&self) -> impl Iterator<Item = &FuzzInput> {
        self.inputs.values()
    }

    /// Get size
    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Clear
    pub fn clear(&mut self) {
        self.inputs.clear();
        self.coverage_set.clear();
    }
}

impl Default for Corpus {
    fn default() -> Self {
        Self::new(10000)
    }
}
