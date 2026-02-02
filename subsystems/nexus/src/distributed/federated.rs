//! # Federated Learning
//!
//! Year 3 EVOLUTION - Q4 - Federated learning for distributed model training

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{Epoch, NodeId};
use crate::math::F64Ext;

// ============================================================================
// FEDERATED TYPES
// ============================================================================

/// Model ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelId(pub u64);

/// Round ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoundId(pub u64);

/// Aggregation ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AggregationId(pub u64);

static MODEL_COUNTER: AtomicU64 = AtomicU64::new(1);
static ROUND_COUNTER: AtomicU64 = AtomicU64::new(1);
static AGGREGATION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ModelId {
    pub fn generate() -> Self {
        Self(MODEL_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl RoundId {
    pub fn generate() -> Self {
        Self(ROUND_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl AggregationId {
    pub fn generate() -> Self {
        Self(AGGREGATION_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

// ============================================================================
// MODEL DEFINITION
// ============================================================================

/// Federated model
#[derive(Debug, Clone)]
pub struct FederatedModel {
    /// Model ID
    pub id: ModelId,
    /// Name
    pub name: String,
    /// Architecture
    pub architecture: ModelArchitecture,
    /// Current weights
    pub weights: ModelWeights,
    /// Version
    pub version: u64,
    /// Training config
    pub training_config: TrainingConfig,
    /// Metrics
    pub metrics: ModelMetrics,
}

/// Model architecture
#[derive(Debug, Clone)]
pub struct ModelArchitecture {
    /// Layers
    pub layers: Vec<Layer>,
    /// Input shape
    pub input_shape: Vec<usize>,
    /// Output shape
    pub output_shape: Vec<usize>,
    /// Total parameters
    pub total_params: usize,
}

/// Layer
#[derive(Debug, Clone)]
pub struct Layer {
    /// Layer type
    pub layer_type: LayerType,
    /// Input size
    pub input_size: usize,
    /// Output size
    pub output_size: usize,
    /// Activation
    pub activation: Activation,
}

/// Layer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    /// Dense/Fully connected
    Dense,
    /// Convolutional
    Conv2D,
    /// Recurrent
    LSTM,
    /// Attention
    Attention,
    /// Embedding
    Embedding,
    /// Normalization
    BatchNorm,
    /// Dropout
    Dropout,
}

/// Activation function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// No activation
    None,
    /// ReLU
    ReLU,
    /// Sigmoid
    Sigmoid,
    /// Tanh
    Tanh,
    /// Softmax
    Softmax,
    /// GELU
    GELU,
}

/// Model weights
#[derive(Debug, Clone)]
pub struct ModelWeights {
    /// Layer weights
    pub layers: Vec<LayerWeights>,
    /// Hash
    pub hash: u64,
}

/// Layer weights
#[derive(Debug, Clone)]
pub struct LayerWeights {
    /// Layer index
    pub index: usize,
    /// Weights
    pub weights: Vec<f64>,
    /// Biases
    pub biases: Vec<f64>,
}

/// Training configuration
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Learning rate
    pub learning_rate: f64,
    /// Batch size
    pub batch_size: usize,
    /// Epochs per round
    pub epochs_per_round: usize,
    /// Optimizer
    pub optimizer: Optimizer,
    /// Loss function
    pub loss_function: LossFunction,
    /// Regularization
    pub regularization: Option<Regularization>,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            batch_size: 32,
            epochs_per_round: 1,
            optimizer: Optimizer::Adam {
                beta1: 0.9,
                beta2: 0.999,
                epsilon: 1e-8,
            },
            loss_function: LossFunction::CrossEntropy,
            regularization: None,
        }
    }
}

/// Optimizer
#[derive(Debug, Clone)]
pub enum Optimizer {
    /// SGD
    SGD { momentum: f64 },
    /// Adam
    Adam {
        beta1: f64,
        beta2: f64,
        epsilon: f64,
    },
    /// AdaGrad
    AdaGrad { epsilon: f64 },
    /// RMSprop
    RMSprop { rho: f64, epsilon: f64 },
}

/// Loss function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossFunction {
    /// MSE
    MSE,
    /// MAE
    MAE,
    /// Cross entropy
    CrossEntropy,
    /// Binary cross entropy
    BinaryCrossEntropy,
    /// Huber
    Huber,
}

/// Regularization
#[derive(Debug, Clone)]
pub struct Regularization {
    /// L1 coefficient
    pub l1: f64,
    /// L2 coefficient
    pub l2: f64,
}

/// Model metrics
#[derive(Debug, Clone, Default)]
pub struct ModelMetrics {
    /// Loss
    pub loss: f64,
    /// Accuracy
    pub accuracy: f64,
    /// Rounds completed
    pub rounds_completed: u64,
    /// Total samples trained
    pub total_samples: u64,
    /// Participants
    pub participants: u64,
}

// ============================================================================
// FEDERATED ROUND
// ============================================================================

/// Training round
#[derive(Debug, Clone)]
pub struct TrainingRound {
    /// Round ID
    pub id: RoundId,
    /// Model ID
    pub model_id: ModelId,
    /// Epoch
    pub epoch: Epoch,
    /// State
    pub state: RoundState,
    /// Participants
    pub participants: Vec<ParticipantInfo>,
    /// Updates received
    pub updates: Vec<GradientUpdate>,
    /// Aggregated result
    pub aggregated: Option<AggregatedUpdate>,
    /// Start time
    pub start_time: u64,
    /// End time
    pub end_time: Option<u64>,
}

/// Round state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundState {
    /// Initializing
    Initializing,
    /// Selecting participants
    Selecting,
    /// Training in progress
    Training,
    /// Collecting updates
    Collecting,
    /// Aggregating
    Aggregating,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

/// Participant info
#[derive(Debug, Clone)]
pub struct ParticipantInfo {
    /// Node ID
    pub node_id: NodeId,
    /// Sample count
    pub sample_count: u64,
    /// Computation capacity
    pub capacity: f64,
    /// Reliability score
    pub reliability: f64,
    /// Status
    pub status: ParticipantStatus,
}

/// Participant status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantStatus {
    /// Selected
    Selected,
    /// Training
    Training,
    /// Submitted
    Submitted,
    /// Failed
    Failed,
    /// Dropped
    Dropped,
}

/// Gradient update
#[derive(Debug, Clone)]
pub struct GradientUpdate {
    /// Source node
    pub source: NodeId,
    /// Model ID
    pub model_id: ModelId,
    /// Round ID
    pub round_id: RoundId,
    /// Layer gradients
    pub gradients: Vec<LayerGradient>,
    /// Sample count
    pub sample_count: u64,
    /// Local loss
    pub local_loss: f64,
    /// Timestamp
    pub timestamp: u64,
}

/// Layer gradient
#[derive(Debug, Clone)]
pub struct LayerGradient {
    /// Layer index
    pub index: usize,
    /// Weight gradients
    pub weight_gradients: Vec<f64>,
    /// Bias gradients
    pub bias_gradients: Vec<f64>,
}

/// Aggregated update
#[derive(Debug, Clone)]
pub struct AggregatedUpdate {
    /// Aggregation ID
    pub id: AggregationId,
    /// Round ID
    pub round_id: RoundId,
    /// Strategy used
    pub strategy: AggregationStrategy,
    /// Aggregated weights
    pub weights: ModelWeights,
    /// Participants included
    pub participants_included: usize,
    /// Total samples
    pub total_samples: u64,
    /// Weighted loss
    pub weighted_loss: f64,
}

// ============================================================================
// AGGREGATION STRATEGIES
// ============================================================================

/// Aggregation strategy
#[derive(Debug, Clone)]
pub enum AggregationStrategy {
    /// Federated averaging
    FedAvg,
    /// Weighted average by sample count
    WeightedAvg,
    /// Federated proximal
    FedProx { mu: f64 },
    /// Secure aggregation
    SecureAgg,
    /// Byzantine-resilient
    ByzantineResilient { fraction: f64 },
    /// Krum
    Krum { num_byzantine: usize },
    /// Median
    Median,
    /// Trimmed mean
    TrimmedMean { trim_ratio: f64 },
}

/// Aggregator trait
pub trait Aggregator: Send + Sync {
    /// Aggregate updates
    fn aggregate(
        &self,
        model: &FederatedModel,
        updates: &[GradientUpdate],
    ) -> Result<AggregatedUpdate, AggregationError>;

    /// Strategy used
    fn strategy(&self) -> AggregationStrategy;
}

/// FedAvg aggregator
pub struct FedAvgAggregator;

impl Aggregator for FedAvgAggregator {
    fn aggregate(
        &self,
        model: &FederatedModel,
        updates: &[GradientUpdate],
    ) -> Result<AggregatedUpdate, AggregationError> {
        if updates.is_empty() {
            return Err(AggregationError::NoUpdates);
        }

        let total_samples: u64 = updates.iter().map(|u| u.sample_count).sum();

        // Aggregate weights
        let mut aggregated_layers = Vec::new();

        for (layer_idx, layer) in model.architecture.layers.iter().enumerate() {
            let weight_size = layer.input_size * layer.output_size;
            let bias_size = layer.output_size;

            let mut aggregated_weights = vec![0.0f64; weight_size];
            let mut aggregated_biases = vec![0.0f64; bias_size];

            for update in updates {
                let weight = update.sample_count as f64 / total_samples as f64;

                if let Some(layer_grad) = update.gradients.get(layer_idx) {
                    for (i, g) in layer_grad.weight_gradients.iter().enumerate() {
                        if i < aggregated_weights.len() {
                            aggregated_weights[i] += g * weight;
                        }
                    }
                    for (i, g) in layer_grad.bias_gradients.iter().enumerate() {
                        if i < aggregated_biases.len() {
                            aggregated_biases[i] += g * weight;
                        }
                    }
                }
            }

            aggregated_layers.push(LayerWeights {
                index: layer_idx,
                weights: aggregated_weights,
                biases: aggregated_biases,
            });
        }

        let weighted_loss: f64 = updates
            .iter()
            .map(|u| u.local_loss * (u.sample_count as f64 / total_samples as f64))
            .sum();

        Ok(AggregatedUpdate {
            id: AggregationId::generate(),
            round_id: updates[0].round_id,
            strategy: AggregationStrategy::FedAvg,
            weights: ModelWeights {
                layers: aggregated_layers,
                hash: 0,
            },
            participants_included: updates.len(),
            total_samples,
            weighted_loss,
        })
    }

    fn strategy(&self) -> AggregationStrategy {
        AggregationStrategy::FedAvg
    }
}

/// Byzantine-resilient aggregator
pub struct ByzantineAggregator {
    fraction: f64,
}

impl ByzantineAggregator {
    pub fn new(fraction: f64) -> Self {
        Self { fraction }
    }
}

impl Aggregator for ByzantineAggregator {
    fn aggregate(
        &self,
        model: &FederatedModel,
        updates: &[GradientUpdate],
    ) -> Result<AggregatedUpdate, AggregationError> {
        if updates.is_empty() {
            return Err(AggregationError::NoUpdates);
        }

        // Filter outliers (simplified Krum-like approach)
        let num_byzantine = (updates.len() as f64 * self.fraction) as usize;
        let num_keep = updates.len().saturating_sub(num_byzantine);

        // Score updates by distance to others
        let mut scores: Vec<(usize, f64)> = Vec::new();

        for (i, update) in updates.iter().enumerate() {
            let mut distances = Vec::new();
            for (j, other) in updates.iter().enumerate() {
                if i != j {
                    let dist = Self::gradient_distance(update, other);
                    distances.push(dist);
                }
            }
            distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let score: f64 = distances.iter().take(num_keep - 1).sum();
            scores.push((i, score));
        }

        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Keep only the best updates
        let filtered: Vec<&GradientUpdate> = scores
            .iter()
            .take(num_keep)
            .map(|(i, _)| &updates[*i])
            .collect();

        // Aggregate filtered updates
        FedAvgAggregator.aggregate(
            model,
            &filtered.iter().map(|u| (*u).clone()).collect::<Vec<_>>(),
        )
    }

    fn strategy(&self) -> AggregationStrategy {
        AggregationStrategy::ByzantineResilient {
            fraction: self.fraction,
        }
    }
}

impl ByzantineAggregator {
    fn gradient_distance(a: &GradientUpdate, b: &GradientUpdate) -> f64 {
        let mut dist = 0.0;
        for (layer_a, layer_b) in a.gradients.iter().zip(b.gradients.iter()) {
            for (ga, gb) in layer_a
                .weight_gradients
                .iter()
                .zip(layer_b.weight_gradients.iter())
            {
                dist += (ga - gb).powi(2);
            }
        }
        dist.sqrt()
    }
}

/// Aggregation error
#[derive(Debug)]
pub enum AggregationError {
    /// No updates
    NoUpdates,
    /// Insufficient participants
    InsufficientParticipants,
    /// Shape mismatch
    ShapeMismatch,
    /// Byzantine attack detected
    ByzantineAttack,
}

// ============================================================================
// FEDERATED ENGINE
// ============================================================================

/// Federated learning engine
pub struct FederatedEngine {
    /// Models
    models: BTreeMap<ModelId, FederatedModel>,
    /// Current rounds
    rounds: BTreeMap<RoundId, TrainingRound>,
    /// Aggregator
    aggregator: Box<dyn Aggregator>,
    /// Configuration
    config: FederatedConfig,
    /// Running
    running: AtomicBool,
    /// Statistics
    stats: FederatedStats,
}

/// Federated configuration
#[derive(Debug, Clone)]
pub struct FederatedConfig {
    /// Minimum participants per round
    pub min_participants: usize,
    /// Maximum participants per round
    pub max_participants: usize,
    /// Round timeout (ms)
    pub round_timeout: u64,
    /// Enable differential privacy
    pub differential_privacy: bool,
    /// Privacy epsilon
    pub privacy_epsilon: f64,
    /// Privacy delta
    pub privacy_delta: f64,
    /// Enable secure aggregation
    pub secure_aggregation: bool,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            min_participants: 3,
            max_participants: 100,
            round_timeout: 60000,
            differential_privacy: false,
            privacy_epsilon: 1.0,
            privacy_delta: 1e-5,
            secure_aggregation: false,
        }
    }
}

