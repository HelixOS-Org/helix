//! NEXUS Year 2: Vector Embeddings
//!
//! Dense vector representations for concepts, entities, and symbols.
//! Supports various encoding strategies and embedding operations.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for embeddings
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmbeddingId(pub u64);

impl EmbeddingId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// A dense vector embedding
#[derive(Debug, Clone)]
pub struct Embedding {
    pub id: EmbeddingId,
    pub name: Option<String>,
    pub vector: Vec<f32>,
    pub metadata: EmbeddingMetadata,
}

/// Metadata associated with an embedding
#[derive(Debug, Clone, Default)]
pub struct EmbeddingMetadata {
    pub source: Option<String>,
    pub created_at: u64,
    pub version: u32,
    pub tags: Vec<String>,
}

impl Embedding {
    pub fn new(id: EmbeddingId, vector: Vec<f32>) -> Self {
        Self {
            id,
            name: None,
            vector,
            metadata: EmbeddingMetadata::default(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_metadata(mut self, metadata: EmbeddingMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn dim(&self) -> usize {
        self.vector.len()
    }

    /// L2 norm of the embedding
    pub fn norm(&self) -> f32 {
        let sum_sq: f32 = self.vector.iter().map(|x| x * x).sum();
        libm::sqrtf(sum_sq)
    }

    /// Normalize to unit vector
    pub fn normalize(&mut self) {
        let n = self.norm();
        if n > 1e-10 {
            for x in &mut self.vector {
                *x /= n;
            }
        }
    }

    /// Return normalized copy
    pub fn normalized(&self) -> Self {
        let mut copy = self.clone();
        copy.normalize();
        copy
    }

    /// Dot product with another embedding
    pub fn dot(&self, other: &Embedding) -> f32 {
        self.vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    /// Cosine similarity with another embedding
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        let dot = self.dot(other);
        let norm_a = self.norm();
        let norm_b = other.norm();

        if norm_a > 1e-10 && norm_b > 1e-10 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    /// Euclidean distance to another embedding
    pub fn euclidean_distance(&self, other: &Embedding) -> f32 {
        let sum_sq: f32 = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum();
        libm::sqrtf(sum_sq)
    }

    /// Add two embeddings element-wise
    pub fn add(&self, other: &Embedding) -> Embedding {
        let vector: Vec<f32> = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a + b)
            .collect();

        Embedding::new(EmbeddingId::new(0), vector)
    }

    /// Subtract embedding element-wise
    pub fn sub(&self, other: &Embedding) -> Embedding {
        let vector: Vec<f32> = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a - b)
            .collect();

        Embedding::new(EmbeddingId::new(0), vector)
    }

    /// Scale embedding by scalar
    pub fn scale(&self, factor: f32) -> Embedding {
        let vector: Vec<f32> = self.vector.iter().map(|x| x * factor).collect();

        Embedding::new(EmbeddingId::new(0), vector)
    }

    /// Hadamard (element-wise) product
    pub fn hadamard(&self, other: &Embedding) -> Embedding {
        let vector: Vec<f32> = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a * b)
            .collect();

        Embedding::new(EmbeddingId::new(0), vector)
    }

    /// Linear interpolation between two embeddings
    pub fn lerp(&self, other: &Embedding, t: f32) -> Embedding {
        let vector: Vec<f32> = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a * (1.0 - t) + b * t)
            .collect();

        Embedding::new(EmbeddingId::new(0), vector)
    }
}

// ============================================================================
// Embedding Space
// ============================================================================

/// Collection of embeddings in a vector space
pub struct EmbeddingSpace {
    name: String,
    dimension: usize,
    embeddings: BTreeMap<EmbeddingId, Embedding>,
    name_index: BTreeMap<String, EmbeddingId>,
    next_id: u64,
}

impl EmbeddingSpace {
    pub fn new(name: impl Into<String>, dimension: usize) -> Self {
        Self {
            name: name.into(),
            dimension,
            embeddings: BTreeMap::new(),
            name_index: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    /// Add an embedding to the space
    pub fn add(&mut self, mut embedding: Embedding) -> EmbeddingId {
        if embedding.dim() != self.dimension {
            // Resize or reject
            embedding.vector.resize(self.dimension, 0.0);
        }

        let id = EmbeddingId::new(self.next_id);
        self.next_id += 1;

        embedding.id = id;

        if let Some(ref name) = embedding.name {
            self.name_index.insert(name.clone(), id);
        }

        self.embeddings.insert(id, embedding);
        id
    }

    /// Get embedding by ID
    pub fn get(&self, id: EmbeddingId) -> Option<&Embedding> {
        self.embeddings.get(&id)
    }

    /// Get embedding by name
    pub fn get_by_name(&self, name: &str) -> Option<&Embedding> {
        self.name_index
            .get(name)
            .and_then(|id| self.embeddings.get(id))
    }

    /// Remove an embedding
    pub fn remove(&mut self, id: EmbeddingId) -> Option<Embedding> {
        if let Some(embedding) = self.embeddings.remove(&id) {
            if let Some(ref name) = embedding.name {
                self.name_index.remove(name);
            }
            Some(embedding)
        } else {
            None
        }
    }

    /// Find K nearest neighbors
    pub fn knn(&self, query: &Embedding, k: usize) -> Vec<(EmbeddingId, f32)> {
        let mut distances: Vec<(EmbeddingId, f32)> = self
            .embeddings
            .iter()
            .map(|(id, emb)| (*id, query.cosine_similarity(emb)))
            .collect();

        // Sort by similarity (descending)
        distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        distances.truncate(k);
        distances
    }

    /// Find all embeddings within radius
    pub fn radius_search(&self, query: &Embedding, radius: f32) -> Vec<(EmbeddingId, f32)> {
        self.embeddings
            .iter()
            .filter_map(|(id, emb)| {
                let sim = query.cosine_similarity(emb);
                if sim >= radius {
                    Some((*id, sim))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Compute centroid of all embeddings
    pub fn centroid(&self) -> Option<Embedding> {
        if self.embeddings.is_empty() {
            return None;
        }

        let n = self.embeddings.len() as f32;
        let mut sum = vec![0.0f32; self.dimension];

        for emb in self.embeddings.values() {
            for (i, v) in emb.vector.iter().enumerate() {
                sum[i] += v;
            }
        }

        for v in &mut sum {
            *v /= n;
        }

        Some(Embedding::new(EmbeddingId::new(0), sum))
    }

    /// Get all embeddings
    pub fn all(&self) -> impl Iterator<Item = &Embedding> {
        self.embeddings.values()
    }
}

// ============================================================================
// Embedding Encoder
// ============================================================================

/// Trait for encoding objects to embeddings
pub trait EmbeddingEncoder<T>: Send + Sync {
    fn encode(&self, input: &T) -> Embedding;
    fn dimension(&self) -> usize;
}

/// Hash-based encoder for strings
pub struct HashEncoder {
    dimension: usize,
    seed: u64,
}

impl HashEncoder {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            seed: 0x517cc1b727220a95,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    fn hash(&self, input: &[u8]) -> u64 {
        let mut h = self.seed;
        for &byte in input {
            h = h.wrapping_mul(0x5851f42d4c957f2d);
            h = h.wrapping_add(byte as u64);
            h ^= h >> 33;
        }
        h
    }
}

impl EmbeddingEncoder<str> for HashEncoder {
    fn encode(&self, input: &str) -> Embedding {
        let mut vector = vec![0.0f32; self.dimension];

        // Generate pseudo-random vector from hash
        let bytes = input.as_bytes();

        for i in 0..self.dimension {
            // Create position-dependent hash
            let mut h = self.hash(bytes);
            h = h.wrapping_add(i as u64);
            h = h.wrapping_mul(0x5851f42d4c957f2d);
            h ^= h >> 33;

            // Convert to float in [-1, 1]
            vector[i] = ((h as i64 as f64) / (i64::MAX as f64)) as f32;
        }

        let mut emb = Embedding::new(EmbeddingId::new(0), vector);
        emb.normalize();
        emb
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// One-hot encoder for categorical values
pub struct OneHotEncoder {
    categories: BTreeMap<String, usize>,
    dimension: usize,
}

impl OneHotEncoder {
    pub fn new() -> Self {
        Self {
            categories: BTreeMap::new(),
            dimension: 0,
        }
    }

    pub fn add_category(&mut self, category: impl Into<String>) -> usize {
        let cat = category.into();
        if let Some(&idx) = self.categories.get(&cat) {
            idx
        } else {
            let idx = self.dimension;
            self.categories.insert(cat, idx);
            self.dimension += 1;
            idx
        }
    }

    pub fn encode_category(&self, category: &str) -> Option<Embedding> {
        self.categories.get(category).map(|&idx| {
            let mut vector = vec![0.0f32; self.dimension];
            vector[idx] = 1.0;
            Embedding::new(EmbeddingId::new(0), vector)
        })
    }
}

impl Default for OneHotEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingEncoder<str> for OneHotEncoder {
    fn encode(&self, input: &str) -> Embedding {
        self.encode_category(input)
            .unwrap_or_else(|| Embedding::new(EmbeddingId::new(0), vec![0.0; self.dimension]))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

// ============================================================================
// Embedding Decoder
// ============================================================================

/// Trait for decoding embeddings back to objects
pub trait EmbeddingDecoder<T>: Send + Sync {
    fn decode(&self, embedding: &Embedding) -> T;
}

/// Decoder that finds nearest embedding in space
pub struct NearestDecoder {
    space: EmbeddingSpace,
}

impl NearestDecoder {
    pub fn new(space: EmbeddingSpace) -> Self {
        Self { space }
    }

    pub fn decode_to_name(&self, embedding: &Embedding) -> Option<String> {
        let neighbors = self.space.knn(embedding, 1);
        neighbors
            .first()
            .and_then(|(id, _)| self.space.get(*id))
            .and_then(|e| e.name.clone())
    }
}

// ============================================================================
// Embedding Transformation
// ============================================================================

/// Linear transformation matrix
pub struct LinearTransform {
    matrix: Vec<Vec<f32>>,
    input_dim: usize,
    output_dim: usize,
}

impl LinearTransform {
    pub fn new(input_dim: usize, output_dim: usize) -> Self {
        let matrix = vec![vec![0.0f32; input_dim]; output_dim];
        Self {
            matrix,
            input_dim,
            output_dim,
        }
    }

    pub fn identity(dim: usize) -> Self {
        let mut transform = Self::new(dim, dim);
        for i in 0..dim {
            transform.matrix[i][i] = 1.0;
        }
        transform
    }

    pub fn random(input_dim: usize, output_dim: usize, seed: u64) -> Self {
        let mut transform = Self::new(input_dim, output_dim);
        let mut rng = seed;

        for i in 0..output_dim {
            for j in 0..input_dim {
                rng = rng
                    .wrapping_mul(0x5851f42d4c957f2d)
                    .wrapping_add(0x14057b7ef767814f);
                rng ^= rng >> 33;
                transform.matrix[i][j] = ((rng as i64) as f32) / (i64::MAX as f32);
            }
        }

        // Normalize rows
        for row in &mut transform.matrix {
            let norm: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 1e-10 {
                for x in row.iter_mut() {
                    *x /= norm;
                }
            }
        }

        transform
    }

    pub fn apply(&self, embedding: &Embedding) -> Embedding {
        let mut output = vec![0.0f32; self.output_dim];

        for i in 0..self.output_dim {
            for j in 0..self.input_dim.min(embedding.dim()) {
                output[i] += self.matrix[i][j] * embedding.vector[j];
            }
        }

        Embedding::new(EmbeddingId::new(0), output)
    }

    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    pub fn output_dim(&self) -> usize {
        self.output_dim
    }
}

// ============================================================================
// Embedding Aggregation
// ============================================================================

/// Strategy for aggregating multiple embeddings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// Average all embeddings
    Mean,
    /// Sum all embeddings
    Sum,
    /// Max pooling (element-wise max)
    MaxPool,
    /// Min pooling (element-wise min)
    MinPool,
    /// Weighted average
    WeightedMean,
}

/// Aggregates multiple embeddings into one
pub struct EmbeddingAggregator {
    strategy: AggregationStrategy,
}

impl EmbeddingAggregator {
    pub fn new(strategy: AggregationStrategy) -> Self {
        Self { strategy }
    }

    pub fn aggregate(&self, embeddings: &[Embedding]) -> Option<Embedding> {
        if embeddings.is_empty() {
            return None;
        }

        let dim = embeddings[0].dim();

        match self.strategy {
            AggregationStrategy::Mean => {
                let n = embeddings.len() as f32;
                let mut sum = vec![0.0f32; dim];

                for emb in embeddings {
                    for (i, v) in emb.vector.iter().enumerate() {
                        sum[i] += v;
                    }
                }

                for v in &mut sum {
                    *v /= n;
                }

                Some(Embedding::new(EmbeddingId::new(0), sum))
            },
            AggregationStrategy::Sum => {
                let mut sum = vec![0.0f32; dim];

                for emb in embeddings {
                    for (i, v) in emb.vector.iter().enumerate() {
                        sum[i] += v;
                    }
                }

                Some(Embedding::new(EmbeddingId::new(0), sum))
            },
            AggregationStrategy::MaxPool => {
                let mut max = vec![f32::NEG_INFINITY; dim];

                for emb in embeddings {
                    for (i, v) in emb.vector.iter().enumerate() {
                        if *v > max[i] {
                            max[i] = *v;
                        }
                    }
                }

                Some(Embedding::new(EmbeddingId::new(0), max))
            },
            AggregationStrategy::MinPool => {
                let mut min = vec![f32::INFINITY; dim];

                for emb in embeddings {
                    for (i, v) in emb.vector.iter().enumerate() {
                        if *v < min[i] {
                            min[i] = *v;
                        }
                    }
                }

                Some(Embedding::new(EmbeddingId::new(0), min))
            },
            AggregationStrategy::WeightedMean => {
                // For weighted mean, use embedding norm as weight
                let weights: Vec<f32> = embeddings.iter().map(|e| e.norm()).collect();
                let total_weight: f32 = weights.iter().sum();

                if total_weight < 1e-10 {
                    return self.aggregate_with_strategy(embeddings, AggregationStrategy::Mean);
                }

                let mut result = vec![0.0f32; dim];

                for (emb, w) in embeddings.iter().zip(weights.iter()) {
                    for (i, v) in emb.vector.iter().enumerate() {
                        result[i] += v * w;
                    }
                }

                for v in &mut result {
                    *v /= total_weight;
                }

                Some(Embedding::new(EmbeddingId::new(0), result))
            },
        }
    }

    fn aggregate_with_strategy(
        &self,
        embeddings: &[Embedding],
        strategy: AggregationStrategy,
    ) -> Option<Embedding> {
        let agg = EmbeddingAggregator::new(strategy);
        agg.aggregate(embeddings)
    }

    pub fn aggregate_weighted(&self, embeddings: &[(Embedding, f32)]) -> Option<Embedding> {
        if embeddings.is_empty() {
            return None;
        }

        let dim = embeddings[0].0.dim();
        let total_weight: f32 = embeddings.iter().map(|(_, w)| w).sum();

        if total_weight < 1e-10 {
            return None;
        }

        let mut result = vec![0.0f32; dim];

        for (emb, w) in embeddings {
            for (i, v) in emb.vector.iter().enumerate() {
                result[i] += v * w;
            }
        }

        for v in &mut result {
            *v /= total_weight;
        }

        Some(Embedding::new(EmbeddingId::new(0), result))
    }
}

// ============================================================================
// Kernel Embeddings
// ============================================================================

/// Predefined embedding dimensions for kernel use
pub const KERNEL_EMBEDDING_DIM: usize = 64;

/// Create an embedding for kernel state
pub fn encode_kernel_state(
    cpu_load: f32,
    memory_pressure: f32,
    io_wait: f32,
    process_count: u32,
) -> Embedding {
    let mut vector = vec![0.0f32; KERNEL_EMBEDDING_DIM];

    // Encode basic metrics
    vector[0] = cpu_load;
    vector[1] = memory_pressure;
    vector[2] = io_wait;
    vector[3] = (process_count as f32) / 1000.0; // Normalize

    // Derived features
    vector[4] = cpu_load * memory_pressure; // Load interaction
    vector[5] = (cpu_load + memory_pressure) / 2.0; // Average load
    vector[6] = if cpu_load > 0.8 { 1.0 } else { 0.0 }; // High CPU flag
    vector[7] = if memory_pressure > 0.8 { 1.0 } else { 0.0 }; // High memory flag

    // Polynomial features
    vector[8] = cpu_load * cpu_load;
    vector[9] = memory_pressure * memory_pressure;
    vector[10] = io_wait * io_wait;

    // Cross features
    vector[11] = cpu_load * io_wait;
    vector[12] = memory_pressure * io_wait;

    // Trigonometric features (for periodic patterns)
    vector[13] = libm::sinf(cpu_load * core::f32::consts::PI);
    vector[14] = libm::cosf(cpu_load * core::f32::consts::PI);
    vector[15] = libm::sinf(memory_pressure * core::f32::consts::PI);

    // Fill remaining with zeros (reserved for future features)

    Embedding::new(EmbeddingId::new(0), vector).with_name("kernel_state")
}

/// Create embeddings for process characteristics
pub fn encode_process(
    pid: u32,
    cpu_usage: f32,
    memory_usage: f32,
    io_rate: f32,
    priority: i32,
) -> Embedding {
    let mut vector = vec![0.0f32; KERNEL_EMBEDDING_DIM];

    // Hash PID for consistent random-looking features
    let pid_hash = (pid as u64).wrapping_mul(0x517cc1b727220a95);

    vector[0] = cpu_usage;
    vector[1] = memory_usage;
    vector[2] = io_rate;
    vector[3] = (priority as f32 + 20.0) / 40.0; // Normalize priority

    // Process behavior indicators
    vector[4] = if cpu_usage > 0.5 { 1.0 } else { 0.0 }; // CPU bound
    vector[5] = if io_rate > 0.5 { 1.0 } else { 0.0 }; // IO bound
    vector[6] = if memory_usage > 0.3 { 1.0 } else { 0.0 }; // Memory heavy

    // PID-derived features (for consistent process identification)
    for i in 0..8 {
        let h = pid_hash
            .wrapping_add(i as u64)
            .wrapping_mul(0x5851f42d4c957f2d);
        vector[16 + i] = ((h as i64) as f32) / (i64::MAX as f32) * 0.1;
    }

    Embedding::new(EmbeddingId::new(pid as u64), vector)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_creation() {
        let emb = Embedding::new(EmbeddingId::new(1), vec![1.0, 0.0, 0.0]);
        assert_eq!(emb.dim(), 3);
        assert!((emb.norm() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_dot_product() {
        let a = Embedding::new(EmbeddingId::new(1), vec![1.0, 0.0]);
        let b = Embedding::new(EmbeddingId::new(2), vec![0.0, 1.0]);
        assert!((a.dot(&b) - 0.0).abs() < 1e-6);

        let c = Embedding::new(EmbeddingId::new(3), vec![1.0, 0.0]);
        assert!((a.dot(&c) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_cosine_similarity() {
        let a = Embedding::new(EmbeddingId::new(1), vec![1.0, 0.0]);
        let b = Embedding::new(EmbeddingId::new(2), vec![1.0, 0.0]);
        assert!((a.cosine_similarity(&b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_space() {
        let mut space = EmbeddingSpace::new("test", 3);
        let id =
            space.add(Embedding::new(EmbeddingId::new(0), vec![1.0, 0.0, 0.0]).with_name("test"));

        assert!(space.get(id).is_some());
        assert!(space.get_by_name("test").is_some());
    }

    #[test]
    fn test_hash_encoder() {
        let encoder = HashEncoder::new(32);
        let emb = encoder.encode("hello");
        assert_eq!(emb.dim(), 32);
        assert!((emb.norm() - 1.0).abs() < 1e-5);
    }
}
