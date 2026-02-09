//! # Self-Modification Learning
//!
//! Year 3 EVOLUTION - Adaptive learning for self-modifying code

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// LEARNING TYPES
// ============================================================================

/// Learning session ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub u64);

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl SessionId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(SESSION_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Experience ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExperienceId(pub u64);

static EXPERIENCE_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ExperienceId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(EXPERIENCE_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Learning configuration
#[derive(Debug, Clone)]
pub struct LearningConfig {
    /// Learning rate
    pub learning_rate: f64,
    /// Discount factor (gamma)
    pub discount: f64,
    /// Exploration rate (epsilon)
    pub exploration: f64,
    /// Batch size
    pub batch_size: usize,
    /// Memory capacity
    pub memory_capacity: usize,
    /// Update frequency
    pub update_frequency: usize,
    /// Momentum
    pub momentum: f64,
    /// Regularization
    pub regularization: f64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            discount: 0.99,
            exploration: 0.1,
            batch_size: 64,
            memory_capacity: 10000,
            update_frequency: 100,
            momentum: 0.9,
            regularization: 0.0001,
        }
    }
}

// ============================================================================
// EXPERIENCE
// ============================================================================

/// Experience (state, action, reward, next_state)
#[derive(Debug, Clone)]
pub struct Experience {
    /// ID
    pub id: ExperienceId,
    /// State
    pub state: State,
    /// Action taken
    pub action: Action,
    /// Reward received
    pub reward: f64,
    /// Next state
    pub next_state: State,
    /// Terminal flag
    pub terminal: bool,
    /// Timestamp
    pub timestamp: u64,
    /// Metadata
    pub metadata: BTreeMap<String, f64>,
}

/// State representation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct State {
    /// Feature vector
    pub features: Vec<f64>,
    /// State type
    pub state_type: StateType,
    /// Hash for deduplication
    pub hash: u64,
}

impl State {
    pub fn new(features: Vec<f64>) -> Self {
        let hash = Self::compute_hash(&features);
        Self {
            features,
            state_type: StateType::Continuous,
            hash,
        }
    }

    fn compute_hash(features: &[f64]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &f in features {
            let bits = f.to_bits();
            hash ^= bits;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Euclidean distance to another state
    #[inline]
    pub fn distance(&self, other: &State) -> f64 {
        self.features
            .iter()
            .zip(other.features.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    /// Cosine similarity
    pub fn similarity(&self, other: &State) -> f64 {
        let dot: f64 = self
            .features
            .iter()
            .zip(other.features.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f64 = self.features.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = other.features.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

        if norm_a * norm_b < 1e-10 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }
}

/// State type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateType {
    Discrete,
    Continuous,
    Mixed,
    Image,
    Sequence,
}

/// Action representation
#[derive(Debug, Clone)]
pub struct Action {
    /// Action ID
    pub id: u64,
    /// Action value (for continuous)
    pub value: Option<Vec<f64>>,
    /// Action name
    pub name: String,
    /// Probability/confidence
    pub probability: f64,
}

impl Action {
    #[inline]
    pub fn discrete(id: u64, name: String) -> Self {
        Self {
            id,
            value: None,
            name,
            probability: 1.0,
        }
    }

    #[inline]
    pub fn continuous(value: Vec<f64>) -> Self {
        Self {
            id: 0,
            value: Some(value),
            name: String::from("continuous"),
            probability: 1.0,
        }
    }
}

// ============================================================================
// EXPERIENCE REPLAY
// ============================================================================

/// Experience replay buffer
#[repr(align(64))]
pub struct ReplayBuffer {
    /// Experiences
    experiences: Vec<Experience>,
    /// Capacity
    capacity: usize,
    /// Position (circular buffer)
    position: usize,
    /// Random state
    random_state: AtomicU64,
}

impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            experiences: Vec::with_capacity(capacity),
            capacity,
            position: 0,
            random_state: AtomicU64::new(0xDEADBEEF),
        }
    }

    /// Add experience
    #[inline]
    pub fn add(&mut self, experience: Experience) {
        if self.experiences.len() < self.capacity {
            self.experiences.push(experience);
        } else {
            self.experiences[self.position] = experience;
        }
        self.position = (self.position + 1) % self.capacity;
    }

    /// Sample batch
    pub fn sample(&self, batch_size: usize) -> Vec<&Experience> {
        let mut indices = Vec::with_capacity(batch_size);
        let len = self.experiences.len();

        if len == 0 {
            return Vec::new();
        }

        for _ in 0..batch_size {
            let mut x = self.random_state.load(Ordering::Relaxed);
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.random_state.store(x, Ordering::Relaxed);

            indices.push((x as usize) % len);
        }

        indices.iter().map(|&i| &self.experiences[i]).collect()
    }

    /// Size
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.experiences.len()
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.experiences.is_empty()
    }

