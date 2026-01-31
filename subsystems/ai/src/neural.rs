//! # Neural Inference Engine
//!
//! Lightweight neural network inference engine for kernel-space AI operations.
//! Designed for no_std environments with optional GPU/NPU acceleration.
//!
//! ## Features
//!
//! - **Tensor Operations**: Basic tensor math for inference
//! - **Pattern Matching**: Neural pattern recognition
//! - **Decision Trees**: Fast decision making structures
//! - **Model Execution**: Run pre-trained models
//! - **Hardware Acceleration**: Optional GPU/NPU offloading
//!
//! ## Architecture
//!
//! ```text
//!   Input Tensor ────►┌─────────────────────────────────────┐
//!                     │         Neural Engine               │
//!                     │                                     │
//!                     │  ┌─────────────────────────────┐    │
//!                     │  │     Model Registry          │    │
//!                     │  │  - Pattern Matchers         │    │
//!                     │  │  - Decision Trees           │    │
//!                     │  │  - Neural Networks          │    │
//!                     │  └──────────────┬──────────────┘    │
//!                     │                 │                    │
//!                     │                 ▼                    │
//!                     │  ┌─────────────────────────────┐    │
//!                     │  │    Execution Backend        │    │
//!                     │  │  ┌─────┐ ┌─────┐ ┌─────┐   │    │
//!                     │  │  │ CPU │ │ GPU │ │ NPU │   │    │
//!                     │  │  └─────┘ └─────┘ └─────┘   │    │
//!                     │  └──────────────┬──────────────┘    │
//!                     │                 │                    │
//!                     │                 ▼                    │
//!                     │  ┌─────────────────────────────┐    │
//!                     │  │    Output Processor         │───────► Inference Result
//!                     │  └─────────────────────────────┘    │
//!                     │                                     │
//!                     └─────────────────────────────────────┘
//! ```

use crate::core::{AiAction, AiEvent, Confidence, DecisionContext};

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

// =============================================================================
// Tensor Types
// =============================================================================

/// Shape of a tensor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorShape {
    pub dims: Vec<usize>,
}

impl TensorShape {
    /// Create a new shape
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    /// Scalar (0-dimensional)
    pub fn scalar() -> Self {
        Self { dims: Vec::new() }
    }

    /// 1D vector
    pub fn vector(len: usize) -> Self {
        Self { dims: vec![len] }
    }

    /// 2D matrix
    pub fn matrix(rows: usize, cols: usize) -> Self {
        Self { dims: vec![rows, cols] }
    }

    /// Total number of elements
    pub fn size(&self) -> usize {
        if self.dims.is_empty() {
            1
        } else {
            self.dims.iter().product()
        }
    }

    /// Number of dimensions
    pub fn ndim(&self) -> usize {
        self.dims.len()
    }
}

/// Data type for tensor elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorDtype {
    F32,
    F16,
    I32,
    I8,
    U8,
    Bool,
}

impl TensorDtype {
    /// Size in bytes of one element
    pub fn element_size(&self) -> usize {
        match self {
            Self::F32 | Self::I32 => 4,
            Self::F16 => 2,
            Self::I8 | Self::U8 | Self::Bool => 1,
        }
    }
}

/// A tensor (n-dimensional array)
#[derive(Debug, Clone)]
pub struct Tensor {
    /// Shape of the tensor
    pub shape: TensorShape,
    /// Data type
    pub dtype: TensorDtype,
    /// Raw data (stored as f32 for simplicity)
    data: Vec<f32>,
}

impl Tensor {
    /// Create a new tensor filled with zeros
    pub fn zeros(shape: TensorShape, dtype: TensorDtype) -> Self {
        let size = shape.size();
        Self {
            shape,
            dtype,
            data: vec![0.0; size],
        }
    }

    /// Create a new tensor filled with ones
    pub fn ones(shape: TensorShape, dtype: TensorDtype) -> Self {
        let size = shape.size();
        Self {
            shape,
            dtype,
            data: vec![1.0; size],
        }
    }

