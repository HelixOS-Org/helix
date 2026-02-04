//! # Transfer Learning for NEXUS
//!
//! Knowledge transfer between domains and tasks.
//!
//! ## Features
//!
//! - Feature transformation
//! - Domain adaptation
//! - Knowledge distillation
//! - Cross-domain transfer

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::math::F64Ext;

// ============================================================================
// DOMAIN TYPES
// ============================================================================

/// Domain identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainId(pub u32);

/// A domain for transfer learning
#[derive(Debug, Clone)]
pub struct Domain {
    /// Domain ID
    pub id: DomainId,
    /// Domain name
    pub name: String,
    /// Feature dimension
    pub feature_dim: usize,
    /// Domain statistics
    pub mean: Vec<f64>,
    /// Domain std dev
    pub std: Vec<f64>,
    /// Sample count
    pub sample_count: u64,
}

impl Domain {
    /// Create new domain
    pub fn new(id: DomainId, name: String, feature_dim: usize) -> Self {
        Self {
            id,
            name,
            feature_dim,
            mean: vec![0.0; feature_dim],
            std: vec![1.0; feature_dim],
            sample_count: 0,
        }
    }

    /// Update statistics with new sample
    pub fn update_stats(&mut self, features: &[f64]) {
        let n = self.sample_count as f64 + 1.0;

        for (i, &f) in features.iter().enumerate() {
            if i < self.feature_dim {
                // Online mean update
                let delta = f - self.mean[i];
                self.mean[i] += delta / n;

                // Online variance update (simplified)
                if n > 1.0 {
                    let new_delta = f - self.mean[i];
                    let new_var = (self.std[i].powi(2) * (n - 1.0) + delta * new_delta) / n;
                    self.std[i] = new_var.sqrt().max(0.001);
                }
            }
        }

        self.sample_count += 1;
    }

    /// Normalize features for this domain
    pub fn normalize(&self, features: &[f64]) -> Vec<f64> {
        features
            .iter()
            .enumerate()
            .map(|(i, &f)| {
                if i < self.feature_dim {
                    (f - self.mean[i]) / self.std[i].max(0.001)
                } else {
                    f
                }
            })
            .collect()
    }

    /// Denormalize features
    pub fn denormalize(&self, features: &[f64]) -> Vec<f64> {
        features
            .iter()
            .enumerate()
            .map(|(i, &f)| {
                if i < self.feature_dim {
                    f * self.std[i] + self.mean[i]
                } else {
                    f
                }
            })
            .collect()
    }
}

// ============================================================================
// FEATURE TRANSFORMER
// ============================================================================

/// Feature transformer for domain alignment
pub struct FeatureTransformer {
    /// Source domain
    source: DomainId,
    /// Target domain
    target: DomainId,
    /// Transformation matrix
    transform: Vec<Vec<f64>>,
    /// Bias
    bias: Vec<f64>,
    /// Input dimension
    in_dim: usize,
    /// Output dimension
    out_dim: usize,
}

impl FeatureTransformer {
    /// Create identity transformer
    pub fn identity(source: DomainId, target: DomainId, dim: usize) -> Self {
        let transform: Vec<Vec<f64>> = (0..dim)
            .map(|i| {
                let mut row = vec![0.0; dim];
                row[i] = 1.0;
                row
            })
            .collect();

        Self {
            source,
            target,
            transform,
            bias: vec![0.0; dim],
            in_dim: dim,
            out_dim: dim,
        }
    }

    /// Create random initialization
    pub fn random(
        source: DomainId,
        target: DomainId,
        in_dim: usize,
        out_dim: usize,
        seed: u64,
    ) -> Self {
        let mut rng = seed;

        let transform: Vec<Vec<f64>> = (0..out_dim)
            .map(|_| {
                (0..in_dim)
                    .map(|_| {
                        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                        (rng >> 32) as f64 / u32::MAX as f64 * 0.2 - 0.1
                    })
                    .collect()
            })
            .collect();

        Self {
            source,
            target,
            transform,
            bias: vec![0.0; out_dim],
            in_dim,
            out_dim,
        }
    }

