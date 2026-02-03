//! # Metacognition Strategy Engine for NEXUS
//!
//! Year 2 "COGNITION" - Adaptive metacognitive strategies that enable
//! the kernel AI to dynamically adjust its cognitive approach based on
//! task requirements, resource availability, and performance feedback.
//!
//! ## Features
//!
//! - Adaptive strategy selection
//! - Resource-aware cognition
//! - Performance-based learning
//! - Multi-strategy optimization
//! - Cognitive resource budgeting
//! - Strategy composition

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum strategies
const MAX_STRATEGIES: usize = 100;

/// Exploration rate for strategy selection
const DEFAULT_EXPLORATION_RATE: f64 = 0.1;

/// Learning rate for strategy performance
const DEFAULT_LEARNING_RATE: f64 = 0.1;

/// Minimum samples before trusting strategy performance
const MIN_SAMPLES: u64 = 10;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Strategy identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrategyId(pub u32);

/// Task type for strategy selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskType {
    /// Classification task
    Classification,
    /// Regression/prediction
    Regression,
    /// Anomaly detection
    AnomalyDetection,
    /// Pattern matching
    PatternMatching,
    /// Scheduling decision
    Scheduling,
    /// Resource allocation
    ResourceAllocation,
    /// Security analysis
    Security,
    /// Optimization
    Optimization,
    /// Planning
    Planning,
    /// Diagnosis
    Diagnosis,
    /// Learning
    Learning,
    /// Inference
    Inference,
    /// Generic
    Generic,
}

/// Resource budget for a cognitive task
#[derive(Debug, Clone)]
pub struct ResourceBudget {
    /// Maximum CPU cycles
    pub max_cycles: u64,
    /// Maximum memory (bytes)
    pub max_memory: u64,
    /// Maximum time (microseconds)
    pub max_time: u64,
    /// Minimum acceptable quality
    pub min_quality: f64,
    /// Priority (higher = more important)
    pub priority: u8,
}

impl ResourceBudget {
    /// Create a new budget
    pub fn new(max_cycles: u64, max_memory: u64, max_time: u64) -> Self {
        Self {
            max_cycles,
            max_memory,
            max_time,
            min_quality: 0.5,
            priority: 5,
        }
    }

    /// Create a tight budget
    pub fn tight() -> Self {
        Self {
            max_cycles: 10_000,
            max_memory: 1024,
            max_time: 100,
            min_quality: 0.3,
            priority: 5,
        }
    }

    /// Create a normal budget
    pub fn normal() -> Self {
        Self {
            max_cycles: 1_000_000,
            max_memory: 1024 * 1024,
            max_time: 10_000,
            min_quality: 0.7,
            priority: 5,
        }
    }

    /// Create a generous budget
    pub fn generous() -> Self {
        Self {
            max_cycles: 100_000_000,
            max_memory: 100 * 1024 * 1024,
            max_time: 1_000_000,
            min_quality: 0.9,
            priority: 5,
        }
    }

    /// Check if another budget fits within this one
    pub fn fits(&self, other: &ResourceBudget) -> bool {
        other.max_cycles <= self.max_cycles
            && other.max_memory <= self.max_memory
            && other.max_time <= self.max_time
    }
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self::normal()
    }
}

/// Cognitive strategy definition
#[derive(Debug, Clone)]
pub struct CognitiveStrategy {
    /// Strategy ID
    pub id: StrategyId,
    /// Strategy name
    pub name: String,
    /// Description
    pub description: String,
    /// Applicable task types
    pub applicable_tasks: Vec<TaskType>,
    /// Resource requirements
    pub resource_requirements: ResourceBudget,
    /// Expected quality (0-1)
    pub expected_quality: f64,
    /// Speed category (1-10, higher = faster)
    pub speed_rating: u8,
    /// Accuracy category (1-10, higher = more accurate)
    pub accuracy_rating: u8,
    /// Is this strategy adaptive?
    pub adaptive: bool,
    /// Can be composed with other strategies?
    pub composable: bool,
    /// Parameters
    pub parameters: BTreeMap<String, StrategyParameter>,
}

/// Strategy parameter
#[derive(Debug, Clone)]
pub struct StrategyParameter {
    /// Parameter name
    pub name: String,
    /// Current value
    pub value: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Default value
    pub default: f64,
    /// Is tunable?
    pub tunable: bool,
}