    /// Create a tensor from data
    pub fn from_vec(data: Vec<f32>, shape: TensorShape) -> Self {
        assert_eq!(data.len(), shape.size());
        Self {
            shape,
            dtype: TensorDtype::F32,
            data,
        }
    }

    /// Get raw data
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Get mutable raw data
    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Element-wise addition
    pub fn add(&self, other: &Self) -> Option<Self> {
        if self.shape != other.shape {
            return None;
        }

        let data: Vec<f32> = self.data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a + b)
            .collect();

        Some(Self {
            shape: self.shape.clone(),
            dtype: self.dtype,
            data,
        })
    }

    /// Element-wise multiplication
    pub fn mul(&self, other: &Self) -> Option<Self> {
        if self.shape != other.shape {
            return None;
        }

        let data: Vec<f32> = self.data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a * b)
            .collect();

        Some(Self {
            shape: self.shape.clone(),
            dtype: self.dtype,
            data,
        })
    }

    /// Scale by a constant
    pub fn scale(&self, factor: f32) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| x * factor).collect();
        Self {
            shape: self.shape.clone(),
            dtype: self.dtype,
            data,
        }
    }

    /// Apply ReLU activation
    pub fn relu(&self) -> Self {
        let data: Vec<f32> = self.data
            .iter()
            .map(|x| if *x > 0.0 { *x } else { 0.0 })
            .collect();
        Self {
            shape: self.shape.clone(),
            dtype: self.dtype,
            data,
        }
    }

    /// Apply sigmoid activation
    pub fn sigmoid(&self) -> Self {
        let data: Vec<f32> = self.data
            .iter()
            .map(|x| 1.0 / (1.0 + crate::math::exp_f32(-x)))
            .collect();
        Self {
            shape: self.shape.clone(),
            dtype: self.dtype,
            data,
        }
    }

    /// Apply softmax (1D only)
    pub fn softmax(&self) -> Self {
        let max = self.data.iter().fold(f32::NEG_INFINITY, |a, b| a.max(*b));
        let exp_sum: f32 = self.data.iter().map(|x| crate::math::exp_f32(x - max)).sum();

        let data: Vec<f32> = self.data
            .iter()
            .map(|x| crate::math::exp_f32(x - max) / exp_sum)
            .collect();

        Self {
            shape: self.shape.clone(),
            dtype: self.dtype,
            data,
        }
    }

    /// Matrix multiplication (2D only)
    pub fn matmul(&self, other: &Self) -> Option<Self> {
        if self.shape.ndim() != 2 || other.shape.ndim() != 2 {
            return None;
        }

        let m = self.shape.dims[0];
        let k1 = self.shape.dims[1];
        let k2 = other.shape.dims[0];
        let n = other.shape.dims[1];

        if k1 != k2 {
            return None;
        }

        let mut result = vec![0.0f32; m * n];

        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0f32;
                for k in 0..k1 {
                    sum += self.data[i * k1 + k] * other.data[k * n + j];
                }
                result[i * n + j] = sum;
            }
        }

        Some(Self {
            shape: TensorShape::matrix(m, n),
            dtype: TensorDtype::F32,
            data: result,
        })
    }

    /// Sum all elements
    pub fn sum(&self) -> f32 {
        self.data.iter().sum()
    }

    /// Mean of all elements
    pub fn mean(&self) -> f32 {
        self.sum() / self.shape.size() as f32
    }

    /// Get argmax index
    pub fn argmax(&self) -> usize {
        self.data
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

// =============================================================================
// Neural Network Layers
// =============================================================================

/// A layer in a neural network
pub trait Layer {
    fn forward(&self, input: &Tensor) -> Tensor;
    fn name(&self) -> &'static str;
}

/// Dense (fully connected) layer
pub struct DenseLayer {
    weights: Tensor,
    bias: Tensor,
    activation: Activation,
}

impl DenseLayer {
    pub fn new(weights: Tensor, bias: Tensor, activation: Activation) -> Self {
        Self { weights, bias, activation }
    }

    /// Create with random initialization (for testing)
    pub fn random(input_size: usize, output_size: usize, activation: Activation) -> Self {
        // Simple initialization (in practice would use proper random init)
        let weights = Tensor::ones(
            TensorShape::matrix(input_size, output_size),
            TensorDtype::F32,
        ).scale(0.01);

        let bias = Tensor::zeros(TensorShape::vector(output_size), TensorDtype::F32);

        Self { weights, bias, activation }
    }
}

impl Layer for DenseLayer {
    fn forward(&self, input: &Tensor) -> Tensor {
        // input: [batch, input_size] or [input_size]
        // weights: [input_size, output_size]
        // output: [batch, output_size] or [output_size]

        let reshaped = if input.shape.ndim() == 1 {
            Tensor::from_vec(
                input.data().to_vec(),
                TensorShape::matrix(1, input.shape.dims[0]),
            )
        } else {
            input.clone()
        };

        let linear = reshaped.matmul(&self.weights).unwrap();

        // Add bias (broadcast)
        let mut output = linear;
        for row in 0..output.shape.dims[0] {
            for col in 0..output.shape.dims[1] {
                let idx = row * output.shape.dims[1] + col;
                output.data_mut()[idx] += self.bias.data()[col];
            }
        }

        // Apply activation
        match self.activation {
            Activation::None => output,
            Activation::ReLU => output.relu(),
            Activation::Sigmoid => output.sigmoid(),
            Activation::Softmax => output.softmax(),
        }
    }

    fn name(&self) -> &'static str {
        "Dense"
    }
}

