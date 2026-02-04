//! # Reinforcement Learning for NEXUS
//!
//! Q-learning, policy gradient, actor-critic algorithms for kernel decision-making.
//!
//! ## Features
//!
//! - Tabular Q-learning with eligibility traces
//! - Function approximation with linear models
//! - Policy gradient (REINFORCE)
//! - Actor-Critic architecture
//! - Experience replay buffer
//! - Epsilon-greedy and softmax exploration

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::math::F64Ext;

// ============================================================================
// STATE AND ACTION SPACES
// ============================================================================

/// State identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(pub u32);

/// Action identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub u32);

/// State space definition
#[derive(Debug, Clone)]
pub struct StateSpace {
    /// States in the space
    states: Vec<StateId>,
    /// State names
    names: BTreeMap<StateId, String>,
    /// State features
    features: BTreeMap<StateId, Vec<f64>>,
    /// Feature dimension
    feature_dim: usize,
}

impl StateSpace {
    /// Create new state space
    pub fn new(feature_dim: usize) -> Self {
        Self {
            states: Vec::new(),
            names: BTreeMap::new(),
            features: BTreeMap::new(),
            feature_dim,
        }
    }

    /// Add state
    pub fn add_state(&mut self, id: StateId, name: String, features: Vec<f64>) {
        self.states.push(id);
        self.names.insert(id, name);
        if features.len() == self.feature_dim {
            self.features.insert(id, features);
        }
    }

    /// Get state count
    pub fn size(&self) -> usize {
        self.states.len()
    }

    /// Get features for state
    pub fn get_features(&self, state: StateId) -> Option<&Vec<f64>> {
        self.features.get(&state)
    }

    /// Get all states
    pub fn states(&self) -> &[StateId] {
        &self.states
    }
}

/// Action space definition
#[derive(Debug, Clone)]
pub struct ActionSpace {
    /// Actions in the space
    actions: Vec<ActionId>,
    /// Action names
    names: BTreeMap<ActionId, String>,
    /// Action costs
    costs: BTreeMap<ActionId, f64>,
}

impl ActionSpace {
    /// Create new action space
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            names: BTreeMap::new(),
            costs: BTreeMap::new(),
        }
    }

    /// Add action
    pub fn add_action(&mut self, id: ActionId, name: String, cost: f64) {
        self.actions.push(id);
        self.names.insert(id, name);
        self.costs.insert(id, cost);
    }

    /// Get action count
    pub fn size(&self) -> usize {
        self.actions.len()
    }

    /// Get action cost
    pub fn get_cost(&self, action: ActionId) -> f64 {
        self.costs.get(&action).copied().unwrap_or(0.0)
    }

    /// Get all actions
    pub fn actions(&self) -> &[ActionId] {
        &self.actions
    }
}

impl Default for ActionSpace {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REWARD SIGNAL
// ============================================================================

/// Reward signal for RL
#[derive(Debug, Clone)]
pub struct RewardSignal {
    /// Immediate reward
    pub immediate: f64,
    /// Shaping reward (helps guide learning)
    pub shaping: f64,
    /// Intrinsic motivation reward
    pub intrinsic: f64,
    /// Penalty for constraint violations
    pub penalty: f64,
}

impl RewardSignal {
    /// Create simple reward
    pub fn simple(reward: f64) -> Self {
        Self {
            immediate: reward,
            shaping: 0.0,
            intrinsic: 0.0,
            penalty: 0.0,
        }
    }

    /// Get total reward
    pub fn total(&self) -> f64 {
        self.immediate + self.shaping + self.intrinsic - self.penalty
    }

    /// Add shaping
    pub fn with_shaping(mut self, shaping: f64) -> Self {
        self.shaping = shaping;
        self
    }

    /// Add intrinsic motivation
    pub fn with_intrinsic(mut self, intrinsic: f64) -> Self {
        self.intrinsic = intrinsic;
        self
    }

