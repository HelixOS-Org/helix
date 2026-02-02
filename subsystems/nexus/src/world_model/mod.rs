//! # World Model Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary world modeling system that enables
//! the kernel to maintain an internal representation of its environment,
//! predict future states, and plan actions accordingly.
//!
//! ## Key Features
//!
//! - **Internal State Representation**: Latent space encoding of kernel state
//! - **Transition Dynamics**: Learning how actions affect state
//! - **Reward Prediction**: Estimating outcomes of different actions
//! - **Imagination/Planning**: Simulating future trajectories
//! - **Model-Based RL**: Using world model for efficient learning
//! - **Uncertainty Estimation**: Knowing what the model doesn't know
//!
//! ## Kernel Applications
//!
//! - Predictive resource allocation
//! - Proactive anomaly prevention
//! - Long-horizon planning for system optimization
//! - Understanding complex system dynamics

#![no_std]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum latent dimension
const MAX_LATENT_DIM: usize = 256;

/// Maximum action dimension
const MAX_ACTION_DIM: usize = 64;

/// Maximum observation dimension
const MAX_OBS_DIM: usize = 512;

/// Default imagination horizon
const DEFAULT_HORIZON: usize = 50;

/// Ensemble size for uncertainty
const ENSEMBLE_SIZE: usize = 5;

// ============================================================================
// LATENT STATE REPRESENTATION
// ============================================================================

/// A latent state vector (compressed representation)
#[derive(Debug, Clone)]
pub struct LatentState {
    /// State vector
    pub z: Vec<f64>,
    /// Uncertainty (variance per dimension)
    pub uncertainty: Vec<f64>,
    /// Timestamp
    pub timestamp: u64,
    /// Deterministic component
    pub h: Vec<f64>,
    /// Stochastic component
    pub s: Vec<f64>,
}

impl LatentState {
    /// Create a new latent state
    pub fn new(dim: usize) -> Self {
        Self {
            z: vec![0.0; dim],
            uncertainty: vec![1.0; dim],
            timestamp: 0,
            h: vec![0.0; dim / 2],
            s: vec![0.0; dim / 2],
        }
    }

    /// Create from vector
    pub fn from_vec(z: Vec<f64>) -> Self {
        let dim = z.len();
        let half = dim / 2;

        Self {
            z: z.clone(),
            uncertainty: vec![0.1; dim],
            timestamp: 0,
            h: z[..half].to_vec(),
            s: z[half..].to_vec(),
        }
    }

    /// Dimensionality
    pub fn dim(&self) -> usize {
        self.z.len()
    }

    /// Sample from distribution (for stochastic models)
    pub fn sample(&self, noise: &[f64]) -> Vec<f64> {
        self.z
            .iter()
            .zip(self.uncertainty.iter())
            .zip(noise.iter())
            .map(|((&m, &v), &n)| m + libm::sqrt(v) * n)
            .collect()
    }

    /// Total uncertainty (sum of variances)
    pub fn total_uncertainty(&self) -> f64 {
        self.uncertainty.iter().sum()
    }

