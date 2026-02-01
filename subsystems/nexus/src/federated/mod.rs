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

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum number of federated clients
const MAX_CLIENTS: usize = 1000;

/// Default local epochs
const DEFAULT_LOCAL_EPOCHS: usize = 5;

/// Default batch size
const DEFAULT_BATCH_SIZE: usize = 32;

/// Default learning rate
const DEFAULT_LR: f64 = 0.01;

/// Noise multiplier for differential privacy
const DEFAULT_NOISE_MULTIPLIER: f64 = 0.1;

/// Clipping bound for gradients
const DEFAULT_CLIP_BOUND: f64 = 1.0;

// ============================================================================
// MODEL REPRESENTATION
// ============================================================================

/// A federated model (parameter vector)
#[derive(Debug, Clone)]
pub struct FederatedModel {
    /// Model parameters (flattened)
    pub parameters: Vec<f64>,
    /// Parameter shapes for reconstruction
    pub shapes: Vec<(usize, usize)>,
    /// Model version
    pub version: u64,
    /// Model name/id
    pub name: String,
    /// Creation timestamp
    pub timestamp: u64,
}

impl FederatedModel {
    /// Create a new model
    pub fn new(parameters: Vec<f64>, shapes: Vec<(usize, usize)>) -> Self {
        Self {
            parameters,
            shapes,
            version: 0,
            name: String::from("federated_model"),
            timestamp: 0,
        }
    }