/// Federated statistics
#[derive(Debug, Clone, Default)]
pub struct FederatedStats {
    /// Rounds completed
    pub rounds_completed: u64,
    /// Rounds failed
    pub rounds_failed: u64,
    /// Total updates aggregated
    pub updates_aggregated: u64,
    /// Total samples trained
    pub samples_trained: u64,
    /// Average round time (ms)
    pub avg_round_time: u64,
}

impl FederatedEngine {
    /// Create new federated engine
    pub fn new(config: FederatedConfig) -> Self {
        Self {
            models: BTreeMap::new(),
            rounds: BTreeMap::new(),
            aggregator: Box::new(FedAvgAggregator),
            config,
            running: AtomicBool::new(false),
            stats: FederatedStats::default(),
        }
    }

    /// Start the engine
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop the engine
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Register a model
    pub fn register_model(&mut self, model: FederatedModel) -> ModelId {
        let id = model.id;
        self.models.insert(id, model);
        id
    }

    /// Start a training round
    pub fn start_round(&mut self, model_id: ModelId) -> Result<RoundId, FederatedError> {
        if !self.models.contains_key(&model_id) {
            return Err(FederatedError::ModelNotFound(model_id));
        }

        let round_id = RoundId::generate();
        let round = TrainingRound {
            id: round_id,
            model_id,
            epoch: Epoch(0),
            state: RoundState::Initializing,
            participants: Vec::new(),
            updates: Vec::new(),
            aggregated: None,
            start_time: 0,
            end_time: None,
        };

        self.rounds.insert(round_id, round);
        Ok(round_id)
    }

