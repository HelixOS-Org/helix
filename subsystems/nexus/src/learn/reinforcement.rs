//! # Reinforcement Learning
//!
//! Learning through rewards and experience.
//! Implements value-based and policy-based methods.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// RL TYPES
// ============================================================================

/// State
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct State {
    /// State ID
    pub id: u64,
    /// Features
    pub features: Vec<i64>, // Using i64 for hashable state
}

/// Action
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Action {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
}

/// Transition
#[derive(Debug, Clone)]
pub struct Transition {
    /// State
    pub state: State,
    /// Action taken
    pub action: Action,
    /// Reward received
    pub reward: f64,
    /// Next state
    pub next_state: State,
    /// Is terminal
    pub terminal: bool,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Episode
#[derive(Debug, Clone)]
pub struct Episode {
    /// Episode ID
    pub id: u64,
    /// Transitions
    pub transitions: Vec<Transition>,
    /// Total reward
    pub total_reward: f64,
    /// Length
    pub length: usize,
}

/// Q-Value (state-action value)
#[derive(Debug, Clone)]
pub struct QValue {
    /// Value
    pub value: f64,
    /// Update count
    pub updates: u64,
    /// Last update
    pub last_update: Timestamp,
}

/// Policy
#[derive(Debug, Clone)]
pub struct Policy {
    /// Policy ID
    pub id: u64,
    /// Policy type
    pub policy_type: PolicyType,
    /// Parameters
    pub params: BTreeMap<String, f64>,
}

/// Policy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyType {
    Greedy,
    EpsilonGreedy,
    Softmax,
    UCB,
}

// ============================================================================
// RL AGENT
// ============================================================================

/// Reinforcement learning agent
pub struct RLAgent {
    /// Q-table (state -> action -> value)
    q_table: BTreeMap<State, BTreeMap<Action, QValue>>,
    /// Available actions
    actions: Vec<Action>,
    /// Current policy
    policy: Policy,
    /// Episodes
    episodes: BTreeMap<u64, Episode>,
    /// Current episode
    current_episode: Option<Episode>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: RLConfig,
    /// Statistics
    stats: RLStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct RLConfig {
    /// Learning rate (alpha)
    pub learning_rate: f64,
    /// Discount factor (gamma)
    pub discount_factor: f64,
    /// Exploration rate (epsilon)
    pub epsilon: f64,
    /// Epsilon decay
    pub epsilon_decay: f64,
    /// Minimum epsilon
    pub min_epsilon: f64,
    /// Temperature (for softmax)
    pub temperature: f64,
}

impl Default for RLConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount_factor: 0.99,
            epsilon: 1.0,
            epsilon_decay: 0.995,
            min_epsilon: 0.01,
            temperature: 1.0,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct RLStats {
    /// Episodes completed
    pub episodes_completed: u64,
    /// Total steps
    pub total_steps: u64,
    /// Average reward
    pub avg_reward: f64,
    /// Best episode reward
    pub best_reward: f64,
}

impl RLAgent {
    /// Create new agent
    pub fn new(actions: Vec<Action>, config: RLConfig) -> Self {
        Self {
            q_table: BTreeMap::new(),
            actions,
            policy: Policy {
                id: 1,
                policy_type: PolicyType::EpsilonGreedy,
                params: BTreeMap::new(),
            },
            episodes: BTreeMap::new(),
            current_episode: None,
            next_id: AtomicU64::new(1),
            config,
            stats: RLStats::default(),
        }
    }

    /// Start episode
    pub fn start_episode(&mut self) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        self.current_episode = Some(Episode {
            id,
            transitions: Vec::new(),
            total_reward: 0.0,
            length: 0,
        });

        id
    }

    /// Select action
    pub fn select_action(&self, state: &State) -> Action {
        match self.policy.policy_type {
            PolicyType::Greedy => self.greedy_action(state),
            PolicyType::EpsilonGreedy => self.epsilon_greedy_action(state),
            PolicyType::Softmax => self.softmax_action(state),
            PolicyType::UCB => self.ucb_action(state),
        }
    }

    fn greedy_action(&self, state: &State) -> Action {
        self.best_action(state)
            .unwrap_or_else(|| self.random_action())
    }

    fn epsilon_greedy_action(&self, state: &State) -> Action {
        // Simplified: use epsilon from config
        let explore = self.simple_random() < self.config.epsilon;

        if explore {
            self.random_action()
        } else {
            self.greedy_action(state)
        }
    }