    /// Clear
    #[inline(always)]
    pub fn clear(&mut self) {
        self.experiences.clear();
        self.position = 0;
    }
}

// ============================================================================
// PRIORITIZED REPLAY
// ============================================================================

/// Prioritized experience replay
#[repr(align(64))]
pub struct PrioritizedReplayBuffer {
    /// Experiences with priorities
    experiences: Vec<(Experience, f64)>,
    /// Capacity
    capacity: usize,
    /// Alpha (priority exponent)
    alpha: f64,
    /// Beta (importance sampling)
    beta: f64,
    /// Minimum priority
    min_priority: f64,
    /// Maximum priority
    max_priority: f64,
    /// Sum tree for efficient sampling
    sum_tree: Vec<f64>,
    /// Random state
    random_state: AtomicU64,
}

impl PrioritizedReplayBuffer {
    pub fn new(capacity: usize, alpha: f64, beta: f64) -> Self {
        let tree_size = 2 * capacity;
        Self {
            experiences: Vec::with_capacity(capacity),
            capacity,
            alpha,
            beta,
            min_priority: 1e-6,
            max_priority: 1.0,
            sum_tree: vec![0.0; tree_size],
            random_state: AtomicU64::new(0xCAFEBABE),
        }
    }

    /// Add with priority
    pub fn add(&mut self, experience: Experience, priority: f64) {
        let priority = priority.max(self.min_priority).powf(self.alpha);

        if self.experiences.len() < self.capacity {
            self.experiences.push((experience, priority));
            let idx = self.experiences.len() - 1;
            self.update_tree(idx, priority);
        } else {
            // Replace random or lowest priority
            let idx = self.find_lowest_priority();
            self.experiences[idx] = (experience, priority);
            self.update_tree(idx, priority);
        }

        self.max_priority = self.max_priority.max(priority);
    }