    /// Submit gradient update
    pub fn submit_update(&mut self, update: GradientUpdate) -> Result<(), FederatedError> {
        let round = self
            .rounds
            .get_mut(&update.round_id)
            .ok_or(FederatedError::RoundNotFound(update.round_id))?;

        if round.state != RoundState::Training && round.state != RoundState::Collecting {
            return Err(FederatedError::InvalidRoundState);
        }

        round.updates.push(update);

        // Check if we have enough updates
        if round.updates.len() >= self.config.min_participants {
            round.state = RoundState::Aggregating;
        }

        Ok(())
    }

    /// Aggregate updates for a round
    pub fn aggregate(&mut self, round_id: RoundId) -> Result<AggregatedUpdate, FederatedError> {
        // Extract needed data before mutable operations
        let (model_id, updates) = {
            let round = self
                .rounds
                .get(&round_id)
                .ok_or(FederatedError::RoundNotFound(round_id))?;
            (round.model_id, round.updates.clone())
        };

        let model = self
            .models
            .get(&model_id)
            .ok_or(FederatedError::ModelNotFound(model_id))?;

        let aggregated = self
            .aggregator
            .aggregate(model, &updates)
            .map_err(FederatedError::Aggregation)?;

        // Update round
        if let Some(round) = self.rounds.get_mut(&round_id) {
            round.aggregated = Some(aggregated.clone());
            round.state = RoundState::Completed;
            round.end_time = Some(0);
        }

        // Update model
        if let Some(model) = self.models.get_mut(&model_id) {
            model.weights = aggregated.weights.clone();
            model.version += 1;
            model.metrics.rounds_completed += 1;
            model.metrics.total_samples += aggregated.total_samples;
            model.metrics.participants += aggregated.participants_included as u64;
            model.metrics.loss = aggregated.weighted_loss;
        }

        self.stats.rounds_completed += 1;
        self.stats.updates_aggregated += aggregated.participants_included as u64;
        self.stats.samples_trained += aggregated.total_samples;

        Ok(aggregated)
    }

