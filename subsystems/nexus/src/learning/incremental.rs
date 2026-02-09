//! # Incremental Learning
//!
//! Implements incremental learning for continuous adaptation.
//! Supports online updates without full retraining.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::math::F64Ext;
use crate::types::Timestamp;

// ============================================================================
// INCREMENTAL TYPES
// ============================================================================

/// Incremental model
#[derive(Debug, Clone)]
pub struct IncrementalModel {
    /// Model ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub model_type: ModelType,
    /// Parameters
    pub parameters: BTreeMap<String, f64>,
    /// Examples seen
    pub examples_seen: u64,
    /// Created
    pub created: Timestamp,
    /// Last updated
    pub last_updated: Timestamp,
}

/// Model type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    NaiveBayes,
    Perceptron,
    KNearestBuffer,
    OnlineGradient,
    RunningStats,
}

/// Training example
#[derive(Debug, Clone)]
pub struct Example {
    /// Example ID
    pub id: u64,
    /// Features
    pub features: Vec<f64>,
    /// Label
    pub label: Label,
    /// Weight
    pub weight: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Label
#[derive(Debug, Clone)]
pub enum Label {
    Class(u64),
    Regression(f64),
    MultiLabel(Vec<u64>),
}

/// Prediction
#[derive(Debug, Clone)]
pub struct Prediction {
    /// Predicted label
    pub label: Label,
    /// Confidence
    pub confidence: f64,
    /// Probabilities
    pub probabilities: LinearMap<f64, 64>,
}

/// Update result
#[derive(Debug, Clone)]
pub struct UpdateResult {
    /// Updated successfully
    pub success: bool,
    /// Parameters changed
    pub parameters_changed: usize,
    /// Old loss
    pub old_loss: Option<f64>,
    /// New loss
    pub new_loss: Option<f64>,
}

/// Running statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RunningStats {
    /// Count
    pub count: u64,
    /// Mean
    pub mean: Vec<f64>,
    /// M2 for variance
    pub m2: Vec<f64>,
    /// Min
    pub min: Vec<f64>,
    /// Max
    pub max: Vec<f64>,
}

impl RunningStats {
    /// Create new running stats with given dimension
    pub fn new(dim: usize) -> Self {
        Self {
            count: 0,
            mean: vec![0.0; dim],
            m2: vec![0.0; dim],
            min: vec![f64::INFINITY; dim],
            max: vec![f64::NEG_INFINITY; dim],
        }
    }

    fn update(&mut self, values: &[f64]) {
        self.count += 1;
        let n = self.count as f64;

        for (i, &v) in values.iter().enumerate() {
            if i >= self.mean.len() {
                continue;
            }

            // Welford's algorithm
            let delta = v - self.mean[i];
            self.mean[i] += delta / n;
            let delta2 = v - self.mean[i];
            self.m2[i] += delta * delta2;

            // Min/max
            if v < self.min[i] {
                self.min[i] = v;
            }
            if v > self.max[i] {
                self.max[i] = v;
            }
        }
    }

    fn variance(&self) -> Vec<f64> {
        if self.count < 2 {
            return vec![0.0; self.m2.len()];
        }
        self.m2
            .iter()
            .map(|m| m / (self.count - 1) as f64)
            .collect()
    }
}

// ============================================================================
// INCREMENTAL ENGINE
// ============================================================================

