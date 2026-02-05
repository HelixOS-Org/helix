//! Ensemble world model for uncertainty quantification.

use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::latent::LatentState;
use crate::world_model::types::ENSEMBLE_SIZE;
use crate::world_model::world::WorldModel;

/// An ensemble of world models for uncertainty estimation
pub struct EnsembleWorldModel {
    /// Individual models
    pub models: Vec<WorldModel>,
    /// Ensemble size
    pub size: usize,
    /// Current state (mean)
    pub current_state: LatentState,
    /// State uncertainty
    pub epistemic_uncertainty: Vec<f64>,
}

impl EnsembleWorldModel {
    /// Create a new ensemble
    pub fn new(
        obs_dim: usize,
        action_dim: usize,
        latent_dim: usize,
        hidden_sizes: &[usize],
        ensemble_size: usize,
    ) -> Self {
        let size = ensemble_size.min(ENSEMBLE_SIZE);
        let mut models = Vec::with_capacity(size);

        for i in 0..size {
            let mut model = WorldModel::new(obs_dim, action_dim, latent_dim, hidden_sizes);
            model.name = format!("Ensemble_{}", i);
            // Each model has different initialization (already done via different seeds)
            models.push(model);
        }

        Self {
            models,
            size,
            current_state: LatentState::new(latent_dim),
            epistemic_uncertainty: vec![0.0; latent_dim],
        }
    }

    /// Observe and update all models
    pub fn observe(&mut self, observation: &[f64]) {
        let mut states: Vec<LatentState> = Vec::new();

        for model in &mut self.models {
            let state = model.observe(observation);
            states.push(state.clone());
        }

        // Compute mean and variance
        if !states.is_empty() {
            let dim = states[0].z.len();
            let mut mean = vec![0.0; dim];

            for state in &states {
                for (m, &z) in mean.iter_mut().zip(state.z.iter()) {
                    *m += z;
                }
            }

            for m in &mut mean {
                *m /= states.len() as f64;
            }

            // Epistemic uncertainty (disagreement)
            let mut variance = vec![0.0; dim];
            for state in &states {
                for (v, (&z, &m)) in variance.iter_mut().zip(state.z.iter().zip(mean.iter())) {
                    *v += (z - m).powi(2);
                }
            }

            for v in &mut variance {
                *v /= states.len() as f64;
            }

            self.epistemic_uncertainty = variance;
            self.current_state = LatentState::from_vec(mean);
        }
    }

    /// Predict with uncertainty
    pub fn predict_with_uncertainty(&self, action: &[f64]) -> (LatentState, f64, f64) {
        let mut next_states: Vec<LatentState> = Vec::new();
        let mut rewards: Vec<f64> = Vec::new();

        for model in &self.models {
            let (state, reward) = model.step(action);
            next_states.push(state);
            rewards.push(reward);
        }

        // Mean state
        if !next_states.is_empty() {
            let dim = next_states[0].z.len();
            let mut mean = vec![0.0; dim];

            for state in &next_states {
                for (m, &z) in mean.iter_mut().zip(state.z.iter()) {
                    *m += z;
                }
            }

            for m in &mut mean {
                *m /= next_states.len() as f64;
            }

            let mean_reward: f64 = rewards.iter().sum::<f64>() / rewards.len() as f64;

            // Reward uncertainty
            let reward_var: f64 = rewards
                .iter()
                .map(|&r| (r - mean_reward).powi(2))
                .sum::<f64>()
                / rewards.len() as f64;

            (
                LatentState::from_vec(mean),
                mean_reward,
                libm::sqrt(reward_var),
            )
        } else {
            (self.current_state.clone(), 0.0, 0.0)
        }
    }

    /// Get total epistemic uncertainty
    pub fn total_epistemic_uncertainty(&self) -> f64 {
        self.epistemic_uncertainty.iter().sum()
    }
}
