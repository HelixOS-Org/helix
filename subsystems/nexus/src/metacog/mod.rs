//! # Metacognitive Controller
//!
//! Revolutionary self-awareness system that enables the kernel to monitor,
//! evaluate, and regulate its own cognitive processes. The kernel becomes
//! aware of its own decision-making, learning, and performance.
//!
//! ## Metacognitive Hierarchy
//!
//! 1. **Meta-Monitoring**: Track performance of all subsystems
//! 2. **Meta-Evaluation**: Assess quality of decisions and predictions
//! 3. **Meta-Control**: Adjust learning rates, strategies, resource allocation
//! 4. **Meta-Learning**: Learn how to learn better
//! 5. **Meta-Reasoning**: Reason about reasoning processes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    METACOGNITIVE CONTROLLER                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    INTROSPECTION ENGINE                         │     │
//! │  │   Monitors internal states, decisions, confidence levels        │     │
//! │  │   Tracks cognitive load, attention, resource usage             │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    CONFIDENCE CALIBRATOR                        │     │
//! │  │   Calibrates confidence estimates across all subsystems        │     │
//! │  │   Platt scaling, temperature scaling, isotonic regression      │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    STRATEGY SELECTOR                            │     │
//! │  │   Chooses optimal learning/inference strategies                │     │
//! │  │   UCB-based exploration-exploitation balance                   │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    REGULATOR                                    │     │
//! │  │   Adjusts hyperparameters, allocates cognitive resources       │     │
//! │  │   Controls exploration vs exploitation across subsystems       │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

extern crate alloc;

// Year 2 COGNITION sub-modules
pub mod monitor;
pub mod strategy;

// Re-exports
use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

pub use monitor::{
    AnomalyDetector, CognitiveProcess, ConfidenceCalibrator, DomainMetrics, HealthReport,
    MetacognitionMonitor, ProcessState,
};
pub use strategy::{
    CognitiveStrategy, CompositeStrategy, ResourceBudget, SelectionAlgorithm, StrategyFactory,
    StrategyId, StrategySelector, TaskType,
};

/// Maximum history length
const MAX_HISTORY: usize = 1000;

/// Cognitive subsystem identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubsystemId {
    Scheduler,
    Memory,
    Io,
    Network,
    Security,
    Learning,
    Prediction,
    Planning,
    Reasoning,
    Attention,
    Custom(u32),
}

/// Cognitive state snapshot
#[derive(Debug, Clone, Default)]
pub struct CognitiveState {
    /// Current cognitive load (0-1)
    pub load: f64,
    /// Attention allocation per subsystem
    pub attention: BTreeMap<SubsystemId, f64>,
    /// Active goals
    pub goals: Vec<GoalState>,
    /// Current strategy
    pub strategy: StrategyState,
    /// Confidence in current decisions
    pub confidence: f64,
    /// Uncertainty estimate
    pub uncertainty: f64,
    /// Resource budget
    pub resource_budget: f64,
}

/// Goal state
#[derive(Debug, Clone)]
pub struct GoalState {
    /// Goal identifier
    pub id: u64,
    /// Goal description
    pub description: String,
    /// Priority (0-1)
    pub priority: f64,
    /// Progress (0-1)
    pub progress: f64,
    /// Deadline (if any)
    pub deadline: Option<u64>,
    /// Subgoals
    pub subgoals: Vec<u64>,
}

/// Strategy state
#[derive(Debug, Clone, Default)]
pub struct StrategyState {
    /// Current strategy ID
    pub strategy_id: u32,
    /// Exploration rate
    pub exploration: f64,
    /// Learning rate
    pub learning_rate: f64,
    /// Planning horizon
    pub horizon: usize,
    /// Risk tolerance
    pub risk_tolerance: f64,
}

/// Decision record for reflection
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    /// Timestamp
    pub timestamp: u64,
    /// Subsystem that made the decision
    pub subsystem: SubsystemId,
    /// Decision type
    pub decision_type: DecisionType,
    /// Input features (summary)
    pub input_summary: Vec<f64>,
    /// Output/action taken
    pub action: u32,
    /// Predicted outcome
    pub predicted_outcome: f64,
    /// Actual outcome (filled later)
    pub actual_outcome: Option<f64>,
    /// Confidence at decision time
    pub confidence: f64,
}

/// Decision types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    Scheduling,
    MemoryAllocation,
    IoRouting,
    Prediction,
    Classification,
    Optimization,
    Recovery,
}

/// Introspection engine
#[derive(Debug, Clone)]
pub struct IntrospectionEngine {
    /// Current cognitive state
    state: CognitiveState,
    /// Decision history
    history: VecDeque<DecisionRecord>,
    /// Performance metrics per subsystem
    metrics: BTreeMap<SubsystemId, SubsystemMetrics>,
    /// Current timestamp
    time: u64,
}

