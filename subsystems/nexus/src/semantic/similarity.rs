//! NEXUS Year 2: Similarity Metrics
//!
//! Various similarity and distance metrics for comparing embeddings,
//! concepts, and entities.

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::embeddings::{Embedding, EmbeddingId};

// ============================================================================
// Similarity Metric Trait
// ============================================================================

/// Trait for similarity/distance metrics
pub trait SimilarityMetric: Send + Sync {
    /// Compute similarity between two embeddings
    /// Returns value typically in [0, 1] where 1 is most similar
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32;

    /// Compute distance between two embeddings
    /// Default implementation: 1 - similarity
    fn distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        1.0 - self.similarity(a, b)
    }

    /// Name of this metric
    fn name(&self) -> &str;
}

// ============================================================================
// Standard Metrics
// ============================================================================

/// Cosine similarity metric
pub struct CosineSimilarity;

impl SimilarityMetric for CosineSimilarity {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        a.cosine_similarity(b)
    }

    fn distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        1.0 - a.cosine_similarity(b)
    }

    fn name(&self) -> &str {
        "cosine"
    }
}

/// Euclidean distance metric
pub struct EuclideanDistance {
    normalize: bool,
}

impl EuclideanDistance {
    pub fn new() -> Self {
        Self { normalize: true }
    }

    pub fn raw() -> Self {
        Self { normalize: false }
    }
}

impl Default for EuclideanDistance {
    fn default() -> Self {
        Self::new()
    }
}

impl SimilarityMetric for EuclideanDistance {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        let dist = a.euclidean_distance(b);
        if self.normalize {
            // Normalize to [0, 1] using sigmoid-like transform
            1.0 / (1.0 + dist)
        } else {
            // Inverse distance
            if dist > 1e-10 { 1.0 / dist } else { f32::MAX }
        }
    }

    fn distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        a.euclidean_distance(b)
    }

    fn name(&self) -> &str {
        "euclidean"
    }
}

/// Manhattan (L1) distance metric
pub struct ManhattanDistance;

impl ManhattanDistance {
    fn l1_distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        a.vector
            .iter()
            .zip(b.vector.iter())
            .map(|(x, y)| (x - y).abs())
            .sum()
    }
}

impl SimilarityMetric for ManhattanDistance {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        let dist = self.l1_distance(a, b);
        1.0 / (1.0 + dist)
    }

    fn distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        self.l1_distance(a, b)
    }

    fn name(&self) -> &str {
        "manhattan"
    }
}

/// Chebyshev (L∞) distance metric
pub struct ChebyshevDistance;

impl ChebyshevDistance {
    fn linf_distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        a.vector
            .iter()
            .zip(b.vector.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0f32, |max, d| if d > max { d } else { max })
    }
}

impl SimilarityMetric for ChebyshevDistance {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        let dist = self.linf_distance(a, b);
        1.0 / (1.0 + dist)
    }

    fn distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        self.linf_distance(a, b)
    }

    fn name(&self) -> &str {
        "chebyshev"
    }
}

/// Minkowski distance (generalized Lp norm)
pub struct MinkowskiDistance {
    p: f32,
}

impl MinkowskiDistance {
    pub fn new(p: f32) -> Self {
        Self { p: p.max(1.0) }
    }

    fn lp_distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        let sum: f32 = a
            .vector
            .iter()
            .zip(b.vector.iter())
            .map(|(x, y)| libm::powf((x - y).abs(), self.p))
            .sum();
        libm::powf(sum, 1.0 / self.p)
    }
}

impl SimilarityMetric for MinkowskiDistance {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        let dist = self.lp_distance(a, b);
        1.0 / (1.0 + dist)
    }

    fn distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        self.lp_distance(a, b)
    }

    fn name(&self) -> &str {
        "minkowski"
    }
}

/// Dot product similarity (inner product)
pub struct DotProductSimilarity;

impl SimilarityMetric for DotProductSimilarity {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        a.dot(b)
    }

    fn name(&self) -> &str {
        "dot_product"
    }
}

/// Jaccard similarity for binary vectors
pub struct JaccardSimilarity {
    threshold: f32,
}

impl JaccardSimilarity {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    fn binarize(&self, v: f32) -> bool {
        v > self.threshold
    }
}

impl Default for JaccardSimilarity {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl SimilarityMetric for JaccardSimilarity {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        let mut intersection = 0;
        let mut union = 0;

        for (x, y) in a.vector.iter().zip(b.vector.iter()) {
            let bx = self.binarize(*x);
            let by = self.binarize(*y);

            if bx || by {
                union += 1;
            }
            if bx && by {
                intersection += 1;
            }
        }

        if union == 0 {
            1.0
        } else {
            intersection as f32 / union as f32
        }
    }

    fn name(&self) -> &str {
        "jaccard"
    }
}

/// Angular distance
pub struct AngularDistance;

