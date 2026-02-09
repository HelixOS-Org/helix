//! # Meta-Learning for NEXUS
//!
//! Learning to learn - MAML-style algorithms for fast adaptation.
//!
//! ## Features
//!
//! - MAML (Model-Agnostic Meta-Learning)
//! - Task distribution management
//! - Few-shot learning
//! - Learning rate meta-optimization
//! - Task embeddings

#![allow(clippy::excessive_nesting)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::online::StreamingSample;
use crate::math::F64Ext;

// ============================================================================
// TASK TYPES
// ============================================================================

/// Task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u32);

/// A meta-learning task
#[derive(Debug, Clone)]
pub struct MetaTask {
    /// Task ID
    pub id: TaskId,
    /// Task name
    pub name: String,
    /// Support set (training examples)
    pub support: Vec<StreamingSample>,
    /// Query set (test examples)
    pub query: Vec<StreamingSample>,
    /// Task metadata
    pub metadata: BTreeMap<String, f64>,
}

impl MetaTask {
    /// Create new task
    pub fn new(id: TaskId, name: String) -> Self {
        Self {
            id,
            name,
            support: Vec::new(),
            query: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Add support sample
    #[inline(always)]
    pub fn add_support(&mut self, sample: StreamingSample) {
        self.support.push(sample);
    }

    /// Add query sample
    #[inline(always)]
    pub fn add_query(&mut self, sample: StreamingSample) {
        self.query.push(sample);
    }

    /// Get k-shot (number of support examples per class)
    #[inline(always)]
    pub fn support_size(&self) -> usize {
        self.support.len()
    }

    /// Get query size
    #[inline(always)]
    pub fn query_size(&self) -> usize {
        self.query.len()
    }
}

/// Task distribution for meta-learning
pub struct TaskDistribution {
    /// Available tasks
    tasks: BTreeMap<TaskId, MetaTask>,
    /// Task weights (for sampling)
    weights: BTreeMap<TaskId, f64>,
    /// Next task ID
    next_id: u32,
}

impl TaskDistribution {
    /// Create new distribution
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            weights: BTreeMap::new(),
            next_id: 0,
        }
    }

    /// Add task
    #[inline]
    pub fn add_task(&mut self, task: MetaTask) -> TaskId {
        let id = task.id;
        self.weights.insert(id, 1.0);
        self.tasks.insert(id, task);
        id
    }

    /// Create and add task
    #[inline]
    pub fn create_task(&mut self, name: String) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;
        let task = MetaTask::new(id, name);
        self.add_task(task)
    }

    /// Get task
    #[inline(always)]
    pub fn get_task(&self, id: TaskId) -> Option<&MetaTask> {
        self.tasks.get(&id)
    }

    /// Get task mut
    #[inline(always)]
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut MetaTask> {
        self.tasks.get_mut(&id)
    }

    /// Sample task (weighted)
    pub fn sample_task(&self, seed: u64) -> Option<&MetaTask> {
        if self.tasks.is_empty() {
            return None;
        }

        let total_weight: f64 = self.weights.values().sum();
        if total_weight <= 0.0 {
            return self.tasks.values().next();
        }

        let rng = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let random = (rng >> 32) as f64 / u32::MAX as f64 * total_weight;

        let mut cumsum = 0.0;
        for (&id, &weight) in &self.weights {
            cumsum += weight;
            if random < cumsum {
                return self.tasks.get(&id);
            }
        }

        self.tasks.values().last()
    }

    /// Update task weight
    #[inline]
    pub fn update_weight(&mut self, id: TaskId, weight: f64) {
        if let Some(w) = self.weights.get_mut(&id) {
            *w = weight.max(0.0);
        }
    }

    /// Get task count
    #[inline(always)]
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get all task IDs
    #[inline(always)]
    pub fn task_ids(&self) -> Vec<TaskId> {
        self.tasks.keys().copied().collect()
    }
}

