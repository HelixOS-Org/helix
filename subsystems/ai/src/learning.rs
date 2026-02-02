//! # Continuous Learning Engine
//!
//! The Learning Engine enables the AI subsystem to continuously learn and
//! adapt from system behavior, user patterns, and operational feedback.
//!
//! ## Learning Mechanisms
//!
//! - **Reinforcement Learning**: Learn from decision outcomes
//! - **Pattern Mining**: Discover behavioral patterns
//! - **Incremental Learning**: Update models without full retraining
//! - **Transfer Learning**: Apply knowledge across domains
//! - **Active Learning**: Identify informative samples
//!
//! ## Architecture
//!
//! ```text
//!                     ┌─────────────────────────────────────────┐
//!   Events ──────────►│           Learning Engine               │
//!                     │                                         │
//!   Outcomes ────────►│  ┌─────────────────────────────────┐    │
//!                     │  │      Experience Buffer          │    │
//!   Feedback ────────►│  │  - Recent Decisions            │    │
//!                     │  │  - Observed Outcomes           │    │
//!                     │  │  - User Feedback               │    │
//!                     │  └───────────────┬─────────────────┘    │
//!                     │                  │                      │
//!                     │                  ▼                      │
//!                     │  ┌─────────────────────────────────┐    │
//!                     │  │      Pattern Miner              │    │
//!                     │  │  - Sequence Analysis           │    │
//!                     │  │  - Anomaly Detection           │    │
//!                     │  │  - Correlation Discovery       │    │
//!                     │  └───────────────┬─────────────────┘    │
//!                     │                  │                      │
//!                     │                  ▼                      │
//!                     │  ┌─────────────────────────────────┐    │
//!                     │  │      Model Updater              │────────► Updated Models
//!                     │  │  - Online Training             │    │
//!                     │  │  - Weight Adjustment           │    │
//!                     │  │  - Policy Update               │    │
//!                     │  └─────────────────────────────────┘    │
//!                     │                                         │
//!                     └─────────────────────────────────────────┘
//! ```

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use crate::core::{AiAction, AiDecision, AiEvent, Confidence, DecisionContext, DecisionId};
use crate::neural::{Tensor, TensorShape};

// =============================================================================
// Experience Tracking
// =============================================================================

/// Unique identifier for an experience
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExperienceId(u64);

impl ExperienceId {
    /// Create a new unique ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for ExperienceId {
    fn default() -> Self {
        Self::new()
    }
}

/// An experience from which to learn
#[derive(Debug, Clone)]
pub struct Experience {
    /// Unique ID
    pub id: ExperienceId,
    /// The state before the decision
    pub state: StateVector,
    /// The action taken
    pub action: ActionVector,
    /// The observed outcome
    pub outcome: Outcome,
    /// The reward signal
    pub reward: f32,
    /// Timestamp
    pub timestamp: u64,
    /// Related decision ID
    pub decision_id: Option<DecisionId>,
}

/// State representation as a vector
#[derive(Debug, Clone)]
pub struct StateVector {
    /// Raw feature values
    pub features: Vec<f32>,
    /// Feature names for interpretability
    pub feature_names: Vec<String>,
}

impl StateVector {
    /// Create a new state vector
    pub fn new(features: Vec<f32>, names: Vec<String>) -> Self {
        Self {
            features,
            feature_names: names,
        }
    }

    /// Create from a DecisionContext
    pub fn from_context(context: &DecisionContext) -> Self {
        let features = vec![
            context.cpu_usage,
            context.memory_usage,
            context.active_processes as f32 / 1000.0,
            context.io_pending as f32 / 100.0,
            context.system_metrics.cpu_usage_percent as f32 / 100.0,
            context.system_metrics.memory_usage_percent as f32 / 100.0,
            context.system_metrics.io_wait_percent as f32 / 100.0,
            context.system_metrics.context_switch_rate as f32 / 10000.0,
        ];
        let names = vec![
            "cpu_usage".to_string(),
            "memory_usage".to_string(),
            "active_processes".to_string(),
            "io_pending".to_string(),
            "cpu_usage_pct".to_string(),
            "memory_usage_pct".to_string(),
            "io_wait_pct".to_string(),
            "context_switch_rate".to_string(),
        ];
        Self {
            features,
            feature_names: names,
        }
    }