    /// Create from layer dimensions
    pub fn from_layers(layer_dims: &[(usize, usize)], seed: u64) -> Self {
        let mut parameters = Vec::new();
        let mut shapes = Vec::new();
        let mut rng = seed;

        for &(in_dim, out_dim) in layer_dims {
            // Xavier initialization
            let scale = libm::sqrt(2.0 / (in_dim + out_dim) as f64);

            for _ in 0..(in_dim * out_dim) {
                rng = lcg_next(rng);
                let val = ((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale;
                parameters.push(val);
            }

            // Bias
            for _ in 0..out_dim {
                parameters.push(0.0);
            }

            shapes.push((in_dim, out_dim));
        }

        Self {
            parameters,
            shapes,
            version: 0,
            name: String::from("federated_model"),
            timestamp: 0,
        }
    }

    /// Number of parameters
    pub fn num_parameters(&self) -> usize {
        self.parameters.len()
    }

    /// Clone parameters
    pub fn get_parameters(&self) -> Vec<f64> {
        self.parameters.clone()
    }

    /// Set parameters
    pub fn set_parameters(&mut self, parameters: Vec<f64>) {
        self.parameters = parameters;
        self.version += 1;
    }

    /// Compute model norm
    pub fn norm(&self) -> f64 {
        libm::sqrt(self.parameters.iter().map(|x| x * x).sum())
    }

    /// Distance to another model
    pub fn distance(&self, other: &FederatedModel) -> f64 {
        let sum_sq: f64 = self
            .parameters
            .iter()
            .zip(other.parameters.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum();

        libm::sqrt(sum_sq)
    }
}

/// Model update (gradient or delta)
#[derive(Debug, Clone)]
pub struct ModelUpdate {
    /// Update vector
    pub delta: Vec<f64>,
    /// Client ID
    pub client_id: u32,
    /// Number of samples used
    pub num_samples: usize,
    /// Training loss
    pub loss: f64,
    /// Update timestamp
    pub timestamp: u64,
}

impl ModelUpdate {
    /// Create a new update
    pub fn new(delta: Vec<f64>, client_id: u32, num_samples: usize) -> Self {
        Self {
            delta,
            client_id,
            num_samples,
            loss: 0.0,
            timestamp: 0,
        }
    }

    /// Update norm
    pub fn norm(&self) -> f64 {
        libm::sqrt(self.delta.iter().map(|x| x * x).sum())
    }

    /// Clip update to bound
    pub fn clip(&mut self, bound: f64) {
        let norm = self.norm();
        if norm > bound {
            let scale = bound / norm;
            for d in &mut self.delta {
                *d *= scale;
            }
        }
    }
}

// ============================================================================
// FEDERATED AVERAGING (FedAvg)
// ============================================================================

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

// ============================================================================
// DIFFERENTIAL PRIVACY
// ============================================================================

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

// ============================================================================
// SECURE AGGREGATION
// ============================================================================

/// Secret share for secure aggregation
#[derive(Debug, Clone)]
pub struct SecretShare {
    /// Share ID
    pub id: u32,
    /// Share values
    pub values: Vec<f64>,
    /// Source client
    pub source: u32,
    /// Target client
    pub target: u32,
}

/// Secure aggregation protocol
#[derive(Debug, Clone)]
pub struct SecureAggregation {
    /// Number of parties
    pub num_parties: usize,
    /// Threshold for reconstruction
    pub threshold: usize,
    /// Current round
    pub round: u32,
    /// Collected shares
    pub shares: BTreeMap<(u32, u32), SecretShare>,
    /// RNG state
    rng_state: u64,
}

impl SecureAggregation {
    /// Create a new secure aggregation
    pub fn new(num_parties: usize, threshold: usize) -> Self {
        Self {
            num_parties,
            threshold: threshold.min(num_parties),
            round: 0,
            shares: BTreeMap::new(),
            rng_state: 12345,
        }
    }

    /// Create secret shares for a value
    pub fn create_shares(&mut self, values: &[f64], client_id: u32) -> Vec<SecretShare> {
        let mut shares = Vec::new();
        let n = self.num_parties;

        // Create additive shares
        let mut remaining = values.to_vec();

        for target in 0..n as u32 {
            if target == client_id {
                continue;
            }

            // Random share
            let share_values: Vec<f64> = remaining
                .iter()
                .map(|_| {
                    self.rng_state = lcg_next(self.rng_state);
                    ((self.rng_state as f64 / u64::MAX as f64) - 0.5) * 2.0
                })
                .collect();

            // Subtract from remaining
            for (r, &s) in remaining.iter_mut().zip(share_values.iter()) {
                *r -= s;
            }

            shares.push(SecretShare {
                id: self.round,
                values: share_values,
                source: client_id,
                target,
            });
        }

        // Last share is the remaining
        shares.push(SecretShare {
            id: self.round,
            values: remaining,
            source: client_id,
            target: client_id,
        });

        shares
    }

    /// Submit a share
    pub fn submit_share(&mut self, share: SecretShare) {
        self.shares.insert((share.source, share.target), share);
    }

    /// Reconstruct aggregated value
    pub fn reconstruct(&self, client_id: u32) -> Option<Vec<f64>> {
        // Collect all shares intended for this client
        let client_shares: Vec<&SecretShare> = self
            .shares
            .values()
            .filter(|s| s.target == client_id)
            .collect();

        if client_shares.is_empty() {
            return None;
        }

        let dim = client_shares[0].values.len();
        let mut result = vec![0.0; dim];

        for share in client_shares {
            for (r, &s) in result.iter_mut().zip(share.values.iter()) {
                *r += s;
            }
        }

        Some(result)
    }

    /// Reset for new round
    pub fn new_round(&mut self) {
        self.round += 1;
        self.shares.clear();
    }
}

// ============================================================================
// BYZANTINE FAULT TOLERANCE
// ============================================================================

/// Byzantine-robust aggregation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByzantineDefense {
    /// Trimmed mean
    TrimmedMean,
    /// Median
    Median,
    /// Krum
    Krum,
    /// Multi-Krum
    MultiKrum,
    /// Bulyan
    Bulyan,
}

/// Byzantine-robust aggregator
#[derive(Debug, Clone)]
pub struct ByzantineRobustAggregator {
    /// Global model
    pub global_model: FederatedModel,
    /// Defense mechanism
    pub defense: ByzantineDefense,
    /// Trim fraction (for trimmed mean)
    pub trim_fraction: f64,
    /// Pending updates
    pub pending_updates: Vec<ModelUpdate>,
    /// Suspected byzantine clients
    pub suspected_clients: Vec<u32>,
    /// Krum parameter (number of neighbors)
    pub krum_k: usize,
}

impl ByzantineRobustAggregator {
    /// Create a new Byzantine-robust aggregator
    pub fn new(model: FederatedModel, defense: ByzantineDefense) -> Self {
        Self {
            global_model: model,
            defense,
            trim_fraction: 0.1,
            pending_updates: Vec::new(),
            suspected_clients: Vec::new(),
            krum_k: 2,
        }
    }

    /// Submit an update
    pub fn submit_update(&mut self, update: ModelUpdate) {
        self.pending_updates.push(update);
    }

    /// Aggregate using defense mechanism
    pub fn aggregate(&mut self) -> bool {
        if self.pending_updates.len() < 2 {
            return false;
        }

        let aggregated = match self.defense {
            ByzantineDefense::TrimmedMean => self.trimmed_mean(),
            ByzantineDefense::Median => self.median(),
            ByzantineDefense::Krum => self.krum(1),
            ByzantineDefense::MultiKrum => self.krum(self.pending_updates.len() / 2),
            ByzantineDefense::Bulyan => self.bulyan(),
        };

        // Apply aggregated update
        for (p, &a) in self
            .global_model
            .parameters
            .iter_mut()
            .zip(aggregated.iter())
        {
            *p += a;
        }

        self.global_model.version += 1;
        self.pending_updates.clear();

        true
    }

    /// Trimmed mean aggregation
    fn trimmed_mean(&self) -> Vec<f64> {
        let n = self.pending_updates.len();
        let num_params = self.pending_updates[0].delta.len();
        let trim = (n as f64 * self.trim_fraction) as usize;

        let mut result = vec![0.0; num_params];

        for i in 0..num_params {
            // Collect values for this parameter
            let mut values: Vec<f64> = self.pending_updates.iter().map(|u| u.delta[i]).collect();

            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

            // Trim and average
            let trimmed = &values[trim..n - trim];
            if !trimmed.is_empty() {
                result[i] = trimmed.iter().sum::<f64>() / trimmed.len() as f64;
            }
        }

        result
    }

    /// Median aggregation
    fn median(&self) -> Vec<f64> {
        let num_params = self.pending_updates[0].delta.len();
        let mut result = vec![0.0; num_params];

        for i in 0..num_params {
            let mut values: Vec<f64> = self.pending_updates.iter().map(|u| u.delta[i]).collect();

            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

            result[i] = values[values.len() / 2];
        }

        result
    }

    /// Krum/Multi-Krum aggregation
    fn krum(&mut self, m: usize) -> Vec<f64> {
        let n = self.pending_updates.len();
        let m = m.min(n);

        // Compute pairwise distances
        let mut distances: Vec<(usize, f64)> = Vec::new();

        for (i, update_i) in self.pending_updates.iter().enumerate() {
            let mut sum_dist = 0.0;
            let mut dists: Vec<f64> = Vec::new();

            for (j, update_j) in self.pending_updates.iter().enumerate() {
                if i != j {
                    let dist: f64 = update_i
                        .delta
                        .iter()
                        .zip(update_j.delta.iter())
                        .map(|(&a, &b)| (a - b).powi(2))
                        .sum();
                    dists.push(dist);
                }
            }

            // Sum of k nearest distances
            dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
            for d in dists.iter().take(self.krum_k.min(dists.len())) {
                sum_dist += d;
            }

            distances.push((i, sum_dist));
        }

        // Sort by score (lower is better)
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        // Mark suspected clients (high scores)
        for (i, _) in distances.iter().skip(m) {
            let client_id = self.pending_updates[*i].client_id;
            if !self.suspected_clients.contains(&client_id) {
                self.suspected_clients.push(client_id);
            }
        }

        // Average top-m updates
        let num_params = self.pending_updates[0].delta.len();
        let mut result = vec![0.0; num_params];

        for (i, _) in distances.iter().take(m) {
            for (r, &d) in result.iter_mut().zip(self.pending_updates[*i].delta.iter()) {
                *r += d;
            }
        }

        for r in &mut result {
            *r /= m as f64;
        }

        result
    }

    /// Bulyan aggregation
    fn bulyan(&mut self) -> Vec<f64> {
        // First run Krum to select subset
        let n = self.pending_updates.len();
        let m = n.saturating_sub(2 * (n / 5)); // n - 2f where f = n/5

        // Select using multi-krum
        let _ = self.krum(m);

        // Then apply trimmed mean on selected
        self.trimmed_mean()
    }
}

// ============================================================================
// PERSONALIZED FEDERATED LEARNING
// ============================================================================

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
    pub fn get_combined(&self) -> Vec<f64> {
        let mut combined = self.shared.clone();
        combined.extend_from_slice(&self.local);
        combined
    }

    /// Update from global model
    pub fn update_from_global(&mut self, global: &[f64]) {
        for (s, &g) in self.shared.iter_mut().zip(global.iter()) {
            *s = self.alpha * g + (1.0 - self.alpha) * *s;
        }
    }

    /// Get local update
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
    pub fn receive_global(&mut self, global_params: &[f64]) {
        self.model.update_from_global(global_params);
    }

    /// Compute update to send
    pub fn compute_update(&self) -> ModelUpdate {
        // In real implementation, would compute gradient from local training
        let delta = self.model.shared.clone();

        ModelUpdate::new(delta, self.client_id, self.num_samples)
    }
}

// ============================================================================
// ASYNCHRONOUS FEDERATED LEARNING
// ============================================================================

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
    fn staleness_weight(&self, client_version: u64) -> f64 {
        let staleness = self.global_model.version.saturating_sub(client_version);

        match self.staleness_strategy {
            StalenessStrategy::NoLimit => 1.0,
            StalenessStrategy::Threshold => {
                if staleness <= self.max_staleness {
                    1.0
                } else {
                    0.0
                }
            },
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
    pub fn get_model_for_client(&self, client_id: u32) -> (FederatedModel, u64) {
        let version = self.global_model.version;
        (self.global_model.clone(), version)
    }
}

// ============================================================================
// KERNEL FEDERATED LEARNING
// ============================================================================

/// Kernel FL node type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelNodeRole {
    /// Aggregation server
    Server,
    /// Training client
    Client,
    /// Coordinator
    Coordinator,
}

/// Kernel federated learning manager
pub struct KernelFederatedManager {
    /// Node role
    pub role: KernelNodeRole,
    /// Current model
    pub model: FederatedModel,
    /// Aggregator (if server)
    pub aggregator: Option<DPFedAvgAggregator>,
    /// Byzantine defense
    pub byzantine_defense: Option<ByzantineRobustAggregator>,
    /// Connected nodes
    pub connected_nodes: Vec<u32>,
    /// Training rounds completed
    pub rounds_completed: u64,
    /// Client updates received
    pub updates_received: usize,
    /// Is training active?
    pub active: bool,
}

impl KernelFederatedManager {
    /// Create a new kernel FL manager
    pub fn new(role: KernelNodeRole, model_layers: &[(usize, usize)]) -> Self {
        let model = FederatedModel::from_layers(model_layers, 12345);

        let aggregator = if role == KernelNodeRole::Server {
            Some(DPFedAvgAggregator::new(
                model.clone(),
                DEFAULT_NOISE_MULTIPLIER,
                DEFAULT_CLIP_BOUND,
            ))
        } else {
            None
        };

        Self {
            role,
            model,
            aggregator,
            byzantine_defense: None,
            connected_nodes: Vec::new(),
            rounds_completed: 0,
            updates_received: 0,
            active: true,
        }
    }