/// Incremental learning engine
pub struct IncrementalEngine {
    /// Models
    models: BTreeMap<u64, IncrementalModel>,
    /// Statistics per model
    model_stats: BTreeMap<u64, RunningStats>,
    /// Class counts (for Naive Bayes)
    class_counts: BTreeMap<u64, BTreeMap<u64, u64>>,
    /// Feature sums per class
    feature_sums: BTreeMap<u64, BTreeMap<u64, Vec<f64>>>,
    /// Recent examples (for KNN)
    example_buffer: BTreeMap<u64, Vec<Example>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: IncrementalConfig,
    /// Statistics
    stats: IncrementalStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct IncrementalConfig {
    /// Buffer size for KNN
    pub buffer_size: usize,
    /// Learning rate
    pub learning_rate: f64,
    /// Decay rate
    pub decay_rate: f64,
}

impl Default for IncrementalConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            learning_rate: 0.01,
            decay_rate: 0.99,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct IncrementalStats {
    /// Models created
    pub models_created: u64,
    /// Examples processed
    pub examples_processed: u64,
    /// Predictions made
    pub predictions_made: u64,
}

impl IncrementalEngine {
    /// Create new engine
    pub fn new(config: IncrementalConfig) -> Self {
        Self {
            models: BTreeMap::new(),
            model_stats: BTreeMap::new(),
            class_counts: BTreeMap::new(),
            feature_sums: BTreeMap::new(),
            example_buffer: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: IncrementalStats::default(),
        }
    }

    /// Create model
    pub fn create_model(&mut self, name: &str, model_type: ModelType) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let model = IncrementalModel {
            id,
            name: name.into(),
            model_type,
            parameters: BTreeMap::new(),
            examples_seen: 0,
            created: now,
            last_updated: now,
        };

        self.models.insert(id, model);
        self.model_stats.insert(id, RunningStats::default());
        self.class_counts.insert(id, BTreeMap::new());
        self.feature_sums.insert(id, BTreeMap::new());
        self.example_buffer.insert(id, Vec::new());

        self.stats.models_created += 1;

        id
    }

    /// Update model with example
    pub fn update(&mut self, model_id: u64, example: Example) -> UpdateResult {
        let model = match self.models.get_mut(&model_id) {
            Some(m) => m,
            None => {
                return UpdateResult {
                    success: false,
                    parameters_changed: 0,
                    old_loss: None,
                    new_loss: None,
                };
            },
        };

        model.examples_seen += 1;
        model.last_updated = Timestamp::now();

        // Update statistics
        if let Some(stats) = self.model_stats.get_mut(&model_id) {
            if stats.mean.is_empty() {
                *stats = RunningStats::new(example.features.len());
            }
            stats.update(&example.features);
        }

        let result = match model.model_type {
            ModelType::NaiveBayes => self.update_naive_bayes(model_id, &example),
            ModelType::Perceptron => self.update_perceptron(model_id, &example),
            ModelType::KNearestBuffer => self.update_knn_buffer(model_id, example),
            ModelType::OnlineGradient => self.update_gradient(model_id, &example),
            ModelType::RunningStats => UpdateResult {
                success: true,
                parameters_changed: 1,
                old_loss: None,
                new_loss: None,
            },
        };

        self.stats.examples_processed += 1;

        result
    }

    fn update_naive_bayes(&mut self, model_id: u64, example: &Example) -> UpdateResult {
        let class = match &example.label {
            Label::Class(c) => *c,
            _ => {
                return UpdateResult {
                    success: false,
                    parameters_changed: 0,
                    old_loss: None,
                    new_loss: None,
                };
            },
        };

        // Update class counts
        if let Some(counts) = self.class_counts.get_mut(&model_id) {
            *counts.entry(class).or_insert(0) += 1;
        }

        // Update feature sums
        if let Some(sums) = self.feature_sums.get_mut(&model_id) {
            let class_sums = sums
                .entry(class)
                .or_insert_with(|| vec![0.0; example.features.len()]);

            for (i, &f) in example.features.iter().enumerate() {
                if i < class_sums.len() {
                    class_sums[i] += f;
                }
            }
        }

        UpdateResult {
            success: true,
            parameters_changed: example.features.len(),
            old_loss: None,
            new_loss: None,
        }
    }