impl StrategyParameter {
    /// Create a new parameter
    pub fn new(name: &str, value: f64, min: f64, max: f64) -> Self {
        Self {
            name: String::from(name),
            value,
            min,
            max,
            default: value,
            tunable: true,
        }
    }

    /// Clamp value to valid range
    pub fn clamp(&mut self) {
        self.value = self.value.clamp(self.min, self.max);
    }
}

impl CognitiveStrategy {
    /// Create a new strategy
    pub fn new(id: StrategyId, name: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            applicable_tasks: Vec::new(),
            resource_requirements: ResourceBudget::normal(),
            expected_quality: 0.7,
            speed_rating: 5,
            accuracy_rating: 5,
            adaptive: false,
            composable: false,
            parameters: BTreeMap::new(),
        }
    }

    /// Check if applicable to task type
    pub fn is_applicable(&self, task_type: TaskType) -> bool {
        self.applicable_tasks.contains(&task_type)
            || self.applicable_tasks.contains(&TaskType::Generic)
    }

    /// Check if fits within budget
    pub fn fits_budget(&self, budget: &ResourceBudget) -> bool {
        budget.fits(&self.resource_requirements)
    }

    /// Get parameter value
    pub fn get_parameter(&self, name: &str) -> Option<f64> {
        self.parameters.get(name).map(|p| p.value)
    }

    /// Set parameter value
    pub fn set_parameter(&mut self, name: &str, value: f64) -> bool {
        if let Some(param) = self.parameters.get_mut(name) {
            param.value = value;
            param.clamp();
            true
        } else {
            false
        }
    }
}

// ============================================================================
// STRATEGY PERFORMANCE TRACKING
// ============================================================================

/// Performance statistics for a strategy
#[derive(Debug, Clone)]
pub struct StrategyPerformance {
    /// Strategy ID
    pub strategy_id: StrategyId,
    /// Number of times used
    pub usage_count: u64,
    /// Number of successes
    pub success_count: u64,
    /// Total reward accumulated
    pub total_reward: f64,
    /// Average reward
    pub avg_reward: f64,
    /// Average quality achieved
    pub avg_quality: f64,
    /// Average execution time
    pub avg_time: f64,
    /// Average CPU cycles
    pub avg_cycles: f64,
    /// Upper confidence bound
    pub ucb: f64,
    /// Performance per task type
    pub task_performance: BTreeMap<TaskType, TaskPerformance>,
}

/// Performance for a specific task type
#[derive(Debug, Clone)]
pub struct TaskPerformance {
    /// Usage count
    pub count: u64,
    /// Average reward
    pub avg_reward: f64,
    /// Average quality
    pub avg_quality: f64,
}

impl TaskPerformance {
    fn new() -> Self {
        Self {
            count: 0,
            avg_reward: 0.0,
            avg_quality: 0.0,
        }
    }

    fn update(&mut self, reward: f64, quality: f64, learning_rate: f64) {
        self.count += 1;
        self.avg_reward = self.avg_reward * (1.0 - learning_rate) + reward * learning_rate;
        self.avg_quality = self.avg_quality * (1.0 - learning_rate) + quality * learning_rate;
    }
}

impl StrategyPerformance {
    /// Create new performance tracker
    fn new(strategy_id: StrategyId) -> Self {
        Self {
            strategy_id,
            usage_count: 0,
            success_count: 0,
            total_reward: 0.0,
            avg_reward: 0.5, // Optimistic start
            avg_quality: 0.5,
            avg_time: 0.0,
            avg_cycles: 0.0,
            ucb: f64::MAX, // Start with high UCB for exploration
            task_performance: BTreeMap::new(),
        }
    }

    /// Update with execution result
    fn update(
        &mut self,
        task_type: TaskType,
        reward: f64,
        quality: f64,
        time: f64,
        cycles: f64,
        success: bool,
        learning_rate: f64,
    ) {
        self.usage_count += 1;
        if success {
            self.success_count += 1;
        }
        self.total_reward += reward;

        // Update averages with EMA
        self.avg_reward = self.avg_reward * (1.0 - learning_rate) + reward * learning_rate;
        self.avg_quality = self.avg_quality * (1.0 - learning_rate) + quality * learning_rate;
        self.avg_time = self.avg_time * (1.0 - learning_rate) + time * learning_rate;
        self.avg_cycles = self.avg_cycles * (1.0 - learning_rate) + cycles * learning_rate;

        // Update task-specific performance
        let task_perf = self
            .task_performance
            .entry(task_type)
            .or_insert_with(TaskPerformance::new);
        task_perf.update(reward, quality, learning_rate);
    }