    /// Set aggregation strategy
    pub fn set_aggregator(&mut self, aggregator: Box<dyn Aggregator>) {
        self.aggregator = aggregator;
    }

    /// Get model
    pub fn get_model(&self, id: ModelId) -> Option<&FederatedModel> {
        self.models.get(&id)
    }

    /// Get round
    pub fn get_round(&self, id: RoundId) -> Option<&TrainingRound> {
        self.rounds.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &FederatedStats {
        &self.stats
    }
}

impl Default for FederatedEngine {
    fn default() -> Self {
        Self::new(FederatedConfig::default())
    }
}

/// Federated error
#[derive(Debug)]
pub enum FederatedError {
    /// Model not found
    ModelNotFound(ModelId),
    /// Round not found
    RoundNotFound(RoundId),
    /// Invalid round state
    InvalidRoundState,
    /// Aggregation error
    Aggregation(AggregationError),
    /// Insufficient participants
    InsufficientParticipants,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fedavg_aggregator() {
        let model = create_test_model();
        let updates = vec![
            create_test_update(1, 100, 0.5),
            create_test_update(2, 200, 0.4),
        ];

        let result = FedAvgAggregator.aggregate(&model, &updates);
        assert!(result.is_ok());

        let agg = result.unwrap();
        assert_eq!(agg.participants_included, 2);
        assert_eq!(agg.total_samples, 300);
    }