impl Default for TaskDistribution {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// META-LEARNER
// ============================================================================

/// Meta-learner configuration
#[derive(Debug, Clone)]
pub struct MetaLearnerConfig {
    /// Feature dimension
    pub feature_dim: usize,
    /// Inner learning rate (for task adaptation)
    pub inner_lr: f64,
    /// Outer learning rate (for meta-update)
    pub outer_lr: f64,
    /// Number of inner steps
    pub inner_steps: usize,
    /// Number of tasks per meta-batch
    pub meta_batch_size: usize,
    /// Use first-order approximation (FOMAML)
    pub first_order: bool,
}

impl Default for MetaLearnerConfig {
    fn default() -> Self {
        Self {
            feature_dim: 10,
            inner_lr: 0.1,
            outer_lr: 0.001,
            inner_steps: 5,
            meta_batch_size: 4,
            first_order: true, // FOMAML is simpler and often works well
        }
    }
}

/// Meta-learner trait
pub trait MetaLearner {
    /// Adapt to a new task (few-shot)
    fn adapt(&mut self, support: &[StreamingSample]) -> AdaptedModel;

    /// Meta-update from batch of tasks
    fn meta_update(&mut self, tasks: &[MetaTask]);

    /// Get meta-parameters
    fn meta_params(&self) -> &[f64];
}

/// Adapted model after few-shot learning
#[derive(Debug, Clone)]
pub struct AdaptedModel {
    /// Adapted weights
    pub weights: Vec<f64>,
    /// Bias
    pub bias: f64,
    /// Steps taken
    pub adaptation_steps: usize,
    /// Final loss
    pub final_loss: f64,
}

impl AdaptedModel {
    /// Predict with adapted model
    #[inline]
    pub fn predict(&self, features: &[f64]) -> f64 {
        let dot: f64 = self
            .weights
            .iter()
            .zip(features.iter())
            .map(|(w, f)| w * f)
            .sum();
        dot + self.bias
    }
}

// ============================================================================
// MAML LEARNER
// ============================================================================

/// MAML (Model-Agnostic Meta-Learning) implementation
pub struct MAMLLearner {
    /// Configuration
    config: MetaLearnerConfig,
    /// Meta-parameters (initial weights)
    meta_weights: Vec<f64>,
    /// Meta bias
    meta_bias: f64,
    /// Meta-learning rate (learned)
    learned_inner_lr: Vec<f64>,
    /// Meta-training iterations
    meta_iterations: u64,
}

impl MAMLLearner {
    /// Create new MAML learner
    pub fn new(config: MetaLearnerConfig) -> Self {
        let dim = config.feature_dim;

        // Initialize meta-parameters with small random values
        let meta_weights = (0..dim)
            .map(|i| ((i * 17 + 31) % 100) as f64 / 1000.0 - 0.05)
            .collect();

        let learned_inner_lr = vec![config.inner_lr; dim];

        Self {
            config,
            meta_weights,
            meta_bias: 0.0,
            learned_inner_lr,
            meta_iterations: 0,
        }
    }

    /// Inner loop: adapt to task
    fn inner_loop(&self, support: &[StreamingSample]) -> AdaptedModel {
        let mut weights = self.meta_weights.clone();
        let mut bias = self.meta_bias;
        let mut final_loss = 0.0;

        for _step in 0..self.config.inner_steps {
            let mut grad_w = vec![0.0; weights.len()];
            let mut grad_b = 0.0;
            let mut total_loss = 0.0;

            for sample in support {
                if let Some(label) = sample.label {
                    // Forward pass
                    let pred: f64 = weights
                        .iter()
                        .zip(sample.features.iter())
                        .map(|(w, f)| w * f)
                        .sum::<f64>()
                        + bias;

                    let error = pred - label;
                    let loss = error * error * 0.5;
                    total_loss += loss;

                    // Accumulate gradients
                    for (i, &f) in sample.features.iter().enumerate() {
                        if i < grad_w.len() {
                            grad_w[i] += error * f;
                        }
                    }
                    grad_b += error;
                }
            }

            // Average gradients
            let n = support.len().max(1) as f64;
            for g in &mut grad_w {
                *g /= n;
            }
            grad_b /= n;
            final_loss = total_loss / n;

            // Update weights (inner update)
            for (i, w) in weights.iter_mut().enumerate() {
                let lr = if i < self.learned_inner_lr.len() {
                    self.learned_inner_lr[i]
                } else {
                    self.config.inner_lr
                };
                *w -= lr * grad_w[i];
            }
            bias -= self.config.inner_lr * grad_b;
        }

        AdaptedModel {
            weights,
            bias,
            adaptation_steps: self.config.inner_steps,
            final_loss,
        }
    }