    /// Calculate UCB score
    fn calculate_ucb(&mut self, total_count: u64, exploration_factor: f64) {
        if self.usage_count == 0 {
            self.ucb = f64::MAX;
            return;
        }

        let exploration = (2.0 * (total_count as f64).ln() / self.usage_count as f64).sqrt();
        self.ucb = self.avg_reward + exploration_factor * exploration;
    }

    /// Get performance for specific task type
    fn get_task_performance(&self, task_type: TaskType) -> Option<&TaskPerformance> {
        self.task_performance.get(&task_type)
    }
}

// ============================================================================
// STRATEGY SELECTOR
// ============================================================================

/// Strategy selection algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionAlgorithm {
    /// Epsilon-greedy selection
    EpsilonGreedy,
    /// Upper Confidence Bound (UCB1)
    UCB,
    /// Thompson Sampling
    ThompsonSampling,
    /// Softmax/Boltzmann selection
    Softmax,
    /// Context-aware bandit
    ContextualBandit,
    /// Best known (pure exploitation)
    BestKnown,
}

/// Strategy selector using multi-armed bandit approach
pub struct StrategySelector {
    /// Available strategies
    strategies: BTreeMap<StrategyId, CognitiveStrategy>,
    /// Performance tracking
    performance: BTreeMap<StrategyId, StrategyPerformance>,
    /// Selection algorithm
    algorithm: SelectionAlgorithm,
    /// Exploration rate (epsilon for epsilon-greedy)
    exploration_rate: f64,
    /// Learning rate
    learning_rate: f64,
    /// Temperature (for softmax)
    temperature: f64,
    /// Total selection count
    total_selections: u64,
    /// Next strategy ID
    next_strategy_id: u32,
    /// RNG state
    rng_state: u64,
}

impl StrategySelector {
    /// Create a new selector
    pub fn new() -> Self {
        Self {
            strategies: BTreeMap::new(),
            performance: BTreeMap::new(),
            algorithm: SelectionAlgorithm::UCB,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            learning_rate: DEFAULT_LEARNING_RATE,
            temperature: 1.0,
            total_selections: 0,
            next_strategy_id: 0,
            rng_state: 12345,
        }
    }