    fn update_perceptron(&mut self, model_id: u64, example: &Example) -> UpdateResult {
        let target = match &example.label {
            Label::Class(c) => {
                if *c == 0 {
                    -1.0
                } else {
                    1.0
                }
            },
            Label::Regression(r) => *r,
            _ => {
                return UpdateResult {
                    success: false,
                    parameters_changed: 0,
                    old_loss: None,
                    new_loss: None,
                };
            },
        };

        let model = match self.models.get_mut(&model_id) {
            Some(m) => m,
            None => {
                return UpdateResult {
                    success: false,
                    parameters_changed: 0,
                    old_loss: None,
                    new_loss: None,
                };
            },
        };

        // Initialize weights if needed
        if model.parameters.is_empty() {
            for i in 0..example.features.len() {
                model.parameters.insert(format!("w{}", i), 0.0);
            }
            model.parameters.insert("bias".into(), 0.0);
        }

        // Compute prediction
        let mut pred = *model.parameters.get("bias").unwrap_or(&0.0);
        for (i, &f) in example.features.iter().enumerate() {
            pred += f * model.parameters.get(&format!("w{}", i)).unwrap_or(&0.0);
        }

        let old_loss = (target - pred).abs();

        // Update if wrong
        if pred * target <= 0.0 {
            let lr = self.config.learning_rate;

            for (i, &f) in example.features.iter().enumerate() {
                let key = format!("w{}", i);
                let w = model.parameters.get(&key).copied().unwrap_or(0.0);
                model.parameters.insert(key, w + lr * target * f);
            }

            let bias = model.parameters.get("bias").copied().unwrap_or(0.0);
            model.parameters.insert("bias".into(), bias + lr * target);
        }

        // Compute new prediction
        let mut new_pred = *model.parameters.get("bias").unwrap_or(&0.0);
        for (i, &f) in example.features.iter().enumerate() {
            new_pred += f * model.parameters.get(&format!("w{}", i)).unwrap_or(&0.0);
        }

        UpdateResult {
            success: true,
            parameters_changed: example.features.len() + 1,
            old_loss: Some(old_loss),
            new_loss: Some((target - new_pred).abs()),
        }
    }

    fn update_knn_buffer(&mut self, model_id: u64, example: Example) -> UpdateResult {
        if let Some(buffer) = self.example_buffer.get_mut(&model_id) {
            buffer.push(example);

            if buffer.len() > self.config.buffer_size {
                buffer.pop_front();
            }
        }

        UpdateResult {
            success: true,
            parameters_changed: 0,
            old_loss: None,
            new_loss: None,
        }
    }

    fn update_gradient(&mut self, model_id: u64, example: &Example) -> UpdateResult {
        // Similar to perceptron but with sigmoid activation
        let target = match &example.label {
            Label::Regression(r) => *r,
            Label::Class(c) => *c as f64,
            _ => {
                return UpdateResult {
                    success: false,
                    parameters_changed: 0,
                    old_loss: None,
                    new_loss: None,
                };
            },
        };

        let model = match self.models.get_mut(&model_id) {
            Some(m) => m,
            None => {
                return UpdateResult {
                    success: false,
                    parameters_changed: 0,
                    old_loss: None,
                    new_loss: None,
                };
            },
        };

        // Initialize weights if needed
        if model.parameters.is_empty() {
            for i in 0..example.features.len() {
                model.parameters.insert(format!("w{}", i), 0.0);
            }
            model.parameters.insert("bias".into(), 0.0);
        }

        // Compute linear combination
        let mut z = *model.parameters.get("bias").unwrap_or(&0.0);
        for (i, &f) in example.features.iter().enumerate() {
            z += f * model.parameters.get(&format!("w{}", i)).unwrap_or(&0.0);
        }

        // Sigmoid
        let pred = 1.0 / (1.0 + (-z).exp());
        let error = pred - target;
        let old_loss = error * error;

        // Gradient descent update
        let lr = self.config.learning_rate;
        let grad = error * pred * (1.0 - pred);

        for (i, &f) in example.features.iter().enumerate() {
            let key = format!("w{}", i);
            let w = model.parameters.get(&key).copied().unwrap_or(0.0);
            model.parameters.insert(key, w - lr * grad * f);
        }

        let bias = model.parameters.get("bias").copied().unwrap_or(0.0);
        model.parameters.insert("bias".into(), bias - lr * grad);

        UpdateResult {
            success: true,
            parameters_changed: example.features.len() + 1,
            old_loss: Some(old_loss),
            new_loss: None,
        }
    }