    /// Add penalty
    pub fn with_penalty(mut self, penalty: f64) -> Self {
        self.penalty = penalty;
        self
    }
}

// ============================================================================
// EXPERIENCE REPLAY
// ============================================================================

/// Experience tuple
#[derive(Debug, Clone)]
pub struct Experience {
    /// Current state
    pub state: StateId,
    /// Action taken
    pub action: ActionId,
    /// Reward received
    pub reward: f64,
    /// Next state
    pub next_state: StateId,
    /// Is terminal?
    pub done: bool,
}

/// Episode (sequence of experiences)
#[derive(Debug, Clone)]
pub struct Episode {
    /// Experiences in this episode
    pub experiences: Vec<Experience>,
    /// Total return
    pub total_return: f64,
    /// Episode length
    pub length: usize,
}

impl Episode {
    /// Create new episode
    pub fn new() -> Self {
        Self {
            experiences: Vec::new(),
            total_return: 0.0,
            length: 0,
        }
    }

    /// Add experience
    pub fn add(&mut self, exp: Experience) {
        self.total_return += exp.reward;
        self.length += 1;
        self.experiences.push(exp);
    }

    /// Calculate discounted returns
    pub fn calculate_returns(&self, gamma: f64) -> Vec<f64> {
        let n = self.experiences.len();
        let mut returns = vec![0.0; n];

        if n == 0 {
            return returns;
        }

        returns[n - 1] = self.experiences[n - 1].reward;

        for i in (0..n - 1).rev() {
            returns[i] = self.experiences[i].reward + gamma * returns[i + 1];
        }

        returns
    }
}

impl Default for Episode {
    fn default() -> Self {
        Self::new()
    }
}

/// Experience replay buffer
#[derive(Debug)]
pub struct ReplayBuffer {
    /// Buffer capacity
    capacity: usize,
    /// Stored experiences
    buffer: Vec<Experience>,
    /// Write position
    position: usize,
}

impl ReplayBuffer {
    /// Create new replay buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    /// Add experience
    pub fn push(&mut self, exp: Experience) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(exp);
        } else {
            self.buffer[self.position] = exp;
        }
        self.position = (self.position + 1) % self.capacity;
    }

    /// Sample batch (simple uniform sampling without external RNG)
    pub fn sample(&self, batch_size: usize, seed: u64) -> Vec<Experience> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        let mut batch = Vec::with_capacity(batch_size);
        let mut rng = seed;

        for _ in 0..batch_size.min(self.buffer.len()) {
            // Simple LCG for randomness
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let idx = (rng >> 33) as usize % self.buffer.len();
            batch.push(self.buffer[idx].clone());
        }

        batch
    }

    /// Get buffer size
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

// ============================================================================
// Q-LEARNING
// ============================================================================

/// Q-Learning configuration
#[derive(Debug, Clone)]
pub struct QLearnerConfig {
    /// Learning rate (alpha)
    pub learning_rate: f64,
    /// Discount factor (gamma)
    pub discount: f64,
    /// Exploration rate (epsilon)
    pub epsilon: f64,
    /// Epsilon decay rate
    pub epsilon_decay: f64,
    /// Minimum epsilon
    pub epsilon_min: f64,
    /// Eligibility trace decay (lambda)
    pub trace_decay: f64,
}

impl Default for QLearnerConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount: 0.99,
            epsilon: 1.0,
            epsilon_decay: 0.995,
            epsilon_min: 0.01,
            trace_decay: 0.0, // No traces by default
        }
    }
}

/// Tabular Q-Learner
pub struct QLearner {
    /// Configuration
    config: QLearnerConfig,
    /// Q-table: (state, action) -> value
    q_table: BTreeMap<(StateId, ActionId), f64>,
    /// Eligibility traces
    traces: BTreeMap<(StateId, ActionId), f64>,
    /// Action space
    action_space: ActionSpace,
    /// Training steps
    steps: u64,
    /// Episodes completed
    episodes: u64,
}

impl QLearner {
    /// Create new Q-learner
    pub fn new(config: QLearnerConfig, action_space: ActionSpace) -> Self {
        Self {
            config,
            q_table: BTreeMap::new(),
            traces: BTreeMap::new(),
            action_space,
            steps: 0,
            episodes: 0,
        }
    }

    /// Get Q-value
    pub fn get_q(&self, state: StateId, action: ActionId) -> f64 {
        self.q_table.get(&(state, action)).copied().unwrap_or(0.0)
    }

