//! Federated Averaging (FedAvg) aggregator.

use alloc::vec;
use alloc::vec::Vec;

use crate::federated::model::FederatedModel;
use crate::federated::update::ModelUpdate;

/// FedAvg aggregator
#[derive(Debug, Clone)]
pub struct FedAvgAggregator {
    /// Current global model
    pub global_model: FederatedModel,
    /// Pending updates
    pub pending_updates: Vec<ModelUpdate>,
    /// Minimum updates before aggregation
    pub min_updates: usize,
    /// Learning rate for updates
    pub learning_rate: f64,
    /// Weight by sample count
    pub weighted: bool,
}

impl FedAvgAggregator {
    /// Create a new FedAvg aggregator
    pub fn new(model: FederatedModel) -> Self {
        Self {
            global_model: model,
            pending_updates: Vec::new(),
            min_updates: 2,
            learning_rate: 1.0,
            weighted: true,
        }
    }

    /// Submit a client update
    pub fn submit_update(&mut self, update: ModelUpdate) {
        self.pending_updates.push(update);
    }

    /// Check if ready to aggregate
    pub fn ready_to_aggregate(&self) -> bool {
        self.pending_updates.len() >= self.min_updates
    }

    /// Aggregate updates using FedAvg
    pub fn aggregate(&mut self) -> bool {
        if !self.ready_to_aggregate() {
            return false;
        }

        let num_params = self.global_model.num_parameters();
        let mut aggregated = vec![0.0; num_params];

        // Compute weights
        let total_samples: usize = self.pending_updates.iter().map(|u| u.num_samples).sum();

        if total_samples == 0 {
            self.pending_updates.clear();
            return false;
        }

        // Weighted average of updates
        for update in &self.pending_updates {
            let weight = if self.weighted && total_samples > 0 {
                update.num_samples as f64 / total_samples as f64
            } else {
                1.0 / self.pending_updates.len() as f64
            };

            for (a, &d) in aggregated.iter_mut().zip(update.delta.iter()) {
                *a += weight * d;
            }
        }

        // Apply aggregated update
        for (p, &a) in self
            .global_model
            .parameters
            .iter_mut()
            .zip(aggregated.iter())
        {
            *p += self.learning_rate * a;
        }

        self.global_model.version += 1;
        self.pending_updates.clear();

        true
    }

    /// Get current global model
    pub fn get_global_model(&self) -> &FederatedModel {
        &self.global_model
    }
}