    fn find_lowest_priority(&self) -> usize {
        self.experiences
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.1.partial_cmp(&b.1.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn update_tree(&mut self, idx: usize, priority: f64) {
        let tree_idx = self.capacity + idx;
        if tree_idx < self.sum_tree.len() {
            let diff = priority - self.sum_tree[tree_idx];
            self.sum_tree[tree_idx] = priority;

            // Propagate up
            let mut parent = tree_idx / 2;
            while parent > 0 {
                self.sum_tree[parent] += diff;
                parent /= 2;
            }
        }
    }

    fn total_priority(&self) -> f64 {
        if self.sum_tree.len() > 1 {
            self.sum_tree[1]
        } else {
            0.0
        }
    }

    /// Sample with importance weights
    pub fn sample(&self, batch_size: usize) -> Vec<(&Experience, f64)> {
        if self.experiences.is_empty() {
            return Vec::new();
        }

        let total = self.total_priority();
        let segment = total / batch_size as f64;

        let mut samples = Vec::with_capacity(batch_size);
        let max_weight =
            (self.experiences.len() as f64 * self.min_priority / total).powf(-self.beta);

        for i in 0..batch_size {
            let mut x = self.random_state.load(Ordering::Relaxed);
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.random_state.store(x, Ordering::Relaxed);

            let target = segment * (i as f64 + (x as f64) / (u64::MAX as f64));
            let idx = self.sample_index(target);

            if idx < self.experiences.len() {
                let (ref exp, priority) = self.experiences[idx];
                let prob = priority / total;
                let weight = (self.experiences.len() as f64 * prob).powf(-self.beta) / max_weight;
                samples.push((exp, weight));
            }
        }

        samples
    }

    fn sample_index(&self, target: f64) -> usize {
        let mut idx = 1;
        let mut target = target;

        while idx < self.capacity {
            let left = 2 * idx;
            let right = left + 1;

            if left >= self.sum_tree.len() {
                break;
            }

            if target <= self.sum_tree[left] {
                idx = left;
            } else {
                target -= self.sum_tree[left];
                idx = right;
            }
        }

        idx.saturating_sub(self.capacity)
    }

    /// Update priority
    #[inline]
    pub fn update_priority(&mut self, idx: usize, priority: f64) {
        if idx < self.experiences.len() {
            let priority = priority.max(self.min_priority).powf(self.alpha);
            self.experiences[idx].1 = priority;
            self.update_tree(idx, priority);
        }
    }
}

// ============================================================================
// Q-LEARNING
// ============================================================================

/// Q-Table for discrete state/action
pub struct QTable {
    /// Q-values: state_hash -> action -> value
    values: BTreeMap<u64, BTreeMap<u64, f64>>,
    /// Learning rate
    learning_rate: f64,
    /// Discount factor
    discount: f64,
    /// Default value
    default_value: f64,
}

impl QTable {
    pub fn new(learning_rate: f64, discount: f64) -> Self {
        Self {
            values: BTreeMap::new(),
            learning_rate,
            discount,
            default_value: 0.0,
        }
    }

    /// Get Q-value
    #[inline]
    pub fn get(&self, state_hash: u64, action_id: u64) -> f64 {
        self.values
            .get(&state_hash)
            .and_then(|actions| actions.get(&action_id))
            .copied()
            .unwrap_or(self.default_value)
    }

    /// Get all Q-values for state
    #[inline(always)]
    pub fn get_all(&self, state_hash: u64) -> Option<&BTreeMap<u64, f64>> {
        self.values.get(&state_hash)
    }

    /// Get best action for state
    #[inline]
    pub fn best_action(&self, state_hash: u64) -> Option<u64> {
        self.values.get(&state_hash).and_then(|actions| {
            actions
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(&action, _)| action)
        })
    }

    /// Update Q-value
    pub fn update(&mut self, state_hash: u64, action_id: u64, reward: f64, next_state_hash: u64) {
        let current_q = self.get(state_hash, action_id);

        // Max Q for next state
        let next_max_q = self
            .values
            .get(&next_state_hash)
            .and_then(|actions| {
                actions
                    .values()
                    .fold(None, |max, &v| Some(max.map_or(v, |m: f64| m.max(v))))
            })
            .unwrap_or(self.default_value);

        // Q-learning update
        let new_q =
            current_q + self.learning_rate * (reward + self.discount * next_max_q - current_q);

        self.values
            .entry(state_hash)
            .or_insert_with(BTreeMap::new)
            .insert(action_id, new_q);
    }

    /// SARSA update
    #[inline]
    pub fn update_sarsa(
        &mut self,
        state: u64,
        action: u64,
        reward: f64,
        next_state: u64,
        next_action: u64,
    ) {
        let current_q = self.get(state, action);
        let next_q = self.get(next_state, next_action);

        let new_q = current_q + self.learning_rate * (reward + self.discount * next_q - current_q);

        self.values
            .entry(state)
            .or_insert_with(BTreeMap::new)
            .insert(action, new_q);
    }

    /// State count
    #[inline(always)]
    pub fn state_count(&self) -> usize {
        self.values.len()
    }
}

// ============================================================================
// POLICY GRADIENT
// ============================================================================

/// Simple policy network (linear)
pub struct LinearPolicy {
    /// Weights: action -> feature weights
    weights: BTreeMap<u64, Vec<f64>>,
    /// Number of features
    n_features: usize,
    /// Number of actions
    n_actions: usize,
    /// Learning rate
    learning_rate: f64,
}

impl LinearPolicy {
    pub fn new(n_features: usize, n_actions: usize, learning_rate: f64) -> Self {
        let mut weights = BTreeMap::new();
        for action in 0..n_actions as u64 {
            weights.insert(action, vec![0.0; n_features]);
        }

        Self {
            weights,
            n_features,
            n_actions,
            learning_rate,
        }
    }

    /// Compute action probabilities (softmax)
    pub fn action_probabilities(&self, state: &State) -> Vec<(u64, f64)> {
        let mut logits = Vec::with_capacity(self.n_actions);

        for action in 0..self.n_actions as u64 {
            let w = self.weights.get(&action).unwrap();
            let logit: f64 = w
                .iter()
                .zip(state.features.iter())
                .map(|(wi, si)| wi * si)
                .sum();
            logits.push((action, logit));
        }

        // Softmax
        let max_logit = logits
            .iter()
            .map(|(_, l)| *l)
            .fold(f64::NEG_INFINITY, f64::max);
        let exp_sum: f64 = logits.iter().map(|(_, l)| (l - max_logit).exp()).sum();

        logits
            .iter()
            .map(|(a, l)| (*a, (l - max_logit).exp() / exp_sum))
            .collect()
    }

    /// Sample action
    pub fn sample_action(&self, state: &State, random: f64) -> u64 {
        let probs = self.action_probabilities(state);

        let mut cumulative = 0.0;
        for (action, prob) in probs {
            cumulative += prob;
            if random <= cumulative {
                return action;
            }
        }

        0
    }

