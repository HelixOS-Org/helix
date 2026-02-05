//! Type aliases and constants for zero-shot learning.

extern crate alloc;

use alloc::vec::Vec;

/// Semantic embedding dimension
pub const EMBEDDING_DIM: usize = 128;

/// Attribute dimension
pub const ATTRIBUTE_DIM: usize = 64;

/// Feature vector
pub type FeatureVector = Vec<f64>;

/// Embedding vector
pub type EmbeddingVector = Vec<f64>;

/// Attribute vector
pub type AttributeVector = Vec<f64>;

/// Class identifier
pub type ClassId = u32;

/// Euclidean distance between vectors
pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let diff = x - y;
        sum += diff * diff;
    }
    libm::sqrt(sum)
}

/// Cosine similarity
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = libm::sqrt(norm_a) * libm::sqrt(norm_b);
    if denom > 1e-8 { dot / denom } else { 0.0 }
}
