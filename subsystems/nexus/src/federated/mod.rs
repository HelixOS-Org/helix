//! # Federated Learning Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary federated learning system that enables
//! distributed kernel AI training across multiple nodes while preserving
//! privacy and maintaining system security.
//!
//! ## Key Features
//!
//! - **Federated Averaging (FedAvg)**: Distributed model averaging
//! - **Secure Aggregation**: Privacy-preserving gradient aggregation
//! - **Differential Privacy**: Formal privacy guarantees
//! - **Byzantine Fault Tolerance**: Robust to malicious clients
//! - **Personalized FL**: Node-specific model adaptation
//! - **Asynchronous FL**: Handle heterogeneous nodes
//!
//! ## Kernel Applications
//!
//! - Distributed scheduler optimization
//! - Cross-node anomaly detection
//! - Collaborative resource prediction
//! - Privacy-preserving system monitoring

#![no_std]

extern crate alloc;

// Module declarations
mod async_fl;
mod byzantine;
mod fedavg;
mod kernel;
mod model;
mod personalized;
mod privacy;
mod secure;
mod types;
mod update;

// Re-exports for public API
pub use async_fl::{AsyncFedAggregator, StalenessStrategy};
pub use byzantine::{ByzantineDefense, ByzantineRobustAggregator};
pub use fedavg::FedAvgAggregator;
pub use kernel::{FederatedStats, KernelFederatedManager, KernelNodeRole};
pub use model::FederatedModel;
pub use personalized::{PersonalizedFLClient, PersonalizedModel, PersonalizationMethod};
pub use privacy::{DPFedAvgAggregator, DifferentialPrivacy};
pub use secure::{SecretShare, SecureAggregation};
pub use types::{
    box_muller, lcg_next, DEFAULT_BATCH_SIZE, DEFAULT_CLIP_BOUND, DEFAULT_LOCAL_EPOCHS, DEFAULT_LR,
    DEFAULT_NOISE_MULTIPLIER, MAX_CLIENTS,
};
pub use update::ModelUpdate;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federated_model() {
        let model = FederatedModel::from_layers(&[(10, 32), (32, 5)], 12345);

        assert!(model.num_parameters() > 0);
        assert!(model.norm() > 0.0);
    }

    #[test]
    fn test_model_update() {
        let delta = alloc::vec![0.1; 100];
        let mut update = ModelUpdate::new(delta, 1, 50);

        update.clip(0.5);

        assert!(update.norm() <= 0.5 + 1e-10);
    }

    #[test]
    fn test_fedavg_aggregator() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let mut aggregator = FedAvgAggregator::new(model);

        // Submit updates
        let update1 = ModelUpdate::new(
            alloc::vec![0.1; aggregator.global_model.num_parameters()],
            1,
            100,
        );
        let update2 = ModelUpdate::new(
            alloc::vec![0.2; aggregator.global_model.num_parameters()],
            2,
            100,
        );

        aggregator.submit_update(update1);
        aggregator.submit_update(update2);

        assert!(aggregator.ready_to_aggregate());
        assert!(aggregator.aggregate());
    }

    #[test]
    fn test_differential_privacy() {
        let dp = DifferentialPrivacy::new(1.0, 1.0);

        let mut gradient = alloc::vec![2.0; 10];
        dp.clip(&mut gradient);

        // Should be clipped to norm 1
        let norm: f64 = libm::sqrt(gradient.iter().map(|x| x * x).sum());
        assert!(norm <= 1.0 + 1e-10);
    }

    #[test]
    fn test_dp_noise() {
        let mut dp = DifferentialPrivacy::new(1.0, 1.0);

        let mut gradient = alloc::vec![0.5; 10];
        let original = gradient.clone();

        dp.add_noise(&mut gradient);

        // Should be different after noise
        let diff: f64 = gradient
            .iter()
            .zip(original.iter())
            .map(|(&a, &b)| (a - b).abs())
            .sum();

        assert!(diff > 0.0);
    }

    #[test]
    fn test_secure_aggregation() {
        let mut sa = SecureAggregation::new(3, 2);

        let values = alloc::vec![1.0, 2.0, 3.0];
        let shares = sa.create_shares(&values, 0);

        assert_eq!(shares.len(), 3);
    }

    #[test]
    fn test_byzantine_trimmed_mean() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let mut aggregator = ByzantineRobustAggregator::new(model, ByzantineDefense::TrimmedMean);

        let num_params = aggregator.global_model.num_parameters();

        for i in 0..5 {
            let delta = alloc::vec![i as f64; num_params];
            aggregator.submit_update(ModelUpdate::new(delta, i, 100));
        }

        assert!(aggregator.aggregate());
    }

    #[test]
    fn test_byzantine_krum() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let mut aggregator = ByzantineRobustAggregator::new(model, ByzantineDefense::Krum);

        let num_params = aggregator.global_model.num_parameters();

        // Submit normal updates
        for i in 0..4 {
            let delta = alloc::vec![0.1; num_params];
            aggregator.submit_update(ModelUpdate::new(delta, i, 100));
        }

        // Submit byzantine update
        let byzantine_delta = alloc::vec![100.0; num_params];
        aggregator.submit_update(ModelUpdate::new(byzantine_delta, 99, 100));

        assert!(aggregator.aggregate());

        // Byzantine client should be suspected
        assert!(aggregator.suspected_clients.contains(&99));
    }

    #[test]
    fn test_personalized_model() {
        let mut model = PersonalizedModel::new(100, 20, PersonalizationMethod::FineTuning);

        model.alpha = 0.8;

        let global = alloc::vec![1.0; 100];
        model.update_from_global(&global);

        // Shared should be influenced by global
        assert!(model.shared[0] > 0.0);
    }

    #[test]
    fn test_async_aggregator() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let mut aggregator = AsyncFedAggregator::new(model, StalenessStrategy::Weighted);

        let update = ModelUpdate::new(
            alloc::vec![0.1; aggregator.global_model.num_parameters()],
            1,
            100,
        );

        assert!(aggregator.apply_update(&update, 0));
        assert_eq!(aggregator.global_model.version, 1);
    }

    #[test]
    fn test_staleness_weight() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let aggregator = AsyncFedAggregator::new(model, StalenessStrategy::Weighted);

        let weight_fresh = aggregator.staleness_weight(aggregator.global_model.version);
        let weight_stale = aggregator.staleness_weight(0);

        assert!(weight_fresh > weight_stale);
    }

    #[test]
    fn test_kernel_fl_server() {
        let mut manager = KernelFederatedManager::new(KernelNodeRole::Server, &[(10, 32), (32, 5)]);

        manager.register_node(1);
        manager.register_node(2);

        assert_eq!(manager.connected_nodes.len(), 2);
    }

    #[test]
    fn test_kernel_fl_aggregation() {
        let mut manager = KernelFederatedManager::new(KernelNodeRole::Server, &[(10, 32)]);

        let num_params = manager.model.num_parameters();

        manager.submit_client_update(ModelUpdate::new(alloc::vec![0.1; num_params], 1, 100));
        manager.submit_client_update(ModelUpdate::new(alloc::vec![0.2; num_params], 2, 100));

        assert!(manager.aggregate());
        assert_eq!(manager.rounds_completed, 1);
    }

    #[test]
    fn test_fl_stats() {
        let manager = KernelFederatedManager::new(KernelNodeRole::Server, &[(10, 32)]);

        let stats = manager.get_stats();

        assert_eq!(stats.role, KernelNodeRole::Server);
        assert_eq!(stats.rounds_completed, 0);
    }
}
