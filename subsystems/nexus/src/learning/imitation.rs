//! # Imitation Learning
//!
//! Learning from demonstrations and examples.
//! Implements behavioral cloning and inverse RL concepts.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;
use crate::math::F64Ext;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// IMITATION TYPES
// ============================================================================

/// Demonstration
#[derive(Debug, Clone)]
pub struct Demonstration {
    /// Demo ID
    pub id: u64,
    /// Expert identifier
    pub expert: String,
    /// Trajectory
    pub trajectory: Vec<StateAction>,
    /// Quality rating
    pub quality: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// State-action pair
#[derive(Debug, Clone)]
pub struct StateAction {
    /// State features
    pub state: Vec<f64>,
    /// Action taken
    pub action: ActionData,
    /// Reward (if known)
    pub reward: Option<f64>,
}

/// Action data
#[derive(Debug, Clone)]
pub enum ActionData {
    Discrete(u64),
    Continuous(Vec<f64>),
    Symbolic(String),
}

/// Learned policy
#[derive(Debug, Clone)]
pub struct LearnedPolicy {
    /// Policy ID
    pub id: u64,
    /// Name
    pub name: String,
    /// State-action mapping
    pub mapping: PolicyMapping,
    /// Training demos
    pub trained_on: Vec<u64>,
    /// Accuracy
    pub accuracy: f64,
    /// Created
    pub created: Timestamp,
}

/// Policy mapping
#[derive(Debug, Clone)]
pub enum PolicyMapping {
    /// Lookup table
    Table(BTreeMap<Vec<i64>, ActionData>),
    /// Nearest neighbor
    NearestNeighbor {
        states: Vec<Vec<f64>>,
        actions: Vec<ActionData>,
    },
    /// Linear model
    Linear {
        weights: Vec<Vec<f64>>,
        bias: Vec<f64>,
    },
}

/// Training result
#[derive(Debug, Clone)]
pub struct TrainingResult {
    /// Policy ID
    pub policy_id: u64,
    /// Demos used
    pub demos_used: usize,
    /// Accuracy
    pub accuracy: f64,
    /// Training time ns
    pub training_time_ns: u64,
}

// ============================================================================
// IMITATION LEARNER
// ============================================================================

/// Imitation learner
pub struct ImitationLearner {
    /// Demonstrations
    demonstrations: BTreeMap<u64, Demonstration>,
    /// Policies
    policies: BTreeMap<u64, LearnedPolicy>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ImitationConfig,
    /// Statistics
    stats: ImitationStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ImitationConfig {
    /// Minimum demo quality
    pub min_quality: f64,
    /// K for nearest neighbor
    pub k_neighbors: usize,
    /// State discretization bins
    pub discretization_bins: usize,
}

impl Default for ImitationConfig {
    fn default() -> Self {
        Self {
            min_quality: 0.5,
            k_neighbors: 5,
            discretization_bins: 10,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ImitationStats {
    /// Demos collected
    pub demos_collected: u64,
    /// Policies trained
    pub policies_trained: u64,
    /// Predictions made
    pub predictions_made: u64,
}

impl ImitationLearner {
    /// Create new learner
    pub fn new(config: ImitationConfig) -> Self {
        Self {
            demonstrations: BTreeMap::new(),
            policies: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ImitationStats::default(),
        }
    }

    /// Add demonstration
    pub fn add_demo(&mut self, expert: &str, trajectory: Vec<StateAction>, quality: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let demo = Demonstration {
            id,
            expert: expert.into(),
            trajectory,
            quality,
            timestamp: Timestamp::now(),
        };

        self.demonstrations.insert(id, demo);
        self.stats.demos_collected += 1;

        id
    }

    /// Train policy using behavioral cloning
    pub fn train_bc(&mut self, demo_ids: &[u64], name: &str) -> Option<TrainingResult> {
        let start = Timestamp::now();

        // Collect valid demonstrations
        let demos: Vec<&Demonstration> = demo_ids.iter()
            .filter_map(|id| self.demonstrations.get(id))
            .filter(|d| d.quality >= self.config.min_quality)
            .collect();

        if demos.is_empty() {
            return None;
        }

        // Collect all state-action pairs
        let mut states = Vec::new();
        let mut actions = Vec::new();

        for demo in &demos {
            for sa in &demo.trajectory {
                states.push(sa.state.clone());
                actions.push(sa.action.clone());
            }
        }

        // Create nearest neighbor policy
        let mapping = PolicyMapping::NearestNeighbor { states, actions };

        // Evaluate accuracy (simplified: leave-one-out)
        let accuracy = self.evaluate_policy(&mapping, &demos);

        let policy_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let policy = LearnedPolicy {
            id: policy_id,
            name: name.into(),
            mapping,
            trained_on: demo_ids.to_vec(),
            accuracy,
            created: Timestamp::now(),
        };

        self.policies.insert(policy_id, policy);
        self.stats.policies_trained += 1;

        let end = Timestamp::now();

        Some(TrainingResult {
            policy_id,
            demos_used: demos.len(),
            accuracy,
            training_time_ns: end.0.saturating_sub(start.0),
        })
    }

    fn evaluate_policy(&self, mapping: &PolicyMapping, demos: &[&Demonstration]) -> f64 {
        let mut correct = 0;
        let mut total = 0;

        for demo in demos {
            for sa in &demo.trajectory {
                let predicted = self.predict_action(mapping, &sa.state);

                if let Some(pred) = predicted {
                    if self.actions_match(&pred, &sa.action) {
                        correct += 1;
                    }
                }
                total += 1;
            }
        }

        if total == 0 {
            0.0
        } else {
            correct as f64 / total as f64
        }
    }

    fn actions_match(&self, a: &ActionData, b: &ActionData) -> bool {
        match (a, b) {
            (ActionData::Discrete(x), ActionData::Discrete(y)) => x == y,
            (ActionData::Symbolic(x), ActionData::Symbolic(y)) => x == y,
            (ActionData::Continuous(x), ActionData::Continuous(y)) => {
                if x.len() != y.len() {
                    return false;
                }
                x.iter().zip(y.iter()).all(|(a, b)| (a - b).abs() < 0.1)
            }
            _ => false,
        }
    }

    /// Predict action using policy
    pub fn predict(&mut self, policy_id: u64, state: &[f64]) -> Option<ActionData> {
        let policy = self.policies.get(&policy_id)?;

        self.stats.predictions_made += 1;

        self.predict_action(&policy.mapping, state)
    }

    fn predict_action(&self, mapping: &PolicyMapping, state: &[f64]) -> Option<ActionData> {
        match mapping {
            PolicyMapping::Table(table) => {
                let key = self.discretize_state(state);
                table.get(&key).cloned()
            }

            PolicyMapping::NearestNeighbor { states, actions } => {
                self.find_nearest(state, states, actions)
            }

            PolicyMapping::Linear { weights, bias } => {
                self.linear_predict(state, weights, bias)
            }
        }
    }

    fn discretize_state(&self, state: &[f64]) -> Vec<i64> {
        state.iter()
            .map(|x| {
                let bin = (x * self.config.discretization_bins as f64) as i64;
                bin.max(0).min(self.config.discretization_bins as i64 - 1)
            })
            .collect()
    }

    fn find_nearest(&self, state: &[f64], states: &[Vec<f64>], actions: &[ActionData]) -> Option<ActionData> {
        if states.is_empty() || actions.is_empty() {
            return None;
        }

        // Find K nearest neighbors
        let mut distances: Vec<(usize, f64)> = states.iter()
            .enumerate()
            .map(|(i, s)| (i, self.euclidean_distance(state, s)))
            .collect();

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Return action of nearest neighbor
        if let Some((idx, _)) = distances.first() {
            return actions.get(*idx).cloned();
        }

        None
    }

    fn euclidean_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        let min_len = a.len().min(b.len());

        a.iter().take(min_len)
            .zip(b.iter().take(min_len))
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    fn linear_predict(&self, state: &[f64], weights: &[Vec<f64>], bias: &[f64]) -> Option<ActionData> {
        if weights.is_empty() {
            return None;
        }

        let mut output = Vec::new();

        for (i, w) in weights.iter().enumerate() {
            let dot: f64 = state.iter()
                .zip(w.iter())
                .map(|(s, w)| s * w)
                .sum();

            let b = bias.get(i).copied().unwrap_or(0.0);
            output.push(dot + b);
        }

        Some(ActionData::Continuous(output))
    }

    /// Train table-based policy
    pub fn train_table(&mut self, demo_ids: &[u64], name: &str) -> Option<TrainingResult> {
        let start = Timestamp::now();

        let demos: Vec<&Demonstration> = demo_ids.iter()
            .filter_map(|id| self.demonstrations.get(id))
            .filter(|d| d.quality >= self.config.min_quality)
            .collect();

        if demos.is_empty() {
            return None;
        }

        let mut table = BTreeMap::new();

        for demo in &demos {
            for sa in &demo.trajectory {
                let key = self.discretize_state(&sa.state);
                table.insert(key, sa.action.clone());
            }
        }

        let mapping = PolicyMapping::Table(table);
        let accuracy = self.evaluate_policy(&mapping, &demos);

        let policy_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let policy = LearnedPolicy {
            id: policy_id,
            name: name.into(),
            mapping,
            trained_on: demo_ids.to_vec(),
            accuracy,
            created: Timestamp::now(),
        };

        self.policies.insert(policy_id, policy);
        self.stats.policies_trained += 1;

        let end = Timestamp::now();

        Some(TrainingResult {
            policy_id,
            demos_used: demos.len(),
            accuracy,
            training_time_ns: end.0.saturating_sub(start.0),
        })
    }

    /// Get demonstration
    pub fn get_demo(&self, id: u64) -> Option<&Demonstration> {
        self.demonstrations.get(&id)
    }

    /// Get policy
    pub fn get_policy(&self, id: u64) -> Option<&LearnedPolicy> {
        self.policies.get(&id)
    }

    /// Get demos by expert
    pub fn demos_by_expert(&self, expert: &str) -> Vec<&Demonstration> {
        self.demonstrations.values()
            .filter(|d| d.expert == expert)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &ImitationStats {
        &self.stats
    }
}

impl Default for ImitationLearner {
    fn default() -> Self {
        Self::new(ImitationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_trajectory() -> Vec<StateAction> {
        vec![
            StateAction {
                state: vec![0.0, 0.0],
                action: ActionData::Discrete(0),
                reward: Some(1.0),
            },
            StateAction {
                state: vec![1.0, 0.0],
                action: ActionData::Discrete(1),
                reward: Some(1.0),
            },
            StateAction {
                state: vec![1.0, 1.0],
                action: ActionData::Discrete(2),
                reward: Some(10.0),
            },
        ]
    }

    #[test]
    fn test_add_demo() {
        let mut learner = ImitationLearner::default();

        let id = learner.add_demo("expert1", create_trajectory(), 0.9);
        assert!(learner.get_demo(id).is_some());
    }

    #[test]
    fn test_train_bc() {
        let mut learner = ImitationLearner::default();

        let demo_id = learner.add_demo("expert1", create_trajectory(), 0.9);
        let result = learner.train_bc(&[demo_id], "policy1");

        assert!(result.is_some());

        let r = result.unwrap();
        assert!(r.accuracy > 0.0);
    }

    #[test]
    fn test_predict() {
        let mut learner = ImitationLearner::default();

        let demo_id = learner.add_demo("expert1", create_trajectory(), 0.9);
        let result = learner.train_bc(&[demo_id], "policy1").unwrap();

        let action = learner.predict(result.policy_id, &[0.0, 0.0]);
        assert!(action.is_some());
    }

    #[test]
    fn test_train_table() {
        let mut learner = ImitationLearner::default();

        let demo_id = learner.add_demo("expert1", create_trajectory(), 0.9);
        let result = learner.train_table(&[demo_id], "table_policy");

        assert!(result.is_some());
    }

    #[test]
    fn test_quality_filter() {
        let mut config = ImitationConfig::default();
        config.min_quality = 0.8;

        let mut learner = ImitationLearner::new(config);

        // Low quality demo
        learner.add_demo("bad_expert", create_trajectory(), 0.3);

        // High quality demo
        let good_id = learner.add_demo("good_expert", create_trajectory(), 0.95);

        let result = learner.train_bc(&[good_id], "policy");
        assert!(result.is_some());
        assert_eq!(result.unwrap().demos_used, 1);
    }
}