    /// Set Q-value
    pub fn set_q(&mut self, state: StateId, action: ActionId, value: f64) {
        self.q_table.insert((state, action), value);
    }

    /// Select action (epsilon-greedy)
    pub fn select_action(&self, state: StateId, seed: u64) -> ActionId {
        let mut rng = seed.wrapping_mul(self.steps + 1);
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let random_val = (rng >> 32) as f64 / u32::MAX as f64;

        if random_val < self.config.epsilon {
            // Explore: random action
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let idx = (rng >> 33) as usize % self.action_space.size();
            self.action_space.actions()[idx]
        } else {
            // Exploit: best action
            self.best_action(state)
        }
    }

    /// Get best action for state
    pub fn best_action(&self, state: StateId) -> ActionId {
        let actions = self.action_space.actions();
        if actions.is_empty() {
            return ActionId(0);
        }

        let mut best = actions[0];
        let mut best_q = self.get_q(state, best);

        for &action in &actions[1..] {
            let q = self.get_q(state, action);
            if q > best_q {
                best_q = q;
                best = action;
            }
        }

        best
    }

    /// Get max Q-value for state
    pub fn max_q(&self, state: StateId) -> f64 {
        let best = self.best_action(state);
        self.get_q(state, best)
    }

    /// Update Q-value from experience
    pub fn update(&mut self, exp: &Experience) {
        // TD error
        let target = if exp.done {
            exp.reward
        } else {
            exp.reward + self.config.discount * self.max_q(exp.next_state)
        };

        let current = self.get_q(exp.state, exp.action);
        let td_error = target - current;

        if self.config.trace_decay > 0.0 {
            // Q(λ) with eligibility traces
            // Update trace for current state-action
            let trace = self.traces.entry((exp.state, exp.action)).or_insert(0.0);
            *trace = 1.0; // Replacing traces

            // Update all Q-values
            let keys: Vec<(StateId, ActionId)> = self.traces.keys().copied().collect();
            for key in keys {
                let trace_val = self.traces.get(&key).copied().unwrap_or(0.0);
                let old_q = self.get_q(key.0, key.1);
                self.set_q(
                    key.0,
                    key.1,
                    old_q + self.config.learning_rate * td_error * trace_val,
                );

                // Decay trace
                let new_trace = trace_val * self.config.discount * self.config.trace_decay;
                if new_trace < 0.001 {
                    self.traces.remove(&key);
                } else {
                    self.traces.insert(key, new_trace);
                }
            }
        } else {
            // Standard Q-learning
            let new_q = current + self.config.learning_rate * td_error;
            self.set_q(exp.state, exp.action, new_q);
        }

        self.steps += 1;
    }

    /// End episode (decay epsilon, clear traces)
    pub fn end_episode(&mut self) {
        self.episodes += 1;
        self.config.epsilon =
            (self.config.epsilon * self.config.epsilon_decay).max(self.config.epsilon_min);
        self.traces.clear();
    }

    /// Get current epsilon
    pub fn epsilon(&self) -> f64 {
        self.config.epsilon
    }

    /// Get training statistics
    pub fn stats(&self) -> QLearnerStats {
        QLearnerStats {
            steps: self.steps,
            episodes: self.episodes,
            epsilon: self.config.epsilon,
            q_table_size: self.q_table.len(),
        }
    }
}

/// Q-Learner statistics
#[derive(Debug, Clone)]
pub struct QLearnerStats {
    /// Total training steps
    pub steps: u64,
    /// Episodes completed
    pub episodes: u64,
    /// Current epsilon
    pub epsilon: f64,
    /// Q-table size
    pub q_table_size: usize,
}

// ============================================================================
// POLICY GRADIENT
// ============================================================================

/// Policy gradient (REINFORCE) algorithm
pub struct PolicyGradient {
    /// Policy parameters (state features -> action logits)
    weights: Vec<Vec<f64>>,
    /// State feature dimension
    feature_dim: usize,
    /// Number of actions
    num_actions: usize,
    /// Learning rate
    learning_rate: f64,
    /// Discount factor
    discount: f64,
    /// Baseline (average return for variance reduction)
    baseline: f64,
    /// Baseline decay
    baseline_decay: f64,
    /// Training episodes
    episodes: u64,
}

