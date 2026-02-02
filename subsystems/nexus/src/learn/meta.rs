//! # Meta-Learning
//!
//! Learning how to learn more effectively.
//! Optimizes learning strategies based on experience.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// META-LEARNING TYPES
// ============================================================================

/// Learning task
#[derive(Debug, Clone)]
pub struct LearningTask {
    /// Task ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Task type
    pub task_type: TaskType,
    /// Features
    pub features: TaskFeatures,
    /// History
    pub history: Vec<LearningAttempt>,
    /// Best strategy
    pub best_strategy: Option<u64>,
}

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Classification,
    Regression,
    Clustering,
    Reinforcement,
    Sequence,
    Anomaly,
    Optimization,
}

/// Task features
#[derive(Debug, Clone, Default)]
pub struct TaskFeatures {
    /// Data size
    pub data_size: usize,
    /// Feature count
    pub feature_count: usize,
    /// Noise level
    pub noise_level: f64,
    /// Complexity estimate
    pub complexity: f64,
    /// Is time-sensitive
    pub time_sensitive: bool,
    /// Custom features
    pub custom: BTreeMap<String, f64>,
}

/// Learning attempt
#[derive(Debug, Clone)]
pub struct LearningAttempt {
    /// Attempt ID
    pub id: u64,
    /// Strategy used
    pub strategy_id: u64,
    /// Started
    pub started: Timestamp,
    /// Completed
    pub completed: Option<Timestamp>,
    /// Metrics
    pub metrics: LearningMetrics,
    /// Success
    pub success: bool,
}

/// Learning metrics
#[derive(Debug, Clone, Default)]
pub struct LearningMetrics {
    /// Accuracy
    pub accuracy: f64,
    /// Training time (ns)
    pub training_time_ns: u64,
    /// Convergence rate
    pub convergence_rate: f64,
    /// Generalization gap
    pub generalization_gap: f64,
    /// Resource usage
    pub resource_usage: f64,
}

/// Learning strategy
#[derive(Debug, Clone)]
pub struct LearningStrategy {
    /// Strategy ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Algorithm
    pub algorithm: Algorithm,
    /// Hyperparameters
    pub hyperparams: BTreeMap<String, HyperparamValue>,
    /// Applicable task types
    pub applicable_tasks: Vec<TaskType>,
    /// Performance history
    pub performance: StrategyPerformance,
}

/// Algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    GradientDescent,
    RandomForest,
    NeuralNetwork,
    Bayesian,
    Genetic,
    Ensemble,
    TransferLearning,
    FewShot,
}

/// Hyperparameter value
#[derive(Debug, Clone)]
pub enum HyperparamValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Choice(String),
}

/// Strategy performance
#[derive(Debug, Clone, Default)]
pub struct StrategyPerformance {
    /// Uses
    pub uses: u64,
    /// Successes
    pub successes: u64,
    /// Average accuracy
    pub avg_accuracy: f64,
    /// Average time (ns)
    pub avg_time_ns: f64,
    /// By task type
    pub by_task_type: BTreeMap<String, f64>,
}

// ============================================================================
// META-LEARNER
// ============================================================================

