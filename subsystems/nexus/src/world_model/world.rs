//! Complete world model combining encoder, decoder, transition, and reward models.

use alloc::string::String;
use alloc::vec::Vec;

use crate::world_model::decoder::Decoder;
use crate::world_model::dynamics::{RewardModel, TransitionModel};
use crate::world_model::encoder::Encoder;
use crate::world_model::latent::LatentState;
use crate::world_model::types::{MAX_ACTION_DIM, MAX_LATENT_DIM, MAX_OBS_DIM};

/// Complete world model
pub struct WorldModel {
    /// Encoder
    pub encoder: Encoder,
    /// Decoder
    pub decoder: Decoder,
    /// Transition model
    pub transition: TransitionModel,
    /// Reward model
    pub reward: RewardModel,
    /// Current latent state
    pub current_state: LatentState,
    /// State history
    pub state_history: Vec<LatentState>,
    /// Maximum history size
    pub max_history: usize,
    /// Model name
    pub name: String,
}

impl WorldModel {
    /// Create a new world model
    pub fn new(
        obs_dim: usize,
        action_dim: usize,
        latent_dim: usize,
        hidden_sizes: &[usize],
    ) -> Self {
        let obs_dim = obs_dim.min(MAX_OBS_DIM);
        let action_dim = action_dim.min(MAX_ACTION_DIM);
        let latent_dim = latent_dim.min(MAX_LATENT_DIM);

        Self {
            encoder: Encoder::new(obs_dim, latent_dim, hidden_sizes),
            decoder: Decoder::new(latent_dim, obs_dim, hidden_sizes),
            transition: TransitionModel::new(latent_dim, action_dim, hidden_sizes, false),
            reward: RewardModel::new(latent_dim, action_dim, hidden_sizes),
            current_state: LatentState::new(latent_dim),
            state_history: Vec::new(),
            max_history: 1000,
            name: String::from("WorldModel"),
        }
    }

    /// Update state from observation
    pub fn observe(&mut self, observation: &[f64]) -> &LatentState {
        let state = self.encoder.encode(observation);

        // Store in history
        if self.state_history.len() >= self.max_history {
            self.state_history.remove(0);
        }
        self.state_history.push(state.clone());

        self.current_state = state;
        &self.current_state
    }

    /// Predict next state and reward
    pub fn step(&self, action: &[f64]) -> (LatentState, f64) {
        let next_state = self.transition.predict(&self.current_state, action);
        let reward = self.reward.predict(&self.current_state, action);

        (next_state, reward)
    }

    /// Imagine a trajectory
    pub fn imagine(
        &self,
        actions: &[Vec<f64>],
        start_state: Option<&LatentState>,
    ) -> Vec<(LatentState, f64)> {
        let mut state = start_state
            .cloned()
            .unwrap_or_else(|| self.current_state.clone());
        let mut trajectory = Vec::new();

        for action in actions {
            let next_state = self.transition.predict(&state, action);
            let reward = self.reward.predict(&state, action);

            trajectory.push((next_state.clone(), reward));
            state = next_state;
        }

        trajectory
    }

    /// Reconstruct observation from latent state
    pub fn reconstruct(&self, state: &LatentState) -> Vec<f64> {
        self.decoder.decode(state)
    }

    /// Reconstruction loss
    pub fn reconstruction_loss(&self, observation: &[f64]) -> f64 {
        let state = self.encoder.encode(observation);
        let reconstructed = self.decoder.decode(&state);

        observation
            .iter()
            .zip(reconstructed.iter())
            .map(|(&o, &r)| (o - r).powi(2))
            .sum::<f64>()
            / observation.len() as f64
    }
}
