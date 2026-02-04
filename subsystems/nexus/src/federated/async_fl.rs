//! Asynchronous federated learning.

use alloc::collections::BTreeMap;

use crate::federated::model::FederatedModel;
use crate::federated::update::ModelUpdate;

/// Staleness handling for async FL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StalenessStrategy {
    /// Accept all updates
    NoLimit,
    /// Reject if too stale
    Threshold,
    /// Weight by staleness
    Weighted,
    /// Polynomial decay
    PolynomialDecay,
}

/// Async federated aggregator
#[derive(Debug, Clone)]
pub struct AsyncFedAggregator {
    /// Global model
    pub global_model: FederatedModel,
    /// Staleness strategy
    pub staleness_strategy: StalenessStrategy,
    /// Max staleness
    pub max_staleness: u64,
    /// Staleness decay factor
    pub decay_factor: f64,
    /// Client versions
    pub client_versions: BTreeMap<u32, u64>,
}

impl AsyncFedAggregator {
    /// Create a new async aggregator
    pub fn new(model: FederatedModel, strategy: StalenessStrategy) -> Self {
        Self {
            global_model: model,
            staleness_strategy: strategy,
            max_staleness: 10,
            decay_factor: 0.5,
            client_versions: BTreeMap::new(),
        }
    }

    /// Compute staleness weight
    pub fn staleness_weight(&self, client_version: u64) -> f64 {
        let staleness = self.global_model.version.saturating_sub(client_version);

        match self.staleness_strategy {
            StalenessStrategy::NoLimit => 1.0,
            StalenessStrategy::Threshold => {
                if staleness <= self.max_staleness {
                    1.0
                } else {
                    0.0
                }
            }
            StalenessStrategy::Weighted => 1.0 / (1.0 + staleness as f64),
            StalenessStrategy::PolynomialDecay => libm::pow(self.decay_factor, staleness as f64),
        }
    }

    /// Apply update asynchronously
    pub fn apply_update(&mut self, update: &ModelUpdate, client_version: u64) -> bool {
        let weight = self.staleness_weight(client_version);

        if weight <= 0.0 {
            return false;
        }

        // Apply weighted update
        for (p, &d) in self
            .global_model
            .parameters
            .iter_mut()
            .zip(update.delta.iter())
        {
            *p += weight * d;
        }

        self.global_model.version += 1;
        self.client_versions
            .insert(update.client_id, self.global_model.version);

        true
    }

    /// Get model for client
    pub fn get_model_for_client(&self, _client_id: u32) -> (FederatedModel, u64) {
        let version = self.global_model.version;
        (self.global_model.clone(), version)
    }
}