/// Subsystem performance metrics
#[derive(Debug, Clone, Default)]
pub struct SubsystemMetrics {
    /// Total decisions made
    pub decisions: u64,
    /// Correct decisions (if evaluable)
    pub correct: u64,
    /// Total prediction error
    pub total_error: f64,
    /// Average confidence
    pub avg_confidence: f64,
    /// Calibration error
    pub calibration_error: f64,
    /// Response time (moving average)
    pub avg_response_time: f64,
    /// Resource usage
    pub resource_usage: f64,
}

impl IntrospectionEngine {
    /// Create a new introspection engine
    pub fn new() -> Self {
        Self {
            state: CognitiveState::default(),
            history: VecDeque::with_capacity(MAX_HISTORY),
            metrics: BTreeMap::new(),
            time: 0,
        }
    }

    /// Record a decision
    pub fn record_decision(&mut self, record: DecisionRecord) {
        let subsystem = record.subsystem;

        // Update subsystem metrics
        let metrics = self.metrics.entry(subsystem).or_default();
        metrics.decisions += 1;

        // Update moving average of confidence
        let alpha = 0.1;
        metrics.avg_confidence = (1.0 - alpha) * metrics.avg_confidence + alpha * record.confidence;

        // Add to history
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(record);
    }

    /// Record outcome for a decision
    pub fn record_outcome(&mut self, timestamp: u64, subsystem: SubsystemId, outcome: f64) {
        // Find the decision and update it
        for record in self.history.iter_mut().rev() {
            if record.timestamp == timestamp && record.subsystem == subsystem {
                record.actual_outcome = Some(outcome);

                // Update metrics
                if let Some(metrics) = self.metrics.get_mut(&subsystem) {
                    let error = (record.predicted_outcome - outcome).abs();
                    metrics.total_error += error;

                    // Calibration: was confidence appropriate?
                    let was_correct = error < 0.1;
                    if was_correct {
                        metrics.correct += 1;
                    }

                    // ECE (Expected Calibration Error) update
                    let _confidence_bucket = (record.confidence * 10.0) as usize;
                    let expected_accuracy = record.confidence;
                    let actual_accuracy = if was_correct { 1.0 } else { 0.0 };
                    metrics.calibration_error = (1.0 - 0.1) * metrics.calibration_error
                        + 0.1 * (expected_accuracy - actual_accuracy).abs();
                }
                break;
            }
        }
    }

    /// Get current cognitive load
    pub fn cognitive_load(&self) -> f64 {
        self.state.load
    }

    /// Update cognitive load based on activity
    pub fn update_load(&mut self, activity_level: f64) {
        let alpha = 0.2;
        self.state.load = (1.0 - alpha) * self.state.load + alpha * activity_level;
    }

    /// Get attention allocation
    pub fn attention(&self) -> &BTreeMap<SubsystemId, f64> {
        &self.state.attention
    }

    /// Get subsystem metrics
    pub fn get_metrics(&self, subsystem: SubsystemId) -> Option<&SubsystemMetrics> {
        self.metrics.get(&subsystem)
    }

    /// Get overall accuracy
    pub fn overall_accuracy(&self) -> f64 {
        let total_decisions: u64 = self.metrics.values().map(|m| m.decisions).sum();
        let total_correct: u64 = self.metrics.values().map(|m| m.correct).sum();

        if total_decisions > 0 {
            total_correct as f64 / total_decisions as f64
        } else {
            0.0
        }
    }

    /// Get decision trace for debugging/analysis
    pub fn recent_decisions(&self, count: usize) -> Vec<&DecisionRecord> {
        self.history.iter().rev().take(count).collect()
    }
}

/// Local confidence calibrator for internal metacog use
#[derive(Debug, Clone)]
pub struct LocalConfidenceCalibrator {
    /// Temperature for temperature scaling
    temperature: f64,
    /// Platt scaling parameters per subsystem
    platt_params: BTreeMap<SubsystemId, (f64, f64)>,
    /// Isotonic regression bins per subsystem
    isotonic_bins: BTreeMap<SubsystemId, Vec<(f64, f64)>>,
    /// Calibration history
    calibration_data: BTreeMap<SubsystemId, Vec<(f64, bool)>>,
}

impl LocalConfidenceCalibrator {
    /// Create a new confidence calibrator
    pub fn new() -> Self {
        Self {
            temperature: 1.0,
            platt_params: BTreeMap::new(),
            isotonic_bins: BTreeMap::new(),
            calibration_data: BTreeMap::new(),
        }
    }

    /// Add calibration data point
    pub fn add_data(&mut self, subsystem: SubsystemId, confidence: f64, was_correct: bool) {
        self.calibration_data
            .entry(subsystem)
            .or_default()
            .push((confidence, was_correct));

        // Limit history
        if let Some(data) = self.calibration_data.get_mut(&subsystem) {
            if data.len() > 10000 {
                data.drain(0..5000);
            }
        }
    }