impl SimilarityMetric for AngularDistance {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        let cos = a.cosine_similarity(b).clamp(-1.0, 1.0);
        let angle = libm::acosf(cos);
        1.0 - angle / core::f32::consts::PI
    }

    fn distance(&self, a: &Embedding, b: &Embedding) -> f32 {
        let cos = a.cosine_similarity(b).clamp(-1.0, 1.0);
        libm::acosf(cos) / core::f32::consts::PI
    }

    fn name(&self) -> &str {
        "angular"
    }
}

// ============================================================================
// Similarity Matrix
// ============================================================================

/// Precomputed similarity matrix for a set of embeddings
pub struct SimilarityMatrix {
    ids: Vec<EmbeddingId>,
    matrix: Vec<f32>, // Stored as flat array (row-major)
    size: usize,
    metric_name: String,
}

impl SimilarityMatrix {
    /// Compute similarity matrix for a set of embeddings
    pub fn compute<M: SimilarityMetric>(embeddings: &[Embedding], metric: &M) -> Self {
        let n = embeddings.len();
        let mut matrix = vec![0.0f32; n * n];
        let ids: Vec<EmbeddingId> = embeddings.iter().map(|e| e.id).collect();

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    matrix[i * n + j] = 1.0;
                } else if j > i {
                    let sim = metric.similarity(&embeddings[i], &embeddings[j]);
                    matrix[i * n + j] = sim;
                    matrix[j * n + i] = sim;
                }
            }
        }

        Self {
            ids,
            matrix,
            size: n,
            metric_name: metric.name().to_string(),
        }
    }

    /// Get similarity between two embeddings by index
    pub fn get(&self, i: usize, j: usize) -> Option<f32> {
        if i < self.size && j < self.size {
            Some(self.matrix[i * self.size + j])
        } else {
            None
        }
    }

    /// Get similarity between two embeddings by ID
    pub fn get_by_id(&self, id_a: EmbeddingId, id_b: EmbeddingId) -> Option<f32> {
        let i = self.ids.iter().position(|&id| id == id_a)?;
        let j = self.ids.iter().position(|&id| id == id_b)?;
        self.get(i, j)
    }

    /// Find most similar pairs
    pub fn most_similar_pairs(&self, top_k: usize) -> Vec<(EmbeddingId, EmbeddingId, f32)> {
        let mut pairs = Vec::new();

        for i in 0..self.size {
            for j in (i + 1)..self.size {
                pairs.push((self.ids[i], self.ids[j], self.matrix[i * self.size + j]));
            }
        }

        pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
        pairs.truncate(top_k);
        pairs
    }

    /// Get row (all similarities for one embedding)
    pub fn row(&self, i: usize) -> Option<&[f32]> {
        if i < self.size {
            Some(&self.matrix[i * self.size..(i + 1) * self.size])
        } else {
            None
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn metric_name(&self) -> &str {
        &self.metric_name
    }
}

// ============================================================================
// Similarity Search
// ============================================================================

/// Efficient similarity search structure
pub struct SimilaritySearch {
    embeddings: Vec<Embedding>,
    id_to_index: BTreeMap<EmbeddingId, usize>,
}

impl SimilaritySearch {
    pub fn new() -> Self {
        Self {
            embeddings: Vec::new(),
            id_to_index: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, embedding: Embedding) {
        let idx = self.embeddings.len();
        self.id_to_index.insert(embedding.id, idx);
        self.embeddings.push(embedding);
    }

    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    /// Find K nearest neighbors using given metric
    pub fn knn<M: SimilarityMetric>(
        &self,
        query: &Embedding,
        k: usize,
        metric: &M,
    ) -> Vec<(EmbeddingId, f32)> {
        let mut results: Vec<(EmbeddingId, f32)> = self
            .embeddings
            .iter()
            .map(|e| (e.id, metric.similarity(query, e)))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        results.truncate(k);
        results
    }

    /// Find all embeddings within similarity threshold
    pub fn range_search<M: SimilarityMetric>(
        &self,
        query: &Embedding,
        min_similarity: f32,
        metric: &M,
    ) -> Vec<(EmbeddingId, f32)> {
        self.embeddings
            .iter()
            .filter_map(|e| {
                let sim = metric.similarity(query, e);
                if sim >= min_similarity {
                    Some((e.id, sim))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find K most diverse embeddings (farthest from each other)
    pub fn most_diverse<M: SimilarityMetric>(&self, k: usize, metric: &M) -> Vec<EmbeddingId> {
        if self.embeddings.is_empty() || k == 0 {
            return Vec::new();
        }

        if self.embeddings.len() <= k {
            return self.embeddings.iter().map(|e| e.id).collect();
        }

        // Greedy approach: start with random, add most dissimilar
        let mut selected = vec![self.embeddings[0].id];
        let mut remaining: Vec<usize> = (1..self.embeddings.len()).collect();

        while selected.len() < k && !remaining.is_empty() {
            let mut best_idx = 0;
            let mut best_min_dist = f32::NEG_INFINITY;

            for (i, &emb_idx) in remaining.iter().enumerate() {
                let candidate = &self.embeddings[emb_idx];

                // Find minimum similarity to any selected
                let min_dist = selected
                    .iter()
                    .filter_map(|&id| self.id_to_index.get(&id))
                    .map(|&idx| 1.0 - metric.similarity(&self.embeddings[idx], candidate))
                    .fold(f32::INFINITY, |a, b| if b < a { b } else { a });

                if min_dist > best_min_dist {
                    best_min_dist = min_dist;
                    best_idx = i;
                }
            }

            let selected_idx = remaining.remove(best_idx);
            selected.push(self.embeddings[selected_idx].id);
        }

        selected
    }

    /// Cluster embeddings using K-means style approach
    pub fn cluster<M: SimilarityMetric>(
        &self,
        k: usize,
        max_iterations: usize,
        metric: &M,
    ) -> Vec<Vec<EmbeddingId>> {
        if self.embeddings.is_empty() || k == 0 {
            return Vec::new();
        }

        let n = self.embeddings.len();
        let k = k.min(n);

        // Initialize centroids with diverse selection
        let initial_centroids = self.most_diverse(k, metric);
        let mut centroids: Vec<Embedding> = initial_centroids
            .iter()
            .filter_map(|id| self.id_to_index.get(id))
            .map(|&idx| self.embeddings[idx].clone())
            .collect();

        let mut assignments = vec![0usize; n];

        for _ in 0..max_iterations {
            // Assign points to nearest centroid
            let mut changed = false;
            for (i, emb) in self.embeddings.iter().enumerate() {
                let mut best_cluster = 0;
                let mut best_sim = f32::NEG_INFINITY;

                for (c, centroid) in centroids.iter().enumerate() {
                    let sim = metric.similarity(emb, centroid);
                    if sim > best_sim {
                        best_sim = sim;
                        best_cluster = c;
                    }
                }

                if assignments[i] != best_cluster {
                    assignments[i] = best_cluster;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            // Update centroids
            for c in 0..k {
                let cluster_points: Vec<&Embedding> = self
                    .embeddings
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| assignments[*i] == c)
                    .map(|(_, e)| e)
                    .collect();

                if !cluster_points.is_empty() {
                    let dim = cluster_points[0].dim();
                    let count = cluster_points.len() as f32;
                    let mut new_centroid = vec![0.0f32; dim];

                    for emb in &cluster_points {
                        for (i, v) in emb.vector.iter().enumerate() {
                            new_centroid[i] += v;
                        }
                    }

                    for v in &mut new_centroid {
                        *v /= count;
                    }

                    centroids[c] = Embedding::new(EmbeddingId::new(c as u64), new_centroid);
                }
            }
        }

        // Build cluster lists
        let mut clusters = vec![Vec::new(); k];
        for (i, &cluster) in assignments.iter().enumerate() {
            clusters[cluster].push(self.embeddings[i].id);
        }

        clusters
    }
}

impl Default for SimilaritySearch {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Weighted Similarity
// ============================================================================

/// Weighted combination of multiple similarity metrics
pub struct WeightedSimilarity {
    metrics: Vec<(Box<dyn SimilarityMetric>, f32)>,
}

impl WeightedSimilarity {
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }

    pub fn add_metric<M: SimilarityMetric + 'static>(&mut self, metric: M, weight: f32) {
        self.metrics.push((Box::new(metric), weight));
    }
}

impl Default for WeightedSimilarity {
    fn default() -> Self {
        Self::new()
    }
}

impl SimilarityMetric for WeightedSimilarity {
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        let total_weight: f32 = self.metrics.iter().map(|(_, w)| w).sum();
        if total_weight < 1e-10 {
            return 0.0;
        }

        let weighted_sum: f32 = self
            .metrics
            .iter()
            .map(|(m, w)| m.similarity(a, b) * w)
            .sum();

        weighted_sum / total_weight
    }

    fn name(&self) -> &str {
        "weighted"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding(values: &[f32]) -> Embedding {
        Embedding::new(EmbeddingId::new(0), values.to_vec())
    }

    #[test]
    fn test_cosine_similarity() {
        let metric = CosineSimilarity;
        let a = make_embedding(&[1.0, 0.0]);
        let b = make_embedding(&[1.0, 0.0]);
        assert!((metric.similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let metric = EuclideanDistance::new();
        let a = make_embedding(&[0.0, 0.0]);
        let b = make_embedding(&[3.0, 4.0]);
        assert!((metric.distance(&a, &b) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_manhattan_distance() {
        let metric = ManhattanDistance;
        let a = make_embedding(&[0.0, 0.0]);
        let b = make_embedding(&[3.0, 4.0]);
        assert!((metric.distance(&a, &b) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_similarity_search() {
        let mut search = SimilaritySearch::new();
        search.add(Embedding::new(EmbeddingId::new(1), vec![1.0, 0.0]));
        search.add(Embedding::new(EmbeddingId::new(2), vec![0.0, 1.0]));

        assert_eq!(search.len(), 2);
    }
}
