//! Dreamer-style actor-critic agent for model-based RL.

use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::latent::LatentState;
use crate::world_model::types::DEFAULT_HORIZON;
use crate::world_model::utils::{box_muller, lcg_next};
use crate::world_model::world::WorldModel;

/// Actor network for model-based RL
#[derive(Debug, Clone)]
pub struct Actor {
    /// Latent dimension
    pub latent_dim: usize,
    /// Action dimension
    pub action_dim: usize,
    /// Weights
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
}

impl Actor {
    /// Create a new actor
    pub fn new(latent_dim: usize, action_dim: usize, hidden_sizes: &[usize]) -> Self {
        let mut layer_sizes = vec![latent_dim];
        layer_sizes.extend_from_slice(hidden_sizes);
        layer_sizes.push(action_dim * 2); // Mean and std

        let mut weights = Vec::new();
        let mut biases = Vec::new();

        for i in 0..layer_sizes.len() - 1 {
            let in_size = layer_sizes[i];
            let out_size = layer_sizes[i + 1];

            let scale = libm::sqrt(2.0 / (in_size + out_size) as f64);
            let mut w = Vec::with_capacity(out_size);

            for j in 0..out_size {
                let mut row = Vec::with_capacity(in_size);
                for k in 0..in_size {
                    let seed = ((i * 1000 + j * 100 + k + 300000) as u64)
                        .wrapping_mul(6364136223846793005);
                    row.push(((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
                }
                w.push(row);
            }

            weights.push(w);
            biases.push(vec![0.0; out_size]);
        }

        Self {
            latent_dim,
            action_dim,
            weights,
            biases,
        }
    }

    /// Get action distribution
    pub fn get_action(&self, state: &LatentState, noise: f64) -> Vec<f64> {
        let mut x = state.z.clone();

        for i in 0..self.weights.len() {
            let mut y = self.biases[i].clone();

            for (j, bias_j) in y.iter_mut().enumerate() {
                for (k, &xk) in x.iter().enumerate() {
                    if k < self.weights[i][j].len() {
                        *bias_j += self.weights[i][j][k] * xk;
                    }
                }
            }

            if i < self.weights.len() - 1 {
                for v in &mut y {
                    *v = libm::tanh(*v); // Use tanh for intermediate
                }
            }

            x = y;
        }

        // Split into mean and log_std
        let mean: Vec<f64> = x[..self.action_dim]
            .iter()
            .map(|&m| libm::tanh(m)) // Bound actions
            .collect();

        let std: Vec<f64> = x[self.action_dim..]
            .iter()
            .map(|&ls| libm::exp(ls.clamp(-5.0, 2.0)))
            .collect();

        // Sample action
        mean.iter()
            .zip(std.iter())
            .map(|(&m, &s)| m + noise * s)
            .collect()
    }
}

/// Critic network (value function)
#[derive(Debug, Clone)]
pub struct Critic {
    /// Latent dimension
    pub latent_dim: usize,
    /// Weights
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
}

impl Critic {
    /// Create a new critic
    pub fn new(latent_dim: usize, hidden_sizes: &[usize]) -> Self {
        let mut layer_sizes = vec![latent_dim];
        layer_sizes.extend_from_slice(hidden_sizes);
        layer_sizes.push(1);

        let mut weights = Vec::new();
        let mut biases = Vec::new();

        for i in 0..layer_sizes.len() - 1 {
            let in_size = layer_sizes[i];
            let out_size = layer_sizes[i + 1];

            let scale = libm::sqrt(2.0 / (in_size + out_size) as f64);
            let mut w = Vec::with_capacity(out_size);

            for j in 0..out_size {
                let mut row = Vec::with_capacity(in_size);
                for k in 0..in_size {
                    let seed = ((i * 1000 + j * 100 + k + 400000) as u64)
                        .wrapping_mul(6364136223846793005);
                    row.push(((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
                }
                w.push(row);
            }

            weights.push(w);
            biases.push(vec![0.0; out_size]);
        }

        Self {
            latent_dim,
            weights,
            biases,
        }
    }

    /// Estimate value
    pub fn value(&self, state: &LatentState) -> f64 {
        let mut x = state.z.clone();

        for i in 0..self.weights.len() {
            let mut y = self.biases[i].clone();

            for (j, bias_j) in y.iter_mut().enumerate() {
                for (k, &xk) in x.iter().enumerate() {
                    if k < self.weights[i][j].len() {
                        *bias_j += self.weights[i][j][k] * xk;
                    }
                }
            }

            if i < self.weights.len() - 1 {
                for v in &mut y {
                    *v = v.max(0.0);
                }
            }

            x = y;
        }

        x.first().copied().unwrap_or(0.0)
    }
}

/// Dreamer-style agent
pub struct DreamerAgent {
    /// World model
    pub world_model: WorldModel,
    /// Actor
    pub actor: Actor,
    /// Critic
    pub critic: Critic,
    /// Imagination horizon
    pub imagination_horizon: usize,
    /// Discount factor
    pub gamma: f64,
    /// Lambda for TD(lambda)
    pub lambda: f64,
}

impl DreamerAgent {
    /// Create a new Dreamer agent
    pub fn new(obs_dim: usize, action_dim: usize, latent_dim: usize) -> Self {
        let hidden = &[256, 256];

        Self {
            world_model: WorldModel::new(obs_dim, action_dim, latent_dim, hidden),
            actor: Actor::new(latent_dim, action_dim, hidden),
            critic: Critic::new(latent_dim, hidden),
            imagination_horizon: DEFAULT_HORIZON,
            gamma: 0.99,
            lambda: 0.95,
        }
    }

    /// Act in environment
    #[inline]
    pub fn act(&self, observation: &[f64], explore: bool) -> Vec<f64> {
        let state = self.world_model.encoder.encode(observation);
        let noise = if explore { 0.3 } else { 0.0 };
        self.actor.get_action(&state, noise)
    }

    /// Imagine trajectory for learning
    pub fn imagine_trajectory(
        &self,
        start_state: &LatentState,
        seed: u64,
    ) -> Vec<(LatentState, f64, f64)> {
        let mut trajectory = Vec::new();
        let mut state = start_state.clone();
        let mut rng = seed;

        for _ in 0..self.imagination_horizon {
            rng = lcg_next(rng);
            let noise = box_muller(rng) * 0.1;

            let action = self.actor.get_action(&state, noise);
            let (next_state, reward) = self.world_model.step(&action);
            let value = self.critic.value(&state);

            trajectory.push((state.clone(), reward, value));
            state = next_state;
        }

        trajectory
    }

    /// Compute lambda returns
    pub fn compute_returns(&self, trajectory: &[(LatentState, f64, f64)]) -> Vec<f64> {
        let n = trajectory.len();
        if n == 0 {
            return Vec::new();
        }

        // Get final value
        let final_value = trajectory
            .last()
            .map(|(s, _, _)| self.critic.value(s))
            .unwrap_or(0.0);

        // Compute TD(lambda) returns
        let mut returns = vec![0.0; n];
        let mut last_return = final_value;

        for i in (0..n).rev() {
            let (_, reward, value) = &trajectory[i];

            let next_value = if i + 1 < n {
                trajectory[i + 1].2
            } else {
                final_value
            };

            // TD error
            let td_error = reward + self.gamma * next_value - value;

            // Lambda return
            last_return = value + td_error + self.gamma * self.lambda * (last_return - next_value);
            returns[i] = last_return;
        }

        returns
    }
}