impl PolicyGradient {
    /// Create new policy gradient learner
    pub fn new(feature_dim: usize, num_actions: usize, learning_rate: f64) -> Self {
        // Initialize weights to small random values (using deterministic initialization)
        let weights = (0..num_actions)
            .map(|a| {
                (0..feature_dim)
                    .map(|f| ((a * 31 + f * 17) % 100) as f64 / 1000.0 - 0.05)
                    .collect()
            })
            .collect();

        Self {
            weights,
            feature_dim,
            num_actions,
            learning_rate,
            discount: 0.99,
            baseline: 0.0,
            baseline_decay: 0.99,
            episodes: 0,
        }
    }

    /// Compute action probabilities using softmax
    pub fn action_probs(&self, state_features: &[f64]) -> Vec<f64> {
        if state_features.len() != self.feature_dim {
            return vec![1.0 / self.num_actions as f64; self.num_actions];
        }

        // Compute logits
        let logits: Vec<f64> = self
            .weights
            .iter()
            .map(|w| {
                w.iter()
                    .zip(state_features.iter())
                    .map(|(wi, fi)| wi * fi)
                    .sum::<f64>()
            })
            .collect();

        // Softmax
        let max_logit = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp_logits: Vec<f64> = logits.iter().map(|l| (l - max_logit).exp()).collect();
        let sum: f64 = exp_logits.iter().sum();

        exp_logits.into_iter().map(|e| e / sum).collect()
    }

    /// Select action from policy
    pub fn select_action(&self, state_features: &[f64], seed: u64) -> usize {
        let probs = self.action_probs(state_features);

        // Sample from distribution
        let mut rng = seed.wrapping_mul(self.episodes + 1);
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let random_val = (rng >> 32) as f64 / u32::MAX as f64;

        let mut cumsum = 0.0;
        for (i, &p) in probs.iter().enumerate() {
            cumsum += p;
            if random_val < cumsum {
                return i;
            }
        }

        self.num_actions - 1
    }

    /// Update policy from episode
    pub fn update(&mut self, episode: &Episode, state_features: &[Vec<f64>]) {
        if episode.experiences.is_empty() || state_features.len() != episode.experiences.len() {
            return;
        }

        // Calculate discounted returns
        let returns = episode.calculate_returns(self.discount);

        // Update baseline
        let avg_return = episode.total_return / episode.length as f64;
        self.baseline =
            self.baseline_decay * self.baseline + (1.0 - self.baseline_decay) * avg_return;

        // Policy gradient update
        for (i, exp) in episode.experiences.iter().enumerate() {
            let features = &state_features[i];
            let probs = self.action_probs(features);
            let action = exp.action.0 as usize;

            // Advantage = return - baseline (for variance reduction)
            let advantage = returns[i] - self.baseline;

            // Gradient: ∂log π(a|s) / ∂θ = features (for linear policy)
            // For softmax: ∂log π(a|s) / ∂θ_a = features * (1 - π(a|s))
            //              ∂log π(a|s) / ∂θ_b = features * (-π(b|s)) for b ≠ a

            for a in 0..self.num_actions {
                for f in 0..self.feature_dim {
                    let grad = if a == action {
                        features[f] * (1.0 - probs[a])
                    } else {
                        features[f] * (-probs[a])
                    };

                    self.weights[a][f] += self.learning_rate * advantage * grad;
                }
            }
        }

        self.episodes += 1;
    }

    /// Get policy parameters
    pub fn get_weights(&self) -> &Vec<Vec<f64>> {
        &self.weights
    }

    /// Set learning rate
    pub fn set_learning_rate(&mut self, lr: f64) {
        self.learning_rate = lr;
    }
}

// ============================================================================
// ACTOR-CRITIC
// ============================================================================

/// Actor-Critic algorithm
pub struct ActorCritic {
    /// Actor (policy)
    actor: PolicyGradient,
    /// Critic weights (state features -> value)
    critic_weights: Vec<f64>,
    /// State feature dimension
    feature_dim: usize,
    /// Critic learning rate
    critic_lr: f64,
    /// Discount factor
    discount: f64,
    /// Training steps
    steps: u64,
}