    /// Predict
    pub fn predict(&mut self, model_id: u64, features: &[f64]) -> Option<Prediction> {
        let model = self.models.get(&model_id)?;

        self.stats.predictions_made += 1;

        match model.model_type {
            ModelType::NaiveBayes => self.predict_naive_bayes(model_id, features),
            ModelType::Perceptron => self.predict_perceptron(model_id, features),
            ModelType::KNearestBuffer => self.predict_knn(model_id, features),
            ModelType::OnlineGradient => self.predict_gradient(model_id, features),
            ModelType::RunningStats => None,
        }
    }

    fn predict_naive_bayes(&self, model_id: u64, features: &[f64]) -> Option<Prediction> {
        let counts = self.class_counts.get(&model_id)?;
        let sums = self.feature_sums.get(&model_id)?;

        let total: u64 = counts.values().sum();
        if total == 0 {
            return None;
        }

        let mut probs = BTreeMap::new();
        let mut max_class = 0;
        let mut max_prob = f64::NEG_INFINITY;

        for (&class, &count) in counts {
            let prior = count as f64 / total as f64;
            let mut log_prob = prior.ln();

            if let Some(class_sums) = sums.get(&class) {
                for (i, &f) in features.iter().enumerate() {
                    if i < class_sums.len() {
                        let mean = class_sums[i] / count as f64;
                        // Gaussian likelihood (simplified)
                        log_prob += -0.5 * (f - mean) * (f - mean);
                    }
                }
            }

            probs.insert(class, log_prob);

            if log_prob > max_prob {
                max_prob = log_prob;
                max_class = class;
            }
        }

        // Normalize probabilities
        let max_val = probs.values().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = probs.values().map(|&p| (p - max_val).exp()).sum();

        for (_, prob) in probs.iter_mut() {
            *prob = (*prob - max_val).exp() / sum;
        }

        Some(Prediction {
            label: Label::Class(max_class),
            confidence: probs.get(&max_class).copied().unwrap_or(0.0),
            probabilities: probs,
        })
    }

    fn predict_perceptron(&self, model_id: u64, features: &[f64]) -> Option<Prediction> {
        let model = self.models.get(&model_id)?;

        let mut pred = *model.parameters.get("bias").unwrap_or(&0.0);
        for (i, &f) in features.iter().enumerate() {
            pred += f * model.parameters.get(&format!("w{}", i)).unwrap_or(&0.0);
        }

        let class = if pred > 0.0 { 1 } else { 0 };
        let confidence = (1.0 / (1.0 + (-pred.abs()).exp())).max(0.5);

        Some(Prediction {
            label: Label::Class(class),
            confidence,
            probabilities: LinearMap::new(),
        })
    }