    /// Compute loss on query set
    fn query_loss(&self, adapted: &AdaptedModel, query: &[StreamingSample]) -> f64 {
        let mut total_loss = 0.0;

        for sample in query {
            if let Some(label) = sample.label {
                let pred = adapted.predict(&sample.features);
                let error = pred - label;
                total_loss += error * error * 0.5;
            }
        }

        total_loss / query.len().max(1) as f64
    }

    /// Compute gradients for meta-update
    fn compute_meta_gradients(&self, tasks: &[MetaTask]) -> (Vec<f64>, f64) {
        let mut meta_grad_w = vec![0.0; self.meta_weights.len()];
        let mut meta_grad_b = 0.0;

        for task in tasks {
            // Adapt to support set
            let adapted = self.inner_loop(&task.support);

            // Compute gradients on query set
            let mut task_grad_w = vec![0.0; self.meta_weights.len()];
            let mut task_grad_b = 0.0;

            for sample in &task.query {
                if let Some(label) = sample.label {
                    let pred = adapted.predict(&sample.features);
                    let error = pred - label;

                    for (i, &f) in sample.features.iter().enumerate() {
                        if i < task_grad_w.len() {
                            task_grad_w[i] += error * f;
                        }
                    }
                    task_grad_b += error;
                }
            }

            let n = task.query.len().max(1) as f64;
            for (i, g) in task_grad_w.iter().enumerate() {
                meta_grad_w[i] += g / n;
            }
            meta_grad_b += task_grad_b / n;
        }

        // Average across tasks
        let num_tasks = tasks.len().max(1) as f64;
        for g in &mut meta_grad_w {
            *g /= num_tasks;
        }
        meta_grad_b /= num_tasks;

        (meta_grad_w, meta_grad_b)
    }
}

impl MetaLearner for MAMLLearner {
    fn adapt(&mut self, support: &[StreamingSample]) -> AdaptedModel {
        self.inner_loop(support)
    }

    fn meta_update(&mut self, tasks: &[MetaTask]) {
        let (grad_w, grad_b) = self.compute_meta_gradients(tasks);

        // Meta-update
        for (w, g) in self.meta_weights.iter_mut().zip(grad_w.iter()) {
            *w -= self.config.outer_lr * g;
        }
        self.meta_bias -= self.config.outer_lr * grad_b;

        self.meta_iterations += 1;
    }

    fn meta_params(&self) -> &[f64] {
        &self.meta_weights
    }
}

// ============================================================================
// LEARNED LEARNING RATE
// ============================================================================

/// Meta-learned learning rate adapter
pub struct LearnedLRAdapter {
    /// Per-parameter learning rates
    learning_rates: Vec<f64>,
    /// Meta learning rate for LR updates
    meta_lr: f64,
    /// Feature dimension
    feature_dim: usize,
}

impl LearnedLRAdapter {
    /// Create new adapter
    pub fn new(feature_dim: usize, initial_lr: f64) -> Self {
        Self {
            learning_rates: vec![initial_lr; feature_dim],
            meta_lr: 0.001,
            feature_dim,
        }
    }

    /// Get learning rate for parameter
    #[inline]
    pub fn get_lr(&self, param_idx: usize) -> f64 {
        if param_idx < self.learning_rates.len() {
            self.learning_rates[param_idx]
        } else {
            0.01
        }
    }

    /// Update learning rates based on gradients
    pub fn update_lrs(&mut self, gradients: &[f64], meta_gradients: &[f64]) {
        for i in 0..self
            .learning_rates
            .len()
            .min(gradients.len())
            .min(meta_gradients.len())
        {
            // If gradient and meta-gradient have same sign, increase LR
            // If opposite signs, decrease LR
            let sign = gradients[i] * meta_gradients[i];
            if sign > 0.0 {
                self.learning_rates[i] *= 1.0 + self.meta_lr;
            } else {
                self.learning_rates[i] *= 1.0 - self.meta_lr;
            }

            // Clamp
            self.learning_rates[i] = self.learning_rates[i].clamp(0.0001, 1.0);
        }
    }