    /// Calibrate confidence using temperature scaling
    pub fn calibrate_temperature(&self, logits: &[f64]) -> Vec<f64> {
        let mut probs = Vec::with_capacity(logits.len());
        let max_logit = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let sum: f64 = logits
            .iter()
            .map(|l| libm::exp((l - max_logit) / self.temperature))
            .sum();

        for l in logits {
            probs.push(libm::exp((l - max_logit) / self.temperature) / sum);
        }

        probs
    }

    /// Calibrate using Platt scaling
    pub fn calibrate_platt(&self, subsystem: SubsystemId, raw_score: f64) -> f64 {
        if let Some(&(a, b)) = self.platt_params.get(&subsystem) {
            1.0 / (1.0 + libm::exp(a * raw_score + b))
        } else {
            // Default sigmoid
            1.0 / (1.0 + libm::exp(-raw_score))
        }
    }

    /// Fit Platt scaling parameters
    pub fn fit_platt(&mut self, subsystem: SubsystemId) {
        let data = match self.calibration_data.get(&subsystem) {
            Some(d) if d.len() >= 10 => d,
            _ => return,
        };

        // Simple gradient descent for Platt parameters
        let mut a = 0.0;
        let mut b = 0.0;
        let lr = 0.01;
        let epochs = 100;

        for _ in 0..epochs {
            let mut grad_a = 0.0;
            let mut grad_b = 0.0;

            for &(score, correct) in data {
                let p = 1.0 / (1.0 + libm::exp(a * score + b));
                let y = if correct { 1.0 } else { 0.0 };
                let err = p - y;
                grad_a += err * score;
                grad_b += err;
            }

            a -= lr * grad_a / data.len() as f64;
            b -= lr * grad_b / data.len() as f64;
        }

        self.platt_params.insert(subsystem, (a, b));
    }

    /// Fit temperature scaling
    pub fn fit_temperature(&mut self, logits_list: &[Vec<f64>], labels: &[usize]) {
        if logits_list.is_empty() || logits_list.len() != labels.len() {
            return;
        }

        // Grid search for optimal temperature
        let mut best_temp = 1.0;
        let mut best_nll = f64::INFINITY;

        for t in [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0].iter() {
            self.temperature = *t;
            let mut nll = 0.0;

            for (logits, &label) in logits_list.iter().zip(labels) {
                let probs = self.calibrate_temperature(logits);
                if label < probs.len() {
                    nll -= libm::log(probs[label].max(1e-10));
                }
            }

            if nll < best_nll {
                best_nll = nll;
                best_temp = *t;
            }
        }

        self.temperature = best_temp;
    }

    /// Build isotonic regression bins
    pub fn fit_isotonic(&mut self, subsystem: SubsystemId) {
        let data = match self.calibration_data.get(&subsystem) {
            Some(d) if d.len() >= 20 => d,
            _ => return,
        };

        // Sort by confidence
        let mut sorted: Vec<_> = data.clone();
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Pool Adjacent Violators Algorithm (simplified)
        let num_bins = 10;
        let bin_size = sorted.len() / num_bins;
        let mut bins = Vec::new();

        for i in 0..num_bins {
            let start = i * bin_size;
            let end = if i == num_bins - 1 {
                sorted.len()
            } else {
                (i + 1) * bin_size
            };

            let avg_conf: f64 =
                sorted[start..end].iter().map(|x| x.0).sum::<f64>() / (end - start) as f64;
            let accuracy: f64 =
                sorted[start..end].iter().filter(|x| x.1).count() as f64 / (end - start) as f64;

            bins.push((avg_conf, accuracy));
        }

        // Ensure monotonicity
        for i in 1..bins.len() {
            if bins[i].1 < bins[i - 1].1 {
                let avg = (bins[i].1 + bins[i - 1].1) / 2.0;
                bins[i].1 = avg;
                bins[i - 1].1 = avg;
            }
        }

        self.isotonic_bins.insert(subsystem, bins);
    }

    /// Calibrate using isotonic regression
    pub fn calibrate_isotonic(&self, subsystem: SubsystemId, confidence: f64) -> f64 {
        let bins = match self.isotonic_bins.get(&subsystem) {
            Some(b) => b,
            None => return confidence,
        };

        // Find appropriate bin
        for window in bins.windows(2) {
            if confidence >= window[0].0 && confidence < window[1].0 {
                // Linear interpolation
                let t = (confidence - window[0].0) / (window[1].0 - window[0].0 + 1e-10);
                return window[0].1 * (1.0 - t) + window[1].1 * t;
            }
        }

        // Edge cases
        if confidence < bins[0].0 {
            bins[0].1
        } else {
            bins.last().map(|b| b.1).unwrap_or(confidence)
        }
    }

