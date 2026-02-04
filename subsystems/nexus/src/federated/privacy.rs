//! Differential privacy mechanisms for federated learning.

use crate::federated::fedavg::FedAvgAggregator;
use crate::federated::model::FederatedModel;
use crate::federated::types::{box_muller, lcg_next};
use crate::federated::update::ModelUpdate;

/// Differential privacy mechanism
#[derive(Debug, Clone)]
pub struct DifferentialPrivacy {
    /// Noise multiplier (σ)
    pub noise_multiplier: f64,
    /// Clipping bound (C)
    pub clip_bound: f64,
    /// Target epsilon
    pub target_epsilon: f64,
    /// Target delta
    pub target_delta: f64,
    /// Privacy accountant
    pub spent_epsilon: f64,
    /// RNG state
    rng_state: u64,
}

impl DifferentialPrivacy {
    /// Create a new DP mechanism
    pub fn new(noise_multiplier: f64, clip_bound: f64) -> Self {
        Self {
            noise_multiplier,
            clip_bound,
            target_epsilon: 1.0,
            target_delta: 1e-5,
            spent_epsilon: 0.0,
            rng_state: 12345,
        }
    }

    /// Clip gradient
    pub fn clip(&self, gradient: &mut [f64]) {
        let norm: f64 = libm::sqrt(gradient.iter().map(|x| x * x).sum());

        if norm > self.clip_bound {
            let scale = self.clip_bound / norm;
            for g in gradient {
                *g *= scale;
            }
        }
    }

    /// Add Gaussian noise
    pub fn add_noise(&mut self, gradient: &mut [f64]) {
        let noise_scale = self.clip_bound * self.noise_multiplier;

        for g in gradient {
            self.rng_state = lcg_next(self.rng_state);
            let noise = box_muller(self.rng_state) * noise_scale;
            *g += noise;
        }
    }

    /// Privatize update
    pub fn privatize(&mut self, update: &mut ModelUpdate) {
        self.clip(&mut update.delta);
        self.add_noise(&mut update.delta);

        // Update privacy budget (simplified accounting)
        self.spent_epsilon += self.compute_step_epsilon();
    }

    /// Compute epsilon for one step
    fn compute_step_epsilon(&self) -> f64 {
        // Simplified: ε ≈ C / (σ * √n)
        // Using a rough approximation
        let q = 0.01; // Sampling rate
        let sigma = self.noise_multiplier;

        q * libm::sqrt(2.0 * libm::log(1.25 / self.target_delta)) / sigma
    }

    /// Check if privacy budget exhausted
    pub fn is_budget_exhausted(&self) -> bool {
        self.spent_epsilon >= self.target_epsilon
    }

    /// Get remaining privacy budget
    pub fn remaining_budget(&self) -> f64 {
        (self.target_epsilon - self.spent_epsilon).max(0.0)
    }
}

/// DP-FedAvg aggregator
#[derive(Debug, Clone)]
pub struct DPFedAvgAggregator {
    /// Base aggregator
    pub base: FedAvgAggregator,
    /// Differential privacy mechanism
    pub dp: DifferentialPrivacy,
    /// Per-client privacy
    pub per_client_dp: bool,
}

impl DPFedAvgAggregator {
    /// Create a new DP-FedAvg aggregator
    pub fn new(model: FederatedModel, noise_multiplier: f64, clip_bound: f64) -> Self {
        Self {
            base: FedAvgAggregator::new(model),
            dp: DifferentialPrivacy::new(noise_multiplier, clip_bound),
            per_client_dp: false,
        }
    }

    /// Submit update with privacy
    pub fn submit_update(&mut self, mut update: ModelUpdate) {
        if self.per_client_dp {
            self.dp.privatize(&mut update);
        }
        self.base.submit_update(update);
    }

    /// Aggregate with server-side DP
    pub fn aggregate(&mut self) -> bool {
        if !self.per_client_dp {
            // Apply DP to aggregated update
            for update in &mut self.base.pending_updates {
                self.dp.clip(&mut update.delta);
            }
        }

        let result = self.base.aggregate();

        if result && !self.per_client_dp {
            // Add noise to aggregated model
            self.dp.add_noise(&mut self.base.global_model.parameters);
        }

        result
    }
}