    /// Set selection algorithm
    pub fn with_algorithm(mut self, algorithm: SelectionAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set exploration rate
    pub fn with_exploration_rate(mut self, rate: f64) -> Self {
        self.exploration_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Register a strategy
    pub fn register_strategy(&mut self, strategy: CognitiveStrategy) -> StrategyId {
        let id = strategy.id;
        self.performance.insert(id, StrategyPerformance::new(id));
        self.strategies.insert(id, strategy);
        id
    }

    /// Create and register a new strategy
    pub fn create_strategy(&mut self, name: String) -> StrategyId {
        let id = StrategyId(self.next_strategy_id);
        self.next_strategy_id += 1;
        let strategy = CognitiveStrategy::new(id, name);
        self.register_strategy(strategy)
    }

    /// Select best strategy for task
    pub fn select_strategy(
        &mut self,
        task_type: TaskType,
        budget: &ResourceBudget,
    ) -> Option<StrategyId> {
        // Get applicable strategies
        let applicable: Vec<StrategyId> = self
            .strategies
            .values()
            .filter(|s| s.is_applicable(task_type) && s.fits_budget(budget))
            .map(|s| s.id)
            .collect();

        if applicable.is_empty() {
            return None;
        }

        self.total_selections += 1;

        // Update UCB values
        for id in &applicable {
            if let Some(perf) = self.performance.get_mut(id) {
                perf.calculate_ucb(self.total_selections, 2.0);
            }
        }

        let selected = match self.algorithm {
            SelectionAlgorithm::EpsilonGreedy => self.epsilon_greedy_select(&applicable, task_type),
            SelectionAlgorithm::UCB => self.ucb_select(&applicable),
            SelectionAlgorithm::ThompsonSampling => {
                self.thompson_sampling_select(&applicable, task_type)
            },
            SelectionAlgorithm::Softmax => self.softmax_select(&applicable),
            SelectionAlgorithm::ContextualBandit => {
                self.contextual_select(&applicable, task_type, budget)
            },
            SelectionAlgorithm::BestKnown => self.best_known_select(&applicable, task_type),
        };

        selected
    }

    /// Epsilon-greedy selection
    fn epsilon_greedy_select(
        &mut self,
        applicable: &[StrategyId],
        task_type: TaskType,
    ) -> Option<StrategyId> {
        if self.random() < self.exploration_rate {
            // Explore: random selection
            let idx = (self.random() * applicable.len() as f64) as usize;
            Some(applicable[idx.min(applicable.len() - 1)])
        } else {
            // Exploit: best known
            self.best_known_select(applicable, task_type)
        }
    }

    /// UCB selection
    fn ucb_select(&self, applicable: &[StrategyId]) -> Option<StrategyId> {
        applicable
            .iter()
            .max_by(|a, b| {
                let ucb_a = self.performance.get(a).map(|p| p.ucb).unwrap_or(0.0);
                let ucb_b = self.performance.get(b).map(|p| p.ucb).unwrap_or(0.0);
                ucb_a
                    .partial_cmp(&ucb_b)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .copied()
    }

    /// Thompson Sampling selection
    fn thompson_sampling_select(
        &mut self,
        applicable: &[StrategyId],
        task_type: TaskType,
    ) -> Option<StrategyId> {
        // Sample from Beta distributions for each strategy
        applicable
            .iter()
            .max_by(|a, b| {
                let sample_a = self.sample_beta(*a, task_type);
                let sample_b = self.sample_beta(*b, task_type);
                sample_a
                    .partial_cmp(&sample_b)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .copied()
    }

    /// Sample from Beta distribution (approximation)
    fn sample_beta(&mut self, strategy_id: StrategyId, task_type: TaskType) -> f64 {
        let perf = match self.performance.get(&strategy_id) {
            Some(p) => p,
            None => return self.random(),
        };

        let task_perf = perf.get_task_performance(task_type);
        let (alpha, beta) = match task_perf {
            Some(tp) if tp.count > 0 => {
                let successes = (tp.avg_reward * tp.count as f64) as u64;
                let failures = tp.count - successes;
                (successes as f64 + 1.0, failures as f64 + 1.0)
            },
            _ => (1.0, 1.0), // Uniform prior
        };

        // Approximate Beta sample using gamma samples
        let x = self.sample_gamma(alpha);
        let y = self.sample_gamma(beta);

        x / (x + y)
    }

    /// Sample from Gamma distribution (approximation using Marsaglia)
    fn sample_gamma(&mut self, shape: f64) -> f64 {
        if shape < 1.0 {
            return self.sample_gamma(shape + 1.0) * self.random().powf(1.0 / shape);
        }

        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();

        loop {
            let x = self.sample_normal();
            let v = (1.0 + c * x).powi(3);

            if v > 0.0 {
                let u = self.random();
                if u < 1.0 - 0.0331 * x.powi(4) || u.ln() < 0.5 * x.powi(2) + d * (1.0 - v + v.ln())
                {
                    return d * v;
                }
            }
        }
    }

    /// Sample from standard normal (Box-Muller)
    fn sample_normal(&mut self) -> f64 {
        let u1 = self.random();
        let u2 = self.random();

        (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos()
    }

    /// Softmax selection
    fn softmax_select(&mut self, applicable: &[StrategyId]) -> Option<StrategyId> {
        // Calculate softmax probabilities
        let rewards: Vec<f64> = applicable
            .iter()
            .map(|id| {
                self.performance
                    .get(id)
                    .map(|p| p.avg_reward)
                    .unwrap_or(0.5)
            })
            .collect();

        let max_reward = rewards.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp_sum: f64 = rewards
            .iter()
            .map(|r| ((r - max_reward) / self.temperature).exp())
            .sum();

        let probabilities: Vec<f64> = rewards
            .iter()
            .map(|r| ((r - max_reward) / self.temperature).exp() / exp_sum)
            .collect();

        // Sample according to probabilities
        let mut cumulative = 0.0;
        let r = self.random();

        for (i, prob) in probabilities.iter().enumerate() {
            cumulative += prob;
            if r < cumulative {
                return Some(applicable[i]);
            }
        }

        applicable.last().copied()
    }

    /// Context-aware selection
    fn contextual_select(
        &mut self,
        applicable: &[StrategyId],
        task_type: TaskType,
        budget: &ResourceBudget,
    ) -> Option<StrategyId> {
        // Score each strategy based on context
        let scores: Vec<(StrategyId, f64)> = applicable
            .iter()
            .map(|&id| {
                let mut score = 0.0;

                if let Some(perf) = self.performance.get(&id) {
                    // Base: task-specific performance
                    if let Some(tp) = perf.get_task_performance(task_type) {
                        score += tp.avg_reward * 0.4;
                    } else {
                        score += perf.avg_reward * 0.3;
                    }

                    // Factor in overall performance
                    score += perf.avg_quality * 0.3;

                    // Factor in efficiency (inverse of resource usage)
                    if let Some(strategy) = self.strategies.get(&id) {
                        let time_efficiency = 1.0
                            - (strategy.resource_requirements.max_time as f64
                                / budget.max_time as f64)
                                .min(1.0);
                        score += time_efficiency * 0.3;
                    }
                }

                // Exploration bonus for under-used strategies
                let usage = self
                    .performance
                    .get(&id)
                    .map(|p| p.usage_count)
                    .unwrap_or(0);
                if usage < MIN_SAMPLES {
                    score += 0.2 * (1.0 - usage as f64 / MIN_SAMPLES as f64);
                }

                (id, score)
            })
            .collect();

        // Select with softmax over scores
        if scores.is_empty() {
            return None;
        }

        let max_score = scores
            .iter()
            .map(|(_, s)| *s)
            .fold(f64::NEG_INFINITY, f64::max);
        let exp_sum: f64 = scores
            .iter()
            .map(|(_, s)| ((s - max_score) / self.temperature).exp())
            .sum();

        let mut cumulative = 0.0;
        let r = self.random();

        for (id, score) in &scores {
            let prob = ((score - max_score) / self.temperature).exp() / exp_sum;
            cumulative += prob;
            if r < cumulative {
                return Some(*id);
            }
        }

        scores.last().map(|(id, _)| *id)
    }

    /// Best known selection (pure exploitation)
    fn best_known_select(
        &self,
        applicable: &[StrategyId],
        task_type: TaskType,
    ) -> Option<StrategyId> {
        applicable
            .iter()
            .max_by(|a, b| {
                let score_a = self
                    .performance
                    .get(a)
                    .and_then(|p| p.get_task_performance(task_type))
                    .map(|tp| tp.avg_reward)
                    .unwrap_or(0.0);

                let score_b = self
                    .performance
                    .get(b)
                    .and_then(|p| p.get_task_performance(task_type))
                    .map(|tp| tp.avg_reward)
                    .unwrap_or(0.0);

                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .copied()
    }

    /// Report execution result
    pub fn report_result(
        &mut self,
        strategy_id: StrategyId,
        task_type: TaskType,
        reward: f64,
        quality: f64,
        time_used: f64,
        cycles_used: f64,
        success: bool,
    ) {
        if let Some(perf) = self.performance.get_mut(&strategy_id) {
            perf.update(
                task_type,
                reward,
                quality,
                time_used,
                cycles_used,
                success,
                self.learning_rate,
            );
        }
    }

    /// Get strategy
    pub fn get_strategy(&self, id: StrategyId) -> Option<&CognitiveStrategy> {
        self.strategies.get(&id)
    }

    /// Get mutable strategy
    pub fn get_strategy_mut(&mut self, id: StrategyId) -> Option<&mut CognitiveStrategy> {
        self.strategies.get_mut(&id)
    }

    /// Get performance
    pub fn get_performance(&self, id: StrategyId) -> Option<&StrategyPerformance> {
        self.performance.get(&id)
    }

    /// Get all strategies for task type
    pub fn get_strategies_for_task(&self, task_type: TaskType) -> Vec<&CognitiveStrategy> {
        self.strategies
            .values()
            .filter(|s| s.is_applicable(task_type))
            .collect()
    }

    /// Simple random number generator
    fn random(&mut self) -> f64 {
        self.rng_state = self.rng_state.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
        ((self.rng_state >> 17) as f64) / (u32::MAX as f64)
    }
}

impl Default for StrategySelector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// STRATEGY COMPOSITION
// ============================================================================

/// A composite strategy (pipeline or ensemble)
#[derive(Debug, Clone)]
pub struct CompositeStrategy {
    /// Strategy ID
    pub id: StrategyId,
    /// Name
    pub name: String,
    /// Component strategies
    pub components: Vec<StrategyId>,
    /// Composition type
    pub composition_type: CompositionType,
    /// Weights for ensemble
    pub weights: Vec<f64>,
}

/// Type of strategy composition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionType {
    /// Sequential pipeline
    Pipeline,
    /// Parallel ensemble with voting
    EnsembleVoting,
    /// Parallel ensemble with weighted average
    EnsembleWeighted,
    /// Fallback chain (try until success)
    Fallback,
    /// Race (use first to complete)
    Race,
}

impl CompositeStrategy {
    /// Create a pipeline
    pub fn pipeline(id: StrategyId, name: String, components: Vec<StrategyId>) -> Self {
        Self {
            id,
            name,
            components,
            composition_type: CompositionType::Pipeline,
            weights: Vec::new(),
        }
    }

    /// Create a weighted ensemble
    pub fn ensemble(
        id: StrategyId,
        name: String,
        components: Vec<StrategyId>,
        weights: Vec<f64>,
    ) -> Self {
        Self {
            id,
            name,
            components,
            composition_type: CompositionType::EnsembleWeighted,
            weights,
        }
    }

    /// Create a fallback chain
    pub fn fallback(id: StrategyId, name: String, components: Vec<StrategyId>) -> Self {
        Self {
            id,
            name,
            components,
            composition_type: CompositionType::Fallback,
            weights: Vec::new(),
        }
    }
}

// ============================================================================
// PREDEFINED STRATEGIES
// ============================================================================

/// Factory for common strategies
pub struct StrategyFactory;

impl StrategyFactory {
    /// Create a fast/simple strategy
    pub fn fast_simple(id: StrategyId) -> CognitiveStrategy {
        let mut strategy = CognitiveStrategy::new(id, String::from("FastSimple"));
        strategy.description = String::from("Quick heuristic-based approach");
        strategy.applicable_tasks = vec![TaskType::Generic];
        strategy.resource_requirements = ResourceBudget::tight();
        strategy.expected_quality = 0.6;
        strategy.speed_rating = 9;
        strategy.accuracy_rating = 4;
        strategy
    }

    /// Create a balanced strategy
    pub fn balanced(id: StrategyId) -> CognitiveStrategy {
        let mut strategy = CognitiveStrategy::new(id, String::from("Balanced"));
        strategy.description = String::from("Balanced speed/accuracy trade-off");
        strategy.applicable_tasks = vec![TaskType::Generic];
        strategy.resource_requirements = ResourceBudget::normal();
        strategy.expected_quality = 0.75;
        strategy.speed_rating = 5;
        strategy.accuracy_rating = 6;
        strategy
    }

    /// Create a high-accuracy strategy
    pub fn high_accuracy(id: StrategyId) -> CognitiveStrategy {
        let mut strategy = CognitiveStrategy::new(id, String::from("HighAccuracy"));
        strategy.description = String::from("Thorough analysis for high accuracy");
        strategy.applicable_tasks = vec![TaskType::Generic];
        strategy.resource_requirements = ResourceBudget::generous();
        strategy.expected_quality = 0.95;
        strategy.speed_rating = 2;
        strategy.accuracy_rating = 9;
        strategy
    }

    /// Create an anomaly detection strategy
    pub fn anomaly_detection(id: StrategyId) -> CognitiveStrategy {
        let mut strategy = CognitiveStrategy::new(id, String::from("AnomalyDetection"));
        strategy.description = String::from("Statistical anomaly detection");
        strategy.applicable_tasks = vec![TaskType::AnomalyDetection, TaskType::Security];
        strategy.resource_requirements = ResourceBudget::normal();
        strategy.expected_quality = 0.85;
        strategy.speed_rating = 6;
        strategy.accuracy_rating = 7;

        // Add parameters
        strategy.parameters.insert(
            String::from("threshold"),
            StrategyParameter::new("threshold", 2.0, 1.0, 5.0),
        );
        strategy.parameters.insert(
            String::from("window_size"),
            StrategyParameter::new("window_size", 100.0, 10.0, 1000.0),
        );

        strategy
    }

    /// Create a scheduling strategy
    pub fn scheduler(id: StrategyId) -> CognitiveStrategy {
        let mut strategy = CognitiveStrategy::new(id, String::from("AdaptiveScheduler"));
        strategy.description = String::from("Adaptive scheduling strategy");
        strategy.applicable_tasks = vec![TaskType::Scheduling, TaskType::ResourceAllocation];
        strategy.resource_requirements = ResourceBudget::tight();
        strategy.expected_quality = 0.8;
        strategy.speed_rating = 8;
        strategy.accuracy_rating = 7;
        strategy.adaptive = true;

        strategy.parameters.insert(
            String::from("lookahead"),
            StrategyParameter::new("lookahead", 5.0, 1.0, 20.0),
        );

        strategy
    }

    /// Create a learning strategy
    pub fn learning(id: StrategyId) -> CognitiveStrategy {
        let mut strategy = CognitiveStrategy::new(id, String::from("OnlineLearning"));
        strategy.description = String::from("Continuous online learning");
        strategy.applicable_tasks = vec![TaskType::Learning, TaskType::Inference];
        strategy.resource_requirements = ResourceBudget::normal();
        strategy.expected_quality = 0.7;
        strategy.speed_rating = 4;
        strategy.accuracy_rating = 6;
        strategy.adaptive = true;

        strategy.parameters.insert(
            String::from("learning_rate"),
            StrategyParameter::new("learning_rate", 0.01, 0.001, 0.1),
        );

        strategy
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_registration() {
        let mut selector = StrategySelector::new();

        let strategy = StrategyFactory::fast_simple(StrategyId(0));
        let id = selector.register_strategy(strategy);

        assert!(selector.get_strategy(id).is_some());
    }

    #[test]
    fn test_strategy_selection() {
        let mut selector = StrategySelector::new();

        // Register strategies
        let fast = StrategyFactory::fast_simple(StrategyId(0));
        let balanced = StrategyFactory::balanced(StrategyId(1));

        selector.register_strategy(fast);
        selector.register_strategy(balanced);

        let budget = ResourceBudget::normal();
        let selected = selector.select_strategy(TaskType::Generic, &budget);

        assert!(selected.is_some());
    }

    #[test]
    fn test_performance_tracking() {
        let mut selector = StrategySelector::new();

        let strategy = StrategyFactory::balanced(StrategyId(0));
        let id = selector.register_strategy(strategy);

        // Report some results
        for _ in 0..10 {
            selector.report_result(id, TaskType::Generic, 0.8, 0.85, 100.0, 1000.0, true);
        }

        let perf = selector.get_performance(id).unwrap();
        assert_eq!(perf.usage_count, 10);
        assert!(perf.avg_reward > 0.7);
    }

    #[test]
    fn test_resource_budget() {
        let tight = ResourceBudget::tight();
        let normal = ResourceBudget::normal();
        let generous = ResourceBudget::generous();

        assert!(normal.fits(&tight));
        assert!(generous.fits(&normal));
        assert!(!tight.fits(&normal));
    }

    #[test]
    fn test_ucb_selection() {
        let mut selector = StrategySelector::new().with_algorithm(SelectionAlgorithm::UCB);

        for i in 0..5 {
            let strategy = StrategyFactory::balanced(StrategyId(i));
            selector.register_strategy(strategy);
        }

        // Initially should explore
        let budget = ResourceBudget::normal();
        for _ in 0..10 {
            let selected = selector.select_strategy(TaskType::Generic, &budget);
            if let Some(id) = selected {
                selector.report_result(id, TaskType::Generic, 0.5, 0.5, 50.0, 500.0, true);
            }
        }

        // All strategies should have been tried
        for i in 0..5 {
            let perf = selector.get_performance(StrategyId(i)).unwrap();
            assert!(perf.usage_count > 0);
        }
    }

    #[test]
    fn test_composite_strategy() {
        let pipeline = CompositeStrategy::pipeline(StrategyId(10), String::from("Pipeline"), vec![
            StrategyId(0),
            StrategyId(1),
            StrategyId(2),
        ]);

        assert_eq!(pipeline.composition_type, CompositionType::Pipeline);
        assert_eq!(pipeline.components.len(), 3);
    }

    #[test]
    fn test_strategy_parameters() {
        let mut strategy = StrategyFactory::anomaly_detection(StrategyId(0));

        // Get parameter
        assert_eq!(strategy.get_parameter("threshold"), Some(2.0));

        // Set parameter
        assert!(strategy.set_parameter("threshold", 3.5));
        assert_eq!(strategy.get_parameter("threshold"), Some(3.5));

        // Value should be clamped
        strategy.set_parameter("threshold", 10.0);
        assert_eq!(strategy.get_parameter("threshold"), Some(5.0)); // Max is 5.0
    }
}