impl ActorCritic {
    /// Create new actor-critic
    pub fn new(feature_dim: usize, num_actions: usize, actor_lr: f64, critic_lr: f64) -> Self {
        Self {
            actor: PolicyGradient::new(feature_dim, num_actions, actor_lr),
            critic_weights: vec![0.0; feature_dim],
            feature_dim,
            critic_lr,
            discount: 0.99,
            steps: 0,
        }
    }

    /// Get state value from critic
    pub fn value(&self, state_features: &[f64]) -> f64 {
        if state_features.len() != self.feature_dim {
            return 0.0;
        }

        self.critic_weights
            .iter()
            .zip(state_features.iter())
            .map(|(w, f)| w * f)
            .sum()
    }

    /// Select action
    pub fn select_action(&self, state_features: &[f64], seed: u64) -> usize {
        self.actor.select_action(state_features, seed)
    }

    /// Update from single transition (online)
    pub fn update(
        &mut self,
        state_features: &[f64],
        action: usize,
        reward: f64,
        next_features: &[f64],
        done: bool,
    ) {
        // TD error for critic
        let current_value = self.value(state_features);
        let next_value = if done { 0.0 } else { self.value(next_features) };
        let td_error = reward + self.discount * next_value - current_value;

        // Update critic
        for i in 0..self.feature_dim {
            self.critic_weights[i] += self.critic_lr * td_error * state_features[i];
        }

        // Update actor using TD error as advantage
        let probs = self.actor.action_probs(state_features);

        for a in 0..self.actor.num_actions {
            for f in 0..self.feature_dim {
                let grad = if a == action {
                    state_features[f] * (1.0 - probs[a])
                } else {
                    state_features[f] * (-probs[a])
                };

                self.actor.weights[a][f] += self.actor.learning_rate * td_error * grad;
            }
        }

        self.steps += 1;
    }

    /// Get action probabilities
    pub fn action_probs(&self, state_features: &[f64]) -> Vec<f64> {
        self.actor.action_probs(state_features)
    }

    /// Get training steps
    pub fn steps(&self) -> u64 {
        self.steps
    }
}

// ============================================================================
// KERNEL RL AGENT
// ============================================================================

/// RL agent for kernel resource management
pub struct KernelRLAgent {
    /// Q-learner for discrete decisions
    q_learner: QLearner,
    /// State encoder
    state_encoder: KernelStateEncoder,
    /// Action decoder
    action_decoder: KernelActionDecoder,
    /// Reward shaper
    reward_shaper: KernelRewardShaper,
}

/// Kernel state encoder
pub struct KernelStateEncoder {
    /// State ID counter
    next_id: u32,
    /// State cache
    cache: BTreeMap<KernelState, StateId>,
}

/// Kernel state representation
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelState {
    /// CPU load bucket (0-10)
    pub cpu_bucket: u8,
    /// Memory pressure bucket (0-10)
    pub memory_bucket: u8,
    /// IO wait bucket (0-10)
    pub io_bucket: u8,
    /// Number of runnable tasks bucket
    pub task_bucket: u8,
}

impl KernelStateEncoder {
    /// Create new encoder
    pub fn new() -> Self {
        Self {
            next_id: 0,
            cache: BTreeMap::new(),
        }
    }

    /// Encode raw metrics to state
    pub fn encode(
        &mut self,
        cpu_load: f64,
        memory_usage: f64,
        io_wait: f64,
        tasks: u32,
    ) -> StateId {
        let state = KernelState {
            cpu_bucket: ((cpu_load * 10.0).clamp(0.0, 10.0)) as u8,
            memory_bucket: ((memory_usage * 10.0).clamp(0.0, 10.0)) as u8,
            io_bucket: ((io_wait * 10.0).clamp(0.0, 10.0)) as u8,
            task_bucket: (tasks.min(100) / 10) as u8,
        };

        if let Some(&id) = self.cache.get(&state) {
            id
        } else {
            let id = StateId(self.next_id);
            self.next_id += 1;
            self.cache.insert(state, id);
            id
        }
    }
}

impl Default for KernelStateEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel action decoder
pub struct KernelActionDecoder {
    /// Actions
    actions: Vec<KernelAction>,
}

