//! # Active Learning
//!
//! Active learning strategies for efficient knowledge acquisition.
//! Selects informative samples for learning.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::math::F64Ext;

use crate::types::Timestamp;

// ============================================================================
// ACTIVE LEARNING TYPES
// ============================================================================

/// Sample
#[derive(Debug, Clone)]
pub struct Sample {
    /// Sample ID
    pub id: u64,
    /// Features
    pub features: Vec<f64>,
    /// Label (if known)
    pub label: Option<Label>,
    /// Uncertainty score
    pub uncertainty: f64,
    /// Information value
    pub information_value: f64,
    /// Selected for labeling
    pub selected: bool,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Label
#[derive(Debug, Clone)]
pub enum Label {
    /// Classification label
    Class(u32),
    /// Regression value
    Value(f64),
    /// Multi-label
    MultiLabel(Vec<u32>),
    /// Structured output
    Structured(String),
}

/// Query strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryStrategy {
    /// Most uncertain samples
    Uncertainty,
    /// Samples near decision boundary
    Margin,
    /// Samples with highest entropy
    Entropy,
    /// Expected model change
    ExpectedModelChange,
    /// Query by committee
    QueryByCommittee,
    /// Diversity-based
    Diversity,
    /// Random sampling
    Random,
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Selected samples
    pub selected: Vec<u64>,
    /// Strategy used
    pub strategy: QueryStrategy,
    /// Timestamp
    pub timestamp: Timestamp,
}

// ============================================================================
// ACTIVE LEARNER
// ============================================================================

/// Active learner
pub struct ActiveLearner {
    /// Sample pool
    samples: BTreeMap<u64, Sample>,
    /// Labeled samples
    labeled: BTreeMap<u64, Sample>,
    /// Query history
    history: Vec<QueryResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: LearnerConfig,
    /// Statistics
    stats: LearnerStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct LearnerConfig {
    /// Default strategy
    pub default_strategy: QueryStrategy,
    /// Batch size
    pub batch_size: usize,
    /// Diversity weight
    pub diversity_weight: f64,
    /// Minimum uncertainty for selection
    pub min_uncertainty: f64,
}

impl Default for LearnerConfig {
    fn default() -> Self {
        Self {
            default_strategy: QueryStrategy::Uncertainty,
            batch_size: 10,
            diversity_weight: 0.3,
            min_uncertainty: 0.1,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct LearnerStats {
    /// Samples in pool
    pub pool_size: u64,
    /// Labeled samples
    pub labeled_count: u64,
    /// Queries made
    pub queries_made: u64,
    /// Labels received
    pub labels_received: u64,
}

impl ActiveLearner {
    /// Create new active learner
    pub fn new(config: LearnerConfig) -> Self {
        Self {
            samples: BTreeMap::new(),
            labeled: BTreeMap::new(),
            history: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: LearnerStats::default(),
        }
    }

    /// Add sample to pool
    pub fn add_sample(&mut self, features: Vec<f64>, metadata: BTreeMap<String, String>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let sample = Sample {
            id,
            features,
            label: None,
            uncertainty: 1.0, // Maximum uncertainty for new samples
            information_value: 0.0,
            selected: false,
            metadata,
        };

        self.samples.insert(id, sample);
        self.stats.pool_size += 1;

        id
    }

    /// Add labeled sample
    pub fn add_labeled(&mut self, features: Vec<f64>, label: Label) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let sample = Sample {
            id,
            features,
            label: Some(label),
            uncertainty: 0.0,
            information_value: 0.0,
            selected: false,
            metadata: BTreeMap::new(),
        };

        self.labeled.insert(id, sample);
        self.stats.labeled_count += 1;

        id
    }

    /// Update uncertainty scores
    pub fn update_uncertainty(&mut self, predictions: &BTreeMap<u64, Vec<f64>>) {
        for (id, probs) in predictions {
            if let Some(sample) = self.samples.get_mut(id) {
                sample.uncertainty = self.compute_uncertainty(probs);
            }
        }
    }

    fn compute_uncertainty(&self, probs: &[f64]) -> f64 {
        if probs.is_empty() {
            return 1.0;
        }

        // Entropy-based uncertainty
        let entropy: f64 = probs.iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.ln())
            .sum();

        // Normalize by max entropy
        let max_entropy = (probs.len() as f64).ln();
        if max_entropy > 0.0 {
            entropy / max_entropy
        } else {
            0.0
        }
    }

    /// Query samples for labeling
    pub fn query(&mut self, count: Option<usize>) -> QueryResult {
        self.query_with_strategy(self.config.default_strategy, count)
    }

    /// Query with specific strategy
    pub fn query_with_strategy(&mut self, strategy: QueryStrategy, count: Option<usize>) -> QueryResult {
        let batch_size = count.unwrap_or(self.config.batch_size);

        let selected = match strategy {
            QueryStrategy::Uncertainty => self.query_uncertainty(batch_size),
            QueryStrategy::Margin => self.query_margin(batch_size),
            QueryStrategy::Entropy => self.query_entropy(batch_size),
            QueryStrategy::Diversity => self.query_diversity(batch_size),
            QueryStrategy::Random => self.query_random(batch_size),
            _ => self.query_uncertainty(batch_size),
        };

        // Mark as selected
        for &id in &selected {
            if let Some(sample) = self.samples.get_mut(&id) {
                sample.selected = true;
            }
        }

        let result = QueryResult {
            selected: selected.clone(),
            strategy,
            timestamp: Timestamp::now(),
        };

        self.history.push(result.clone());
        self.stats.queries_made += 1;

        result
    }

    fn query_uncertainty(&self, count: usize) -> Vec<u64> {
        let mut candidates: Vec<_> = self.samples.iter()
            .filter(|(_, s)| !s.selected && s.label.is_none())
            .filter(|(_, s)| s.uncertainty >= self.config.min_uncertainty)
            .map(|(&id, s)| (id, s.uncertainty))
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        candidates.into_iter()
            .take(count)
            .map(|(id, _)| id)
            .collect()
    }

    fn query_margin(&self, count: usize) -> Vec<u64> {
        // Margin sampling: select samples with smallest difference between
        // top two class probabilities (similar to uncertainty for this impl)
        self.query_uncertainty(count)
    }

    fn query_entropy(&self, count: usize) -> Vec<u64> {
        // Same as uncertainty for this implementation
        self.query_uncertainty(count)
    }

    fn query_diversity(&self, count: usize) -> Vec<u64> {
        let mut candidates: Vec<_> = self.samples.iter()
            .filter(|(_, s)| !s.selected && s.label.is_none())
            .map(|(&id, s)| (id, s.features.clone(), s.uncertainty))
            .collect();

        if candidates.is_empty() {
            return Vec::new();
        }

        let mut selected = Vec::new();
        let mut selected_features: Vec<Vec<f64>> = Vec::new();

        while selected.len() < count && !candidates.is_empty() {
            // Score each candidate by uncertainty + diversity
            let mut best_idx = 0;
            let mut best_score = f64::NEG_INFINITY;

            for (idx, (_, features, uncertainty)) in candidates.iter().enumerate() {
                let diversity = if selected_features.is_empty() {
                    1.0
                } else {
                    // Min distance to any selected sample
                    selected_features.iter()
                        .map(|sf| self.euclidean_distance(features, sf))
                        .fold(f64::INFINITY, f64::min)
                };

                let score = (1.0 - self.config.diversity_weight) * uncertainty +
                            self.config.diversity_weight * diversity;

                if score > best_score {
                    best_score = score;
                    best_idx = idx;
                }
            }

            let (id, features, _) = candidates.remove(best_idx);
            selected.push(id);
            selected_features.push(features);
        }

        selected
    }

    fn query_random(&self, count: usize) -> Vec<u64> {
        // Simple deterministic "random" for no_std
        let candidates: Vec<u64> = self.samples.iter()
            .filter(|(_, s)| !s.selected && s.label.is_none())
            .map(|(&id, _)| id)
            .collect();

        candidates.into_iter()
            .take(count)
            .collect()
    }

    fn euclidean_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return f64::INFINITY;
        }

        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    /// Provide label for sample
    pub fn label(&mut self, sample_id: u64, label: Label) {
        if let Some(sample) = self.samples.remove(&sample_id) {
            let mut labeled_sample = sample;
            labeled_sample.label = Some(label);
            labeled_sample.uncertainty = 0.0;

            self.labeled.insert(sample_id, labeled_sample);
            self.stats.pool_size -= 1;
            self.stats.labeled_count += 1;
            self.stats.labels_received += 1;
        }
    }

