//! ML Core Types — Feature and Sample Definitions
//!
//! Fundamental types for machine learning operations.

use alloc::vec::Vec;

use crate::math;

// ============================================================================
// FEATURE
// ============================================================================

/// A feature for ML models
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feature {
    /// Feature index
    pub index: usize,
    /// Feature value
    pub value: f64,
}

impl Feature {
    /// Create a new feature
    pub const fn new(index: usize, value: f64) -> Self {
        Self { index, value }
    }
}

/// A feature vector
#[derive(Debug, Clone, Default)]
pub struct FeatureVector {
    /// Features
    features: Vec<Feature>,
    /// Dense representation (if used)
    dense: Option<Vec<f64>>,
}

impl FeatureVector {
    /// Create empty feature vector
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from dense vector
    pub fn from_dense(values: Vec<f64>) -> Self {
        let features = values
            .iter()
            .enumerate()
            .filter(|&(_, v)| *v != 0.0)
            .map(|(i, &v)| Feature::new(i, v))
            .collect();

        Self {
            features,
            dense: Some(values),
        }
    }

    /// Create from sparse features
    pub fn from_sparse(features: Vec<Feature>) -> Self {
        Self {
            features,
            dense: None,
        }
    }

    /// Add a feature
    pub fn add(&mut self, index: usize, value: f64) {
        self.features.push(Feature::new(index, value));
        self.dense = None; // Invalidate dense cache
    }

    /// Get feature value
    pub fn get(&self, index: usize) -> f64 {
        if let Some(ref dense) = self.dense {
            dense.get(index).copied().unwrap_or(0.0)
        } else {
            self.features
                .iter()
                .find(|f| f.index == index)
                .map(|f| f.value)
                .unwrap_or(0.0)
        }
    }

    /// Dot product with weights
    pub fn dot(&self, weights: &[f64]) -> f64 {
        self.features
            .iter()
            .filter_map(|f| weights.get(f.index).map(|w| w * f.value))
            .sum()
    }

    /// L2 norm
    pub fn norm(&self) -> f64 {
        math::sqrt(self.features.iter().map(|f| f.value * f.value).sum::<f64>())
    }

    /// Normalize the vector
    pub fn normalize(&mut self) {
        let norm = self.norm();
        if norm > 0.0 {
            for f in &mut self.features {
                f.value /= norm;
            }
        }
    }

    /// Number of features
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Iterate over features
    pub fn iter(&self) -> impl Iterator<Item = &Feature> {
        self.features.iter()
    }
}

// ============================================================================
// LABELED SAMPLE
// ============================================================================

/// A labeled training sample
#[derive(Debug, Clone)]
pub struct LabeledSample {
    /// Features
    pub features: FeatureVector,
    /// Label (class index or regression target)
    pub label: f64,
    /// Weight (for weighted samples)
    pub weight: f64,
}

impl LabeledSample {
    /// Create a new sample
    pub fn new(features: FeatureVector, label: f64) -> Self {
        Self {
            features,
            label,
            weight: 1.0,
        }
    }

    /// Set weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}