/// Kernel action
#[derive(Debug, Clone)]
pub enum KernelAction {
    /// Do nothing
    NoOp,
    /// Increase scheduler priority for tasks
    IncreasePriority,
    /// Decrease scheduler priority
    DecreasePriority,
    /// Trigger garbage collection
    TriggerGC,
    /// Compact memory
    CompactMemory,
    /// Prefetch pages
    PrefetchPages,
    /// Throttle IO
    ThrottleIO,
    /// Boost IO
    BoostIO,
}

impl KernelActionDecoder {
    /// Create decoder with standard actions
    pub fn new() -> Self {
        Self {
            actions: vec![
                KernelAction::NoOp,
                KernelAction::IncreasePriority,
                KernelAction::DecreasePriority,
                KernelAction::TriggerGC,
                KernelAction::CompactMemory,
                KernelAction::PrefetchPages,
                KernelAction::ThrottleIO,
                KernelAction::BoostIO,
            ],
        }
    }

    /// Decode action ID to action
    pub fn decode(&self, action: ActionId) -> KernelAction {
        let idx = action.0 as usize;
        if idx < self.actions.len() {
            self.actions[idx].clone()
        } else {
            KernelAction::NoOp
        }
    }

    /// Get action space
    pub fn action_space(&self) -> ActionSpace {
        let mut space = ActionSpace::new();
        for (i, action) in self.actions.iter().enumerate() {
            let name = alloc::format!("{:?}", action);
            space.add_action(ActionId(i as u32), name, 0.0);
        }
        space
    }
}

impl Default for KernelActionDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel reward shaper
pub struct KernelRewardShaper {
    /// Target CPU utilization
    target_cpu: f64,
    /// Target memory usage
    target_memory: f64,
    /// Latency weight
    latency_weight: f64,
    /// Throughput weight
    throughput_weight: f64,
}

impl KernelRewardShaper {
    /// Create new reward shaper
    pub fn new() -> Self {
        Self {
            target_cpu: 0.7,
            target_memory: 0.6,
            latency_weight: 0.5,
            throughput_weight: 0.5,
        }
    }

    /// Shape reward from metrics
    pub fn shape(
        &self,
        cpu_load: f64,
        memory_usage: f64,
        latency_ms: f64,
        throughput: f64,
    ) -> RewardSignal {
        // CPU reward: closer to target is better
        let cpu_diff = (cpu_load - self.target_cpu).abs();
        let cpu_reward = 1.0 - cpu_diff;

        // Memory reward
        let mem_diff = (memory_usage - self.target_memory).abs();
        let mem_reward = 1.0 - mem_diff;

        // Latency penalty (lower is better)
        let latency_penalty = (latency_ms / 100.0).min(1.0) * self.latency_weight;

        // Throughput reward (higher is better, normalized)
        let throughput_reward = (throughput / 1000.0).min(1.0) * self.throughput_weight;

        let immediate = (cpu_reward + mem_reward) / 2.0 + throughput_reward;

        RewardSignal {
            immediate,
            shaping: 0.0,
            intrinsic: 0.0,
            penalty: latency_penalty,
        }
    }
}

impl Default for KernelRewardShaper {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelRLAgent {
    /// Create new kernel RL agent
    pub fn new() -> Self {
        let action_decoder = KernelActionDecoder::new();
        let action_space = action_decoder.action_space();

        Self {
            q_learner: QLearner::new(QLearnerConfig::default(), action_space),
            state_encoder: KernelStateEncoder::new(),
            action_decoder,
            reward_shaper: KernelRewardShaper::new(),
        }
    }

    /// Observe current state and select action
    pub fn decide(
        &mut self,
        cpu_load: f64,
        memory_usage: f64,
        io_wait: f64,
        tasks: u32,
        seed: u64,
    ) -> KernelAction {
        let state = self
            .state_encoder
            .encode(cpu_load, memory_usage, io_wait, tasks);
        let action_id = self.q_learner.select_action(state, seed);
        self.action_decoder.decode(action_id)
    }