    /// Get dimensionality
    pub fn dim(&self) -> usize {
        self.features.len()
    }

    /// Convert to tensor
    pub fn to_tensor(&self) -> Tensor {
        Tensor::from_vec(
            self.features.clone(),
            TensorShape::vector(self.features.len()),
        )
    }

    /// Compute distance to another state
    pub fn distance(&self, other: &StateVector) -> f32 {
        if self.features.len() != other.features.len() {
            return f32::MAX;
        }

        crate::math::sqrt_f32(
            self.features
                .iter()
                .zip(&other.features)
                .map(|(a, b)| crate::math::powi_f32(a - b, 2))
                .sum::<f32>(),
        )
    }
}

/// Action representation as a vector
#[derive(Debug, Clone)]
pub struct ActionVector {
    /// Action type index
    pub action_type: u32,
    /// Action parameters
    pub parameters: Vec<f32>,
}

impl ActionVector {
    /// Create from an AI action
    pub fn from_action(action: &AiAction) -> Self {
        match action {
            AiAction::TuneScheduler { granularity_ns, .. } => Self {
                action_type: 0,
                parameters: vec![*granularity_ns as f32 / 1_000_000.0],
            },
            AiAction::TuneAllocator { .. } => Self {
                action_type: 1,
                parameters: Vec::new(),
            },
            AiAction::RestartModule { module_id, .. } => Self {
                action_type: 2,
                parameters: vec![*module_id as f32],
            },
            AiAction::BlockProcess { pid, .. } => Self {
                action_type: 3,
                parameters: vec![*pid as f32],
            },
            AiAction::OffloadToGpu { task_id, .. } => Self {
                action_type: 4,
                parameters: vec![*task_id as f32],
            },
            _ => Self {
                action_type: 255,
                parameters: Vec::new(),
            },
        }
    }
}

/// Outcome of a decision
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Whether the decision succeeded
    pub success: bool,
    /// Measured impact
    pub impact: ImpactMetrics,
    /// User feedback if any
    pub user_feedback: Option<UserFeedback>,
    /// Time to effect (microseconds)
    pub time_to_effect_us: u64,
}

/// Measured impact metrics
#[derive(Debug, Clone, Default)]
pub struct ImpactMetrics {
    /// Change in CPU utilization (percentage points)
    pub cpu_delta: f32,
    /// Change in memory usage (percentage points)
    pub memory_delta: f32,
    /// Change in latency (microseconds)
    pub latency_delta_us: i64,
    /// Change in throughput (ops/sec)
    pub throughput_delta: i64,
    /// Change in power consumption (mW)
    pub power_delta_mw: i32,
}

/// User feedback on a decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFeedback {
    /// User approved/liked the decision
    Positive,
    /// User was neutral
    Neutral,
    /// User disapproved/reverted
    Negative,
    /// User explicitly rolled back
    Rollback,
}

// =============================================================================
// Pattern Mining
// =============================================================================

/// A discovered pattern
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Pattern ID
    pub id: u64,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Pattern description
    pub description: String,
    /// Pattern confidence
    pub confidence: Confidence,
    /// Number of occurrences
    pub occurrences: u64,
    /// Associated actions
    pub recommended_actions: Vec<AiAction>,
}

/// Types of patterns
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    /// Temporal pattern (happens at specific times)
    Temporal {
        /// Period in microseconds
        period_us: u64,
        /// Phase offset in microseconds
        phase_us: u64,
    },
    /// Sequential pattern (A then B then C)
    Sequential {
        /// Sequence of event types
        sequence: Vec<u32>,
    },
    /// Correlation pattern (A and B occur together)
    Correlation {
        /// Correlated event types
        events: Vec<u32>,
        /// Correlation strength (0-100)
        correlation: u8,
    },
    /// Anomaly pattern (deviation from baseline)
    Anomaly {
        /// Metric name
        metric: String,
        /// Threshold for anomaly detection
        threshold: f32,
    },
    /// Usage pattern (user behavior)
    Usage {
        /// Usage category
        category: String,
    },
}