    fn predict_knn(&self, model_id: u64, features: &[f64]) -> Option<Prediction> {
        let buffer = self.example_buffer.get(&model_id)?;

        if buffer.is_empty() {
            return None;
        }

        // Find k nearest
        let k = 3.min(buffer.len());
        let mut distances: Vec<_> = buffer
            .iter()
            .map(|ex| {
                let dist: f64 = ex
                    .features
                    .iter()
                    .zip(features.iter())
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum();
                (dist.sqrt(), &ex.label)
            })
            .collect();

        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));

        let mut votes: LinearMap<u64, 64> = BTreeMap::new();

        for (_, label) in distances.iter().take(k) {
            if let Label::Class(c) = label {
                *votes.entry(*c).or_insert(0) += 1;
            }
        }

        let (best_class, best_count) = votes.iter().max_by_key(|&(_, &count)| count)?;

        Some(Prediction {
            label: Label::Class(*best_class),
            confidence: *best_count as f64 / k as f64,
            probabilities: LinearMap::new(),
        })
    }

    fn predict_gradient(&self, model_id: u64, features: &[f64]) -> Option<Prediction> {
        let model = self.models.get(&model_id)?;

        let mut z = *model.parameters.get("bias").unwrap_or(&0.0);
        for (i, &f) in features.iter().enumerate() {
            z += f * model.parameters.get(&format!("w{}", i)).unwrap_or(&0.0);
        }

        let prob = 1.0 / (1.0 + (-z).exp());
        let class = if prob > 0.5 { 1 } else { 0 };

        let mut probs = BTreeMap::new();
        probs.insert(0, 1.0 - prob);
        probs.insert(1, prob);

        Some(Prediction {
            label: Label::Class(class),
            confidence: if class == 1 { prob } else { 1.0 - prob },
            probabilities: probs,
        })
    }

    /// Get model
    #[inline(always)]
    pub fn get_model(&self, id: u64) -> Option<&IncrementalModel> {
        self.models.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &IncrementalStats {
        &self.stats
    }
}

impl Default for IncrementalEngine {
    fn default() -> Self {
        Self::new(IncrementalConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_example(features: Vec<f64>, class: u64) -> Example {
        Example {
            id: 0,
            features,
            label: Label::Class(class),
            weight: 1.0,
            timestamp: Timestamp::now(),
        }
    }

    #[test]
    fn test_create_model() {
        let mut engine = IncrementalEngine::default();

        let id = engine.create_model("test", ModelType::Perceptron);
        assert!(engine.get_model(id).is_some());
    }

    #[test]
    fn test_perceptron_update() {
        let mut engine = IncrementalEngine::default();

        let model = engine.create_model("perceptron", ModelType::Perceptron);

        let result = engine.update(model, make_example(vec![1.0, 2.0], 1));
        assert!(result.success);
    }

    #[test]
    fn test_perceptron_predict() {
        let mut engine = IncrementalEngine::default();

        let model = engine.create_model("perceptron", ModelType::Perceptron);

        // Train
        for _ in 0..10 {
            engine.update(model, make_example(vec![1.0, 1.0], 1));
            engine.update(model, make_example(vec![-1.0, -1.0], 0));
        }

        // Predict
        let pred = engine.predict(model, &[0.5, 0.5]);
        assert!(pred.is_some());
    }

    #[test]
    fn test_naive_bayes() {
        let mut engine = IncrementalEngine::default();

        let model = engine.create_model("nb", ModelType::NaiveBayes);

        // Train
        engine.update(model, make_example(vec![1.0, 1.0], 0));
        engine.update(model, make_example(vec![1.1, 0.9], 0));
        engine.update(model, make_example(vec![5.0, 5.0], 1));
        engine.update(model, make_example(vec![4.9, 5.1], 1));

        // Predict
        let pred = engine.predict(model, &[1.0, 1.0]);
        assert!(pred.is_some());
    }

    #[test]
    fn test_knn() {
        let mut engine = IncrementalEngine::default();

        let model = engine.create_model("knn", ModelType::KNearestBuffer);

        // Add examples
        engine.update(model, make_example(vec![0.0, 0.0], 0));
        engine.update(model, make_example(vec![1.0, 1.0], 1));
        engine.update(model, make_example(vec![0.1, 0.1], 0));

        // Predict
        let pred = engine.predict(model, &[0.05, 0.05]);
        assert!(pred.is_some());

        if let Some(p) = pred {
            assert!(matches!(p.label, Label::Class(0)));
        }
    }
}