/// Meta-learner
pub struct MetaLearner {
    /// Learning tasks
    tasks: BTreeMap<u64, LearningTask>,
    /// Strategies
    strategies: BTreeMap<u64, LearningStrategy>,
    /// Task-strategy mapping (task features hash -> strategy)
    task_strategy_map: BTreeMap<u64, u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: MetaLearnerConfig,
    /// Statistics
    stats: MetaLearnerStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MetaLearnerConfig {
    /// Exploration rate
    pub exploration_rate: f64,
    /// Min attempts before recommending
    pub min_attempts: usize,
    /// Enable transfer
    pub enable_transfer: bool,
}

impl Default for MetaLearnerConfig {
    fn default() -> Self {
        Self {
            exploration_rate: 0.1,
            min_attempts: 3,
            enable_transfer: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct MetaLearnerStats {
    /// Tasks created
    pub tasks_created: u64,
    /// Strategies evaluated
    pub strategies_evaluated: u64,
    /// Successful transfers
    pub successful_transfers: u64,
}

impl MetaLearner {
    /// Create new meta-learner
    pub fn new(config: MetaLearnerConfig) -> Self {
        Self {
            tasks: BTreeMap::new(),
            strategies: BTreeMap::new(),
            task_strategy_map: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: MetaLearnerStats::default(),
        }
    }

    /// Register strategy
    pub fn register_strategy(&mut self, strategy: LearningStrategy) -> u64 {
        let id = strategy.id;
        self.strategies.insert(id, strategy);
        id
    }

    /// Create task
    pub fn create_task(&mut self, name: &str, task_type: TaskType, features: TaskFeatures) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let task = LearningTask {
            id,
            name: name.into(),
            task_type,
            features,
            history: Vec::new(),
            best_strategy: None,
        };

        self.tasks.insert(id, task);
        self.stats.tasks_created += 1;

        id
    }

    /// Recommend strategy
    pub fn recommend_strategy(&self, task_id: u64) -> Option<u64> {
        let task = self.tasks.get(&task_id)?;

        // Check if we have enough history
        if task.best_strategy.is_some() && task.history.len() >= self.config.min_attempts {
            return task.best_strategy;
        }

        // Try to find similar task
        if self.config.enable_transfer {
            if let Some(similar_strategy) = self.find_similar_task_strategy(task) {
                return Some(similar_strategy);
            }
        }

        // Find best strategy for task type
        self.find_best_strategy_for_type(task.task_type)
    }

    fn find_similar_task_strategy(&self, task: &LearningTask) -> Option<u64> {
        let feature_hash = self.hash_features(&task.features);

        if let Some(&strategy_id) = self.task_strategy_map.get(&feature_hash) {
            return Some(strategy_id);
        }

        // Find task with similar features
        for other_task in self.tasks.values() {
            if other_task.id != task.id
                && other_task.task_type == task.task_type
                && other_task.best_strategy.is_some()
            {
                let similarity = self.compute_similarity(&task.features, &other_task.features);
                if similarity > 0.8 {
                    return other_task.best_strategy;
                }
            }
        }

        None
    }

    fn hash_features(&self, features: &TaskFeatures) -> u64 {
        let mut hash = features.data_size as u64;
        hash ^= features.feature_count as u64;
        hash ^= (features.complexity * 1000.0) as u64;
        hash
    }

    fn compute_similarity(&self, a: &TaskFeatures, b: &TaskFeatures) -> f64 {
        let size_sim = 1.0
            - ((a.data_size as f64 - b.data_size as f64).abs()
                / (a.data_size.max(b.data_size) as f64 + 1.0));
        let feat_sim = 1.0
            - ((a.feature_count as f64 - b.feature_count as f64).abs()
                / (a.feature_count.max(b.feature_count) as f64 + 1.0));
        let comp_sim = 1.0 - (a.complexity - b.complexity).abs();

        (size_sim + feat_sim + comp_sim) / 3.0
    }

    fn find_best_strategy_for_type(&self, task_type: TaskType) -> Option<u64> {
        let type_key = format!("{:?}", task_type);

        self.strategies
            .values()
            .filter(|s| s.applicable_tasks.contains(&task_type))
            .max_by(|a, b| {
                let a_score = a.performance.by_task_type.get(&type_key).unwrap_or(&0.0);
                let b_score = b.performance.by_task_type.get(&type_key).unwrap_or(&0.0);
                a_score.partial_cmp(b_score).unwrap()
            })
            .map(|s| s.id)
    }

    /// Record attempt
    pub fn record_attempt(
        &mut self,
        task_id: u64,
        strategy_id: u64,
        metrics: LearningMetrics,
        success: bool,
    ) {
        let attempt_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Update task
        if let Some(task) = self.tasks.get_mut(&task_id) {
            let attempt = LearningAttempt {
                id: attempt_id,
                strategy_id,
                started: Timestamp(0),
                completed: Some(Timestamp::now()),
                metrics: metrics.clone(),
                success,
            };

            task.history.push(attempt);

            // Update best strategy
            self.update_best_strategy(task);
        }

        // Update strategy performance
        if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
            strategy.performance.uses += 1;
            if success {
                strategy.performance.successes += 1;
            }

            let n = strategy.performance.uses as f64;
            strategy.performance.avg_accuracy =
                (strategy.performance.avg_accuracy * (n - 1.0) + metrics.accuracy) / n;
            strategy.performance.avg_time_ns = (strategy.performance.avg_time_ns * (n - 1.0)
                + metrics.training_time_ns as f64)
                / n;
        }

        self.stats.strategies_evaluated += 1;
    }

    fn update_best_strategy(&mut self, task: &mut LearningTask) {
        if task.history.len() < self.config.min_attempts {
            return;
        }

        // Group by strategy
        let mut strategy_scores: BTreeMap<u64, (f64, usize)> = BTreeMap::new();

        for attempt in &task.history {
            let entry = strategy_scores
                .entry(attempt.strategy_id)
                .or_insert((0.0, 0));
            entry.0 += attempt.metrics.accuracy;
            entry.1 += 1;
        }

        // Find best
        let best = strategy_scores
            .iter()
            .map(|(&id, (sum, count))| (id, sum / *count as f64))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id);

        if task.best_strategy != best {
            task.best_strategy = best;

            // Cache for transfer
            if let Some(strategy_id) = best {
                let feature_hash = self.hash_features(&task.features);
                self.task_strategy_map.insert(feature_hash, strategy_id);
            }
        }
    }