    /// Get Expected Calibration Error (ECE)
    pub fn compute_ece(&self, subsystem: SubsystemId) -> f64 {
        let data = match self.calibration_data.get(&subsystem) {
            Some(d) if !d.is_empty() => d,
            _ => return 0.0,
        };

        let num_bins = 10;
        let mut bins: Vec<Vec<(f64, bool)>> = (0..num_bins).map(|_| Vec::new()).collect();

        for &(conf, correct) in data {
            let bin_idx = ((conf * num_bins as f64) as usize).min(num_bins - 1);
            bins[bin_idx].push((conf, correct));
        }

        let mut ece = 0.0;
        let total = data.len() as f64;

        for bin in &bins {
            if bin.is_empty() {
                continue;
            }

            let avg_conf: f64 = bin.iter().map(|x| x.0).sum::<f64>() / bin.len() as f64;
            let accuracy = bin.iter().filter(|x| x.1).count() as f64 / bin.len() as f64;
            let weight = bin.len() as f64 / total;

            ece += weight * (avg_conf - accuracy).abs();
        }

        ece
    }
}

/// Strategy arm for UCB selection
#[derive(Debug, Clone)]
struct StrategyArm {
    /// Strategy identifier
    id: u32,
    /// Total reward accumulated
    total_reward: f64,
    /// Number of times selected
    times_selected: u64,
}

impl StrategyArm {
    fn new(id: u32) -> Self {
        Self {
            id,
            total_reward: 0.0,
            times_selected: 0,
        }
    }

    fn mean_reward(&self) -> f64 {
        if self.times_selected > 0 {
            self.total_reward / self.times_selected as f64
        } else {
            0.0
        }
    }

    fn ucb(&self, total_time: u64, c: f64) -> f64 {
        if self.times_selected == 0 {
            f64::INFINITY
        } else {
            self.mean_reward()
                + c * libm::sqrt(libm::log(total_time as f64 + 1.0) / self.times_selected as f64)
        }
    }
}

/// Local strategy selector using UCB for internal metacog use
#[derive(Debug, Clone)]
pub struct LocalStrategySelector {
    /// Available strategies
    strategies: Vec<StrategyArm>,
    /// Exploration constant
    c: f64,
    /// Total time steps
    time: u64,
    /// Current strategy
    current: Option<u32>,
}

impl LocalStrategySelector {
    /// Create a new strategy selector
    pub fn new(num_strategies: u32) -> Self {
        Self {
            strategies: (0..num_strategies).map(StrategyArm::new).collect(),
            c: 2.0,
            time: 0,
            current: None,
        }
    }

    /// Select best strategy using UCB
    pub fn select(&mut self) -> u32 {
        self.time += 1;

        let best = self
            .strategies
            .iter()
            .max_by(|a, b| {
                let ucb_a = a.ucb(self.time, self.c);
                let ucb_b = b.ucb(self.time, self.c);
                ucb_a.partial_cmp(&ucb_b).unwrap()
            })
            .map(|s| s.id)
            .unwrap_or(0);

        self.current = Some(best);
        best
    }

    /// Update with reward
    pub fn update(&mut self, strategy: u32, reward: f64) {
        if let Some(arm) = self.strategies.iter_mut().find(|a| a.id == strategy) {
            arm.total_reward += reward;
            arm.times_selected += 1;
        }
    }

    /// Get current strategy
    pub fn current(&self) -> Option<u32> {
        self.current
    }

    /// Get strategy statistics
    pub fn stats(&self, strategy: u32) -> Option<(f64, u64)> {
        self.strategies
            .iter()
            .find(|a| a.id == strategy)
            .map(|a| (a.mean_reward(), a.times_selected))
    }
}

/// Cognitive regulator
#[derive(Debug, Clone)]
pub struct CognitiveRegulator {
    /// Learning rate per subsystem
    learning_rates: BTreeMap<SubsystemId, f64>,
    /// Exploration rates per subsystem
    exploration_rates: BTreeMap<SubsystemId, f64>,
    /// Resource allocations
    resource_allocation: BTreeMap<SubsystemId, f64>,
    /// Total resource budget
    budget: f64,
    /// Regulation policy
    policy: RegulationPolicy,
}

/// Regulation policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegulationPolicy {
    /// Balanced allocation
    Balanced,
    /// Priority-based
    Priority,
    /// Performance-adaptive
    Adaptive,
    /// Uncertainty-guided
    Uncertainty,
}

impl CognitiveRegulator {
    /// Create a new cognitive regulator
    pub fn new(budget: f64) -> Self {
        Self {
            learning_rates: BTreeMap::new(),
            exploration_rates: BTreeMap::new(),
            resource_allocation: BTreeMap::new(),
            budget,
            policy: RegulationPolicy::Adaptive,
        }
    }

    /// Set policy
    pub fn set_policy(&mut self, policy: RegulationPolicy) {
        self.policy = policy;
    }