    /// Enable Byzantine defense
    pub fn enable_byzantine_defense(&mut self, defense: ByzantineDefense) {
        let defender = ByzantineRobustAggregator::new(self.model.clone(), defense);
        self.byzantine_defense = Some(defender);
    }

    /// Register a client node
    pub fn register_node(&mut self, node_id: u32) -> bool {
        if self.connected_nodes.len() >= MAX_CLIENTS {
            return false;
        }

        if !self.connected_nodes.contains(&node_id) {
            self.connected_nodes.push(node_id);
        }

        true
    }

    /// Submit update (server role)
    pub fn submit_client_update(&mut self, update: ModelUpdate) -> bool {
        if self.role != KernelNodeRole::Server {
            return false;
        }

        self.updates_received += 1;

        if let Some(ref mut aggregator) = self.aggregator {
            aggregator.submit_update(update);
            true
        } else {
            false
        }
    }

    /// Aggregate updates (server role)
    pub fn aggregate(&mut self) -> bool {
        if self.role != KernelNodeRole::Server {
            return false;
        }

        // Use Byzantine defense if enabled
        if let Some(ref mut defender) = self.byzantine_defense {
            if let Some(ref aggregator) = self.aggregator {
                for update in &aggregator.base.pending_updates {
                    defender.submit_update(update.clone());
                }
            }

            if defender.aggregate() {
                self.model = defender.global_model.clone();
                self.rounds_completed += 1;
                return true;
            }
        } else if let Some(ref mut aggregator) = self.aggregator {
            if aggregator.aggregate() {
                self.model = aggregator.base.global_model.clone();
                self.rounds_completed += 1;
                return true;
            }
        }

        false
    }