    /// Learn from experience
    pub fn learn(
        &mut self,
        prev_cpu: f64,
        prev_mem: f64,
        prev_io: f64,
        prev_tasks: u32,
        action: ActionId,
        curr_cpu: f64,
        curr_mem: f64,
        curr_io: f64,
        curr_tasks: u32,
        latency: f64,
        throughput: f64,
    ) {
        let prev_state = self
            .state_encoder
            .encode(prev_cpu, prev_mem, prev_io, prev_tasks);
        let curr_state = self
            .state_encoder
            .encode(curr_cpu, curr_mem, curr_io, curr_tasks);
        let reward = self
            .reward_shaper
            .shape(curr_cpu, curr_mem, latency, throughput);

        let exp = Experience {
            state: prev_state,
            action,
            reward: reward.total(),
            next_state: curr_state,
            done: false,
        };

        self.q_learner.update(&exp);
    }

    /// End episode
    pub fn end_episode(&mut self) {
        self.q_learner.end_episode();
    }
}

impl Default for KernelRLAgent {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_space() {
        let mut space = ActionSpace::new();
        space.add_action(ActionId(0), String::from("up"), 1.0);
        space.add_action(ActionId(1), String::from("down"), 1.0);
        assert_eq!(space.size(), 2);
    }

    #[test]
    fn test_q_learner() {
        let mut space = ActionSpace::new();
        space.add_action(ActionId(0), String::from("left"), 0.0);
        space.add_action(ActionId(1), String::from("right"), 0.0);

        let mut learner = QLearner::new(QLearnerConfig::default(), space);

        let exp = Experience {
            state: StateId(0),
            action: ActionId(0),
            reward: 1.0,
            next_state: StateId(1),
            done: false,
        };

        learner.update(&exp);
        assert!(learner.get_q(StateId(0), ActionId(0)) > 0.0);
    }

    #[test]
    fn test_episode_returns() {
        let mut episode = Episode::new();

        episode.add(Experience {
            state: StateId(0),
            action: ActionId(0),
            reward: 1.0,
            next_state: StateId(1),
            done: false,
        });
        episode.add(Experience {
            state: StateId(1),
            action: ActionId(0),
            reward: 2.0,
            next_state: StateId(2),
            done: false,
        });
        episode.add(Experience {
            state: StateId(2),
            action: ActionId(0),
            reward: 3.0,
            next_state: StateId(3),
            done: true,
        });

        let returns = episode.calculate_returns(0.9);
        assert_eq!(returns.len(), 3);
        // G_2 = 3.0
        // G_1 = 2.0 + 0.9 * 3.0 = 4.7
        // G_0 = 1.0 + 0.9 * 4.7 = 5.23
        assert!((returns[2] - 3.0).abs() < 0.01);
        assert!((returns[1] - 4.7).abs() < 0.01);
        assert!((returns[0] - 5.23).abs() < 0.01);
    }

    #[test]
    fn test_replay_buffer() {
        let mut buffer = ReplayBuffer::new(100);

        for i in 0..10 {
            buffer.push(Experience {
                state: StateId(i),
                action: ActionId(0),
                reward: i as f64,
                next_state: StateId(i + 1),
                done: false,
            });
        }

        assert_eq!(buffer.len(), 10);

        let batch = buffer.sample(5, 42);
        assert_eq!(batch.len(), 5);
    }

    #[test]
    fn test_policy_gradient() {
        let mut pg = PolicyGradient::new(4, 2, 0.01);

        let features = vec![1.0, 0.0, 0.5, 0.5];
        let probs = pg.action_probs(&features);

        assert_eq!(probs.len(), 2);
        assert!((probs[0] + probs[1] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_kernel_rl_agent() {
        let mut agent = KernelRLAgent::new();

        let action = agent.decide(0.5, 0.4, 0.1, 10, 42);
        // Should return some action
        match action {
            KernelAction::NoOp
            | KernelAction::IncreasePriority
            | KernelAction::DecreasePriority
            | KernelAction::TriggerGC
            | KernelAction::CompactMemory
            | KernelAction::PrefetchPages
            | KernelAction::ThrottleIO
            | KernelAction::BoostIO => {},
        }
    }

    #[test]
    fn test_reward_shaping() {
        let shaper = KernelRewardShaper::new();

        // Good state: close to targets
        let reward1 = shaper.shape(0.7, 0.6, 10.0, 500.0);

        // Bad state: high load, high latency
        let reward2 = shaper.shape(1.0, 0.9, 500.0, 100.0);

        assert!(reward1.total() > reward2.total());
    }
}