    /// Register a subsystem
    pub fn register(&mut self, subsystem: SubsystemId, initial_lr: f64, initial_exploration: f64) {
        self.learning_rates.insert(subsystem, initial_lr);
        self.exploration_rates
            .insert(subsystem, initial_exploration);
    }

    /// Regulate based on performance metrics
    pub fn regulate(&mut self, metrics: &BTreeMap<SubsystemId, SubsystemMetrics>) {
        match self.policy {
            RegulationPolicy::Balanced => self.regulate_balanced(),
            RegulationPolicy::Priority => self.regulate_priority(metrics),
            RegulationPolicy::Adaptive => self.regulate_adaptive(metrics),
            RegulationPolicy::Uncertainty => self.regulate_uncertainty(metrics),
        }
    }

    fn regulate_balanced(&mut self) {
        let n = self.learning_rates.len();
        if n == 0 {
            return;
        }

        let per_subsystem = self.budget / n as f64;
        for (&subsystem, _) in &self.learning_rates {
            self.resource_allocation.insert(subsystem, per_subsystem);
        }
    }

    fn regulate_priority(&mut self, metrics: &BTreeMap<SubsystemId, SubsystemMetrics>) {
        // Allocate more to frequently used subsystems
        let total_decisions: u64 = metrics.values().map(|m| m.decisions).sum();
        if total_decisions == 0 {
            return self.regulate_balanced();
        }

        for (&subsystem, m) in metrics {
            let weight = m.decisions as f64 / total_decisions as f64;
            self.resource_allocation
                .insert(subsystem, self.budget * weight);
        }
    }

    fn regulate_adaptive(&mut self, metrics: &BTreeMap<SubsystemId, SubsystemMetrics>) {
        // Adjust learning rates based on performance
        for (&subsystem, m) in metrics {
            if let Some(lr) = self.learning_rates.get_mut(&subsystem) {
                // If accuracy is low, increase learning rate
                let accuracy = if m.decisions > 0 {
                    m.correct as f64 / m.decisions as f64
                } else {
                    0.5
                };

                if accuracy < 0.5 {
                    *lr = (*lr * 1.1).min(0.5);
                } else if accuracy > 0.9 {
                    *lr = (*lr * 0.95).max(0.001);
                }
            }

            if let Some(expl) = self.exploration_rates.get_mut(&subsystem) {
                // If calibration is poor, increase exploration
                if m.calibration_error > 0.1 {
                    *expl = (*expl * 1.1).min(0.5);
                } else {
                    *expl = (*expl * 0.98).max(0.01);
                }
            }
        }

        // Resource allocation based on error
        let total_error: f64 = metrics
            .values()
            .map(|m| m.calibration_error + m.total_error / (m.decisions as f64 + 1.0))
            .sum();

        if total_error > 0.0 {
            for (&subsystem, m) in metrics {
                let subsystem_error =
                    m.calibration_error + m.total_error / (m.decisions as f64 + 1.0);
                let weight = subsystem_error / total_error;
                self.resource_allocation
                    .insert(subsystem, self.budget * weight);
            }
        }
    }

    fn regulate_uncertainty(&mut self, metrics: &BTreeMap<SubsystemId, SubsystemMetrics>) {
        // Allocate more resources to uncertain subsystems
        let mut uncertainties: BTreeMap<SubsystemId, f64> = BTreeMap::new();
        let mut total_uncertainty = 0.0;

        for (&subsystem, m) in metrics {
            // Uncertainty = 1 - avg_confidence + calibration_error
            let uncertainty = (1.0 - m.avg_confidence) + m.calibration_error;
            uncertainties.insert(subsystem, uncertainty);
            total_uncertainty += uncertainty;
        }

        if total_uncertainty > 0.0 {
            for (&subsystem, &uncertainty) in &uncertainties {
                let weight = uncertainty / total_uncertainty;
                self.resource_allocation
                    .insert(subsystem, self.budget * weight);

                // Also adjust exploration based on uncertainty
                if let Some(expl) = self.exploration_rates.get_mut(&subsystem) {
                    *expl = (uncertainty * 0.5).clamp(0.01, 0.5);
                }
            }
        }
    }

    /// Get learning rate for a subsystem
    pub fn learning_rate(&self, subsystem: SubsystemId) -> f64 {
        self.learning_rates.get(&subsystem).copied().unwrap_or(0.01)
    }

    /// Get exploration rate for a subsystem
    pub fn exploration_rate(&self, subsystem: SubsystemId) -> f64 {
        self.exploration_rates
            .get(&subsystem)
            .copied()
            .unwrap_or(0.1)
    }

    /// Get resource allocation for a subsystem
    pub fn resource_for(&self, subsystem: SubsystemId) -> f64 {
        self.resource_allocation
            .get(&subsystem)
            .copied()
            .unwrap_or(0.0)
    }
}