    /// Get all learning rates
    #[inline(always)]
    pub fn learning_rates(&self) -> &[f64] {
        &self.learning_rates
    }
}

// ============================================================================
// TASK EMBEDDING
// ============================================================================

/// Task embedding for task-aware learning
pub struct TaskEmbedding {
    /// Embedding dimension
    embed_dim: usize,
    /// Task embeddings
    embeddings: BTreeMap<TaskId, Vec<f64>>,
    /// Embedding matrix (for learning)
    embedding_matrix: Vec<Vec<f64>>,
    /// Number of tasks
    num_tasks: usize,
}

impl TaskEmbedding {
    /// Create new task embedding
    pub fn new(num_tasks: usize, embed_dim: usize) -> Self {
        // Initialize random embeddings
        let embedding_matrix: Vec<Vec<f64>> = (0..num_tasks)
            .map(|t| {
                (0..embed_dim)
                    .map(|e| {
                        let seed = (t * 31 + e * 17) as u64;
                        let rng = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                        (rng >> 32) as f64 / u32::MAX as f64 * 0.2 - 0.1
                    })
                    .collect()
            })
            .collect();

        Self {
            embed_dim,
            embeddings: BTreeMap::new(),
            embedding_matrix,
            num_tasks,
        }
    }

    /// Get task embedding
    pub fn get_embedding(&self, task_id: TaskId) -> Vec<f64> {
        if let Some(emb) = self.embeddings.get(&task_id) {
            emb.clone()
        } else {
            let idx = task_id.0 as usize % self.num_tasks;
            if idx < self.embedding_matrix.len() {
                self.embedding_matrix[idx].clone()
            } else {
                vec![0.0; self.embed_dim]
            }
        }
    }

    /// Update task embedding
    #[inline]
    pub fn update_embedding(&mut self, task_id: TaskId, gradient: &[f64], lr: f64) {
        let embedding = self
            .embeddings
            .entry(task_id)
            .or_insert_with(|| vec![0.0; self.embed_dim]);

        for (e, g) in embedding.iter_mut().zip(gradient.iter()) {
            *e -= lr * g;
        }
    }

    /// Compute similarity between tasks
    pub fn task_similarity(&self, task1: TaskId, task2: TaskId) -> f64 {
        let emb1 = self.get_embedding(task1);
        let emb2 = self.get_embedding(task2);

        // Cosine similarity
        let dot: f64 = emb1.iter().zip(emb2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f64 = emb1.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm2: f64 = emb2.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm1 > 0.0 && norm2 > 0.0 {
            dot / (norm1 * norm2)
        } else {
            0.0
        }
    }
}

// ============================================================================
// FEW-SHOT LEARNER
// ============================================================================

/// Few-shot learning wrapper
pub struct FewShotLearner {
    /// MAML backend
    maml: MAMLLearner,
    /// Task distribution
    tasks: TaskDistribution,
    /// Current adapted model
    current_model: Option<AdaptedModel>,
    /// Current task ID
    current_task: Option<TaskId>,
}

impl FewShotLearner {
    /// Create new few-shot learner
    pub fn new(config: MetaLearnerConfig) -> Self {
        Self {
            maml: MAMLLearner::new(config),
            tasks: TaskDistribution::new(),
            current_model: None,
            current_task: None,
        }
    }

    /// Register a new task
    #[inline]
    pub fn register_task(&mut self, name: String, support: Vec<StreamingSample>) -> TaskId {
        let id = self.tasks.create_task(name);

        if let Some(task) = self.tasks.get_task_mut(id) {
            for sample in support {
                task.add_support(sample);
            }
        }

        id
    }

    /// Adapt to task
    #[inline]
    pub fn adapt_to_task(&mut self, task_id: TaskId) -> Option<f64> {
        let task = self.tasks.get_task(task_id)?;
        let adapted = self.maml.adapt(&task.support);
        let loss = adapted.final_loss;

        self.current_model = Some(adapted);
        self.current_task = Some(task_id);

        Some(loss)
    }

    /// Predict using current adapted model
    #[inline(always)]
    pub fn predict(&self, features: &[f64]) -> Option<f64> {
        self.current_model.as_ref().map(|m| m.predict(features))
    }