/// Activation functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    None,
    ReLU,
    Sigmoid,
    Softmax,
}

// =============================================================================
// Neural Network Model
// =============================================================================

/// A neural network model
pub struct NeuralModel {
    /// Model identifier
    pub id: u64,
    /// Model name
    pub name: String,
    /// Layers in order
    layers: Vec<Box<dyn Layer + Send + Sync>>,
    /// Input shape
    pub input_shape: TensorShape,
    /// Output shape
    pub output_shape: TensorShape,
}

impl NeuralModel {
    /// Create a new empty model
    pub fn new(id: u64, name: String, input_shape: TensorShape, output_shape: TensorShape) -> Self {
        Self {
            id,
            name,
            layers: Vec::new(),
            input_shape,
            output_shape,
        }
    }

    /// Add a layer
    pub fn add_layer(&mut self, layer: Box<dyn Layer + Send + Sync>) {
        self.layers.push(layer);
    }

    /// Forward pass through the network
    pub fn forward(&self, input: &Tensor) -> Tensor {
        let mut current = input.clone();
        for layer in &self.layers {
            current = layer.forward(&current);
        }
        current
    }

    /// Number of layers
    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }
}

// =============================================================================
// Decision Trees
// =============================================================================

/// A decision tree for fast classification
pub struct DecisionTree {
    /// Tree identifier
    pub id: u64,
    /// Root node
    root: DecisionNode,
    /// Feature names
    pub feature_names: Vec<String>,
    /// Class labels
    pub class_labels: Vec<String>,
}

/// Node in a decision tree
pub enum DecisionNode {
    /// Internal node (decision)
    Split {
        feature_index: usize,
        threshold: f32,
        left: Box<DecisionNode>,
        right: Box<DecisionNode>,
    },
    /// Leaf node (prediction)
    Leaf {
        class_index: usize,
        confidence: f32,
    },
}

impl DecisionTree {
    /// Create a new decision tree
    pub fn new(
        id: u64,
        root: DecisionNode,
        feature_names: Vec<String>,
        class_labels: Vec<String>,
    ) -> Self {
        Self {
            id,
            root,
            feature_names,
            class_labels,
        }
    }

