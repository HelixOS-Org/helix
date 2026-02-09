//! Personalized federated learning.

use alloc::vec;
use alloc::vec::Vec;

use crate::federated::update::ModelUpdate;

/// Personalization method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalizationMethod {
    /// Fine-tuning on local data
    FineTuning,
    /// MAML-style meta-learning
    MAML,
    /// Local adaptation layers
    LocalAdapter,
    /// Mixture of experts
    MixtureOfExperts,
}

/// Personalized federated model
#[derive(Debug, Clone)]
pub struct PersonalizedModel {
    /// Shared global parameters
    pub shared: Vec<f64>,
    /// Local personalized parameters
    pub local: Vec<f64>,
    /// Personalization method
    pub method: PersonalizationMethod,
    /// Mixing coefficient (global vs local)
    pub alpha: f64,
}

impl PersonalizedModel {
    /// Create a new personalized model
    pub fn new(shared_size: usize, local_size: usize, method: PersonalizationMethod) -> Self {
        Self {
            shared: vec![0.0; shared_size],
            local: vec![0.0; local_size],
            method,
            alpha: 0.5,
        }
    }

    /// Get combined parameters
    #[inline]
    pub fn get_combined(&self) -> Vec<f64> {
        let mut combined = self.shared.clone();
        combined.extend_from_slice(&self.local);
        combined
    }

    /// Update from global model
    #[inline]
    pub fn update_from_global(&mut self, global: &[f64]) {
        for (s, &g) in self.shared.iter_mut().zip(global.iter()) {
            *s = self.alpha * g + (1.0 - self.alpha) * *s;
        }
    }

    /// Get local update
    #[inline]
    pub fn get_local_update(&self, new_local: &[f64]) -> Vec<f64> {
        self.local
            .iter()
            .zip(new_local.iter())
            .map(|(&old, &new)| new - old)
            .collect()
    }
}

/// Personalized FL client
#[derive(Debug, Clone)]
pub struct PersonalizedFLClient {
    /// Client ID
    pub client_id: u32,
    /// Personalized model
    pub model: PersonalizedModel,
    /// Local data statistics
    pub num_samples: usize,
    /// Local adaptation steps
    pub adaptation_steps: usize,
}

impl PersonalizedFLClient {
    /// Create a new client
    pub fn new(client_id: u32, shared_size: usize, local_size: usize) -> Self {
        Self {
            client_id,
            model: PersonalizedModel::new(
                shared_size,
                local_size,
                PersonalizationMethod::FineTuning,
            ),
            num_samples: 0,
            adaptation_steps: 5,
        }
    }

    /// Receive global model update
    #[inline(always)]
    pub fn receive_global(&mut self, global_params: &[f64]) {
        self.model.update_from_global(global_params);
    }

    /// Compute update to send
    #[inline]
    pub fn compute_update(&self) -> ModelUpdate {
        // In real implementation, would compute gradient from local training
        let delta = self.model.shared.clone();

        ModelUpdate::new(delta, self.client_id, self.num_samples)
    }
}