    /// Meta-train on all registered tasks
    pub fn meta_train(&mut self, iterations: usize) {
        for _ in 0..iterations {
            let task_ids = self.tasks.task_ids();
            let tasks: Vec<MetaTask> = task_ids
                .iter()
                .filter_map(|id| self.tasks.get_task(*id).cloned())
                .collect();

            if !tasks.is_empty() {
                self.maml.meta_update(&tasks);
            }
        }
    }

    /// Get task count
    #[inline(always)]
    pub fn task_count(&self) -> usize {
        self.tasks.task_count()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(features: Vec<f64>, label: f64) -> StreamingSample {
        StreamingSample::new(features, Some(label))
    }

    #[test]
    fn test_meta_task() {
        let mut task = MetaTask::new(TaskId(0), String::from("test"));

        task.add_support(make_sample(vec![1.0, 0.0], 1.0));
        task.add_query(make_sample(vec![0.0, 1.0], 0.0));

        assert_eq!(task.support_size(), 1);
        assert_eq!(task.query_size(), 1);
    }

    #[test]
    fn test_task_distribution() {
        let mut dist = TaskDistribution::new();

        let id1 = dist.create_task(String::from("task1"));
        let id2 = dist.create_task(String::from("task2"));

        assert_eq!(dist.task_count(), 2);
        assert!(dist.get_task(id1).is_some());
        assert!(dist.get_task(id2).is_some());

        let sampled = dist.sample_task(42);
        assert!(sampled.is_some());
    }

    #[test]
    fn test_maml_adapt() {
        let config = MetaLearnerConfig {
            feature_dim: 2,
            ..Default::default()
        };
        let mut maml = MAMLLearner::new(config);

        // Simple linear task: y = x1 + x2
        let support = vec![
            make_sample(vec![1.0, 0.0], 1.0),
            make_sample(vec![0.0, 1.0], 1.0),
            make_sample(vec![1.0, 1.0], 2.0),
        ];

        let adapted = maml.adapt(&support);

        assert_eq!(adapted.adaptation_steps, 5);

        // Should predict reasonably after adaptation
        let pred = adapted.predict(&[0.5, 0.5]);
        // Not exact, but should be in right direction
        assert!(pred > 0.0);
    }

    #[test]
    fn test_few_shot_learner() {
        let config = MetaLearnerConfig {
            feature_dim: 2,
            ..Default::default()
        };
        let mut learner = FewShotLearner::new(config);

        // Register task
        let support = vec![
            make_sample(vec![1.0, 0.0], 1.0),
            make_sample(vec![0.0, 1.0], 1.0),
        ];
        let task_id = learner.register_task(String::from("linear"), support);

        // Adapt
        let loss = learner.adapt_to_task(task_id);
        assert!(loss.is_some());

        // Predict
        let pred = learner.predict(&[0.5, 0.5]);
        assert!(pred.is_some());
    }

    #[test]
    fn test_task_embedding() {
        let embedding = TaskEmbedding::new(10, 8);

        let emb1 = embedding.get_embedding(TaskId(0));
        let emb2 = embedding.get_embedding(TaskId(1));

        assert_eq!(emb1.len(), 8);
        assert_eq!(emb2.len(), 8);

        let sim = embedding.task_similarity(TaskId(0), TaskId(0));
        assert!((sim - 1.0).abs() < 0.01); // Self-similarity should be ~1
    }

    #[test]
    fn test_learned_lr_adapter() {
        let mut adapter = LearnedLRAdapter::new(4, 0.1);

        assert!((adapter.get_lr(0) - 0.1).abs() < 0.001);

        // Gradients with same sign -> increase LR
        let gradients = vec![1.0, 1.0, -1.0, -1.0];
        let meta_gradients = vec![1.0, -1.0, -1.0, 1.0];

        let old_lr = adapter.get_lr(0);
        adapter.update_lrs(&gradients, &meta_gradients);

        // First param: same sign -> increased
        assert!(adapter.get_lr(0) > old_lr);
        // Second param: opposite sign -> decreased
        assert!(adapter.get_lr(1) < 0.1);
    }
}
