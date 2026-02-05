//! Model-based planning components.

use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::utils::{box_muller, lcg_next};
use crate::world_model::world::WorldModel;

/// Action sequence for planning
#[derive(Debug, Clone)]
pub struct ActionSequence {
    /// Actions
    pub actions: Vec<Vec<f64>>,
    /// Expected total reward
    pub expected_reward: f64,
    /// Confidence
    pub confidence: f64,
}

impl ActionSequence {
    /// Create an empty sequence
    pub fn new(action_dim: usize, horizon: usize) -> Self {
        Self {
            actions: vec![vec![0.0; action_dim]; horizon],
            expected_reward: 0.0,
            confidence: 1.0,
        }
    }
}

/// Model-Predictive Control (MPC) planner
pub struct MPCPlanner {
    /// Planning horizon
    pub horizon: usize,
    /// Number of trajectories to sample
    pub num_samples: usize,
    /// Action dimension
    pub action_dim: usize,
    /// Action bounds
    pub action_bounds: (f64, f64),
    /// Best action sequence
    pub best_sequence: ActionSequence,
    /// RNG state
    rng_state: u64,
}

impl MPCPlanner {
    /// Create a new MPC planner
    pub fn new(action_dim: usize, horizon: usize, num_samples: usize) -> Self {
        Self {
            horizon,
            num_samples,
            action_dim,
            action_bounds: (-1.0, 1.0),
            best_sequence: ActionSequence::new(action_dim, horizon),
            rng_state: 12345,
        }
    }

    /// Plan using random shooting
    pub fn plan(&mut self, model: &WorldModel) -> Vec<f64> {
        let mut best_reward = f64::NEG_INFINITY;
        let mut best_actions: Vec<Vec<f64>> = Vec::new();

        for _ in 0..self.num_samples {
            // Sample random action sequence
            let actions = self.sample_actions();

            // Evaluate trajectory
            let trajectory = model.imagine(&actions, None);

            let total_reward: f64 = trajectory
                .iter()
                .enumerate()
                .map(|(t, (_, r))| {
                    // Discount factor
                    let gamma = 0.99_f64.powi(t as i32);
                    gamma * r
                })
                .sum();

            if total_reward > best_reward {
                best_reward = total_reward;
                best_actions = actions;
            }
        }

        self.best_sequence.actions = best_actions.clone();
        self.best_sequence.expected_reward = best_reward;

        // Return first action
        best_actions
            .first()
            .cloned()
            .unwrap_or_else(|| vec![0.0; self.action_dim])
    }

    /// Plan with cross-entropy method
    pub fn plan_cem(&mut self, model: &WorldModel, iterations: usize, elite_frac: f64) -> Vec<f64> {
        let elite_count = ((self.num_samples as f64 * elite_frac) as usize).max(1);

        // Initialize mean and std for each action
        let mut mean = vec![vec![0.0; self.action_dim]; self.horizon];
        let mut std = vec![vec![1.0; self.action_dim]; self.horizon];

        for _ in 0..iterations {
            // Sample action sequences
            let mut samples: Vec<(Vec<Vec<f64>>, f64)> = Vec::new();

            for _ in 0..self.num_samples {
                let actions = self.sample_from_gaussian(&mean, &std);
                let trajectory = model.imagine(&actions, None);

                let reward: f64 = trajectory
                    .iter()
                    .enumerate()
                    .map(|(t, (_, r))| 0.99_f64.powi(t as i32) * r)
                    .sum();

                samples.push((actions, reward));
            }

            // Sort by reward
            samples.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

            // Update mean and std from elites
            let elites = &samples[..elite_count];

            for t in 0..self.horizon {
                for d in 0..self.action_dim {
                    let elite_values: Vec<f64> = elites.iter().map(|(a, _)| a[t][d]).collect();

                    mean[t][d] = elite_values.iter().sum::<f64>() / elite_count as f64;

                    let var: f64 = elite_values
                        .iter()
                        .map(|&v| (v - mean[t][d]).powi(2))
                        .sum::<f64>()
                        / elite_count as f64;

                    std[t][d] = libm::sqrt(var).max(0.01);
                }
            }
        }

        self.best_sequence.actions = mean.clone();

        mean.first()
            .cloned()
            .unwrap_or_else(|| vec![0.0; self.action_dim])
    }

    /// Sample random actions
    fn sample_actions(&mut self) -> Vec<Vec<f64>> {
        let mut actions = Vec::with_capacity(self.horizon);

        for _ in 0..self.horizon {
            let mut action = Vec::with_capacity(self.action_dim);
            for _ in 0..self.action_dim {
                self.rng_state = lcg_next(self.rng_state);
                let u = self.rng_state as f64 / u64::MAX as f64;
                let a = self.action_bounds.0 + u * (self.action_bounds.1 - self.action_bounds.0);
                action.push(a);
            }
            actions.push(action);
        }

        actions
    }

    /// Sample from Gaussian
    fn sample_from_gaussian(&mut self, mean: &[Vec<f64>], std: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut actions = Vec::with_capacity(self.horizon);

        for t in 0..self.horizon {
            let mut action = Vec::with_capacity(self.action_dim);
            for d in 0..self.action_dim {
                self.rng_state = lcg_next(self.rng_state);
                let z = box_muller(self.rng_state);
                let a = mean[t][d] + std[t][d] * z;
                let a = a.clamp(self.action_bounds.0, self.action_bounds.1);
                action.push(a);
            }
            actions.push(action);
        }

        actions
    }
}