    /// Predict class for input features
    pub fn predict(&self, features: &[f32]) -> (usize, f32) {
        self.traverse(&self.root, features)
    }

    fn traverse(&self, node: &DecisionNode, features: &[f32]) -> (usize, f32) {
        match node {
            DecisionNode::Split { feature_index, threshold, left, right } => {
                if *feature_index < features.len() && features[*feature_index] <= *threshold {
                    self.traverse(left, features)
                } else {
                    self.traverse(right, features)
                }
            }
            DecisionNode::Leaf { class_index, confidence } => (*class_index, *confidence),
        }
    }

    /// Get class label for index
    pub fn class_label(&self, index: usize) -> Option<&str> {
        self.class_labels.get(index).map(|s| s.as_str())
    }
}

// =============================================================================
// Pattern Matcher
// =============================================================================

/// Pattern matcher using feature vectors
pub struct PatternMatcher {
    /// Matcher identifier
    pub id: u64,
    /// Stored patterns
    patterns: Vec<StoredPattern>,
    /// Similarity threshold
    pub threshold: f32,
}

/// A stored pattern
struct StoredPattern {
    id: u64,
    features: Vec<f32>,
    label: String,
    action: Option<AiAction>,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new(id: u64, threshold: f32) -> Self {
        Self {
            id,
            patterns: Vec::new(),
            threshold,
        }
    }

    /// Add a pattern
    pub fn add_pattern(&mut self, features: Vec<f32>, label: String, action: Option<AiAction>) {
        static PATTERN_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = PATTERN_COUNTER.fetch_add(1, Ordering::Relaxed);

        self.patterns.push(StoredPattern {
            id,
            features,
            label,
            action,
        });
    }

    /// Find matching patterns
    pub fn find_matches(&self, query: &[f32]) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            let similarity = self.cosine_similarity(query, &pattern.features);
            if similarity >= self.threshold {
                matches.push(PatternMatch {
                    pattern_id: pattern.id,
                    label: pattern.label.clone(),
                    similarity,
                    action: pattern.action.clone(),
                });
            }
        }

        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        matches
    }

    /// Cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = crate::math::sqrt_f32(a.iter().map(|x| x * x).sum::<f32>());
        let norm_b: f32 = crate::math::sqrt_f32(b.iter().map(|x| x * x).sum::<f32>());

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    /// Number of stored patterns
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

/// A pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern_id: u64,
    pub label: String,
    pub similarity: f32,
    pub action: Option<AiAction>,
}

// =============================================================================
// Inference Result
// =============================================================================

/// Result of neural inference
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Output tensor
    pub output: Vec<f32>,
    /// Predicted class index (for classification)
    pub class_index: Option<usize>,
    /// Class label (if available)
    pub class_label: Option<String>,
    /// Confidence score
    pub confidence: Confidence,
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Backend used
    pub backend: ExecutionBackend,
}

/// Execution backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionBackend {
    Cpu,
    Gpu,
    Npu,
}

// =============================================================================
// Neural Engine
// =============================================================================

/// The Neural Inference Engine
pub struct NeuralEngine {
    /// Enable GPU acceleration
    gpu_enabled: bool,

    /// Enable NPU acceleration
    npu_enabled: bool,

    /// Registered models
    models: RwLock<Vec<NeuralModel>>,

    /// Decision trees
    decision_trees: RwLock<Vec<DecisionTree>>,

    /// Pattern matchers
    pattern_matchers: RwLock<Vec<PatternMatcher>>,

    /// Statistics
    stats: NeuralStats,
}

struct NeuralStats {
    inferences_run: AtomicU64,
    patterns_matched: AtomicU64,
    total_inference_time_us: AtomicU64,
    gpu_inferences: AtomicU64,
    npu_inferences: AtomicU64,
}