    /// Get sample
    pub fn get_sample(&self, id: u64) -> Option<&Sample> {
        self.samples.get(&id).or_else(|| self.labeled.get(&id))
    }

    /// Get unlabeled samples
    pub fn unlabeled(&self) -> Vec<&Sample> {
        self.samples.values()
            .filter(|s| s.label.is_none())
            .collect()
    }

    /// Get labeled samples
    pub fn get_labeled(&self) -> Vec<&Sample> {
        self.labeled.values().collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &LearnerStats {
        &self.stats
    }
}

impl Default for ActiveLearner {
    fn default() -> Self {
        Self::new(LearnerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_sample() {
        let mut learner = ActiveLearner::default();

        let id = learner.add_sample(vec![1.0, 2.0, 3.0], BTreeMap::new());
        assert!(learner.get_sample(id).is_some());
    }

    #[test]
    fn test_query_uncertainty() {
        let mut learner = ActiveLearner::default();

        for i in 0..10 {
            learner.add_sample(vec![i as f64], BTreeMap::new());
        }

        let result = learner.query(Some(3));
        assert_eq!(result.selected.len(), 3);
    }

    #[test]
    fn test_update_uncertainty() {
        let mut learner = ActiveLearner::default();

        let id = learner.add_sample(vec![1.0], BTreeMap::new());

        let mut predictions = BTreeMap::new();
        predictions.insert(id, vec![0.9, 0.1]); // Low entropy = low uncertainty

        learner.update_uncertainty(&predictions);

        let sample = learner.get_sample(id).unwrap();
        assert!(sample.uncertainty < 1.0);
    }

    #[test]
    fn test_label() {
        let mut learner = ActiveLearner::default();

        let id = learner.add_sample(vec![1.0], BTreeMap::new());
        learner.label(id, Label::Class(1));

        let sample = learner.get_sample(id).unwrap();
        assert!(sample.label.is_some());
    }

    #[test]
    fn test_diversity_query() {
        let mut learner = ActiveLearner::new(LearnerConfig {
            diversity_weight: 0.5,
            ..Default::default()
        });

        // Add samples at different locations
        learner.add_sample(vec![0.0, 0.0], BTreeMap::new());
        learner.add_sample(vec![10.0, 10.0], BTreeMap::new());
        learner.add_sample(vec![0.0, 0.0], BTreeMap::new()); // Duplicate location

        let result = learner.query_with_strategy(QueryStrategy::Diversity, Some(2));
        assert_eq!(result.selected.len(), 2);
    }
}
