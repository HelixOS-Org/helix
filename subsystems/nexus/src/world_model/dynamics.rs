//! Transition dynamics and reward models for the world model.

use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::latent::LatentState;

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
    #[inline]
    pub fn predict_residual(&self, state: &LatentState, action: &[f64]) -> LatentState {
        let mut next = self.predict(state, action);

        // Add residual
        for (z, &s) in next.z.iter_mut().zip(state.z.iter()) {
            *z += s;
        }

        next
    }
}

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