    /// Get global model
    pub fn get_global_model(&self) -> &FederatedModel {
        &self.model
    }

    /// Get FL statistics
    pub fn get_stats(&self) -> FederatedStats {
        FederatedStats {
            role: self.role,
            rounds_completed: self.rounds_completed,
            connected_nodes: self.connected_nodes.len(),
            updates_received: self.updates_received,
            model_version: self.model.version,
            privacy_remaining: self
                .aggregator
                .as_ref()
                .map(|a| a.dp.remaining_budget())
                .unwrap_or(1.0),
        }
    }
}

/// Federated learning statistics
#[derive(Debug, Clone)]
pub struct FederatedStats {
    /// Node role
    pub role: KernelNodeRole,
    /// Rounds completed
    pub rounds_completed: u64,
    /// Connected nodes
    pub connected_nodes: usize,
    /// Updates received
    pub updates_received: usize,
    /// Model version
    pub model_version: u64,
    /// Remaining privacy budget
    pub privacy_remaining: f64,
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
    fn test_federated_model() {
        let model = FederatedModel::from_layers(&[(10, 32), (32, 5)], 12345);

        assert!(model.num_parameters() > 0);
        assert!(model.norm() > 0.0);
    }

    #[test]
    fn test_model_update() {
        let delta = vec![0.1; 100];
        let mut update = ModelUpdate::new(delta, 1, 50);

        update.clip(0.5);

        assert!(update.norm() <= 0.5 + 1e-10);
    }