impl Default for NeuralStats {
    fn default() -> Self {
        Self {
            inferences_run: AtomicU64::new(0),
            patterns_matched: AtomicU64::new(0),
            total_inference_time_us: AtomicU64::new(0),
            gpu_inferences: AtomicU64::new(0),
            npu_inferences: AtomicU64::new(0),
        }
    }
}

impl NeuralEngine {
    /// Create a new Neural Engine
    pub fn new(gpu_enabled: bool, npu_enabled: bool) -> Self {
        Self {
            gpu_enabled,
            npu_enabled,
            models: RwLock::new(Vec::new()),
            decision_trees: RwLock::new(Vec::new()),
            pattern_matchers: RwLock::new(Vec::new()),
            stats: NeuralStats::default(),
        }
    }

    /// Register a neural model
    pub fn register_model(&self, model: NeuralModel) {
        self.models.write().push(model);
    }

    /// Register a decision tree
    pub fn register_tree(&self, tree: DecisionTree) {
        self.decision_trees.write().push(tree);
    }

    /// Register a pattern matcher
    pub fn register_matcher(&self, matcher: PatternMatcher) {
        self.pattern_matchers.write().push(matcher);
    }

    /// Run inference on a model
    pub fn infer(&self, model_id: u64, input: &Tensor) -> Option<InferenceResult> {
        let start = self.get_timestamp();

        let models = self.models.read();
        let model = models.iter().find(|m| m.id == model_id)?;

        // Run forward pass
        let output = model.forward(input);

        let elapsed = self.get_timestamp() - start;
        self.stats.inferences_run.fetch_add(1, Ordering::Relaxed);
        self.stats.total_inference_time_us.fetch_add(elapsed, Ordering::Relaxed);

        // Determine class (for classification tasks)
        let (class_index, confidence) = if output.shape.ndim() == 1 ||
            (output.shape.ndim() == 2 && output.shape.dims[0] == 1)
        {
            let argmax = output.argmax();
            let max_val = output.data()[argmax];
            (Some(argmax), Confidence::new(max_val.clamp(0.0, 1.0)))
        } else {
            (None, Confidence::new(0.5))
        };

        Some(InferenceResult {
            output: output.data().to_vec(),
            class_index,
            class_label: None,
            confidence,
            execution_time_us: elapsed,
            backend: ExecutionBackend::Cpu,
        })
    }

    /// Run decision tree prediction
    pub fn predict_tree(&self, tree_id: u64, features: &[f32]) -> Option<InferenceResult> {
        let trees = self.decision_trees.read();
        let tree = trees.iter().find(|t| t.id == tree_id)?;

        let (class_index, confidence) = tree.predict(features);
        let class_label = tree.class_label(class_index).map(|s| s.to_string());

        Some(InferenceResult {
            output: vec![class_index as f32],
            class_index: Some(class_index),
            class_label,
            confidence: Confidence::new(confidence),
            execution_time_us: 0,
            backend: ExecutionBackend::Cpu,
        })
    }

    /// Match patterns
    pub fn match_patterns(&self, matcher_id: u64, query: &[f32]) -> Vec<PatternMatch> {
        let matchers = self.pattern_matchers.read();
        if let Some(matcher) = matchers.iter().find(|m| m.id == matcher_id) {
            let matches = matcher.find_matches(query);
            self.stats.patterns_matched.fetch_add(matches.len() as u64, Ordering::Relaxed);
            matches
        } else {
            Vec::new()
        }
    }