    /// Transform features from source to target domain
    pub fn transform(&self, features: &[f64]) -> Vec<f64> {
        self.transform
            .iter()
            .zip(self.bias.iter())
            .map(|(row, &b)| {
                let dot: f64 = row.iter().zip(features.iter()).map(|(w, f)| w * f).sum();
                dot + b
            })
            .collect()
    }

    /// Update transformer (gradient descent)
    pub fn update(&mut self, source_features: &[f64], target_features: &[f64], lr: f64) {
        let output = self.transform(source_features);

        // Compute error
        let errors: Vec<f64> = output
            .iter()
            .zip(target_features.iter())
            .map(|(o, t)| o - t)
            .collect();

        // Update weights
        for (i, row) in self.transform.iter_mut().enumerate() {
            for (j, w) in row.iter_mut().enumerate() {
                if i < errors.len() && j < source_features.len() {
                    *w -= lr * errors[i] * source_features[j];
                }
            }
        }

        // Update bias
        for (i, b) in self.bias.iter_mut().enumerate() {
            if i < errors.len() {
                *b -= lr * errors[i];
            }
        }
    }

    /// Get source domain
    pub fn source(&self) -> DomainId {
        self.source
    }

    /// Get target domain
    pub fn target(&self) -> DomainId {
        self.target
    }
}

// ============================================================================
// DOMAIN ADAPTER
// ============================================================================

/// Domain adaptation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptationStrategy {
    /// Simple feature normalization
    Normalize,
    /// Coral (correlation alignment)
    Coral,
    /// Maximum Mean Discrepancy
    MMD,
    /// Adversarial domain adaptation
    Adversarial,
}

/// Domain adapter for distribution alignment
pub struct DomainAdapter {
    /// Source domain
    source: Domain,
    /// Target domain
    target: Domain,
    /// Feature transformer
    transformer: FeatureTransformer,
    /// Adaptation strategy
    strategy: AdaptationStrategy,
    /// Adaptation iterations
    iterations: u64,
}

impl DomainAdapter {
    /// Create new domain adapter
    pub fn new(source: Domain, target: Domain, strategy: AdaptationStrategy) -> Self {
        let dim = source.feature_dim.max(target.feature_dim);
        let transformer = FeatureTransformer::identity(source.id, target.id, dim);

        Self {
            source,
            target,
            transformer,
            strategy,
            iterations: 0,
        }
    }

    /// Adapt source features to target domain
    pub fn adapt(&self, source_features: &[f64]) -> Vec<f64> {
        match self.strategy {
            AdaptationStrategy::Normalize => {
                // Normalize in source, denormalize in target
                let normalized = self.source.normalize(source_features);
                self.target.denormalize(&normalized)
            },
            _ => {
                // Use learned transformer
                let normalized = self.source.normalize(source_features);
                let transformed = self.transformer.transform(&normalized);
                self.target.denormalize(&transformed)
            },
        }
    }

    /// Update adapter with paired samples
    pub fn update(&mut self, source_features: &[f64], target_features: &[f64], lr: f64) {
        // Update domain statistics
        self.source.update_stats(source_features);
        self.target.update_stats(target_features);

        // Update transformer
        let source_norm = self.source.normalize(source_features);
        let target_norm = self.target.normalize(target_features);
        self.transformer.update(&source_norm, &target_norm, lr);

        self.iterations += 1;
    }

    /// Get adaptation statistics
    pub fn stats(&self) -> AdaptationStats {
        AdaptationStats {
            source_samples: self.source.sample_count,
            target_samples: self.target.sample_count,
            iterations: self.iterations,
        }
    }
}

