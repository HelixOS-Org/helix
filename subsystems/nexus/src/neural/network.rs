//! NEXUS Year 2: Neural Network Composition
//!
//! Network architectures and model building.
//! Pure Rust, no_std compatible.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::activation::ActivationType;
use super::layers::*;
use super::tensor::{Tensor, TensorShape};

// ============================================================================
// Network Configuration
// ============================================================================

/// Configuration for a neural network
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub name: String,
    pub input_shape: TensorShape,
    pub learning_rate: f32,
    pub momentum: f32,
    pub weight_decay: f32,
}

impl NetworkConfig {
    pub fn new(name: &str, input_shape: TensorShape) -> Self {
        Self {
            name: String::from(name),
            input_shape,
            learning_rate: 0.001,
            momentum: 0.9,
            weight_decay: 0.0001,
        }
    }

    pub fn with_learning_rate(mut self, lr: f32) -> Self {
        self.learning_rate = lr;
        self
    }

    pub fn with_momentum(mut self, momentum: f32) -> Self {
        self.momentum = momentum;
        self
    }

    pub fn with_weight_decay(mut self, decay: f32) -> Self {
        self.weight_decay = decay;
        self
    }
}

// ============================================================================
// Sequential Network
// ============================================================================

/// Sequential neural network (layers in sequence)
pub struct Sequential {
    config: NetworkConfig,
    layers: Vec<Box<dyn Layer>>,
    output_shape: TensorShape,
}

impl Sequential {
    pub fn new(config: NetworkConfig) -> Self {
        let output_shape = config.input_shape;
        Self {
            config,
            layers: Vec::new(),
            output_shape,
        }
    }

    pub fn add(&mut self, layer: Box<dyn Layer>) {
        self.output_shape = layer.output_shape(&self.output_shape);
        self.layers.push(layer);
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        let mut current = input.clone();

        for layer in &self.layers {
            current = layer.forward(&current);
        }

        current
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn input_shape(&self) -> &TensorShape {
        &self.config.input_shape
    }

    pub fn output_shape(&self) -> &TensorShape {
        &self.output_shape
    }

    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }

    pub fn num_parameters(&self) -> usize {
        self.layers.iter().map(|l| l.num_parameters()).sum()
    }

    pub fn layer_names(&self) -> Vec<&str> {
        self.layers.iter().map(|l| l.name()).collect()
    }

    /// Get layer by index
    pub fn layer(&self, index: usize) -> Option<&dyn Layer> {
        self.layers.get(index).map(|l| l.as_ref())
    }

    /// Get mutable layer by index
    pub fn layer_mut(&mut self, index: usize) -> Option<&mut (dyn Layer + 'static)> {
        self.layers.get_mut(index).map(|l| &mut **l)
    }
}

// ============================================================================
// Network Builder (Fluent API)
// ============================================================================

/// Builder for creating neural networks with fluent API
pub struct NetworkBuilder {
    config: NetworkConfig,
    layers: Vec<Box<dyn Layer>>,
    current_shape: TensorShape,
    layer_count: usize,
}

impl NetworkBuilder {
    pub fn new(name: &str, input_shape: TensorShape) -> Self {
        Self {
            config: NetworkConfig::new(name, input_shape),
            layers: Vec::new(),
            current_shape: input_shape,
            layer_count: 0,
        }
    }

    fn next_name(&mut self, prefix: &str) -> String {
        self.layer_count += 1;
        alloc::format!("{}_{}", prefix, self.layer_count)
    }

    pub fn learning_rate(mut self, lr: f32) -> Self {
        self.config.learning_rate = lr;
        self
    }

