//! # Online Learning
//!
//! Continuous online learning from streaming data.
//! Implements incremental updates and adaptive learning.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// ONLINE LEARNING TYPES
// ============================================================================

/// Data sample
#[derive(Debug, Clone)]
pub struct Sample {
    /// Sample ID
    pub id: u64,
    /// Features
    pub features: Vec<f64>,
    /// Target (for supervised)
    pub target: Option<TargetValue>,
    /// Weight
    pub weight: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Target value
#[derive(Debug, Clone)]
pub enum TargetValue {
    Continuous(f64),
    Discrete(u64),
    MultiLabel(Vec<u64>),
}

/// Online model
#[derive(Debug, Clone)]
pub struct OnlineModel {
    /// Model ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Model type
    pub model_type: OnlineModelType,
    /// Parameters
    pub params: ModelParams,
    /// Metrics
    pub metrics: ModelMetrics,
    /// Created
    pub created: Timestamp,
    /// Last updated
    pub last_updated: Timestamp,
}

/// Online model type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineModelType {
    /// Perceptron
    Perceptron,
    /// Passive Aggressive
    PassiveAggressive,
    /// Stochastic Gradient Descent
    SGD,
    /// Running Mean
    RunningMean,
    /// Exponential Moving Average
    EMA,
    /// Online K-Means
    OnlineKMeans,
}

/// Model parameters
#[derive(Debug, Clone)]
pub struct ModelParams {
    /// Weights
    pub weights: Vec<f64>,
    /// Bias
    pub bias: f64,
    /// Learning rate
    pub learning_rate: f64,
    /// Regularization
    pub regularization: f64,
    /// Extra params
    pub extra: BTreeMap<String, f64>,
}

impl Default for ModelParams {
    fn default() -> Self {
        Self {
            weights: Vec::new(),
            bias: 0.0,
            learning_rate: 0.01,
            regularization: 0.0001,
            extra: BTreeMap::new(),
        }
    }
}

/// Model metrics
#[derive(Debug, Clone, Default)]
pub struct ModelMetrics {
    /// Samples seen
    pub samples_seen: u64,
    /// Cumulative loss
    pub cumulative_loss: f64,
    /// Running accuracy
    pub accuracy: f64,
    /// Prediction count
    pub predictions: u64,
}

/// Prediction result
#[derive(Debug, Clone)]
pub struct Prediction {
    /// Predicted value
    pub value: TargetValue,
    /// Confidence
    pub confidence: f64,
    /// Latency ns
    pub latency_ns: u64,
}

// ============================================================================
// ONLINE LEARNER
// ============================================================================