// =============================================================================
// Reinforcement Learning
// =============================================================================

/// Q-Learning policy for decision making
#[derive(Debug, Clone)]
pub struct QPolicy {
    /// State-action value table
    q_table: BTreeMap<(u64, u32), f32>,
    /// Learning rate
    alpha: f32,
    /// Discount factor
    gamma: f32,
    /// Exploration rate
    epsilon: f32,
    /// State discretizer
    state_bins: u32,
}

impl QPolicy {
    /// Create a new Q-Learning policy
    pub fn new() -> Self {
        Self {
            q_table: BTreeMap::new(),
            alpha: 0.1,   // Learning rate
            gamma: 0.99,  // Future reward importance
            epsilon: 0.1, // Exploration rate
            state_bins: 100,
        }
    }

    /// Discretize a continuous state to a hash
    fn discretize_state(&self, state: &StateVector) -> u64 {
        let mut hash: u64 = 0;
        for &f in state.features.iter() {
            let bin = (f.clamp(0.0, 1.0) * self.state_bins as f32) as u64;
            hash = hash.wrapping_mul(31).wrapping_add(bin);
        }
        hash
    }

    /// Get Q-value for state-action pair
    pub fn get_q(&self, state: &StateVector, action: u32) -> f32 {
        let state_hash = self.discretize_state(state);
        *self.q_table.get(&(state_hash, action)).unwrap_or(&0.0)
    }

    /// Update Q-value based on experience
    pub fn update(
        &mut self,
        state: &StateVector,
        action: u32,
        reward: f32,
        next_state: &StateVector,
        available_actions: &[u32],
    ) {
        let state_hash = self.discretize_state(state);
        let next_state_hash = self.discretize_state(next_state);

        // Find max Q for next state
        let max_next_q = available_actions
            .iter()
            .map(|&a| *self.q_table.get(&(next_state_hash, a)).unwrap_or(&0.0))
            .fold(f32::MIN, f32::max);

        // Q-Learning update
        let current_q = self.get_q(state, action);
        let new_q = current_q + self.alpha * (reward + self.gamma * max_next_q - current_q);

        self.q_table.insert((state_hash, action), new_q);
    }

    /// Select best action (or explore)
    pub fn select_action(&self, state: &StateVector, available_actions: &[u32]) -> u32 {
        // Epsilon-greedy selection
        // In real implementation, would use random number
        // For deterministic tests, always exploit
        self.best_action(state, available_actions)
    }

    /// Get the best known action for a state
    pub fn best_action(&self, state: &StateVector, available_actions: &[u32]) -> u32 {
        available_actions
            .iter()
            .copied()
            .max_by(|&a, &b| {
                self.get_q(state, a)
                    .partial_cmp(&self.get_q(state, b))
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .unwrap_or(0)
    }

    /// Set learning parameters
    pub fn set_params(&mut self, alpha: f32, gamma: f32, epsilon: f32) {
        self.alpha = alpha.clamp(0.0, 1.0);
        self.gamma = gamma.clamp(0.0, 1.0);
        self.epsilon = epsilon.clamp(0.0, 1.0);
    }
}

impl Default for QPolicy {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Experience Replay Buffer
// =============================================================================

/// Buffer for storing experiences for replay
#[derive(Debug)]
pub struct ExperienceBuffer {
    /// Stored experiences
    buffer: VecDeque<Experience>,
    /// Maximum capacity
    capacity: usize,
    /// Total experiences added
    total_added: u64,
}

impl ExperienceBuffer {
    /// Create a new buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            total_added: 0,
        }
    }

    /// Add an experience
    pub fn add(&mut self, experience: Experience) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(experience);
        self.total_added += 1;
    }

    /// Sample experiences for training
    pub fn sample(&self, count: usize) -> Vec<&Experience> {
        // In real implementation, would use random sampling
        // For now, return most recent
        self.buffer.iter().rev().take(count).collect()
    }