    /// Analyze event and match patterns
    pub fn match_pattern(
        &self,
        event: &AiEvent,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        // Extract features from event
        let features = self.extract_features(event, context);

        // Try all pattern matchers
        let matchers = self.pattern_matchers.read();
        for matcher in matchers.iter() {
            let matches = matcher.find_matches(&features);
            if let Some(best) = matches.first() {
                if best.similarity >= 0.8 {
                    if let Some(action) = &best.action {
                        return Ok(Some((
                            action.clone(),
                            Confidence::new(best.similarity),
                            format!("Pattern match: {} ({})", best.label, best.pattern_id),
                        )));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extract features from event for pattern matching
    fn extract_features(&self, event: &AiEvent, context: &DecisionContext) -> Vec<f32> {
        let mut features = Vec::with_capacity(16);

        // System metrics as features
        features.push(context.system_metrics.cpu_usage_percent as f32 / 100.0);
        features.push(context.system_metrics.memory_usage_percent as f32 / 100.0);
        features.push(context.system_metrics.io_wait_percent as f32 / 100.0);
        features.push(crate::math::ln_1p_f32(context.system_metrics.process_count as f32) / 10.0);
        features.push(crate::math::ln_1p_f32(context.system_metrics.thread_count as f32) / 10.0);

        // Event type encoding
        let event_type = match event {
            AiEvent::SystemBoot => 0.0,
            AiEvent::CpuThreshold { .. } => 0.1,
            AiEvent::MemoryPressure { .. } => 0.2,
            AiEvent::IoBottleneck { .. } => 0.3,
            AiEvent::ProcessSpawn { .. } => 0.4,
            AiEvent::ProcessExit { .. } => 0.5,
            AiEvent::AnomalyDetected { .. } => 0.6,
            AiEvent::ThreatSignature { .. } => 0.7,
            AiEvent::ModuleError { .. } => 0.8,
            AiEvent::HardwareError { .. } => 0.9,
            _ => 0.5,
        };
        features.push(event_type);

        // Pad to fixed size
        while features.len() < 16 {
            features.push(0.0);
        }

        features
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        0 // Would read hardware timer
    }

    /// Check if GPU is available
    pub fn gpu_available(&self) -> bool {
        self.gpu_enabled // Would check actual hardware
    }

    /// Check if NPU is available
    pub fn npu_available(&self) -> bool {
        self.npu_enabled // Would check actual hardware
    }

    /// Get statistics
    pub fn statistics(&self) -> NeuralEngineStatistics {
        NeuralEngineStatistics {
            gpu_enabled: self.gpu_enabled,
            npu_enabled: self.npu_enabled,
            models_registered: self.models.read().len(),
            trees_registered: self.decision_trees.read().len(),
            matchers_registered: self.pattern_matchers.read().len(),
            inferences_run: self.stats.inferences_run.load(Ordering::Relaxed),
            patterns_matched: self.stats.patterns_matched.load(Ordering::Relaxed),
            total_inference_time_us: self.stats.total_inference_time_us.load(Ordering::Relaxed),
            gpu_inferences: self.stats.gpu_inferences.load(Ordering::Relaxed),
            npu_inferences: self.stats.npu_inferences.load(Ordering::Relaxed),
        }
    }
}

/// Public statistics
#[derive(Debug, Clone)]
pub struct NeuralEngineStatistics {
    pub gpu_enabled: bool,
    pub npu_enabled: bool,
    pub models_registered: usize,
    pub trees_registered: usize,
    pub matchers_registered: usize,
    pub inferences_run: u64,
    pub patterns_matched: u64,
    pub total_inference_time_us: u64,
    pub gpu_inferences: u64,
    pub npu_inferences: u64,
}

// =============================================================================
// Pre-built Models
// =============================================================================

/// Create a simple anomaly detection model
pub fn create_anomaly_detector() -> NeuralModel {
    let mut model = NeuralModel::new(
        1,
        "anomaly_detector".to_string(),
        TensorShape::vector(16),
        TensorShape::vector(2),
    );

    model.add_layer(Box::new(DenseLayer::random(16, 32, Activation::ReLU)));
    model.add_layer(Box::new(DenseLayer::random(32, 16, Activation::ReLU)));
    model.add_layer(Box::new(DenseLayer::random(16, 2, Activation::Softmax)));

    model
}

/// Create a workload classifier model
pub fn create_workload_classifier() -> NeuralModel {
    let mut model = NeuralModel::new(
        2,
        "workload_classifier".to_string(),
        TensorShape::vector(8),
        TensorShape::vector(5), // 5 workload classes
    );

    model.add_layer(Box::new(DenseLayer::random(8, 16, Activation::ReLU)));
    model.add_layer(Box::new(DenseLayer::random(16, 5, Activation::Softmax)));

    model
}

/// Create a system health decision tree
pub fn create_health_tree() -> DecisionTree {
    DecisionTree::new(
        1,
        DecisionNode::Split {
            feature_index: 0, // CPU usage
            threshold: 0.8,
            left: Box::new(DecisionNode::Split {
                feature_index: 1, // Memory usage
                threshold: 0.7,
                left: Box::new(DecisionNode::Leaf {
                    class_index: 0, // Healthy
                    confidence: 0.9,
                }),
                right: Box::new(DecisionNode::Leaf {
                    class_index: 1, // Memory pressure
                    confidence: 0.8,
                }),
            }),
            right: Box::new(DecisionNode::Leaf {
                class_index: 2, // CPU overload
                confidence: 0.85,
            }),
        },
        vec!["cpu_usage".to_string(), "memory_usage".to_string()],
        vec!["healthy".to_string(), "memory_pressure".to_string(), "cpu_overload".to_string()],
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_operations() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0], TensorShape::vector(3));
        let b = Tensor::from_vec(vec![4.0, 5.0, 6.0], TensorShape::vector(3));

        let sum = a.add(&b).unwrap();
        assert_eq!(sum.data(), &[5.0, 7.0, 9.0]);

        let prod = a.mul(&b).unwrap();
        assert_eq!(prod.data(), &[4.0, 10.0, 18.0]);
    }

    #[test]
    fn test_tensor_matmul() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], TensorShape::matrix(2, 2));
        let b = Tensor::from_vec(vec![5.0, 6.0, 7.0, 8.0], TensorShape::matrix(2, 2));

        let c = a.matmul(&b).unwrap();
        assert_eq!(c.shape.dims, vec![2, 2]);
        assert_eq!(c.data(), &[19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn test_tensor_activations() {
        let t = Tensor::from_vec(vec![-1.0, 0.0, 1.0, 2.0], TensorShape::vector(4));

        let relu = t.relu();
        assert_eq!(relu.data(), &[0.0, 0.0, 1.0, 2.0]);

        let sigmoid = t.sigmoid();
        for val in sigmoid.data() {
            assert!(*val >= 0.0 && *val <= 1.0);
        }
    }

    #[test]
    fn test_pattern_matcher() {
        let mut matcher = PatternMatcher::new(1, 0.8);

        matcher.add_pattern(vec![1.0, 0.0, 0.0], "pattern_a".to_string(), None);
        matcher.add_pattern(vec![0.0, 1.0, 0.0], "pattern_b".to_string(), None);

        let matches = matcher.find_matches(&[0.9, 0.1, 0.0]);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].label, "pattern_a");
    }

    #[test]
    fn test_decision_tree() {
        let tree = create_health_tree();

        // Low CPU, low memory -> healthy
        let (class, conf) = tree.predict(&[0.3, 0.3]);
        assert_eq!(class, 0);
        assert!(conf > 0.5);

        // High CPU -> overload
        let (class, _) = tree.predict(&[0.9, 0.3]);
        assert_eq!(class, 2);
    }

    #[test]
    fn test_neural_model() {
        let model = create_workload_classifier();

        let input = Tensor::ones(TensorShape::vector(8), TensorDtype::F32);
        let output = model.forward(&input);

        // Output should be softmax (sum to 1)
        let sum: f32 = output.data().iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_neural_engine() {
        let engine = NeuralEngine::new(false, false);

        engine.register_model(create_workload_classifier());
        engine.register_tree(create_health_tree());

        let input = Tensor::ones(TensorShape::vector(8), TensorDtype::F32);
        let result = engine.infer(2, &input);

        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.class_index.is_some());
    }
}