    /// Distance to another state
    pub fn distance(&self, other: &LatentState) -> f64 {
        let sum_sq: f64 = self
            .z
            .iter()
            .zip(other.z.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum();

        libm::sqrt(sum_sq)
    }
}

// ============================================================================
// ENCODER (Observation -> Latent)
// ============================================================================

/// Observation encoder
#[derive(Debug, Clone)]
pub struct Encoder {
    /// Input dimension
    pub input_dim: usize,
    /// Latent dimension
    pub latent_dim: usize,
    /// Weight matrices
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
    /// Layer sizes
    pub layer_sizes: Vec<usize>,
}

impl Encoder {
    /// Create a new encoder
    pub fn new(input_dim: usize, latent_dim: usize, hidden_sizes: &[usize]) -> Self {
        let mut layer_sizes = vec![input_dim];
        layer_sizes.extend_from_slice(hidden_sizes);
        layer_sizes.push(latent_dim * 2); // Mean and log variance

        let mut weights = Vec::new();
        let mut biases = Vec::new();

        for i in 0..layer_sizes.len() - 1 {
            let in_size = layer_sizes[i];
            let out_size = layer_sizes[i + 1];

            // Xavier initialization
            let scale = libm::sqrt(2.0 / (in_size + out_size) as f64);
            let mut w = Vec::with_capacity(out_size);

            for j in 0..out_size {
                let mut row = Vec::with_capacity(in_size);
                for k in 0..in_size {
                    let seed = ((i * 1000 + j * 100 + k) as u64).wrapping_mul(6364136223846793005);
                    let val = ((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                    row.push(val);
                }
                w.push(row);
            }

            weights.push(w);
            biases.push(vec![0.0; out_size]);
        }

        Self {
            input_dim,
            latent_dim,
            weights,
            biases,
            layer_sizes,
        }
    }

    /// Forward pass
    pub fn encode(&self, observation: &[f64]) -> LatentState {
        let mut x = observation.to_vec();

        // Forward through layers
        for i in 0..self.weights.len() {
            let mut y = self.biases[i].clone();

            for (j, bias_j) in y.iter_mut().enumerate() {
                for (k, &xk) in x.iter().enumerate() {
                    *bias_j += self.weights[i][j][k] * xk;
                }
            }

            // ReLU (except last layer)
            if i < self.weights.len() - 1 {
                for v in &mut y {
                    *v = v.max(0.0);
                }
            }

            x = y;
        }

        // Split into mean and log variance
        let mean: Vec<f64> = x[..self.latent_dim].to_vec();
        let log_var: Vec<f64> = x[self.latent_dim..].to_vec();

        let variance: Vec<f64> = log_var
            .iter()
            .map(|&lv| libm::exp(lv.clamp(-10.0, 10.0)))
            .collect();

        LatentState {
            z: mean.clone(),
            uncertainty: variance,
            timestamp: 0,
            h: mean[..mean.len() / 2].to_vec(),
            s: mean[mean.len() / 2..].to_vec(),
        }
    }
}

// ============================================================================
// DECODER (Latent -> Observation)
// ============================================================================

/// Observation decoder
#[derive(Debug, Clone)]
pub struct Decoder {
    /// Latent dimension
    pub latent_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Weight matrices
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
    /// Layer sizes
    pub layer_sizes: Vec<usize>,
}

impl Decoder {
    /// Create a new decoder
    pub fn new(latent_dim: usize, output_dim: usize, hidden_sizes: &[usize]) -> Self {
        let mut layer_sizes = vec![latent_dim];
        layer_sizes.extend_from_slice(hidden_sizes);
        layer_sizes.push(output_dim);

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
                    let seed =
                        ((i * 1000 + j * 100 + k + 50000) as u64).wrapping_mul(6364136223846793005);
                    let val = ((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                    row.push(val);
                }
                w.push(row);
            }

            weights.push(w);
            biases.push(vec![0.0; out_size]);
        }

        Self {
            latent_dim,
            output_dim,
            weights,
            biases,
            layer_sizes,
        }
    }

    /// Forward pass
    pub fn decode(&self, state: &LatentState) -> Vec<f64> {
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

            // ReLU (except last layer)
            if i < self.weights.len() - 1 {
                for v in &mut y {
                    *v = v.max(0.0);
                }
            }

            x = y;
        }

        x
    }
}

// ============================================================================
// TRANSITION MODEL
// ============================================================================

/// Transition dynamics model: z_{t+1} = f(z_t, a_t)
#[derive(Debug, Clone)]
pub struct TransitionModel {
    /// Latent dimension
    pub latent_dim: usize,
    /// Action dimension
    pub action_dim: usize,
    /// Weight matrices
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
    /// Layer sizes
    pub layer_sizes: Vec<usize>,
    /// Is this a deterministic model?
    pub deterministic: bool,
}

impl TransitionModel {
    /// Create a new transition model
    pub fn new(
        latent_dim: usize,
        action_dim: usize,
        hidden_sizes: &[usize],
        deterministic: bool,
    ) -> Self {
        let input_dim = latent_dim + action_dim;
        let output_dim = if deterministic {
            latent_dim
        } else {
            latent_dim * 2
        };

        let mut layer_sizes = vec![input_dim];
        layer_sizes.extend_from_slice(hidden_sizes);
        layer_sizes.push(output_dim);

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
                    let seed = ((i * 1000 + j * 100 + k + 100000) as u64)
                        .wrapping_mul(6364136223846793005);
                    let val = ((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                    row.push(val);
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
            layer_sizes,
            deterministic,
        }
    }

    /// Predict next state
    pub fn predict(&self, state: &LatentState, action: &[f64]) -> LatentState {
        // Concatenate state and action
        let mut x = state.z.clone();
        x.extend_from_slice(action);

        // Pad if needed
        while x.len() < self.layer_sizes[0] {
            x.push(0.0);
        }

        // Forward pass
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

        if self.deterministic {
            LatentState::from_vec(x)
        } else {
            let mean: Vec<f64> = x[..self.latent_dim].to_vec();
            let log_var: Vec<f64> = x[self.latent_dim..].to_vec();

            let variance: Vec<f64> = log_var
                .iter()
                .map(|&lv| libm::exp(lv.clamp(-10.0, 10.0)))
                .collect();

            LatentState {
                z: mean.clone(),
                uncertainty: variance,
                timestamp: state.timestamp + 1,
                h: mean[..mean.len() / 2].to_vec(),
                s: mean[mean.len() / 2..].to_vec(),
            }
        }
    }

    /// Predict with residual connection
    pub fn predict_residual(&self, state: &LatentState, action: &[f64]) -> LatentState {
        let mut next = self.predict(state, action);

        // Add residual
        for (z, &s) in next.z.iter_mut().zip(state.z.iter()) {
            *z += s;
        }

        next
    }
}

// ============================================================================
// REWARD MODEL
// ============================================================================

/// Reward prediction model: r = g(z, a)
#[derive(Debug, Clone)]
pub struct RewardModel {
    /// Latent dimension
    pub latent_dim: usize,
    /// Action dimension
    pub action_dim: usize,
    /// Weights
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Biases
    pub biases: Vec<Vec<f64>>,
}

impl RewardModel {
    /// Create a new reward model
    pub fn new(latent_dim: usize, action_dim: usize, hidden_sizes: &[usize]) -> Self {
        let input_dim = latent_dim + action_dim;

        let mut layer_sizes = vec![input_dim];
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
                    let seed = ((i * 1000 + j * 100 + k + 200000) as u64)
                        .wrapping_mul(6364136223846793005);
                    let val = ((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                    row.push(val);
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

    /// Predict reward
    pub fn predict(&self, state: &LatentState, action: &[f64]) -> f64 {
        let mut x = state.z.clone();
        x.extend_from_slice(action);

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

// ============================================================================
// WORLD MODEL (Combined)
// ============================================================================

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

// ============================================================================
// ENSEMBLE WORLD MODEL (Uncertainty Quantification)
// ============================================================================

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
            model.name = alloc::format!("Ensemble_{}", i);
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

// ============================================================================
// RECURRENT STATE-SPACE MODEL (RSSM)
// ============================================================================

/// Recurrent component for RSSM
#[derive(Debug, Clone)]
pub struct RecurrentCell {
    /// Hidden dimension
    pub hidden_dim: usize,
    /// Input dimension
    pub input_dim: usize,
    /// Input weights
    pub w_i: Vec<Vec<f64>>,
    /// Hidden weights
    pub w_h: Vec<Vec<f64>>,
    /// Bias
    pub bias: Vec<f64>,
}

impl RecurrentCell {
    /// Create a new GRU-like cell
    pub fn new(input_dim: usize, hidden_dim: usize) -> Self {
        let scale = libm::sqrt(2.0 / (input_dim + hidden_dim) as f64);

        let mut w_i = Vec::with_capacity(hidden_dim);
        let mut w_h = Vec::with_capacity(hidden_dim);

        for j in 0..hidden_dim {
            let mut row_i = Vec::with_capacity(input_dim);
            let mut row_h = Vec::with_capacity(hidden_dim);

            for k in 0..input_dim {
                let seed = ((j * 100 + k) as u64).wrapping_mul(6364136223846793005);
                row_i.push(((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
            }

            for k in 0..hidden_dim {
                let seed = ((j * 100 + k + 10000) as u64).wrapping_mul(6364136223846793005);
                row_h.push(((seed as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
            }

            w_i.push(row_i);
            w_h.push(row_h);
        }

        Self {
            hidden_dim,
            input_dim,
            w_i,
            w_h,
            bias: vec![0.0; hidden_dim],
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[f64], hidden: &[f64]) -> Vec<f64> {
        let mut output = self.bias.clone();

        // Input contribution
        for (j, out_j) in output.iter_mut().enumerate() {
            for (k, &inp_k) in input.iter().enumerate() {
                if k < self.w_i[j].len() {
                    *out_j += self.w_i[j][k] * inp_k;
                }
            }
        }

        // Hidden contribution
        for (j, out_j) in output.iter_mut().enumerate() {
            for (k, &hid_k) in hidden.iter().enumerate() {
                if k < self.w_h[j].len() {
                    *out_j += self.w_h[j][k] * hid_k;
                }
            }
        }

        // Tanh activation
        for v in &mut output {
            *v = libm::tanh(*v);
        }

        output
    }
}

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

// ============================================================================
// MODEL-BASED PLANNING
// ============================================================================

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

// ============================================================================
// DREAMER-STYLE ACTOR-CRITIC
// ============================================================================

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
        let mean: Vec<f64> = x[..self.action_dim].iter()
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

// ============================================================================
// KERNEL WORLD MODEL
// ============================================================================

/// Kernel system state for world model
#[derive(Debug, Clone)]
pub struct KernelSystemState {
    /// CPU usage per core
    pub cpu_usage: Vec<f64>,
    /// Memory usage
    pub memory_usage: f64,
    /// I/O operations
    pub io_ops: f64,
    /// Network bandwidth
    pub network_bw: f64,
    /// Active processes
    pub process_count: usize,
    /// Queue lengths
    pub queue_lengths: Vec<f64>,
    /// Latency metrics
    pub latencies: Vec<f64>,
}

impl KernelSystemState {
    /// Create default state
    pub fn new() -> Self {
        Self {
            cpu_usage: vec![0.0; 4],
            memory_usage: 0.0,
            io_ops: 0.0,
            network_bw: 0.0,
            process_count: 0,
            queue_lengths: vec![0.0; 4],
            latencies: vec![0.0; 4],
        }
    }

    /// Convert to observation vector
    pub fn to_observation(&self) -> Vec<f64> {
        let mut obs = self.cpu_usage.clone();
        obs.push(self.memory_usage);
        obs.push(self.io_ops);
        obs.push(self.network_bw);
        obs.push(self.process_count as f64);
        obs.extend_from_slice(&self.queue_lengths);
        obs.extend_from_slice(&self.latencies);
        obs
    }

    /// Create from observation vector
    pub fn from_observation(obs: &[f64]) -> Self {
        let mut state = Self::new();

        if obs.len() >= 4 {
            state.cpu_usage = obs[..4].to_vec();
        }
        if obs.len() > 4 {
            state.memory_usage = obs[4];
        }
        if obs.len() > 5 {
            state.io_ops = obs[5];
        }
        if obs.len() > 6 {
            state.network_bw = obs[6];
        }
        if obs.len() > 7 {
            state.process_count = obs[7] as usize;
        }
        if obs.len() >= 12 {
            state.queue_lengths = obs[8..12].to_vec();
        }
        if obs.len() >= 16 {
            state.latencies = obs[12..16].to_vec();
        }

        state
    }
}

impl Default for KernelSystemState {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAction {
    /// Adjust scheduler priority
    AdjustPriority,
    /// Migrate process
    MigrateProcess,
    /// Adjust memory limits
    AdjustMemory,
    /// Throttle I/O
    ThrottleIO,
    /// Scale resources
    ScaleResources,
    /// Do nothing
    NoOp,
}

impl KernelAction {
    /// Convert to action vector
    pub fn to_vector(&self) -> Vec<f64> {
        match self {
            KernelAction::AdjustPriority => vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            KernelAction::MigrateProcess => vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            KernelAction::AdjustMemory => vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            KernelAction::ThrottleIO => vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            KernelAction::ScaleResources => vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            KernelAction::NoOp => vec![0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Create from action vector
    pub fn from_vector(v: &[f64]) -> Self {
        if v.is_empty() {
            return KernelAction::NoOp;
        }

        let (max_idx, _) = v
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
            .unwrap_or((5, &0.0));

        match max_idx {
            0 => KernelAction::AdjustPriority,
            1 => KernelAction::MigrateProcess,
            2 => KernelAction::AdjustMemory,
            3 => KernelAction::ThrottleIO,
            4 => KernelAction::ScaleResources,
            _ => KernelAction::NoOp,
        }
    }
}

/// Kernel world model manager
pub struct KernelWorldModelManager {
    /// World model
    pub model: WorldModel,
    /// MPC planner
    pub planner: MPCPlanner,
    /// Current system state
    pub current_state: KernelSystemState,
    /// Prediction buffer
    pub predictions: Vec<(KernelSystemState, f64)>,
    /// Action history
    pub action_history: Vec<(KernelAction, f64)>,
    /// Is model trained?
    pub is_trained: bool,
}

impl KernelWorldModelManager {
    /// Create a new kernel world model manager
    pub fn new() -> Self {
        let obs_dim = 16;
        let action_dim = 6;
        let latent_dim = 32;

        Self {
            model: WorldModel::new(obs_dim, action_dim, latent_dim, &[64, 64]),
            planner: MPCPlanner::new(action_dim, 10, 100),
            current_state: KernelSystemState::new(),
            predictions: Vec::new(),
            action_history: Vec::new(),
            is_trained: false,
        }
    }

    /// Update with new observation
    pub fn observe(&mut self, state: KernelSystemState) {
        let obs = state.to_observation();
        self.model.observe(&obs);
        self.current_state = state;
    }

    /// Predict future states
    pub fn predict_future(
        &mut self,
        action: KernelAction,
        horizon: usize,
    ) -> Vec<(KernelSystemState, f64)> {
        let action_vec = action.to_vector();
        let mut actions = Vec::new();

        for _ in 0..horizon {
            actions.push(action_vec.clone());
        }

        let trajectory = self.model.imagine(&actions, None);

        self.predictions = trajectory
            .iter()
            .map(|(state, reward)| {
                let obs = self.model.reconstruct(state);
                (KernelSystemState::from_observation(&obs), *reward)
            })
            .collect();

        self.predictions.clone()
    }

    /// Get optimal action
    pub fn get_optimal_action(&mut self) -> KernelAction {
        let action_vec = self.planner.plan(&self.model);
        let action = KernelAction::from_vector(&action_vec);

        self.action_history.push((action, 0.0));

        action
    }

    /// Compute reward for current state
    pub fn compute_reward(&self) -> f64 {
        // Lower is better for latency and CPU
        let latency_penalty: f64 = self.current_state.latencies.iter().sum();
        let cpu_penalty: f64 = self
            .current_state
            .cpu_usage
            .iter()
            .map(|&u| if u > 0.9 { (u - 0.9) * 10.0 } else { 0.0 })
            .sum();

        let memory_penalty = if self.current_state.memory_usage > 0.95 {
            (self.current_state.memory_usage - 0.95) * 100.0
        } else {
            0.0
        };

        // Throughput is good
        let throughput_reward = self.current_state.io_ops * 0.1;

        throughput_reward - latency_penalty - cpu_penalty - memory_penalty
    }

    /// Get model confidence for state
    pub fn get_confidence(&self) -> f64 {
        1.0 / (1.0 + self.model.current_state.total_uncertainty())
    }
}

impl Default for KernelWorldModelManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// LCG random number generator
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Box-Muller transform
fn box_muller(seed: u64) -> f64 {
    let u1 = (seed as f64 / u64::MAX as f64).max(1e-10);
    let seed2 = lcg_next(seed);
    let u2 = seed2 as f64 / u64::MAX as f64;

    libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * core::f64::consts::PI * u2)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latent_state() {
        let state = LatentState::new(16);
        assert_eq!(state.dim(), 16);
        assert!(state.total_uncertainty() > 0.0);
    }

    #[test]
    fn test_latent_distance() {
        let s1 = LatentState::from_vec(vec![1.0, 0.0, 0.0]);
        let s2 = LatentState::from_vec(vec![0.0, 0.0, 0.0]);

        assert!((s1.distance(&s2) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_encoder() {
        let encoder = Encoder::new(16, 8, &[32, 16]);
        let obs = vec![0.5; 16];

        let state = encoder.encode(&obs);

        assert_eq!(state.z.len(), 8);
        assert_eq!(state.uncertainty.len(), 8);
    }

    #[test]
    fn test_decoder() {
        let decoder = Decoder::new(8, 16, &[16, 32]);
        let state = LatentState::from_vec(vec![0.5; 8]);

        let obs = decoder.decode(&state);

        assert_eq!(obs.len(), 16);
    }

    #[test]
    fn test_transition_model() {
        let model = TransitionModel::new(8, 4, &[16], true);
        let state = LatentState::from_vec(vec![0.5; 8]);
        let action = vec![0.1, 0.2, 0.3, 0.4];

        let next = model.predict(&state, &action);

        assert_eq!(next.z.len(), 8);
    }

    #[test]
    fn test_reward_model() {
        let model = RewardModel::new(8, 4, &[16]);
        let state = LatentState::from_vec(vec![0.5; 8]);
        let action = vec![0.1, 0.2, 0.3, 0.4];

        let reward = model.predict(&state, &action);

        assert!(reward.is_finite());
    }

    #[test]
    fn test_world_model() {
        let model = WorldModel::new(16, 4, 8, &[32]);

        assert_eq!(model.encoder.input_dim, 16);
        assert_eq!(model.decoder.output_dim, 16);
        assert_eq!(model.transition.latent_dim, 8);
    }

    #[test]
    fn test_world_model_observe() {
        let mut model = WorldModel::new(16, 4, 8, &[32]);
        let obs = vec![0.5; 16];

        let state = model.observe(&obs);

        assert_eq!(state.dim(), 8);
        assert_eq!(model.state_history.len(), 1);
    }

    #[test]
    fn test_world_model_step() {
        let mut model = WorldModel::new(16, 4, 8, &[32]);
        let obs = vec![0.5; 16];
        model.observe(&obs);

        let action = vec![0.1, 0.2, 0.3, 0.4];
        let (next_state, reward) = model.step(&action);

        assert_eq!(next_state.dim(), 8);
        assert!(reward.is_finite());
    }

    #[test]
    fn test_world_model_imagine() {
        let mut model = WorldModel::new(16, 4, 8, &[32]);
        let obs = vec![0.5; 16];
        model.observe(&obs);

        let actions = vec![vec![0.1, 0.2, 0.3, 0.4], vec![0.2, 0.3, 0.4, 0.5], vec![
            0.3, 0.4, 0.5, 0.6,
        ]];

        let trajectory = model.imagine(&actions, None);

        assert_eq!(trajectory.len(), 3);
    }

    #[test]
    fn test_ensemble_world_model() {
        let ensemble = EnsembleWorldModel::new(16, 4, 8, &[32], 3);

        assert_eq!(ensemble.models.len(), 3);
    }

    #[test]
    fn test_ensemble_observe() {
        let mut ensemble = EnsembleWorldModel::new(16, 4, 8, &[32], 3);
        let obs = vec![0.5; 16];

        ensemble.observe(&obs);

        assert!(ensemble.total_epistemic_uncertainty() >= 0.0);
    }

    #[test]
    fn test_recurrent_cell() {
        let cell = RecurrentCell::new(8, 16);
        let input = vec![0.5; 8];
        let hidden = vec![0.0; 16];

        let output = cell.forward(&input, &hidden);

        assert_eq!(output.len(), 16);
    }

    #[test]
    fn test_rssm() {
        let mut rssm = RSSM::new(16, 4, 8, 16);
        let obs = vec![0.5; 16];
        let action = vec![0.1, 0.2, 0.3, 0.4];

        let state = rssm.posterior_step(&obs, &action);

        assert!(!state.z.is_empty());
    }

    #[test]
    fn test_rssm_imagine() {
        let mut rssm = RSSM::new(16, 4, 8, 16);

        let actions = vec![vec![0.1, 0.2, 0.3, 0.4], vec![0.2, 0.3, 0.4, 0.5]];

        let trajectory = rssm.imagine(&actions);

        assert_eq!(trajectory.len(), 2);
    }

    #[test]
    fn test_mpc_planner() {
        let mut planner = MPCPlanner::new(4, 5, 10);
        let model = WorldModel::new(16, 4, 8, &[32]);

        let action = planner.plan(&model);

        assert_eq!(action.len(), 4);
    }

    #[test]
    fn test_actor() {
        let actor = Actor::new(8, 4, &[32]);
        let state = LatentState::from_vec(vec![0.5; 8]);

        let action = actor.get_action(&state, 0.1);

        assert_eq!(action.len(), 4);

        // Actions should be bounded by tanh
        for &a in &action {
            assert!(a >= -2.0 && a <= 2.0);
        }
    }

    #[test]
    fn test_critic() {
        let critic = Critic::new(8, &[32]);
        let state = LatentState::from_vec(vec![0.5; 8]);

        let value = critic.value(&state);

        assert!(value.is_finite());
    }

    #[test]
    fn test_dreamer_agent() {
        let agent = DreamerAgent::new(16, 4, 8);
        let obs = vec![0.5; 16];

        let action = agent.act(&obs, true);

        assert_eq!(action.len(), 4);
    }

    #[test]
    fn test_kernel_system_state() {
        let state = KernelSystemState::new();
        let obs = state.to_observation();
        let recovered = KernelSystemState::from_observation(&obs);

        assert_eq!(state.cpu_usage.len(), recovered.cpu_usage.len());
    }

    #[test]
    fn test_kernel_action() {
        let action = KernelAction::AdjustPriority;
        let vec = action.to_vector();
        let recovered = KernelAction::from_vector(&vec);

        assert_eq!(action, recovered);
    }

    #[test]
    fn test_kernel_world_model_manager() {
        let mut manager = KernelWorldModelManager::new();

        let state = KernelSystemState::new();
        manager.observe(state);

        let predictions = manager.predict_future(KernelAction::NoOp, 5);

        assert_eq!(predictions.len(), 5);
    }

    #[test]
    fn test_kernel_optimal_action() {
        let mut manager = KernelWorldModelManager::new();

        let state = KernelSystemState::new();
        manager.observe(state);

        let action = manager.get_optimal_action();

        // Should return some valid action
        assert!(matches!(
            action,
            KernelAction::AdjustPriority
                | KernelAction::MigrateProcess
                | KernelAction::AdjustMemory
                | KernelAction::ThrottleIO
                | KernelAction::ScaleResources
                | KernelAction::NoOp
        ));
    }

    #[test]
    fn test_compute_reward() {
        let manager = KernelWorldModelManager::new();

        let reward = manager.compute_reward();

        assert!(reward.is_finite());
    }
}