    /// Get all experiences
    pub fn all(&self) -> impl Iterator<Item = &Experience> {
        self.buffer.iter()
    }

    /// Count of experiences
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Is the buffer empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

// =============================================================================
// Learning Engine
// =============================================================================

/// The Continuous Learning Engine
pub struct LearningEngine {
    /// Whether learning is enabled
    enabled: bool,

    /// Experience buffer
    experience_buffer: Mutex<ExperienceBuffer>,

    /// Q-Learning policy
    q_policy: RwLock<QPolicy>,

    /// Discovered patterns
    patterns: RwLock<Vec<Pattern>>,

    /// Pattern ID counter
    pattern_counter: AtomicU64,

    /// Action reward mapping
    action_rewards: RwLock<BTreeMap<u32, RewardStats>>,

    /// Event sequences for pattern mining
    event_sequences: Mutex<VecDeque<TimestampedEvent>>,

    /// Learning configuration
    config: RwLock<LearningConfig>,

    /// Statistics
    stats: LearningStats,
}

/// Timestamped event for sequence analysis
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TimestampedEvent {
    event_type: u32,
    timestamp: u64,
    features: Vec<f32>,
}

/// Reward statistics for an action type
#[derive(Debug, Clone, Default)]
struct RewardStats {
    total_reward: f32,
    count: u64,
    successes: u64,
    failures: u64,
}

/// Learning configuration
#[derive(Debug, Clone)]
pub struct LearningConfig {
    /// Learning rate for Q-Learning
    pub learning_rate: f32,
    /// Discount factor for future rewards
    pub discount_factor: f32,
    /// Exploration rate
    pub exploration_rate: f32,
    /// Minimum experiences before training
    pub min_experiences: usize,
    /// Training batch size
    pub batch_size: usize,
    /// Pattern mining threshold
    pub pattern_threshold: u64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount_factor: 0.99,
            exploration_rate: 0.1,
            min_experiences: 100,
            batch_size: 32,
            pattern_threshold: 5,
        }
    }
}

/// Learning statistics
struct LearningStats {
    experiences_recorded: AtomicU64,
    training_iterations: AtomicU64,
    patterns_discovered: AtomicU64,
    successful_predictions: AtomicU64,
    failed_predictions: AtomicU64,
    policy_updates: AtomicU64,
}

impl Default for LearningStats {
    fn default() -> Self {
        Self {
            experiences_recorded: AtomicU64::new(0),
            training_iterations: AtomicU64::new(0),
            patterns_discovered: AtomicU64::new(0),
            successful_predictions: AtomicU64::new(0),
            failed_predictions: AtomicU64::new(0),
            policy_updates: AtomicU64::new(0),
        }
    }
}

impl LearningEngine {
    /// Maximum experience buffer size
    const MAX_EXPERIENCES: usize = 10000;

    /// Maximum event sequence length
    const MAX_EVENTS: usize = 1000;