/// Online learner
pub struct OnlineLearner {
    /// Models
    models: BTreeMap<u64, OnlineModel>,
    /// Sample buffer
    buffer: Vec<Sample>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: OnlineConfig,
    /// Statistics
    stats: OnlineStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct OnlineConfig {
    /// Default learning rate
    pub default_learning_rate: f64,
    /// Buffer size
    pub buffer_size: usize,
    /// Learning rate decay
    pub lr_decay: f64,
    /// Minimum learning rate
    pub min_lr: f64,
}

impl Default for OnlineConfig {
    fn default() -> Self {
        Self {
            default_learning_rate: 0.01,
            buffer_size: 1000,
            lr_decay: 0.999,
            min_lr: 0.0001,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct OnlineStats {
    /// Models created
    pub models_created: u64,
    /// Updates performed
    pub updates_performed: u64,
    /// Predictions made
    pub predictions_made: u64,
}

impl OnlineLearner {
    /// Create new learner
    pub fn new(config: OnlineConfig) -> Self {
        Self {
            models: BTreeMap::new(),
            buffer: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: OnlineStats::default(),
        }
    }

    /// Create model
    pub fn create_model(&mut self, name: &str, model_type: OnlineModelType, input_dim: usize) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let params = ModelParams {
            weights: vec![0.0; input_dim],
            bias: 0.0,
            learning_rate: self.config.default_learning_rate,
            regularization: 0.0001,
            extra: BTreeMap::new(),
        };

        let model = OnlineModel {
            id,
            name: name.into(),
            model_type,
            params,
            metrics: ModelMetrics::default(),
            created: Timestamp::now(),
            last_updated: Timestamp::now(),
        };

        self.models.insert(id, model);
        self.stats.models_created += 1;

        id
    }

    /// Update model with sample
    pub fn update(&mut self, model_id: u64, sample: Sample) -> bool {
        let model = match self.models.get_mut(&model_id) {
            Some(m) => m,
            None => return false,
        };

        // Ensure weights match input dimension
        while model.params.weights.len() < sample.features.len() {
            model.params.weights.push(0.0);
        }

        let target = match &sample.target {
            Some(TargetValue::Continuous(v)) => *v,
            Some(TargetValue::Discrete(v)) => *v as f64,
            _ => return false,
        };

        match model.model_type {
            OnlineModelType::Perceptron => {
                self.update_perceptron(model, &sample.features, target);
            }

            OnlineModelType::PassiveAggressive => {
                self.update_pa(model, &sample.features, target);
            }

            OnlineModelType::SGD => {
                self.update_sgd(model, &sample.features, target);
            }

            OnlineModelType::RunningMean => {
                self.update_running_mean(model, target);
            }

            OnlineModelType::EMA => {
                self.update_ema(model, target);
            }

            OnlineModelType::OnlineKMeans => {
                self.update_kmeans(model, &sample.features);
            }
        }

        model.metrics.samples_seen += 1;
        model.last_updated = Timestamp::now();

        // Decay learning rate
        model.params.learning_rate = (model.params.learning_rate * self.config.lr_decay)
            .max(self.config.min_lr);

        self.stats.updates_performed += 1;

        true
    }

    fn update_perceptron(&self, model: &mut OnlineModel, features: &[f64], target: f64) {
        let prediction = self.compute_linear(model, features);
        let y = if target > 0.0 { 1.0 } else { -1.0 };
        let y_hat = if prediction > 0.0 { 1.0 } else { -1.0 };

        if y * prediction <= 0.0 {
            // Misclassification
            let lr = model.params.learning_rate;

            for (i, x) in features.iter().enumerate() {
                if i < model.params.weights.len() {
                    model.params.weights[i] += lr * y * x;
                }
            }
            model.params.bias += lr * y;

            model.metrics.cumulative_loss += 1.0;
        } else {
            // Update accuracy
            let correct = if y == y_hat { 1.0 } else { 0.0 };
            let n = model.metrics.samples_seen as f64;
            model.metrics.accuracy = (model.metrics.accuracy * n + correct) / (n + 1.0);
        }
    }

    fn update_pa(&self, model: &mut OnlineModel, features: &[f64], target: f64) {
        let prediction = self.compute_linear(model, features);
        let y = if target > 0.0 { 1.0 } else { -1.0 };

        let loss = (1.0 - y * prediction).max(0.0);

        if loss > 0.0 {
            let norm_sq: f64 = features.iter().map(|x| x * x).sum();
            let tau = if norm_sq > 0.0 { loss / norm_sq } else { 0.0 };

            for (i, x) in features.iter().enumerate() {
                if i < model.params.weights.len() {
                    model.params.weights[i] += tau * y * x;
                }
            }
            model.params.bias += tau * y;

            model.metrics.cumulative_loss += loss;
        }
    }

    fn update_sgd(&self, model: &mut OnlineModel, features: &[f64], target: f64) {
        let prediction = self.compute_linear(model, features);
        let error = target - prediction;
        let lr = model.params.learning_rate;
        let reg = model.params.regularization;

        for (i, x) in features.iter().enumerate() {
            if i < model.params.weights.len() {
                model.params.weights[i] += lr * (error * x - reg * model.params.weights[i]);
            }
        }
        model.params.bias += lr * error;

        model.metrics.cumulative_loss += error * error;
    }

    fn update_running_mean(&self, model: &mut OnlineModel, value: f64) {
        let n = model.metrics.samples_seen as f64 + 1.0;
        model.params.bias = model.params.bias + (value - model.params.bias) / n;
    }

    fn update_ema(&self, model: &mut OnlineModel, value: f64) {
        let alpha = model.params.learning_rate;
        model.params.bias = alpha * value + (1.0 - alpha) * model.params.bias;
    }

    fn update_kmeans(&self, model: &mut OnlineModel, features: &[f64]) {
        // Simple online k-means update
        let lr = model.params.learning_rate;

        // Move centroid (stored in weights) towards sample
        for (i, x) in features.iter().enumerate() {
            if i < model.params.weights.len() {
                model.params.weights[i] += lr * (x - model.params.weights[i]);
            }
        }
    }

    fn compute_linear(&self, model: &OnlineModel, features: &[f64]) -> f64 {
        let dot: f64 = model.params.weights.iter()
            .zip(features.iter())
            .map(|(w, x)| w * x)
            .sum();

        dot + model.params.bias
    }

    /// Predict
    pub fn predict(&mut self, model_id: u64, features: &[f64]) -> Option<Prediction> {
        let start = Timestamp::now();

        let model = self.models.get_mut(&model_id)?;

        let value = match model.model_type {
            OnlineModelType::Perceptron | OnlineModelType::PassiveAggressive => {
                let score = self.compute_linear(model, features);
                TargetValue::Discrete(if score > 0.0 { 1 } else { 0 })
            }

            OnlineModelType::SGD => {
                let score = self.compute_linear(model, features);
                TargetValue::Continuous(score)
            }

            OnlineModelType::RunningMean | OnlineModelType::EMA => {
                TargetValue::Continuous(model.params.bias)
            }

            OnlineModelType::OnlineKMeans => {
                // Return distance to centroid
                let dist: f64 = model.params.weights.iter()
                    .zip(features.iter())
                    .map(|(c, x)| (c - x).powi(2))
                    .sum::<f64>()
                    .sqrt();

                TargetValue::Continuous(dist)
            }
        };

        model.metrics.predictions += 1;
        self.stats.predictions_made += 1;

        let end = Timestamp::now();

        Some(Prediction {
            value,
            confidence: model.metrics.accuracy,
            latency_ns: end.0.saturating_sub(start.0),
        })
    }

    /// Batch update
    pub fn batch_update(&mut self, model_id: u64, samples: Vec<Sample>) {
        for sample in samples {
            self.update(model_id, sample);
        }
    }

    /// Get model
    pub fn get_model(&self, id: u64) -> Option<&OnlineModel> {
        self.models.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &OnlineStats {
        &self.stats
    }
}

impl Default for OnlineLearner {
    fn default() -> Self {
        Self::new(OnlineConfig::default())
    }
}

// ============================================================================
// SAMPLE BUILDER
// ============================================================================

/// Sample builder
pub struct SampleBuilder {
    id: u64,
    features: Vec<f64>,
    target: Option<TargetValue>,
    weight: f64,
}

impl SampleBuilder {
    /// Create new builder
    pub fn new(id: u64) -> Self {
        Self {
            id,
            features: Vec::new(),
            target: None,
            weight: 1.0,
        }
    }

    /// Set features
    pub fn features(mut self, features: Vec<f64>) -> Self {
        self.features = features;
        self
    }

    /// Set continuous target
    pub fn target_continuous(mut self, value: f64) -> Self {
        self.target = Some(TargetValue::Continuous(value));
        self
    }

    /// Set discrete target
    pub fn target_discrete(mut self, value: u64) -> Self {
        self.target = Some(TargetValue::Discrete(value));
        self
    }

    /// Set weight
    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Build
    pub fn build(self) -> Sample {
        Sample {
            id: self.id,
            features: self.features,
            target: self.target,
            weight: self.weight,
            timestamp: Timestamp::now(),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_model() {
        let mut learner = OnlineLearner::default();

        let id = learner.create_model("test", OnlineModelType::Perceptron, 2);
        assert!(learner.get_model(id).is_some());
    }

    #[test]
    fn test_perceptron_update() {
        let mut learner = OnlineLearner::default();

        let id = learner.create_model("perceptron", OnlineModelType::Perceptron, 2);

        let sample = SampleBuilder::new(1)
            .features(vec![1.0, 0.0])
            .target_discrete(1)
            .build();

        let success = learner.update(id, sample);
        assert!(success);

        let model = learner.get_model(id).unwrap();
        assert_eq!(model.metrics.samples_seen, 1);
    }

    #[test]
    fn test_sgd_regression() {
        let mut learner = OnlineLearner::default();

        let id = learner.create_model("sgd", OnlineModelType::SGD, 1);

        // Train on y = 2x
        for i in 0..100 {
            let x = i as f64 / 100.0;
            let y = 2.0 * x;

            let sample = SampleBuilder::new(i as u64)
                .features(vec![x])
                .target_continuous(y)
                .build();

            learner.update(id, sample);
        }

        // Predict
        let pred = learner.predict(id, &[0.5]).unwrap();

        if let TargetValue::Continuous(v) = pred.value {
            assert!((v - 1.0).abs() < 0.5); // Should be close to 1.0
        }
    }

    #[test]
    fn test_running_mean() {
        let mut learner = OnlineLearner::default();

        let id = learner.create_model("mean", OnlineModelType::RunningMean, 1);

        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            let sample = SampleBuilder::new(1)
                .features(vec![])
                .target_continuous(v)
                .build();

            learner.update(id, sample);
        }

        let pred = learner.predict(id, &[]).unwrap();

        if let TargetValue::Continuous(mean) = pred.value {
            assert!((mean - 3.0).abs() < 0.01);
        }
    }
}
