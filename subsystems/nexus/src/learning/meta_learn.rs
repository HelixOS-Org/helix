//! # Meta Learning
//!
//! Implements learning about learning itself.
//! Optimizes learning strategies based on outcomes.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::math::F64Ext;
use crate::types::Timestamp;

// ============================================================================
// META LEARNING TYPES
// ============================================================================

/// Learning strategy
#[derive(Debug, Clone)]
pub struct LearningStrategy {
    /// Strategy ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub strategy_type: StrategyType,
    /// Parameters
    pub params: BTreeMap<String, f64>,
    /// Success rate
    pub success_rate: f64,
    /// Use count
    pub use_count: u64,
    /// Created
    pub created: Timestamp,
}

/// Strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyType {
    Supervised,
    Unsupervised,
    Reinforcement,
    Imitation,
    Transfer,
    Active,
    MetaGradient,
}

/// Learning episode
#[derive(Debug, Clone)]
pub struct LearningEpisode {
    /// Episode ID
    pub id: u64,
    /// Strategy used
    pub strategy: u64,
    /// Task type
    pub task_type: String,
    /// Outcome
    pub outcome: LearningOutcome,
    /// Duration ns
    pub duration_ns: u64,
    /// Metrics
    pub metrics: EpisodeMetrics,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Learning outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearningOutcome {
    Success,
    PartialSuccess,
    Failure,
    Timeout,
}

/// Episode metrics
#[derive(Debug, Clone, Default)]
pub struct EpisodeMetrics {
    /// Error rate
    pub error_rate: f64,
    /// Convergence speed
    pub convergence_speed: f64,
    /// Generalization score
    pub generalization: f64,
    /// Sample efficiency
    pub sample_efficiency: f64,
}

/// Strategy adaptation
#[derive(Debug, Clone)]
pub struct StrategyAdaptation {
    /// Adaptation ID
    pub id: u64,
    /// Strategy ID
    pub strategy: u64,
    /// Parameter changes
    pub changes: BTreeMap<String, f64>,
    /// Reason
    pub reason: String,
    /// Applied
    pub applied: Timestamp,
}

/// Task model
#[derive(Debug, Clone)]
pub struct TaskModel {
    /// Task type
    pub task_type: String,
    /// Best strategy
    pub best_strategy: Option<u64>,
    /// Strategy performance
    pub strategy_performance: BTreeMap<u64, f64>,
    /// Difficulty
    pub difficulty: f64,
    /// Samples seen
    pub samples_seen: u64,
}

// ============================================================================
// META LEARNER
// ============================================================================

/// Meta learner
pub struct MetaLearner {
    /// Strategies
    strategies: BTreeMap<u64, LearningStrategy>,
    /// Episodes
    episodes: Vec<LearningEpisode>,
    /// Adaptations
    adaptations: Vec<StrategyAdaptation>,
    /// Task models
    tasks: BTreeMap<String, TaskModel>,
    /// Active strategy
    active: Option<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: MetaConfig,
    /// Statistics
    stats: MetaStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MetaConfig {
    /// Exploration rate
    pub exploration_rate: f64,
    /// Adaptation threshold
    pub adaptation_threshold: f64,
    /// History window
    pub history_window: usize,
    /// Min episodes for adaptation
    pub min_episodes: usize,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            exploration_rate: 0.2,
            adaptation_threshold: 0.6,
            history_window: 50,
            min_episodes: 10,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct MetaStats {
    /// Strategies created
    pub strategies_created: u64,
    /// Episodes recorded
    pub episodes_recorded: u64,
    /// Adaptations made
    pub adaptations_made: u64,
    /// Strategy switches
    pub strategy_switches: u64,
}

impl MetaLearner {
    /// Create new learner
    pub fn new(config: MetaConfig) -> Self {
        Self {
            strategies: BTreeMap::new(),
            episodes: Vec::new(),
            adaptations: Vec::new(),
            tasks: BTreeMap::new(),
            active: None,
            next_id: AtomicU64::new(1),
            config,
            stats: MetaStats::default(),
        }
    }

    /// Register strategy
    pub fn register_strategy(
        &mut self,
        name: &str,
        strategy_type: StrategyType,
        params: BTreeMap<String, f64>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let strategy = LearningStrategy {
            id,
            name: name.into(),
            strategy_type,
            params,
            success_rate: 0.5,
            use_count: 0,
            created: Timestamp::now(),
        };

        self.strategies.insert(id, strategy);
        self.stats.strategies_created += 1;

        id
    }

    /// Select strategy for task
    pub fn select_strategy(&mut self, task_type: &str) -> Option<u64> {
        // Ensure task model exists
        if !self.tasks.contains_key(task_type) {
            self.tasks.insert(task_type.into(), TaskModel {
                task_type: task_type.into(),
                best_strategy: None,
                strategy_performance: BTreeMap::new(),
                difficulty: 0.5,
                samples_seen: 0,
            });
        }

        let task = self.tasks.get(task_type)?;

        // Explore vs exploit
        let explore = self.should_explore(task);

        let selected = if explore {
            // Random unexplored strategy
            self.select_exploratory(task_type)
        } else {
            // Best performing
            task.best_strategy
        };

        if selected.is_some() {
            self.active = selected;

            if let Some(s) = self.strategies.get_mut(&selected.unwrap()) {
                s.use_count += 1;
            }
        }

        selected
    }

    fn should_explore(&self, task: &TaskModel) -> bool {
        if task.samples_seen < self.config.min_episodes as u64 {
            return true;
        }

        // UCB-like exploration
        let explore_score = self.config.exploration_rate
            * ((task.samples_seen as f64).ln() / task.strategy_performance.len().max(1) as f64)
                .sqrt();

        explore_score > 0.5
    }

    fn select_exploratory(&self, task_type: &str) -> Option<u64> {
        let task = self.tasks.get(task_type)?;

        // Find least used strategy
        self.strategies
            .values()
            .filter(|s| !task.strategy_performance.contains_key(&s.id))
            .min_by_key(|s| s.use_count)
            .map(|s| s.id)
            .or_else(|| task.best_strategy)
    }

    /// Record episode
    pub fn record_episode(
        &mut self,
        strategy: u64,
        task_type: &str,
        outcome: LearningOutcome,
        duration_ns: u64,
        metrics: EpisodeMetrics,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let episode = LearningEpisode {
            id,
            strategy,
            task_type: task_type.into(),
            outcome,
            duration_ns,
            metrics: metrics.clone(),
            timestamp: Timestamp::now(),
        };

        self.episodes.push(episode);
        self.stats.episodes_recorded += 1;

        // Update strategy success rate
        self.update_strategy_stats(strategy, outcome);

        // Update task model
        self.update_task_model(task_type, strategy, &metrics);

        // Check for adaptation
        self.check_adaptation(strategy);

        id
    }

    fn update_strategy_stats(&mut self, strategy_id: u64, outcome: LearningOutcome) {
        if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
            let success = match outcome {
                LearningOutcome::Success => 1.0,
                LearningOutcome::PartialSuccess => 0.5,
                _ => 0.0,
            };

            // Exponential moving average
            let alpha = 0.1;
            strategy.success_rate = alpha * success + (1.0 - alpha) * strategy.success_rate;
        }
    }

    fn update_task_model(&mut self, task_type: &str, strategy: u64, metrics: &EpisodeMetrics) {
        if let Some(task) = self.tasks.get_mut(task_type) {
            task.samples_seen += 1;

            // Update strategy performance
            let score = (1.0 - metrics.error_rate) * 0.4
                + metrics.convergence_speed * 0.3
                + metrics.generalization * 0.3;

            let current = task.strategy_performance.entry(strategy).or_insert(0.5);
            *current = 0.9 * *current + 0.1 * score;

            // Update best strategy
            let best = task
                .strategy_performance
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
                .map(|(&id, _)| id);

            if best != task.best_strategy {
                task.best_strategy = best;
                self.stats.strategy_switches += 1;
            }

            // Update difficulty
            task.difficulty = 1.0 - metrics.error_rate.clamp(0.0, 1.0);
        }
    }

    fn check_adaptation(&mut self, strategy_id: u64) {
        let recent: Vec<_> = self
            .episodes
            .iter()
            .rev()
            .take(self.config.history_window)
            .filter(|e| e.strategy == strategy_id)
            .collect();

        if recent.len() < self.config.min_episodes {
            return;
        }

        // Calculate average error rate
        let avg_error: f64 =
            recent.iter().map(|e| e.metrics.error_rate).sum::<f64>() / recent.len() as f64;

        // If underperforming, adapt
        if avg_error > self.config.adaptation_threshold {
            self.adapt_strategy(strategy_id, avg_error);
        }
    }

    fn adapt_strategy(&mut self, strategy_id: u64, error_rate: f64) {
        let strategy = match self.strategies.get(&strategy_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Determine adaptation
        let mut changes = BTreeMap::new();

        // Reduce learning rate if oscillating
        if let Some(&lr) = strategy.params.get("learning_rate") {
            changes.insert("learning_rate".into(), lr * 0.8);
        }

        // Increase regularization if overfitting
        if error_rate > 0.7 {
            if let Some(&reg) = strategy.params.get("regularization") {
                changes.insert("regularization".into(), reg * 1.2);
            }
        }

        if changes.is_empty() {
            return;
        }

        let adaptation = StrategyAdaptation {
            id,
            strategy: strategy_id,
            changes: changes.clone(),
            reason: format!("High error rate: {:.2}", error_rate),
            applied: Timestamp::now(),
        };

        // Apply changes
        if let Some(s) = self.strategies.get_mut(&strategy_id) {
            for (key, value) in changes {
                s.params.insert(key, value);
            }
        }

        self.adaptations.push(adaptation);
        self.stats.adaptations_made += 1;
    }

    /// Get strategy
    pub fn get_strategy(&self, id: u64) -> Option<&LearningStrategy> {
        self.strategies.get(&id)
    }

    /// Get active strategy
    pub fn active(&self) -> Option<&LearningStrategy> {
        self.active.and_then(|id| self.strategies.get(&id))
    }

    /// Get task model
    pub fn task(&self, task_type: &str) -> Option<&TaskModel> {
        self.tasks.get(task_type)
    }

    /// Get all strategies
    pub fn strategies(&self) -> Vec<&LearningStrategy> {
        self.strategies.values().collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &MetaStats {
        &self.stats
    }
}

impl Default for MetaLearner {
    fn default() -> Self {
        Self::new(MetaConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_strategy() {
        let mut learner = MetaLearner::default();

        let id = learner.register_strategy("test", StrategyType::Supervised, BTreeMap::new());

        assert!(learner.get_strategy(id).is_some());
    }

    #[test]
    fn test_select_strategy() {
        let mut learner = MetaLearner::default();

        let id = learner.register_strategy("default", StrategyType::Supervised, BTreeMap::new());

        let selected = learner.select_strategy("classification");
        assert!(selected.is_some());
    }

    #[test]
    fn test_record_episode() {
        let mut learner = MetaLearner::default();

        let strategy =
            learner.register_strategy("test", StrategyType::Reinforcement, BTreeMap::new());

        learner.record_episode(
            strategy,
            "task",
            LearningOutcome::Success,
            1000,
            EpisodeMetrics {
                error_rate: 0.1,
                convergence_speed: 0.8,
                generalization: 0.7,
                sample_efficiency: 0.6,
            },
        );

        assert_eq!(learner.episodes.len(), 1);
    }

    #[test]
    fn test_strategy_update() {
        let mut learner = MetaLearner::default();

        let strategy = learner.register_strategy("test", StrategyType::Supervised, BTreeMap::new());

        // Record multiple successes
        for _ in 0..5 {
            learner.record_episode(
                strategy,
                "task",
                LearningOutcome::Success,
                1000,
                EpisodeMetrics::default(),
            );
        }

        let s = learner.get_strategy(strategy).unwrap();
        assert!(s.success_rate > 0.5);
    }

    #[test]
    fn test_task_model() {
        let mut learner = MetaLearner::default();

        let s1 = learner.register_strategy("s1", StrategyType::Supervised, BTreeMap::new());
        let s2 = learner.register_strategy("s2", StrategyType::Reinforcement, BTreeMap::new());

        // s1 performs better
        learner.record_episode(s1, "task", LearningOutcome::Success, 1000, EpisodeMetrics {
            error_rate: 0.1,
            ..Default::default()
        });
        learner.record_episode(s2, "task", LearningOutcome::Failure, 1000, EpisodeMetrics {
            error_rate: 0.8,
            ..Default::default()
        });

        let task = learner.task("task").unwrap();
        assert_eq!(task.strategy_performance.len(), 2);
    }
}
