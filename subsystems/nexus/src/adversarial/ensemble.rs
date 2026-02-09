//! Ensemble defense mechanisms for adversarial robustness.

use alloc::vec;
use alloc::vec::Vec;

use crate::adversarial::types::ENSEMBLE_SIZE;

/// Diverse ensemble for robustness
#[derive(Debug, Clone)]
pub struct EnsembleDefense {
    /// Number of models
    pub num_models: usize,
    /// Diversity regularization strength
    pub diversity_weight: f64,
    /// Voting threshold
    pub voting_threshold: f64,
    /// Model predictions (stored for voting)
    predictions: Vec<Vec<f64>>,
}

impl EnsembleDefense {
    /// Create a new ensemble defense
    pub fn new(num_models: usize) -> Self {
        Self {
            num_models: num_models.min(ENSEMBLE_SIZE),
            diversity_weight: 0.1,
            voting_threshold: 0.5,
            predictions: Vec::new(),
        }
    }

    /// Aggregate predictions using voting
    pub fn vote(&mut self, model_predictions: &[Vec<f64>]) -> Vec<f64> {
        self.predictions = model_predictions.to_vec();

        if self.predictions.is_empty() {
            return Vec::new();
        }

        let dim = self.predictions[0].len();
        let num_models = self.predictions.len();

        // Average predictions
        let mut avg = vec![0.0; dim];

        for pred in &self.predictions {
            for (a, &p) in avg.iter_mut().zip(pred.iter()) {
                *a += p;
            }
        }

        for a in &mut avg {
            *a /= num_models as f64;
        }

        avg
    }

    /// Compute prediction variance (uncertainty)
    pub fn prediction_variance(&self) -> Vec<f64> {
        if self.predictions.len() < 2 {
            return vec![0.0; self.predictions.first().map(|p| p.len()).unwrap_or(0)];
        }

        let dim = self.predictions[0].len();
        let num = self.predictions.len() as f64;

        // Compute mean
        let mut mean = vec![0.0; dim];
        for pred in &self.predictions {
            for (m, &p) in mean.iter_mut().zip(pred.iter()) {
                *m += p;
            }
        }
        for m in &mut mean {
            *m /= num;
        }

        // Compute variance
        let mut variance = vec![0.0; dim];
        for pred in &self.predictions {
            for (v, (&p, &m)) in variance.iter_mut().zip(pred.iter().zip(mean.iter())) {
                *v += (p - m).powi(2);
            }
        }
        for v in &mut variance {
            *v /= num;
        }

        variance
    }

    /// Check for ensemble disagreement (potential attack indicator)
    #[inline]
    pub fn check_disagreement(&self, threshold: f64) -> bool {
        let variance = self.prediction_variance();
        let max_var = variance.iter().fold(0.0, |a, &b| f64::max(a, b));

        max_var > threshold
    }
}