/// Meta-reasoning module
#[derive(Debug, Clone)]
pub struct MetaReasoner {
    /// Reasoning traces
    traces: VecDeque<ReasoningTrace>,
    /// Reasoning patterns
    patterns: Vec<ReasoningPattern>,
    /// Pattern usage statistics
    pattern_stats: BTreeMap<u32, PatternStats>,
}

/// Reasoning trace
#[derive(Debug, Clone)]
pub struct ReasoningTrace {
    /// Timestamp
    pub timestamp: u64,
    /// Problem type
    pub problem_type: u32,
    /// Reasoning steps
    pub steps: Vec<ReasoningStep>,
    /// Outcome quality
    pub outcome: f64,
    /// Time taken
    pub time_taken: u64,
}

/// Single reasoning step
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    /// Step type
    pub step_type: StepType,
    /// Input size
    pub input_size: usize,
    /// Output size
    pub output_size: usize,
    /// Time taken
    pub time: u64,
}

/// Step types in reasoning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    Observation,
    Inference,
    Search,
    Comparison,
    Decision,
    Backtrack,
}

/// Reasoning pattern
#[derive(Debug, Clone)]
pub struct ReasoningPattern {
    /// Pattern ID
    pub id: u32,
    /// Sequence of step types
    pub sequence: Vec<StepType>,
    /// Applicable problem types
    pub applicable_to: Vec<u32>,
}

/// Pattern statistics
#[derive(Debug, Clone, Default)]
pub struct PatternStats {
    pub uses: u64,
    pub successes: u64,
    pub total_time: u64,
}

impl MetaReasoner {
    /// Create a new meta-reasoner
    pub fn new() -> Self {
        // Initialize with some default patterns
        let patterns = vec![
            ReasoningPattern {
                id: 0,
                sequence: vec![
                    StepType::Observation,
                    StepType::Inference,
                    StepType::Decision,
                ],
                applicable_to: vec![0, 1, 2],
            },
            ReasoningPattern {
                id: 1,
                sequence: vec![
                    StepType::Observation,
                    StepType::Search,
                    StepType::Comparison,
                    StepType::Decision,
                ],
                applicable_to: vec![1, 2, 3],
            },
            ReasoningPattern {
                id: 2,
                sequence: vec![
                    StepType::Observation,
                    StepType::Inference,
                    StepType::Search,
                    StepType::Backtrack,
                    StepType::Decision,
                ],
                applicable_to: vec![3, 4],
            },
        ];

        Self {
            traces: VecDeque::with_capacity(1000),
            patterns,
            pattern_stats: BTreeMap::new(),
        }
    }

    /// Record a reasoning trace
    pub fn record_trace(&mut self, trace: ReasoningTrace) {
        // Match to pattern
        let matched_pattern = self.match_pattern(&trace.steps);

        if let Some(pattern_id) = matched_pattern {
            let stats = self.pattern_stats.entry(pattern_id).or_default();
            stats.uses += 1;
            if trace.outcome > 0.5 {
                stats.successes += 1;
            }
            stats.total_time += trace.time_taken;
        }

        if self.traces.len() >= 1000 {
            self.traces.pop_front();
        }
        self.traces.push_back(trace);
    }

    /// Match trace to pattern
    fn match_pattern(&self, steps: &[ReasoningStep]) -> Option<u32> {
        let step_types: Vec<_> = steps.iter().map(|s| s.step_type).collect();

        for pattern in &self.patterns {
            if step_types == pattern.sequence {
                return Some(pattern.id);
            }
        }

        None
    }

    /// Recommend reasoning pattern for a problem type
    pub fn recommend_pattern(&self, problem_type: u32) -> Option<u32> {
        let applicable: Vec<_> = self
            .patterns
            .iter()
            .filter(|p| p.applicable_to.contains(&problem_type))
            .collect();

        if applicable.is_empty() {
            return None;
        }

        // Choose pattern with best success rate
        applicable
            .iter()
            .max_by(|a, b| {
                let stats_a = self.pattern_stats.get(&a.id);
                let stats_b = self.pattern_stats.get(&b.id);

                let success_rate_a = stats_a
                    .map(|s| {
                        if s.uses > 0 {
                            s.successes as f64 / s.uses as f64
                        } else {
                            0.0
                        }
                    })
                    .unwrap_or(0.5);

                let success_rate_b = stats_b
                    .map(|s| {
                        if s.uses > 0 {
                            s.successes as f64 / s.uses as f64
                        } else {
                            0.0
                        }
                    })
                    .unwrap_or(0.5);

                success_rate_a.partial_cmp(&success_rate_b).unwrap()
            })
            .map(|p| p.id)
    }

