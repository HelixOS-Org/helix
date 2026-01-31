//! Online Learning with SGD
//!
//! Stochastic Gradient Descent classifier for incremental learning.

use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{FeatureVector, LabeledSample, sigmoid};

// ============================================================================
// STOCHASTIC GRADIENT DESCENT
// ============================================================================

/// Stochastic Gradient Descent classifier
pub struct SGDClassifier {
    /// Weights
    weights: Vec<f64>,
    /// Bias
    bias: f64,
    /// Learning rate
    learning_rate: f64,
    /// Regularization
    alpha: f64,
    /// Number of updates
    n_updates: AtomicU64,
}

impl SGDClassifier {
    /// Create new SGD classifier
    pub fn new(n_features: usize, learning_rate: f64) -> Self {
        Self {
            weights: vec![0.0; n_features],
            bias: 0.0,
            learning_rate,
            alpha: 0.0001,
            n_updates: AtomicU64::new(0),
        }
    }

    /// Set regularization
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    /// Partial fit (online update)
    pub fn partial_fit(&mut self, features: &FeatureVector, label: f64) {
        let prediction = self.decision_function(features);
        let error = label - prediction;

        // Update weights (SGD with L2 regularization)
        for f in features.iter() {
            if f.index < self.weights.len() {
                self.weights[f.index] +=
                    self.learning_rate * (error * f.value - self.alpha * self.weights[f.index]);
            }
        }

        // Update bias
        self.bias += self.learning_rate * error;

        self.n_updates.fetch_add(1, Ordering::Relaxed);
    }

    /// Batch fit
    pub fn fit(&mut self, samples: &[LabeledSample], epochs: usize) {
        for _ in 0..epochs {
            for sample in samples {
                self.partial_fit(&sample.features, sample.label);
            }
        }
    }

    /// Decision function (raw score)
    pub fn decision_function(&self, features: &FeatureVector) -> f64 {
        features.dot(&self.weights) + self.bias
    }

    /// Predict class (0 or 1)
    pub fn predict(&self, features: &FeatureVector) -> f64 {
        if self.decision_function(features) > 0.0 {
            1.0
        } else {
            0.0
        }
    }

    /// Predict probability
    pub fn predict_proba(&self, features: &FeatureVector) -> f64 {
        sigmoid(self.decision_function(features))
    }

    /// Get weights
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// Get number of updates
    pub fn n_updates(&self) -> u64 {
        self.n_updates.load(Ordering::Relaxed)
    }
}

impl Default for SGDClassifier {
    fn default() -> Self {
        Self::new(10, 0.01)
    }
}