    /// Optimize hyperparameters
    pub fn optimize_hyperparams(
        &mut self,
        strategy_id: u64,
        param_name: &str,
        values: &[HyperparamValue],
        evaluate: impl Fn(&BTreeMap<String, HyperparamValue>) -> f64,
    ) -> Option<HyperparamValue> {
        let strategy = self.strategies.get_mut(&strategy_id)?;

        let mut best_value = None;
        let mut best_score = f64::NEG_INFINITY;

        for value in values {
            let mut test_params = strategy.hyperparams.clone();
            test_params.insert(param_name.into(), value.clone());

            let score = evaluate(&test_params);

            if score > best_score {
                best_score = score;
                best_value = Some(value.clone());
            }
        }

        // Apply best
        if let Some(ref value) = best_value {
            strategy
                .hyperparams
                .insert(param_name.into(), value.clone());
        }

        best_value
    }

    /// Get task
    pub fn get_task(&self, id: u64) -> Option<&LearningTask> {
        self.tasks.get(&id)
    }

    /// Get strategy
    pub fn get_strategy(&self, id: u64) -> Option<&LearningStrategy> {
        self.strategies.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &MetaLearnerStats {
        &self.stats
    }
}

impl Default for MetaLearner {
    fn default() -> Self {
        Self::new(MetaLearnerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_strategy(id: u64, name: &str) -> LearningStrategy {
        LearningStrategy {
            id,
            name: name.into(),
            algorithm: Algorithm::GradientDescent,
            hyperparams: BTreeMap::new(),
            applicable_tasks: vec![TaskType::Classification],
            performance: StrategyPerformance::default(),
        }
    }

    #[test]
    fn test_create_task() {
        let mut learner = MetaLearner::default();

        let id = learner.create_task("test", TaskType::Classification, TaskFeatures::default());

        assert!(learner.get_task(id).is_some());
    }

    #[test]
    fn test_register_strategy() {
        let mut learner = MetaLearner::default();

        let strategy = create_test_strategy(1, "test_strategy");
        learner.register_strategy(strategy);

        assert!(learner.get_strategy(1).is_some());
    }

    #[test]
    fn test_recommend_strategy() {
        let mut learner = MetaLearner::default();

        let strategy = create_test_strategy(1, "classifier");
        learner.register_strategy(strategy);

        let task_id = learner.create_task(
            "classify",
            TaskType::Classification,
            TaskFeatures::default(),
        );

        let recommended = learner.recommend_strategy(task_id);
        assert!(recommended.is_some());
    }

    #[test]
    fn test_record_attempt() {
        let mut learner = MetaLearner::default();

        let strategy = create_test_strategy(1, "classifier");
        learner.register_strategy(strategy);

        let task_id = learner.create_task(
            "classify",
            TaskType::Classification,
            TaskFeatures::default(),
        );

        learner.record_attempt(
            task_id,
            1,
            LearningMetrics {
                accuracy: 0.95,
                ..Default::default()
            },
            true,
        );

        let task = learner.get_task(task_id).unwrap();
        assert_eq!(task.history.len(), 1);
    }

    #[test]
    fn test_best_strategy_selection() {
        let mut config = MetaLearnerConfig::default();
        config.min_attempts = 2;

        let mut learner = MetaLearner::new(config);

        learner.register_strategy(create_test_strategy(1, "s1"));
        learner.register_strategy(create_test_strategy(2, "s2"));

        let task_id =
            learner.create_task("test", TaskType::Classification, TaskFeatures::default());

        // Record attempts
        learner.record_attempt(
            task_id,
            1,
            LearningMetrics {
                accuracy: 0.80,
                ..Default::default()
            },
            true,
        );
        learner.record_attempt(
            task_id,
            2,
            LearningMetrics {
                accuracy: 0.90,
                ..Default::default()
            },
            true,
        );
        learner.record_attempt(
            task_id,
            2,
            LearningMetrics {
                accuracy: 0.92,
                ..Default::default()
            },
            true,
        );

        let task = learner.get_task(task_id).unwrap();
        assert_eq!(task.best_strategy, Some(2));
    }
}