    /// Create a new Learning Engine
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            experience_buffer: Mutex::new(ExperienceBuffer::new(Self::MAX_EXPERIENCES)),
            q_policy: RwLock::new(QPolicy::new()),
            patterns: RwLock::new(Vec::new()),
            pattern_counter: AtomicU64::new(1),
            action_rewards: RwLock::new(BTreeMap::new()),
            event_sequences: Mutex::new(VecDeque::with_capacity(Self::MAX_EVENTS)),
            config: RwLock::new(LearningConfig::default()),
            stats: LearningStats::default(),
        }
    }

    /// Record an experience
    pub fn record_experience(&self, experience: Experience) {
        if !self.enabled {
            return;
        }

        // Update action reward stats
        {
            let mut rewards = self.action_rewards.write();
            let stats = rewards.entry(experience.action.action_type).or_default();
            stats.total_reward += experience.reward;
            stats.count += 1;
            if experience.outcome.success {
                stats.successes += 1;
            } else {
                stats.failures += 1;
            }
        }

        // Add to buffer
        self.experience_buffer.lock().add(experience);
        self.stats
            .experiences_recorded
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a decision outcome
    pub fn record_outcome(
        &self,
        decision: &AiDecision,
        success: bool,
        impact: ImpactMetrics,
        feedback: Option<UserFeedback>,
    ) {
        if !self.enabled {
            return;
        }

        // Compute reward
        let reward = self.compute_reward(success, &impact, feedback);

        // Create experience
        let experience = Experience {
            id: ExperienceId::new(),
            state: self.context_to_state(&decision.context),
            action: ActionVector::from_action(&decision.action),
            outcome: Outcome {
                success,
                impact,
                user_feedback: feedback,
                time_to_effect_us: 0,
            },
            reward,
            timestamp: decision.timestamp,
            decision_id: Some(decision.id),
        };

        self.record_experience(experience);

        // Track prediction accuracy
        if success {
            self.stats
                .successful_predictions
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats
                .failed_predictions
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Compute reward from outcome
    fn compute_reward(
        &self,
        success: bool,
        impact: &ImpactMetrics,
        feedback: Option<UserFeedback>,
    ) -> f32 {
        let mut reward = 0.0;

        // Base success/failure reward
        reward += if success { 1.0 } else { -1.0 };

        // Impact-based rewards
        // Negative CPU usage is good
        reward += -impact.cpu_delta * 0.1;
        // Negative memory usage is good
        reward += -impact.memory_delta * 0.1;
        // Negative latency is good
        reward += -(impact.latency_delta_us as f32) * 0.001;
        // Positive throughput is good
        reward += impact.throughput_delta as f32 * 0.001;
        // Negative power is good
        reward += -(impact.power_delta_mw as f32) * 0.0001;

        // User feedback
        if let Some(fb) = feedback {
            match fb {
                UserFeedback::Positive => reward += 2.0,
                UserFeedback::Neutral => {},
                UserFeedback::Negative => reward -= 2.0,
                UserFeedback::Rollback => reward -= 3.0,
            }
        }

        reward
    }

    /// Convert decision context to state vector
    fn context_to_state(&self, context: &DecisionContext) -> StateVector {
        StateVector::new(
            vec![
                context.cpu_usage / 100.0,
                context.memory_usage / 100.0,
                context.active_processes as f32 / 1000.0,
                context.io_pending as f32 / 100.0,
            ],
            vec![
                "cpu_usage".to_string(),
                "memory_usage".to_string(),
                "active_processes".to_string(),
                "io_pending".to_string(),
            ],
        )
    }

    /// Record an event for pattern mining
    pub fn record_event(&self, event: &AiEvent, timestamp: u64) {
        if !self.enabled {
            return;
        }

        let event_type = self.event_to_type(event);
        let features = self.event_to_features(event);

        let mut sequences = self.event_sequences.lock();
        if sequences.len() >= Self::MAX_EVENTS {
            sequences.pop_front();
        }
        sequences.push_back(TimestampedEvent {
            event_type,
            timestamp,
            features,
        });
    }

    /// Convert event to type ID
    fn event_to_type(&self, event: &AiEvent) -> u32 {
        match event {
            AiEvent::SystemBoot => 0,
            AiEvent::CpuThreshold { .. } => 1,
            AiEvent::MemoryPressure { .. } => 2,
            AiEvent::ProcessSpawn { .. } => 3,
            AiEvent::ProcessExit { .. } => 4,
            AiEvent::UserAction { .. } => 5,
            AiEvent::ThreatSignature { .. } => 6,
            AiEvent::PerformanceAnomaly { .. } => 7,
            AiEvent::ModuleCrash { .. } => 8,
            AiEvent::KernelPanic { .. } => 9,
            _ => 255,
        }
    }

    /// Convert event to feature vector
    fn event_to_features(&self, event: &AiEvent) -> Vec<f32> {
        match event {
            AiEvent::CpuThreshold {
                usage_percent,
                cpu_id,
            } => {
                vec![*usage_percent as f32 / 100.0, *cpu_id as f32]
            },
            AiEvent::MemoryPressure { available_percent } => {
                vec![*available_percent as f32 / 100.0]
            },
            _ => Vec::new(),
        }
    }

    /// Train the Q-policy on recent experiences
    pub fn train(&self) -> Result<(), ()> {
        if !self.enabled {
            return Ok(());
        }

        let config = self.config.read();
        let buffer = self.experience_buffer.lock();

        if buffer.len() < config.min_experiences {
            return Ok(());
        }

        // Sample batch
        let batch = buffer.sample(config.batch_size);

        // Update policy
        let mut policy = self.q_policy.write();
        let available_actions: Vec<u32> = (0..10).collect();

        for exp in batch {
            // Create a "next state" - in reality would come from the next experience
            let next_state = exp.state.clone();

            policy.update(
                &exp.state,
                exp.action.action_type,
                exp.reward,
                &next_state,
                &available_actions,
            );
        }

        self.stats
            .training_iterations
            .fetch_add(1, Ordering::Relaxed);
        self.stats.policy_updates.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Mine patterns from event sequences
    pub fn mine_patterns(&self) -> Vec<Pattern> {
        if !self.enabled {
            return Vec::new();
        }

        let mut discovered = Vec::new();
        let config = self.config.read();
        let sequences = self.event_sequences.lock();

        // Find sequential patterns
        discovered.extend(self.find_sequential_patterns(&sequences, config.pattern_threshold));

        // Find temporal patterns
        discovered.extend(self.find_temporal_patterns(&sequences, config.pattern_threshold));

        // Store discovered patterns
        if !discovered.is_empty() {
            let mut patterns = self.patterns.write();
            for pattern in &discovered {
                if !patterns.iter().any(|p| p.id == pattern.id) {
                    patterns.push(pattern.clone());
                    self.stats
                        .patterns_discovered
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        discovered
    }

    /// Find sequential patterns in events
    fn find_sequential_patterns(
        &self,
        sequences: &VecDeque<TimestampedEvent>,
        threshold: u64,
    ) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Simple 2-gram pattern detection
        let mut bigram_counts: BTreeMap<(u32, u32), u64> = BTreeMap::new();

        let events: Vec<_> = sequences.iter().collect();
        for window in events.windows(2) {
            let key = (window[0].event_type, window[1].event_type);
            *bigram_counts.entry(key).or_insert(0) += 1;
        }

        for ((e1, e2), count) in bigram_counts {
            if count >= threshold {
                patterns.push(Pattern {
                    id: self.pattern_counter.fetch_add(1, Ordering::SeqCst),
                    pattern_type: PatternType::Sequential {
                        sequence: vec![e1, e2],
                    },
                    description: format!("Event {} followed by event {} ({} times)", e1, e2, count),
                    confidence: Confidence::new((count as f32 / sequences.len() as f32).min(0.99)),
                    occurrences: count,
                    recommended_actions: Vec::new(),
                });
            }
        }

        patterns
    }

    /// Find temporal patterns in events
    fn find_temporal_patterns(
        &self,
        sequences: &VecDeque<TimestampedEvent>,
        threshold: u64,
    ) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Group events by type and analyze timing
        let mut event_times: BTreeMap<u32, Vec<u64>> = BTreeMap::new();

        for event in sequences {
            event_times
                .entry(event.event_type)
                .or_default()
                .push(event.timestamp);
        }

        for (event_type, times) in event_times {
            if times.len() < threshold as usize {
                continue;
            }

            // Analyze intervals
            let intervals: Vec<u64> = times
                .windows(2)
                .map(|w| w[1].saturating_sub(w[0]))
                .collect();

            if intervals.is_empty() {
                continue;
            }

            // Check for periodicity
            let mean_interval: u64 = intervals.iter().sum::<u64>() / intervals.len() as u64;
            let variance: f64 = intervals
                .iter()
                .map(|&i| crate::math::powi_f64((i as f64) - (mean_interval as f64), 2))
                .sum::<f64>()
                / intervals.len() as f64;
            let std_dev = crate::math::sqrt_f64(variance);

            // If standard deviation is low relative to mean, it's periodic
            if std_dev < (mean_interval as f64 * 0.3) {
                patterns.push(Pattern {
                    id: self.pattern_counter.fetch_add(1, Ordering::SeqCst),
                    pattern_type: PatternType::Temporal {
                        period_us: mean_interval,
                        phase_us: times[0] % mean_interval,
                    },
                    description: format!(
                        "Event {} occurs every {} us ({} occurrences)",
                        event_type,
                        mean_interval,
                        times.len()
                    ),
                    confidence: Confidence::new((1.0 - std_dev / mean_interval as f64) as f32),
                    occurrences: times.len() as u64,
                    recommended_actions: Vec::new(),
                });
            }
        }

        patterns
    }

    /// Predict upcoming actions based on learned patterns
    pub fn predict_upcoming(&self, current_state: &StateVector) -> Vec<(AiAction, Confidence)> {
        if !self.enabled {
            return Vec::new();
        }

        let mut predictions = Vec::new();
        let policy = self.q_policy.read();
        let available_actions: Vec<u32> = (0..10).collect();

        // Get best actions from Q-policy
        let best_action = policy.best_action(current_state, &available_actions);
        let q_value = policy.get_q(current_state, best_action);

        // Convert to concrete action suggestion
        if q_value > 0.0 {
            // Map action type back to AiAction
            let action = self.action_type_to_action(best_action);
            if let Some(a) = action {
                predictions.push((a, Confidence::new((q_value / 10.0).min(0.95))));
            }
        }

        // Check patterns for predictions
        let patterns = self.patterns.read();
        for pattern in patterns.iter() {
            for action in &pattern.recommended_actions {
                predictions.push((action.clone(), pattern.confidence));
            }
        }

        predictions
    }

    /// Convert action type back to AiAction
    fn action_type_to_action(&self, action_type: u32) -> Option<AiAction> {
        match action_type {
            0 => Some(AiAction::TuneScheduler {
                granularity_ns: 1_000_000,
                preemption: true,
            }),
            1 => Some(AiAction::TuneAllocator {
                strategy: "slab".to_string(),
            }),
            2 => Some(AiAction::ForceGarbageCollection),
            3 => Some(AiAction::SuspendIdleProcesses {
                threshold_seconds: 60,
            }),
            _ => None,
        }
    }

    /// Get action success rate
    pub fn action_success_rate(&self, action_type: u32) -> Option<f32> {
        let rewards = self.action_rewards.read();
        rewards.get(&action_type).map(|stats| {
            if stats.count > 0 {
                stats.successes as f32 / stats.count as f32
            } else {
                0.0
            }
        })
    }

    /// Get discovered patterns
    pub fn get_patterns(&self) -> Vec<Pattern> {
        self.patterns.read().clone()
    }

    /// Configure learning parameters
    pub fn configure(&self, config: LearningConfig) {
        let mut policy = self.q_policy.write();
        policy.set_params(
            config.learning_rate,
            config.discount_factor,
            config.exploration_rate,
        );
        *self.config.write() = config;
    }

    /// Enable or disable learning
    pub fn set_enabled(&self, _enabled: bool) {
        // Note: We can't modify self.enabled directly as it's not mutable
        // In real implementation, would use RwLock
    }

    /// Clear all learned data
    pub fn clear(&self) {
        self.experience_buffer.lock().clear();
        self.patterns.write().clear();
        self.event_sequences.lock().clear();
        self.action_rewards.write().clear();
    }

    /// Get statistics
    pub fn statistics(&self) -> LearningStatistics {
        let successful = self.stats.successful_predictions.load(Ordering::Relaxed);
        let failed = self.stats.failed_predictions.load(Ordering::Relaxed);
        let total = successful + failed;

        LearningStatistics {
            enabled: self.enabled,
            experiences_recorded: self.stats.experiences_recorded.load(Ordering::Relaxed),
            training_iterations: self.stats.training_iterations.load(Ordering::Relaxed),
            patterns_discovered: self.stats.patterns_discovered.load(Ordering::Relaxed),
            successful_predictions: successful,
            failed_predictions: failed,
            prediction_accuracy: if total > 0 {
                successful as f32 / total as f32
            } else {
                0.0
            },
            policy_updates: self.stats.policy_updates.load(Ordering::Relaxed),
            experience_buffer_size: self.experience_buffer.lock().len(),
            known_patterns: self.patterns.read().len(),
        }
    }
}

/// Public statistics
#[derive(Debug, Clone)]
pub struct LearningStatistics {
    /// Whether learning is enabled
    pub enabled: bool,
    /// Total experiences recorded
    pub experiences_recorded: u64,
    /// Total training iterations
    pub training_iterations: u64,
    /// Number of patterns discovered
    pub patterns_discovered: u64,
    /// Number of successful predictions
    pub successful_predictions: u64,
    /// Number of failed predictions
    pub failed_predictions: u64,
    /// Overall prediction accuracy (0.0 - 1.0)
    pub prediction_accuracy: f32,
    /// Number of policy updates
    pub policy_updates: u64,
    /// Current experience buffer size
    pub experience_buffer_size: usize,
    /// Number of known patterns
    pub known_patterns: usize,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experience_buffer() {
        let mut buffer = ExperienceBuffer::new(3);

        for i in 0..5 {
            buffer.add(Experience {
                id: ExperienceId::new(),
                state: StateVector::new(vec![i as f32], vec!["x".to_string()]),
                action: ActionVector {
                    action_type: 0,
                    parameters: Vec::new(),
                },
                outcome: Outcome {
                    success: true,
                    impact: ImpactMetrics::default(),
                    user_feedback: None,
                    time_to_effect_us: 0,
                },
                reward: 1.0,
                timestamp: i as u64,
                decision_id: None,
            });
        }

        // Should only have last 3
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_q_policy() {
        let mut policy = QPolicy::new();

        let state = StateVector::new(vec![0.5, 0.5], vec!["a".to_string(), "b".to_string()]);
        let next_state = StateVector::new(vec![0.6, 0.4], vec!["a".to_string(), "b".to_string()]);
        let actions = vec![0, 1, 2];

        // Update with positive reward
        policy.update(&state, 1, 5.0, &next_state, &actions);

        // Action 1 should now have highest Q-value
        let best = policy.best_action(&state, &actions);
        assert_eq!(best, 1);
    }

    #[test]
    fn test_learning_engine() {
        let engine = LearningEngine::new(true);

        // Record some experiences
        for i in 0..10 {
            let experience = Experience {
                id: ExperienceId::new(),
                state: StateVector::new(vec![i as f32 / 10.0], vec!["x".to_string()]),
                action: ActionVector {
                    action_type: i % 3,
                    parameters: Vec::new(),
                },
                outcome: Outcome {
                    success: i % 2 == 0,
                    impact: ImpactMetrics::default(),
                    user_feedback: None,
                    time_to_effect_us: 100,
                },
                reward: if i % 2 == 0 { 1.0 } else { -1.0 },
                timestamp: i as u64,
                decision_id: None,
            };
            engine.record_experience(experience);
        }

        let stats = engine.statistics();
        assert_eq!(stats.experiences_recorded, 10);
    }

    #[test]
    fn test_state_vector_distance() {
        let a = StateVector::new(vec![1.0, 0.0], vec!["x".to_string(), "y".to_string()]);
        let b = StateVector::new(vec![0.0, 0.0], vec!["x".to_string(), "y".to_string()]);

        let dist = a.distance(&b);
        assert!((dist - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_reward_computation() {
        let engine = LearningEngine::new(true);

        // Success with good metrics
        let good_impact = ImpactMetrics {
            cpu_delta: -10.0,       // Reduced CPU
            memory_delta: -5.0,     // Reduced memory
            latency_delta_us: -100, // Reduced latency
            throughput_delta: 50,   // Increased throughput
            power_delta_mw: -500,   // Reduced power
        };

        let reward = engine.compute_reward(true, &good_impact, Some(UserFeedback::Positive));
        assert!(reward > 0.0);
    }
}