    /// Get pattern efficiency (success / time)
    pub fn pattern_efficiency(&self, pattern_id: u32) -> f64 {
        self.pattern_stats
            .get(&pattern_id)
            .map(|s| {
                if s.total_time > 0 && s.uses > 0 {
                    (s.successes as f64 / s.uses as f64) / (s.total_time as f64 / s.uses as f64)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0)
    }
}

/// Complete metacognitive controller
pub struct MetacognitiveController {
    /// Introspection engine
    introspection: IntrospectionEngine,
    /// Confidence calibrator
    calibrator: LocalConfidenceCalibrator,
    /// Strategy selector
    strategy: LocalStrategySelector,
    /// Cognitive regulator
    regulator: CognitiveRegulator,
    /// Meta-reasoner
    reasoner: MetaReasoner,
    /// Current time
    time: u64,
}

impl MetacognitiveController {
    /// Create a new metacognitive controller
    pub fn new(num_strategies: u32, resource_budget: f64) -> Self {
        Self {
            introspection: IntrospectionEngine::new(),
            calibrator: LocalConfidenceCalibrator::new(),
            strategy: LocalStrategySelector::new(num_strategies),
            regulator: CognitiveRegulator::new(resource_budget),
            reasoner: MetaReasoner::new(),
            time: 0,
        }
    }

    /// Register a subsystem
    pub fn register_subsystem(&mut self, subsystem: SubsystemId) {
        self.regulator.register(subsystem, 0.01, 0.1);
    }

    /// Record a decision
    pub fn record_decision(&mut self, record: DecisionRecord) {
        let _subsystem = record.subsystem;
        let _confidence = record.confidence;

        self.introspection.record_decision(record);

        // Track for calibration (actual outcome unknown yet)
    }

    /// Record outcome of a decision
    pub fn record_outcome(
        &mut self,
        timestamp: u64,
        subsystem: SubsystemId,
        predicted: f64,
        actual: f64,
    ) {
        self.introspection
            .record_outcome(timestamp, subsystem, actual);

        // Add to calibration data
        let was_correct = (predicted - actual).abs() < 0.1;
        // Use predicted as confidence proxy
        self.calibrator.add_data(subsystem, predicted, was_correct);
    }

    /// Get calibrated confidence
    pub fn calibrate_confidence(&self, subsystem: SubsystemId, raw_confidence: f64) -> f64 {
        self.calibrator
            .calibrate_isotonic(subsystem, raw_confidence)
    }

    /// Select strategy
    pub fn select_strategy(&mut self) -> u32 {
        self.strategy.select()
    }

    /// Update strategy with reward
    pub fn update_strategy(&mut self, strategy: u32, reward: f64) {
        self.strategy.update(strategy, reward);
    }

    /// Regulate all subsystems
    pub fn regulate(&mut self) {
        let metrics = self.introspection.metrics.clone();
        self.regulator.regulate(&metrics);
    }

    /// Get learning rate for subsystem
    pub fn learning_rate(&self, subsystem: SubsystemId) -> f64 {
        self.regulator.learning_rate(subsystem)
    }

    /// Get exploration rate for subsystem
    pub fn exploration_rate(&self, subsystem: SubsystemId) -> f64 {
        self.regulator.exploration_rate(subsystem)
    }

    /// Get overall system health
    pub fn system_health(&self) -> MetacognitiveHealth {
        let accuracy = self.introspection.overall_accuracy();
        let load = self.introspection.cognitive_load();

        // Average ECE across subsystems
        let ece: f64 = self
            .introspection
            .metrics
            .keys()
            .map(|&s| self.calibrator.compute_ece(s))
            .sum::<f64>()
            / (self.introspection.metrics.len() as f64 + 1.0);

        MetacognitiveHealth {
            overall_accuracy: accuracy,
            cognitive_load: load,
            calibration_quality: 1.0 - ece,
            resource_efficiency: self.compute_efficiency(),
            self_awareness_level: self.compute_awareness(),
        }
    }

    fn compute_efficiency(&self) -> f64 {
        let metrics = &self.introspection.metrics;
        if metrics.is_empty() {
            return 0.0;
        }

        let total_work: f64 = metrics
            .values()
            .map(|m| m.correct as f64 / (m.resource_usage + 1.0))
            .sum();

        total_work / metrics.len() as f64
    }

    fn compute_awareness(&self) -> f64 {
        // Awareness = how well calibrated confidence matches actual performance
        let metrics = &self.introspection.metrics;
        if metrics.is_empty() {
            return 0.0;
        }

        let avg_cal_error: f64 =
            metrics.values().map(|m| m.calibration_error).sum::<f64>() / metrics.len() as f64;

        1.0 - avg_cal_error.min(1.0)
    }

    /// Recommend reasoning pattern
    pub fn recommend_reasoning(&self, problem_type: u32) -> Option<u32> {
        self.reasoner.recommend_pattern(problem_type)
    }

    /// Step time forward
    pub fn step(&mut self) {
        self.time += 1;

        // Periodic recalibration
        if self.time % 100 == 0 {
            for &subsystem in self.introspection.metrics.keys() {
                self.calibrator.fit_platt(subsystem);
                self.calibrator.fit_isotonic(subsystem);
            }
        }

        // Periodic regulation
        if self.time % 10 == 0 {
            self.regulate();
        }
    }
}

/// Metacognitive health metrics
#[derive(Debug, Clone, Default)]
pub struct MetacognitiveHealth {
    /// Overall decision accuracy
    pub overall_accuracy: f64,
    /// Current cognitive load
    pub cognitive_load: f64,
    /// Quality of confidence calibration (1 = perfect)
    pub calibration_quality: f64,
    /// Resource usage efficiency
    pub resource_efficiency: f64,
    /// Self-awareness level (how well the system knows itself)
    pub self_awareness_level: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspection() {
        let mut engine = IntrospectionEngine::new();

        let record = DecisionRecord {
            timestamp: 1,
            subsystem: SubsystemId::Scheduler,
            decision_type: DecisionType::Scheduling,
            input_summary: alloc::vec![0.5, 0.3],
            action: 1,
            predicted_outcome: 0.8,
            actual_outcome: None,
            confidence: 0.7,
        };

        engine.record_decision(record);
        engine.record_outcome(1, SubsystemId::Scheduler, 0.75);

        let metrics = engine.get_metrics(SubsystemId::Scheduler).unwrap();
        assert_eq!(metrics.decisions, 1);
    }

    #[test]
    fn test_confidence_calibration() {
        let mut calibrator = LocalConfidenceCalibrator::new();

        // Add data points
        for i in 0..100 {
            let conf = (i as f64) / 100.0;
            let correct = i > 50;
            calibrator.add_data(SubsystemId::Prediction, conf, correct);
        }

        calibrator.fit_platt(SubsystemId::Prediction);
        calibrator.fit_isotonic(SubsystemId::Prediction);

        let ece = calibrator.compute_ece(SubsystemId::Prediction);
        assert!(ece >= 0.0 && ece <= 1.0);
    }

    #[test]
    fn test_strategy_selector() {
        let mut selector = LocalStrategySelector::new(3);

        // Select and update
        for _ in 0..10 {
            let s = selector.select();
            let reward = if s == 1 { 1.0 } else { 0.5 };
            selector.update(s, reward);
        }

        // Strategy 1 should have better stats
        let (mean, _) = selector.stats(1).unwrap();
        assert!(mean > 0.0);
    }

    #[test]
    fn test_cognitive_regulator() {
        let mut regulator = CognitiveRegulator::new(100.0);

        regulator.register(SubsystemId::Scheduler, 0.01, 0.1);
        regulator.register(SubsystemId::Memory, 0.01, 0.1);

        let mut metrics = BTreeMap::new();
        metrics.insert(SubsystemId::Scheduler, SubsystemMetrics {
            decisions: 100,
            correct: 80,
            total_error: 5.0,
            avg_confidence: 0.7,
            calibration_error: 0.1,
            avg_response_time: 1.0,
            resource_usage: 10.0,
        });

        regulator.regulate(&metrics);

        assert!(regulator.resource_for(SubsystemId::Scheduler) > 0.0);
    }

    #[test]
    fn test_metacognitive_controller() {
        let mut controller = MetacognitiveController::new(3, 100.0);

        controller.register_subsystem(SubsystemId::Scheduler);
        controller.register_subsystem(SubsystemId::Memory);

        let record = DecisionRecord {
            timestamp: 1,
            subsystem: SubsystemId::Scheduler,
            decision_type: DecisionType::Scheduling,
            input_summary: alloc::vec![0.5],
            action: 1,
            predicted_outcome: 0.8,
            actual_outcome: None,
            confidence: 0.7,
        };

        controller.record_decision(record);
        controller.record_outcome(1, SubsystemId::Scheduler, 0.8, 0.75);

        controller.step();

        let health = controller.system_health();
        assert!(health.self_awareness_level >= 0.0);
    }

    #[test]
    fn test_meta_reasoner() {
        let mut reasoner = MetaReasoner::new();

        let trace = ReasoningTrace {
            timestamp: 1,
            problem_type: 1,
            steps: vec![
                ReasoningStep {
                    step_type: StepType::Observation,
                    input_size: 10,
                    output_size: 5,
                    time: 1,
                },
                ReasoningStep {
                    step_type: StepType::Search,
                    input_size: 5,
                    output_size: 3,
                    time: 5,
                },
                ReasoningStep {
                    step_type: StepType::Comparison,
                    input_size: 3,
                    output_size: 1,
                    time: 2,
                },
                ReasoningStep {
                    step_type: StepType::Decision,
                    input_size: 1,
                    output_size: 1,
                    time: 1,
                },
            ],
            outcome: 0.9,
            time_taken: 9,
        };

        reasoner.record_trace(trace);

        let recommended = reasoner.recommend_pattern(1);
        assert!(recommended.is_some());
    }
}