/// Adaptation statistics
#[derive(Debug, Clone)]
pub struct AdaptationStats {
    /// Source samples seen
    pub source_samples: u64,
    /// Target samples seen
    pub target_samples: u64,
    /// Adaptation iterations
    pub iterations: u64,
}

// ============================================================================
// KNOWLEDGE TRANSFER
// ============================================================================

/// Knowledge transfer between models
pub struct KnowledgeTransfer {
    /// Source model weights
    source_weights: Vec<f64>,
    /// Target model weights
    target_weights: Vec<f64>,
    /// Transfer ratio (0 = target only, 1 = source only)
    transfer_ratio: f64,
    /// Fine-tune learning rate
    finetune_lr: f64,
}

impl KnowledgeTransfer {
    /// Create new knowledge transfer with default settings
    pub fn default_new() -> Self {
        Self::new(Vec::new(), 0.5)
    }

    /// Create new knowledge transfer
    pub fn new(source_weights: Vec<f64>, transfer_ratio: f64) -> Self {
        let target_weights = source_weights.clone();

        Self {
            source_weights,
            target_weights,
            transfer_ratio: transfer_ratio.clamp(0.0, 1.0),
            finetune_lr: 0.01,
        }
    }

    /// Initialize target from source (transfer)
    pub fn transfer(&mut self) {
        for (t, &s) in self
            .target_weights
            .iter_mut()
            .zip(self.source_weights.iter())
        {
            *t = self.transfer_ratio * s + (1.0 - self.transfer_ratio) * *t;
        }
    }

    /// Fine-tune target with gradient
    pub fn finetune(&mut self, gradient: &[f64]) {
        for (w, g) in self.target_weights.iter_mut().zip(gradient.iter()) {
            *w -= self.finetune_lr * g;
        }
    }

    /// Get target weights
    pub fn target_weights(&self) -> &[f64] {
        &self.target_weights
    }

    /// Set fine-tune learning rate
    pub fn set_finetune_lr(&mut self, lr: f64) {
        self.finetune_lr = lr;
    }

    /// Count the number of weights (for knowledge_items)
    pub fn count(&self) -> usize {
        self.target_weights.len()
    }
}

// ============================================================================
// TRANSFER LEARNER
// ============================================================================

/// Complete transfer learning pipeline
pub struct TransferLearner {
    /// Domains
    domains: BTreeMap<DomainId, Domain>,
    /// Adapters
    adapters: BTreeMap<(DomainId, DomainId), DomainAdapter>,
    /// Source model weights
    source_model: Vec<f64>,
    /// Target model weights
    target_model: Vec<f64>,
    /// Feature dimension
    feature_dim: usize,
    /// Next domain ID
    next_domain_id: u32,
}

impl TransferLearner {
    /// Create new transfer learner
    pub fn new(feature_dim: usize) -> Self {
        Self {
            domains: BTreeMap::new(),
            adapters: BTreeMap::new(),
            source_model: vec![0.0; feature_dim],
            target_model: vec![0.0; feature_dim],
            feature_dim,
            next_domain_id: 0,
        }
    }

    /// Register a domain
    pub fn register_domain(&mut self, name: String) -> DomainId {
        let id = DomainId(self.next_domain_id);
        self.next_domain_id += 1;

        let domain = Domain::new(id, name, self.feature_dim);
        self.domains.insert(id, domain);

        id
    }

    /// Get domain
    pub fn get_domain(&self, id: DomainId) -> Option<&Domain> {
        self.domains.get(&id)
    }

    /// Create adapter between domains
    pub fn create_adapter(
        &mut self,
        source: DomainId,
        target: DomainId,
        strategy: AdaptationStrategy,
    ) -> bool {
        let source_domain = match self.domains.get(&source) {
            Some(d) => d.clone(),
            None => return false,
        };
        let target_domain = match self.domains.get(&target) {
            Some(d) => d.clone(),
            None => return false,
        };

        let adapter = DomainAdapter::new(source_domain, target_domain, strategy);
        self.adapters.insert((source, target), adapter);
        true
    }