    #[test]
    fn test_federated_engine() {
        let mut engine = FederatedEngine::new(FederatedConfig::default());

        let model = create_test_model();
        let model_id = engine.register_model(model);

        let round_id = engine.start_round(model_id).unwrap();

        // Submit updates
        for i in 0..3 {
            let update = create_test_update(i as u64, 100, 0.5);
            let _ = engine.submit_update(GradientUpdate {
                round_id,
                model_id,
                ..update
            });
        }

        let result = engine.aggregate(round_id);
        assert!(result.is_ok());
    }

    fn create_test_model() -> FederatedModel {
        FederatedModel {
            id: ModelId::generate(),
            name: String::from("test"),
            architecture: ModelArchitecture {
                layers: vec![Layer {
                    layer_type: LayerType::Dense,
                    input_size: 10,
                    output_size: 5,
                    activation: Activation::ReLU,
                }],
                input_shape: vec![10],
                output_shape: vec![5],
                total_params: 55,
            },
            weights: ModelWeights {
                layers: Vec::new(),
                hash: 0,
            },
            version: 0,
            training_config: TrainingConfig::default(),
            metrics: ModelMetrics::default(),
        }
    }

    fn create_test_update(source: u64, samples: u64, loss: f64) -> GradientUpdate {
        GradientUpdate {
            source: NodeId(source),
            model_id: ModelId(0),
            round_id: RoundId(0),
            gradients: vec![LayerGradient {
                index: 0,
                weight_gradients: vec![0.1; 50],
                bias_gradients: vec![0.01; 5],
            }],
            sample_count: samples,
            local_loss: loss,
            timestamp: 0,
        }
    }
}