    pub fn dense(mut self, output_size: usize, activation: ActivationType) -> Self {
        let name = self.next_name("dense");
        let input_size = self.current_shape.total_elements();
        let layer = DenseLayer::new(&name, input_size, output_size, activation);
        self.current_shape = TensorShape::vector(output_size);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn layer_norm(mut self) -> Self {
        let name = self.next_name("ln");
        let size = self.current_shape.total_elements();
        let layer = LayerNorm::new(&name, size);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn batch_norm(mut self) -> Self {
        let name = self.next_name("bn");
        let size = self.current_shape.total_elements();
        let layer = BatchNorm::new(&name, size);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn dropout(mut self, rate: f32) -> Self {
        let name = self.next_name("drop");
        let layer = Dropout::new(&name, rate);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn flatten(mut self) -> Self {
        let name = self.next_name("flat");
        self.current_shape = TensorShape::vector(self.current_shape.total_elements());
        let layer = Flatten::new(&name);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn conv1d(
        mut self,
        out_channels: usize,
        kernel_size: usize,
        activation: ActivationType,
    ) -> Self {
        let name = self.next_name("conv");
        let in_channels = if self.current_shape.ndim() >= 2 {
            self.current_shape.dim(0)
        } else {
            1
        };
        let layer = Conv1D::new(&name, in_channels, out_channels, kernel_size, activation);
        self.current_shape = layer.output_shape(&self.current_shape);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn max_pool1d(mut self, kernel_size: usize) -> Self {
        let name = self.next_name("pool");
        let layer = MaxPool1D::new(&name, kernel_size);
        self.current_shape = layer.output_shape(&self.current_shape);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn global_avg_pool(mut self) -> Self {
        let name = self.next_name("gap");
        let layer = GlobalAvgPool::new(&name);
        self.current_shape = TensorShape::vector(1);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn attention(mut self) -> Self {
        let name = self.next_name("attn");
        let embed_dim = self.current_shape.total_elements();
        let layer = SelfAttention::new(&name, embed_dim, self.layer_count as u64 * 31);
        self.layers.push(Box::new(layer));
        self
    }

    pub fn build(self) -> Sequential {
        let mut net = Sequential::new(self.config);
        for layer in self.layers {
            net.add(layer);
        }
        net
    }
}

// ============================================================================
// Multi-Input Network
// ============================================================================

/// Network with multiple input branches
pub struct MultiInputNetwork {
    name: String,
    branches: BTreeMap<String, Sequential>,
    merger: MergeStrategy,
    output_layers: Vec<Box<dyn Layer>>,
}

/// Strategy for merging multiple inputs
#[derive(Debug, Clone, Copy)]
pub enum MergeStrategy {
    Concatenate,
    Add,
    Average,
    Max,
    Multiply,
}

impl MultiInputNetwork {
    pub fn new(name: &str, merger: MergeStrategy) -> Self {
        Self {
            name: String::from(name),
            branches: BTreeMap::new(),
            merger,
            output_layers: Vec::new(),
        }
    }

    pub fn add_branch(&mut self, name: &str, network: Sequential) {
        self.branches.insert(String::from(name), network);
    }

    pub fn add_output_layer(&mut self, layer: Box<dyn Layer>) {
        self.output_layers.push(layer);
    }

    pub fn forward(&self, inputs: &BTreeMap<String, Tensor>) -> Tensor {
        // Process each branch
        let mut branch_outputs: Vec<Tensor> = Vec::new();

        for (branch_name, branch) in &self.branches {
            if let Some(input) = inputs.get(branch_name) {
                let output = branch.forward(input);
                branch_outputs.push(output);
            }
        }

        if branch_outputs.is_empty() {
            return Tensor::zeros(TensorShape::vector(1));
        }

        // Merge branch outputs
        let mut merged = self.merge_outputs(&branch_outputs);

        // Apply output layers
        for layer in &self.output_layers {
            merged = layer.forward(&merged);
        }

        merged
    }

    fn merge_outputs(&self, outputs: &[Tensor]) -> Tensor {
        if outputs.is_empty() {
            return Tensor::zeros(TensorShape::vector(1));
        }

        if outputs.len() == 1 {
            return outputs[0].clone();
        }

        match self.merger {
            MergeStrategy::Concatenate => {
                // Concatenate all outputs
                let total_len: usize = outputs.iter().map(|t| t.len()).sum();
                let mut data = Vec::with_capacity(total_len);
                for output in outputs {
                    data.extend_from_slice(output.data());
                }
                Tensor::from_data(TensorShape::vector(total_len), data)
            },

            MergeStrategy::Add => {
                let mut result = outputs[0].clone();
                for output in &outputs[1..] {
                    if let Some(added) = result.add(output) {
                        result = added;
                    }
                }
                result
            },

            MergeStrategy::Average => {
                let mut result = outputs[0].clone();
                for output in &outputs[1..] {
                    if let Some(added) = result.add(output) {
                        result = added;
                    }
                }
                result.mul_scalar(1.0 / outputs.len() as f32)
            },

            MergeStrategy::Max => {
                let len = outputs[0].len();
                let mut data = outputs[0].data().to_vec();

                for output in &outputs[1..] {
                    for (i, &val) in output.data().iter().enumerate() {
                        if i < data.len() && val > data[i] {
                            data[i] = val;
                        }
                    }
                }

                Tensor::from_data(*outputs[0].shape(), data)
            },

            MergeStrategy::Multiply => {
                let mut result = outputs[0].clone();
                for output in &outputs[1..] {
                    if let Some(multiplied) = result.mul(output) {
                        result = multiplied;
                    }
                }
                result
            },
        }
    }

    pub fn num_parameters(&self) -> usize {
        let branch_params: usize = self.branches.values().map(|b| b.num_parameters()).sum();
        let output_params: usize = self.output_layers.iter().map(|l| l.num_parameters()).sum();
        branch_params + output_params
    }
}

// ============================================================================
// Residual Network Block
// ============================================================================

/// Residual network block with skip connection
pub struct ResidualBlock {
    name: String,
    layers: Vec<Box<dyn Layer>>,
    projection: Option<Box<dyn Layer>>, // For dimension matching
}

impl ResidualBlock {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            layers: Vec::new(),
            projection: None,
        }
    }

    pub fn add_layer(&mut self, layer: Box<dyn Layer>) {
        self.layers.push(layer);
    }

    pub fn with_projection(mut self, projection: Box<dyn Layer>) -> Self {
        self.projection = Some(projection);
        self
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        // Forward through layers
        let mut current = input.clone();
        for layer in &self.layers {
            current = layer.forward(&current);
        }

        // Skip connection
        let skip = if let Some(proj) = &self.projection {
            proj.forward(input)
        } else {
            input.clone()
        };

        // Add residual
        current.add(&skip).unwrap_or(current)
    }
}

// ============================================================================
// Ensemble Network
// ============================================================================

/// Ensemble of multiple networks
pub struct EnsembleNetwork {
    name: String,
    networks: Vec<Sequential>,
    ensemble_method: EnsembleMethod,
}

#[derive(Debug, Clone, Copy)]
pub enum EnsembleMethod {
    Average,
    WeightedAverage,
    Voting,
    Max,
    Stacking,
}

impl EnsembleNetwork {
    pub fn new(name: &str, method: EnsembleMethod) -> Self {
        Self {
            name: String::from(name),
            networks: Vec::new(),
            ensemble_method: method,
        }
    }

    pub fn add_network(&mut self, network: Sequential) {
        self.networks.push(network);
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        if self.networks.is_empty() {
            return Tensor::zeros(TensorShape::vector(1));
        }

        // Get predictions from all networks
        let predictions: Vec<Tensor> = self.networks.iter().map(|net| net.forward(input)).collect();

        self.combine_predictions(&predictions)
    }

    fn combine_predictions(&self, predictions: &[Tensor]) -> Tensor {
        if predictions.is_empty() {
            return Tensor::zeros(TensorShape::vector(1));
        }

        if predictions.len() == 1 {
            return predictions[0].clone();
        }

        match self.ensemble_method {
            EnsembleMethod::Average | EnsembleMethod::WeightedAverage => {
                let mut sum = predictions[0].clone();
                for pred in &predictions[1..] {
                    if let Some(added) = sum.add(pred) {
                        sum = added;
                    }
                }
                sum.mul_scalar(1.0 / predictions.len() as f32)
            },

            EnsembleMethod::Voting => {
                // Hard voting: most common argmax
                let votes: Vec<usize> = predictions.iter().map(|p| p.argmax()).collect();

                let mut counts = BTreeMap::new();
                for vote in &votes {
                    *counts.entry(*vote).or_insert(0) += 1;
                }

                let winner = counts
                    .iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(idx, _)| *idx)
                    .unwrap_or(0);

                // Create one-hot output
                let size = predictions[0].len();
                let mut data = alloc::vec![0.0f32; size];
                if winner < size {
                    data[winner] = 1.0;
                }

                Tensor::from_data(*predictions[0].shape(), data)
            },

            EnsembleMethod::Max => {
                let len = predictions[0].len();
                let mut data = predictions[0].data().to_vec();

                for pred in &predictions[1..] {
                    for (i, &val) in pred.data().iter().enumerate() {
                        if i < data.len() && val > data[i] {
                            data[i] = val;
                        }
                    }
                }

                Tensor::from_data(*predictions[0].shape(), data)
            },

            EnsembleMethod::Stacking => {
                // Simple stacking: concatenate then average
                let total_len: usize = predictions.iter().map(|p| p.len()).sum();
                let mut data = Vec::with_capacity(total_len);
                for pred in predictions {
                    data.extend_from_slice(pred.data());
                }
                Tensor::from_data(TensorShape::vector(total_len), data)
            },
        }
    }

    pub fn num_networks(&self) -> usize {
        self.networks.len()
    }

    pub fn total_parameters(&self) -> usize {
        self.networks.iter().map(|n| n.num_parameters()).sum()
    }
}

// ============================================================================
// Siamese Network
// ============================================================================

/// Siamese network for similarity learning
pub struct SiameseNetwork {
    name: String,
    shared_network: Sequential,
    distance_metric: DistanceMetric,
}

#[derive(Debug, Clone, Copy)]
pub enum DistanceMetric {
    Euclidean,
    Cosine,
    Manhattan,
    ContrastiveLoss,
}

impl SiameseNetwork {
    pub fn new(name: &str, shared_network: Sequential, metric: DistanceMetric) -> Self {
        Self {
            name: String::from(name),
            shared_network,
            distance_metric: metric,
        }
    }

    pub fn forward_one(&self, input: &Tensor) -> Tensor {
        self.shared_network.forward(input)
    }

    pub fn forward_pair(&self, input1: &Tensor, input2: &Tensor) -> (Tensor, Tensor, f32) {
        let embedding1 = self.forward_one(input1);
        let embedding2 = self.forward_one(input2);
        let distance = self.compute_distance(&embedding1, &embedding2);

        (embedding1, embedding2, distance)
    }

    fn compute_distance(&self, a: &Tensor, b: &Tensor) -> f32 {
        match self.distance_metric {
            DistanceMetric::Euclidean => {
                let diff: f32 = a
                    .data()
                    .iter()
                    .zip(b.data().iter())
                    .map(|(x, y)| (x - y) * (x - y))
                    .sum();
                libm::sqrtf(diff)
            },

            DistanceMetric::Cosine => {
                let dot: f32 = a
                    .data()
                    .iter()
                    .zip(b.data().iter())
                    .map(|(x, y)| x * y)
                    .sum();
                let norm_a = a.norm();
                let norm_b = b.norm();

                if norm_a > 1e-10 && norm_b > 1e-10 {
                    1.0 - dot / (norm_a * norm_b)
                } else {
                    1.0
                }
            },

            DistanceMetric::Manhattan => a
                .data()
                .iter()
                .zip(b.data().iter())
                .map(|(x, y)| (x - y).abs())
                .sum(),

            DistanceMetric::ContrastiveLoss => {
                // L2 distance for contrastive loss
                let diff: f32 = a
                    .data()
                    .iter()
                    .zip(b.data().iter())
                    .map(|(x, y)| (x - y) * (x - y))
                    .sum();
                libm::sqrtf(diff)
            },
        }
    }

    pub fn is_similar(&self, input1: &Tensor, input2: &Tensor, threshold: f32) -> bool {
        let (_, _, distance) = self.forward_pair(input1, input2);
        distance < threshold
    }
}

// ============================================================================
// Autoencoder
// ============================================================================

/// Autoencoder network
pub struct Autoencoder {
    name: String,
    encoder: Sequential,
    decoder: Sequential,
    latent_dim: usize,
}

impl Autoencoder {
    pub fn new(name: &str, encoder: Sequential, decoder: Sequential, latent_dim: usize) -> Self {
        Self {
            name: String::from(name),
            encoder,
            decoder,
            latent_dim,
        }
    }

    pub fn encode(&self, input: &Tensor) -> Tensor {
        self.encoder.forward(input)
    }

    pub fn decode(&self, latent: &Tensor) -> Tensor {
        self.decoder.forward(latent)
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        let latent = self.encode(input);
        self.decode(&latent)
    }

    pub fn reconstruction_loss(&self, input: &Tensor) -> f32 {
        let output = self.forward(input);

        // MSE loss
        input
            .data()
            .iter()
            .zip(output.data().iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f32>()
            / input.len() as f32
    }

    pub fn latent_dim(&self) -> usize {
        self.latent_dim
    }
}

// ============================================================================
// Kernel-Specific Networks
// ============================================================================

/// Create a network for kernel resource prediction
pub fn create_resource_predictor() -> Sequential {
    NetworkBuilder::new("resource_predictor", TensorShape::vector(16))
        .learning_rate(0.001)
        .dense(32, ActivationType::ReLU)
        .layer_norm()
        .dropout(0.1)
        .dense(16, ActivationType::ReLU)
        .dense(4, ActivationType::Sigmoid)  // CPU, Memory, IO, Network predictions
        .build()
}

/// Create a network for process scheduling decisions
pub fn create_scheduler_network() -> Sequential {
    NetworkBuilder::new("scheduler_net", TensorShape::vector(32))
        .learning_rate(0.0005)
        .dense(64, ActivationType::ReLU)
        .layer_norm()
        .dense(32, ActivationType::ReLU)
        .dropout(0.1)
        .dense(16, ActivationType::ReLU)
        .dense(8, ActivationType::Softmax)  // Priority classes
        .build()
}

/// Create a network for anomaly detection
pub fn create_anomaly_detector() -> Sequential {
    NetworkBuilder::new("anomaly_detector", TensorShape::vector(64))
        .learning_rate(0.001)
        .dense(32, ActivationType::ReLU)
        .layer_norm()
        .dense(16, ActivationType::ReLU)
        .dense(1, ActivationType::Sigmoid)  // Anomaly probability
        .build()
}

/// Create an autoencoder for kernel state compression
pub fn create_state_autoencoder(input_dim: usize, latent_dim: usize) -> Autoencoder {
    let encoder = NetworkBuilder::new("encoder", TensorShape::vector(input_dim))
        .dense(input_dim / 2, ActivationType::ReLU)
        .layer_norm()
        .dense(latent_dim, ActivationType::Tanh)
        .build();

    let decoder = NetworkBuilder::new("decoder", TensorShape::vector(latent_dim))
        .dense(input_dim / 2, ActivationType::ReLU)
        .layer_norm()
        .dense(input_dim, ActivationType::Sigmoid)
        .build();

    Autoencoder::new("state_autoencoder", encoder, decoder, latent_dim)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_network() {
        let net = NetworkBuilder::new("test", TensorShape::vector(10))
            .dense(8, ActivationType::ReLU)
            .dense(4, ActivationType::Softmax)
            .build();

        let input = Tensor::random(TensorShape::vector(10), 42);
        let output = net.forward(&input);

        assert_eq!(output.shape().dim(0), 4);

        // Softmax output should sum to ~1
        let sum: f32 = output.data().iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_network_builder() {
        let net = NetworkBuilder::new("mlp", TensorShape::vector(16))
            .dense(32, ActivationType::ReLU)
            .layer_norm()
            .dropout(0.5)
            .dense(16, ActivationType::ReLU)
            .dense(4, ActivationType::Identity)
            .build();

        assert_eq!(net.num_layers(), 5);
        assert!(net.num_parameters() > 0);
    }

    #[test]
    fn test_multi_input_network() {
        let branch1 = NetworkBuilder::new("b1", TensorShape::vector(10))
            .dense(8, ActivationType::ReLU)
            .build();

        let branch2 = NetworkBuilder::new("b2", TensorShape::vector(8))
            .dense(8, ActivationType::ReLU)
            .build();

        let mut multi = MultiInputNetwork::new("multi", MergeStrategy::Concatenate);
        multi.add_branch("branch1", branch1);
        multi.add_branch("branch2", branch2);

        let mut inputs = BTreeMap::new();
        inputs.insert(
            String::from("branch1"),
            Tensor::random(TensorShape::vector(10), 1),
        );
        inputs.insert(
            String::from("branch2"),
            Tensor::random(TensorShape::vector(8), 2),
        );

        let output = multi.forward(&inputs);
        assert_eq!(output.len(), 16); // 8 + 8 concatenated
    }

    #[test]
    fn test_ensemble_network() {
        let net1 = NetworkBuilder::new("n1", TensorShape::vector(10))
            .dense(4, ActivationType::Softmax)
            .build();

        let net2 = NetworkBuilder::new("n2", TensorShape::vector(10))
            .dense(4, ActivationType::Softmax)
            .build();

        let mut ensemble = EnsembleNetwork::new("ens", EnsembleMethod::Average);
        ensemble.add_network(net1);
        ensemble.add_network(net2);

        let input = Tensor::random(TensorShape::vector(10), 42);
        let output = ensemble.forward(&input);

        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_siamese_network() {
        let shared = NetworkBuilder::new("shared", TensorShape::vector(10))
            .dense(8, ActivationType::ReLU)
            .dense(4, ActivationType::Identity)
            .build();

        let siamese = SiameseNetwork::new("siamese", shared, DistanceMetric::Euclidean);

        let input1 = Tensor::random(TensorShape::vector(10), 1);
        let input2 = Tensor::random(TensorShape::vector(10), 2);

        let (emb1, emb2, dist) = siamese.forward_pair(&input1, &input2);

        assert_eq!(emb1.len(), 4);
        assert_eq!(emb2.len(), 4);
        assert!(dist >= 0.0);
    }

    #[test]
    fn test_autoencoder() {
        let ae = create_state_autoencoder(32, 8);

        let input = Tensor::random(TensorShape::vector(32), 42);
        let latent = ae.encode(&input);
        let reconstructed = ae.decode(&latent);

        assert_eq!(latent.len(), 8);
        assert_eq!(reconstructed.len(), 32);
    }

    #[test]
    fn test_kernel_networks() {
        let _predictor = create_resource_predictor();
        let _scheduler = create_scheduler_network();
        let _detector = create_anomaly_detector();
    }
}