    fn softmax_action(&self, state: &State) -> Action {
        let q_values = self.get_q_values(state);

        if q_values.is_empty() {
            return self.random_action();
        }

        // Compute softmax probabilities
        let max_q = q_values
            .iter()
            .map(|(_, q)| q.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let exp_sum: f64 = q_values
            .iter()
            .map(|(_, q)| ((q.value - max_q) / self.config.temperature).exp())
            .sum();

        // Select based on probabilities (simplified)
        let r = self.simple_random() * exp_sum;
        let mut cumsum = 0.0;

        for (action, q) in q_values {
            cumsum += ((q.value - max_q) / self.config.temperature).exp();
            if r <= cumsum {
                return action.clone();
            }
        }

        self.random_action()
    }

    fn ucb_action(&self, state: &State) -> Action {
        let q_values = self.get_q_values(state);
        let total_visits: u64 = q_values.iter().map(|(_, q)| q.updates).sum();

        if total_visits == 0 {
            return self.random_action();
        }

        let mut best_action = self.random_action();
        let mut best_ucb = f64::NEG_INFINITY;

        for (action, q) in q_values {
            let ucb = if q.updates == 0 {
                f64::INFINITY
            } else {
                q.value + (2.0 * (total_visits as f64).ln() / q.updates as f64).sqrt()
            };

            if ucb > best_ucb {
                best_ucb = ucb;
                best_action = action.clone();
            }
        }

        best_action
    }

    fn best_action(&self, state: &State) -> Option<Action> {
        self.q_table.get(state).and_then(|actions| {
            actions
                .iter()
                .max_by(|a, b| a.1.value.partial_cmp(&b.1.value).unwrap())
                .map(|(action, _)| action.clone())
        })
    }

    fn random_action(&self) -> Action {
        let idx = (self.simple_random() * self.actions.len() as f64) as usize;
        self.actions
            .get(idx)
            .cloned()
            .unwrap_or(self.actions[0].clone())
    }

    fn simple_random(&self) -> f64 {
        // Simplified pseudo-random for no_std
        let t = Timestamp::now().0;
        ((t % 1000) as f64) / 1000.0
    }

    fn get_q_values(&self, state: &State) -> Vec<(&Action, &QValue)> {
        self.q_table
            .get(state)
            .map(|actions| actions.iter().collect())
            .unwrap_or_default()
    }

    /// Step (observe transition)
    pub fn step(
        &mut self,
        state: State,
        action: Action,
        reward: f64,
        next_state: State,
        terminal: bool,
    ) {
        // Record transition
        let transition = Transition {
            state: state.clone(),
            action: action.clone(),
            reward,
            next_state: next_state.clone(),
            terminal,
            timestamp: Timestamp::now(),
        };

        if let Some(ep) = &mut self.current_episode {
            ep.transitions.push(transition);
            ep.total_reward += reward;
            ep.length += 1;
        }

        self.stats.total_steps += 1;

        // Update Q-value (Q-learning)
        self.update_q_value(&state, &action, reward, &next_state, terminal);

        // End episode if terminal
        if terminal {
            self.end_episode();
        }
    }

    fn update_q_value(
        &mut self,
        state: &State,
        action: &Action,
        reward: f64,
        next_state: &State,
        terminal: bool,
    ) {
        // Get current Q-value
        let current_q = self
            .q_table
            .get(state)
            .and_then(|a| a.get(action))
            .map(|q| q.value)
            .unwrap_or(0.0);

        // Get max Q-value for next state
        let max_next_q = if terminal {
            0.0
        } else {
            self.q_table
                .get(next_state)
                .and_then(|actions| {
                    actions
                        .values()
                        .map(|q| q.value)
                        .fold(None, |max, v| Some(max.map_or(v, |m: f64| m.max(v))))
                })
                .unwrap_or(0.0)
        };

        // Q-learning update
        let new_q = current_q
            + self.config.learning_rate
                * (reward + self.config.discount_factor * max_next_q - current_q);

        // Store updated Q-value
        let action_map = self
            .q_table
            .entry(state.clone())
            .or_insert_with(BTreeMap::new);
        let q_entry = action_map.entry(action.clone()).or_insert_with(|| QValue {
            value: 0.0,
            updates: 0,
            last_update: Timestamp::now(),
        });

        q_entry.value = new_q;
        q_entry.updates += 1;
        q_entry.last_update = Timestamp::now();
    }

    /// End episode
    pub fn end_episode(&mut self) {
        if let Some(episode) = self.current_episode.take() {
            // Update stats
            let n = self.stats.episodes_completed as f64;
            self.stats.avg_reward = (self.stats.avg_reward * n + episode.total_reward) / (n + 1.0);

            if episode.total_reward > self.stats.best_reward {
                self.stats.best_reward = episode.total_reward;
            }

            self.stats.episodes_completed += 1;

            // Decay epsilon
            self.config.epsilon =
                (self.config.epsilon * self.config.epsilon_decay).max(self.config.min_epsilon);

            // Store episode
            self.episodes.insert(episode.id, episode);
        }
    }

    /// Get Q-value
    pub fn get_q(&self, state: &State, action: &Action) -> f64 {
        self.q_table
            .get(state)
            .and_then(|a| a.get(action))
            .map(|q| q.value)
            .unwrap_or(0.0)
    }

    /// Get value (max Q)
    pub fn get_value(&self, state: &State) -> f64 {
        self.q_table
            .get(state)
            .and_then(|actions| {
                actions
                    .values()
                    .map(|q| q.value)
                    .fold(None, |max, v| Some(max.map_or(v, |m: f64| m.max(v))))
            })
            .unwrap_or(0.0)
    }

    /// Set policy
    pub fn set_policy(&mut self, policy_type: PolicyType) {
        self.policy.policy_type = policy_type;
    }

    /// Get episode
    pub fn get_episode(&self, id: u64) -> Option<&Episode> {
        self.episodes.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &RLStats {
        &self.stats
    }
}

impl Default for RLAgent {
    fn default() -> Self {
        Self::new(Vec::new(), RLConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_actions() -> Vec<Action> {
        vec![
            Action {
                id: 1,
                name: "left".into(),
            },
            Action {
                id: 2,
                name: "right".into(),
            },
        ]
    }

    fn create_state(features: &[i64]) -> State {
        State {
            id: 0,
            features: features.to_vec(),
        }
    }

    #[test]
    fn test_create_agent() {
        let agent = RLAgent::new(create_test_actions(), RLConfig::default());
        assert_eq!(agent.actions.len(), 2);
    }

    #[test]
    fn test_start_episode() {
        let mut agent = RLAgent::new(create_test_actions(), RLConfig::default());
        let id = agent.start_episode();
        assert!(agent.current_episode.is_some());
    }

    #[test]
    fn test_step() {
        let mut agent = RLAgent::new(create_test_actions(), RLConfig::default());

        agent.start_episode();

        let s1 = create_state(&[0]);
        let s2 = create_state(&[1]);
        let action = agent.actions[0].clone();

        agent.step(s1.clone(), action.clone(), 1.0, s2, false);

        assert!(agent.get_q(&s1, &action) != 0.0);
    }

    #[test]
    fn test_q_learning() {
        let mut agent = RLAgent::new(create_test_actions(), RLConfig::default());

        agent.start_episode();

        let s1 = create_state(&[0]);
        let s2 = create_state(&[1]);
        let action = agent.actions[0].clone();

        // First update
        agent.step(s1.clone(), action.clone(), 10.0, s2.clone(), false);

        let q1 = agent.get_q(&s1, &action);

        // Second update with higher reward
        agent.step(s1.clone(), action.clone(), 20.0, s2.clone(), false);

        let q2 = agent.get_q(&s1, &action);

        assert!(q2 > q1);
    }

    #[test]
    fn test_episode_stats() {
        let mut agent = RLAgent::new(create_test_actions(), RLConfig::default());

        agent.start_episode();

        let s1 = create_state(&[0]);
        let s2 = create_state(&[1]);
        let action = agent.actions[0].clone();

        agent.step(s1, action, 10.0, s2, true);

        assert_eq!(agent.stats.episodes_completed, 1);
        assert_eq!(agent.stats.avg_reward, 10.0);
    }

    #[test]
    fn test_epsilon_decay() {
        let mut config = RLConfig::default();
        config.epsilon = 1.0;
        config.epsilon_decay = 0.5;

        let mut agent = RLAgent::new(create_test_actions(), config);

        agent.start_episode();
        agent.end_episode();

        assert!(agent.config.epsilon < 1.0);
    }
}
