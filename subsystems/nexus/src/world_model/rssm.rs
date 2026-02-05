//! Recurrent State-Space Model (RSSM) for world modeling.

use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::decoder::Decoder;
use crate::world_model::dynamics::{RecurrentCell, TransitionModel};
use crate::world_model::encoder::Encoder;
use crate::world_model::latent::LatentState;
use crate::world_model::types::MAX_LATENT_DIM;

/// Recurrent State-Space Model
pub struct RSSM {
    /// Deterministic recurrent model
    pub recurrent: RecurrentCell,
    /// Prior transition (predicts stochastic state)
    pub prior: TransitionModel,
    /// Posterior (encodes observation)
    pub posterior: Encoder,
    /// Decoder
    pub decoder: Decoder,
    /// Hidden state
    pub hidden: Vec<f64>,
    /// Stochastic state
    pub stochastic: Vec<f64>,
    /// State dimension
    pub state_dim: usize,
    /// Action dimension
    pub action_dim: usize,
}

impl RSSM {
    /// Create a new RSSM
    pub fn new(obs_dim: usize, action_dim: usize, state_dim: usize, hidden_dim: usize) -> Self {
        let state_dim = state_dim.min(MAX_LATENT_DIM);
        let hidden_dim = hidden_dim.min(MAX_LATENT_DIM);

        Self {
            recurrent: RecurrentCell::new(state_dim + action_dim, hidden_dim),
            prior: TransitionModel::new(hidden_dim, 0, &[128], false),
            posterior: Encoder::new(hidden_dim + obs_dim, state_dim, &[128]),
            decoder: Decoder::new(hidden_dim + state_dim, obs_dim, &[128]),
            hidden: vec![0.0; hidden_dim],
            stochastic: vec![0.0; state_dim],
            state_dim,
            action_dim,
        }
    }

    /// Get combined state
    pub fn get_state(&self) -> LatentState {
        let mut z = self.hidden.clone();
        z.extend_from_slice(&self.stochastic);
        LatentState::from_vec(z)
    }

    /// Prior step (imagination)
    pub fn prior_step(&mut self, action: &[f64]) -> LatentState {
        // Recurrent update
        let mut input = self.stochastic.clone();
        input.extend_from_slice(action);

        self.hidden = self.recurrent.forward(&input, &self.hidden);

        // Prior prediction
        let hidden_state = LatentState::from_vec(self.hidden.clone());
        let prior_state = self.prior.predict(&hidden_state, &[]);

        self.stochastic = prior_state.z;

        self.get_state()
    }

    /// Posterior step (with observation)
    pub fn posterior_step(&mut self, observation: &[f64], action: &[f64]) -> LatentState {
        // First do prior step
        let mut input = self.stochastic.clone();
        input.extend_from_slice(action);

        self.hidden = self.recurrent.forward(&input, &self.hidden);

        // Posterior encoding
        let mut posterior_input = self.hidden.clone();
        posterior_input.extend_from_slice(observation);

        let posterior_state = self.posterior.encode(&posterior_input);
        self.stochastic = posterior_state.z[..self.state_dim.min(posterior_state.z.len())].to_vec();

        self.get_state()
    }

    /// Decode state to observation
    pub fn decode(&self, state: &LatentState) -> Vec<f64> {
        self.decoder.decode(state)
    }

    /// Imagine trajectory
    pub fn imagine(&mut self, actions: &[Vec<f64>]) -> Vec<LatentState> {
        let mut trajectory = Vec::new();

        for action in actions {
            let state = self.prior_step(action);
            trajectory.push(state);
        }

        trajectory
    }

    /// Reset hidden state
    pub fn reset(&mut self) {
        self.hidden = vec![0.0; self.hidden.len()];
        self.stochastic = vec![0.0; self.stochastic.len()];
    }
}