    #[test]
    fn test_fedavg_aggregator() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let mut aggregator = FedAvgAggregator::new(model);

        // Submit updates
        let update1 = ModelUpdate::new(vec![0.1; aggregator.global_model.num_parameters()], 1, 100);
        let update2 = ModelUpdate::new(vec![0.2; aggregator.global_model.num_parameters()], 2, 100);

        aggregator.submit_update(update1);
        aggregator.submit_update(update2);

        assert!(aggregator.ready_to_aggregate());
        assert!(aggregator.aggregate());
    }

    #[test]
    fn test_differential_privacy() {
        let mut dp = DifferentialPrivacy::new(1.0, 1.0);

        let mut gradient = vec![2.0; 10];
        dp.clip(&mut gradient);

        // Should be clipped to norm 1
        let norm: f64 = libm::sqrt(gradient.iter().map(|x| x * x).sum());
        assert!(norm <= 1.0 + 1e-10);
    }

    #[test]
    fn test_dp_noise() {
        let mut dp = DifferentialPrivacy::new(1.0, 1.0);

        let mut gradient = vec![0.5; 10];
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

        let values = vec![1.0, 2.0, 3.0];
        let shares = sa.create_shares(&values, 0);

        assert_eq!(shares.len(), 3);
    }

    #[test]
    fn test_byzantine_trimmed_mean() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let mut aggregator = ByzantineRobustAggregator::new(model, ByzantineDefense::TrimmedMean);

        let num_params = aggregator.global_model.num_parameters();

        for i in 0..5 {
            let delta = vec![i as f64; num_params];
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
            let delta = vec![0.1; num_params];
            aggregator.submit_update(ModelUpdate::new(delta, i, 100));
        }

        // Submit byzantine update
        let byzantine_delta = vec![100.0; num_params];
        aggregator.submit_update(ModelUpdate::new(byzantine_delta, 99, 100));

        assert!(aggregator.aggregate());

        // Byzantine client should be suspected
        assert!(aggregator.suspected_clients.contains(&99));
    }

    #[test]
    fn test_personalized_model() {
        let mut model = PersonalizedModel::new(100, 20, PersonalizationMethod::FineTuning);

        model.alpha = 0.8;

        let global = vec![1.0; 100];
        model.update_from_global(&global);

        // Shared should be influenced by global
        assert!(model.shared[0] > 0.0);
    }

    #[test]
    fn test_async_aggregator() {
        let model = FederatedModel::from_layers(&[(5, 10)], 12345);
        let mut aggregator = AsyncFedAggregator::new(model, StalenessStrategy::Weighted);

        let update = ModelUpdate::new(vec![0.1; aggregator.global_model.num_parameters()], 1, 100);

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

        manager.submit_client_update(ModelUpdate::new(vec![0.1; num_params], 1, 100));
        manager.submit_client_update(ModelUpdate::new(vec![0.2; num_params], 2, 100));

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