    /// Update using REINFORCE
    pub fn update(&mut self, trajectory: &[(State, u64, f64)], baseline: f64) {
        let n = trajectory.len();
        if n == 0 {
            return;
        }

        // Calculate returns
        let mut returns = vec![0.0; n];
        let mut g = 0.0;
        for t in (0..n).rev() {
            g = trajectory[t].2 + 0.99 * g;
            returns[t] = g;
        }

        // Update weights
        for (t, (state, action, _)) in trajectory.iter().enumerate() {
            let probs = self.action_probabilities(state);
            let advantage = returns[t] - baseline;

            // Gradient of log policy
            if let Some(w) = self.weights.get_mut(action) {
                let prob = probs
                    .iter()
                    .find(|(a, _)| a == action)
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0);

                for (i, wi) in w.iter_mut().enumerate() {
                    if i < state.features.len() {
                        let grad = state.features[i] * (1.0 - prob);
                        *wi += self.learning_rate * advantage * grad;
                    }
                }
            }

            // Decrease other actions
            for (&other_action, w) in &mut self.weights {
                if other_action != *action {
                    let prob = probs
                        .iter()
                        .find(|(a, _)| *a == other_action)
                        .map(|(_, p)| *p)
                        .unwrap_or(0.0);

                    for (i, wi) in w.iter_mut().enumerate() {
                        if i < state.features.len() {
                            let grad = -state.features[i] * prob;
                            *wi += self.learning_rate * advantage * grad;
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// ACTOR-CRITIC
// ============================================================================

/// Simple actor-critic
pub struct ActorCritic {
    /// Policy (actor)
    policy: LinearPolicy,
    /// Value function (critic)
    value_weights: Vec<f64>,
    /// Actor learning rate
    actor_lr: f64,
    /// Critic learning rate
    critic_lr: f64,
}

impl ActorCritic {
    pub fn new(n_features: usize, n_actions: usize, actor_lr: f64, critic_lr: f64) -> Self {
        Self {
            policy: LinearPolicy::new(n_features, n_actions, actor_lr),
            value_weights: vec![0.0; n_features],
            actor_lr,
            critic_lr,
        }
    }

    /// Estimate value of state
    #[inline]
    pub fn value(&self, state: &State) -> f64 {
        self.value_weights
            .iter()
            .zip(state.features.iter())
            .map(|(w, s)| w * s)
            .sum()
    }

    /// Get action probabilities
    #[inline(always)]
    pub fn action_probabilities(&self, state: &State) -> Vec<(u64, f64)> {
        self.policy.action_probabilities(state)
    }

    /// Update using TD(0)
    pub fn update(
        &mut self,
        state: &State,
        action: u64,
        reward: f64,
        next_state: &State,
        done: bool,
    ) {
        let value = self.value(state);
        let next_value = if done { 0.0 } else { self.value(next_state) };
        let td_error = reward + 0.99 * next_value - value;

        // Update critic
        for (i, w) in self.value_weights.iter_mut().enumerate() {
            if i < state.features.len() {
                *w += self.critic_lr * td_error * state.features[i];
            }
        }

        // Update actor
        let probs = self.policy.action_probabilities(state);

        if let Some(w) = self.policy.weights.get_mut(&action) {
            let prob = probs
                .iter()
                .find(|(a, _)| *a == action)
                .map(|(_, p)| *p)
                .unwrap_or(0.0);

            for (i, wi) in w.iter_mut().enumerate() {
                if i < state.features.len() {
                    let grad = state.features[i] * (1.0 - prob);
                    *wi += self.actor_lr * td_error * grad;
                }
            }
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
    fn test_experience_replay() {
        let mut buffer = ReplayBuffer::new(100);

        for i in 0..50 {
            let exp = Experience {
                id: ExperienceId::generate(),
                state: State::new(vec![i as f64]),
                action: Action::discrete(0, String::from("action")),
                reward: 1.0,
                next_state: State::new(vec![(i + 1) as f64]),
                terminal: false,
                timestamp: i,
                metadata: BTreeMap::new(),
            };
            buffer.add(exp);
        }

        assert_eq!(buffer.len(), 50);

        let batch = buffer.sample(10);
        assert_eq!(batch.len(), 10);
    }

    #[test]
    fn test_q_table() {
        let mut q_table = QTable::new(0.1, 0.99);

        q_table.update(1, 0, 1.0, 2);
        q_table.update(1, 1, 2.0, 2);

        assert!(q_table.get(1, 1) > q_table.get(1, 0));
        assert_eq!(q_table.best_action(1), Some(1));
    }

    #[test]
    fn test_linear_policy() {
        let policy = LinearPolicy::new(4, 2, 0.01);
        let state = State::new(vec![1.0, 0.5, -0.3, 0.8]);

        let probs = policy.action_probabilities(&state);
        let sum: f64 = probs.iter().map(|(_, p)| p).sum();

        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_state_similarity() {
        let s1 = State::new(vec![1.0, 0.0, 0.0]);
        let s2 = State::new(vec![0.0, 1.0, 0.0]);
        let s3 = State::new(vec![1.0, 0.0, 0.0]);

        assert!(s1.similarity(&s3) > s1.similarity(&s2));
    }
}