    /// Adapt features from source to target domain
    pub fn adapt_features(
        &self,
        source: DomainId,
        target: DomainId,
        features: &[f64],
    ) -> Option<Vec<f64>> {
        self.adapters
            .get(&(source, target))
            .map(|adapter| adapter.adapt(features))
    }

    /// Train source model
    pub fn train_source(&mut self, features: &[f64], label: f64, lr: f64) {
        // Simple linear model
        let pred: f64 = self
            .source_model
            .iter()
            .zip(features.iter())
            .map(|(w, f)| w * f)
            .sum();

        let error = pred - label;

        for (w, f) in self.source_model.iter_mut().zip(features.iter()) {
            *w -= lr * error * f;
        }
    }

    /// Transfer knowledge to target
    pub fn transfer_to_target(&mut self, ratio: f64) {
        for (t, &s) in self.target_model.iter_mut().zip(self.source_model.iter()) {
            *t = ratio * s + (1.0 - ratio) * *t;
        }
    }

    /// Fine-tune target model
    pub fn finetune_target(&mut self, features: &[f64], label: f64, lr: f64) {
        let pred: f64 = self
            .target_model
            .iter()
            .zip(features.iter())
            .map(|(w, f)| w * f)
            .sum();

        let error = pred - label;

        for (w, f) in self.target_model.iter_mut().zip(features.iter()) {
            *w -= lr * error * f;
        }
    }

    /// Predict using target model
    pub fn predict(&self, features: &[f64]) -> f64 {
        self.target_model
            .iter()
            .zip(features.iter())
            .map(|(w, f)| w * f)
            .sum()
    }

    /// Get domain count
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain() {
        let mut domain = Domain::new(DomainId(0), String::from("test"), 3);

        domain.update_stats(&[1.0, 2.0, 3.0]);
        domain.update_stats(&[2.0, 4.0, 6.0]);
        domain.update_stats(&[3.0, 6.0, 9.0]);

        // Mean should be [2, 4, 6]
        assert!((domain.mean[0] - 2.0).abs() < 0.01);
        assert!((domain.mean[1] - 4.0).abs() < 0.01);
        assert!((domain.mean[2] - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_feature_transformer() {
        let transformer = FeatureTransformer::identity(DomainId(0), DomainId(1), 3);

        let input = vec![1.0, 2.0, 3.0];
        let output = transformer.transform(&input);

        assert_eq!(output, input);
    }

    #[test]
    fn test_domain_adapter() {
        let source = Domain::new(DomainId(0), String::from("source"), 2);
        let target = Domain::new(DomainId(1), String::from("target"), 2);

        let adapter = DomainAdapter::new(source, target, AdaptationStrategy::Normalize);

        let adapted = adapter.adapt(&[1.0, 2.0]);
        assert_eq!(adapted.len(), 2);
    }

    #[test]
    fn test_knowledge_transfer() {
        let source_weights = vec![1.0, 2.0, 3.0];
        let mut transfer = KnowledgeTransfer::new(source_weights.clone(), 0.8);

        transfer.transfer();

        // 0.8 * source + 0.2 * target (which started as source)
        // So should equal source
        assert_eq!(transfer.target_weights(), &source_weights);
    }

    #[test]
    fn test_transfer_learner() {
        let mut learner = TransferLearner::new(2);

        let source_id = learner.register_domain(String::from("source"));
        let target_id = learner.register_domain(String::from("target"));

        learner.create_adapter(source_id, target_id, AdaptationStrategy::Normalize);

        // Train on source
        for _ in 0..100 {
            learner.train_source(&[1.0, 0.0], 1.0, 0.1);
            learner.train_source(&[0.0, 1.0], 2.0, 0.1);
        }

        // Transfer
        learner.transfer_to_target(1.0);

        // Predict
        let pred = learner.predict(&[1.0, 1.0]);
        assert!(pred > 0.0);
    }
}
