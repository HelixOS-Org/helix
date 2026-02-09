//! NEXUS Year 2: Neural Network Layers
//!
//! Layer implementations for neural networks.
//! Pure Rust, no_std compatible.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::activation::{Activation, ActivationType, create_activation};
use super::tensor::{Tensor, TensorShape};

// ============================================================================
// Layer Trait
// ============================================================================

/// Neural network layer trait
pub trait Layer: Send + Sync {
    /// Forward pass
    fn forward(&self, input: &Tensor) -> Tensor;

    /// Get layer name
    fn name(&self) -> &str;

    /// Get output shape given input shape
    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape;

    /// Get number of trainable parameters
    fn num_parameters(&self) -> usize;

    /// Get parameters as tensors
    fn parameters(&self) -> Vec<&Tensor>;

    /// Get mutable parameters
    fn parameters_mut(&mut self) -> Vec<&mut Tensor>;
}

// ============================================================================
// Dense Layer (Fully Connected)
// ============================================================================

/// Dense (fully connected) layer: y = Wx + b
pub struct DenseLayer {
    name: String,
    weights: Tensor,
    bias: Tensor,
    input_size: usize,
    output_size: usize,
    activation: Box<dyn Activation>,
}

impl DenseLayer {
    pub fn new(
        name: &str,
        input_size: usize,
        output_size: usize,
        activation_type: ActivationType,
    ) -> Self {
        // Xavier initialization
        let weights = Tensor::xavier(
            TensorShape::matrix(output_size, input_size),
            (input_size as u64 * 31 + output_size as u64 * 37).wrapping_mul(0xdeadbeef),
        );
        let bias = Tensor::zeros(TensorShape::vector(output_size));

        Self {
            name: String::from(name),
            weights,
            bias,
            input_size,
            output_size,
            activation: create_activation(activation_type),
        }
    }

    pub fn with_weights(
        name: &str,
        weights: Tensor,
        bias: Tensor,
        activation_type: ActivationType,
    ) -> Self {
        let output_size = weights.shape().dim(0);
        let input_size = weights.shape().dim(1);

        Self {
            name: String::from(name),
            weights,
            bias,
            input_size,
            output_size,
            activation: create_activation(activation_type),
        }
    }

    #[inline(always)]
    pub fn input_size(&self) -> usize {
        self.input_size
    }

    #[inline(always)]
    pub fn output_size(&self) -> usize {
        self.output_size
    }

    #[inline(always)]
    pub fn weights(&self) -> &Tensor {
        &self.weights
    }

    #[inline(always)]
    pub fn bias(&self) -> &Tensor {
        &self.bias
    }
}

impl Layer for DenseLayer {
    fn forward(&self, input: &Tensor) -> Tensor {
        // Flatten input if needed
        let flat_input = if input.shape().ndim() == 1 {
            input.clone()
        } else {
            input.flatten()
        };

        // y = Wx + b
        let linear = if let Some(result) = self.weights.matvec(&flat_input) {
            result.add(&self.bias).unwrap_or(result)
        } else {
            Tensor::zeros(TensorShape::vector(self.output_size))
        };

        // Apply activation
        self.activation.forward(&linear)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, _input_shape: &TensorShape) -> TensorShape {
        TensorShape::vector(self.output_size)
    }

    fn num_parameters(&self) -> usize {
        self.weights.len() + self.bias.len()
    }

    fn parameters(&self) -> Vec<&Tensor> {
        alloc::vec![&self.weights, &self.bias]
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        alloc::vec![&mut self.weights, &mut self.bias]
    }
}

// ============================================================================
// Layer Normalization
// ============================================================================

/// Layer normalization: normalize across features
pub struct LayerNorm {
    name: String,
    normalized_shape: usize,
    gamma: Tensor, // scale
    beta: Tensor,  // shift
    epsilon: f32,
}

impl LayerNorm {
    pub fn new(name: &str, normalized_shape: usize) -> Self {
        Self {
            name: String::from(name),
            normalized_shape,
            gamma: Tensor::ones(TensorShape::vector(normalized_shape)),
            beta: Tensor::zeros(TensorShape::vector(normalized_shape)),
            epsilon: 1e-5,
        }
    }

    #[inline(always)]
    pub fn with_epsilon(mut self, epsilon: f32) -> Self {
        self.epsilon = epsilon;
        self
    }
}

impl Layer for LayerNorm {
    fn forward(&self, input: &Tensor) -> Tensor {
        let mean = input.mean();
        let variance = input.variance();
        let std = libm::sqrtf(variance + self.epsilon);

        // Normalize
        let normalized: Vec<f32> = input.data().iter().map(|&x| (x - mean) / std).collect();

        // Scale and shift
        let output: Vec<f32> = normalized
            .iter()
            .zip(self.gamma.data().iter().cycle())
            .zip(self.beta.data().iter().cycle())
            .map(|((&n, &g), &b)| n * g + b)
            .collect();

        Tensor::from_data(*input.shape(), output)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        *input_shape
    }

    fn num_parameters(&self) -> usize {
        self.gamma.len() + self.beta.len()
    }

    fn parameters(&self) -> Vec<&Tensor> {
        alloc::vec![&self.gamma, &self.beta]
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        alloc::vec![&mut self.gamma, &mut self.beta]
    }
}

// ============================================================================
// Batch Normalization
// ============================================================================

/// Batch normalization (inference mode)
pub struct BatchNorm {
    name: String,
    num_features: usize,
    gamma: Tensor,
    beta: Tensor,
    running_mean: Tensor,
    running_var: Tensor,
    epsilon: f32,
}

impl BatchNorm {
    pub fn new(name: &str, num_features: usize) -> Self {
        Self {
            name: String::from(name),
            num_features,
            gamma: Tensor::ones(TensorShape::vector(num_features)),
            beta: Tensor::zeros(TensorShape::vector(num_features)),
            running_mean: Tensor::zeros(TensorShape::vector(num_features)),
            running_var: Tensor::ones(TensorShape::vector(num_features)),
            epsilon: 1e-5,
        }
    }
}

impl Layer for BatchNorm {
    fn forward(&self, input: &Tensor) -> Tensor {
        // Use running statistics (inference mode)
        let output: Vec<f32> = input
            .data()
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let feat_idx = i % self.num_features;
                let mean = self.running_mean.data()[feat_idx];
                let var = self.running_var.data()[feat_idx];
                let std = libm::sqrtf(var + self.epsilon);
                let normalized = (x - mean) / std;
                normalized * self.gamma.data()[feat_idx] + self.beta.data()[feat_idx]
            })
            .collect();

        Tensor::from_data(*input.shape(), output)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        *input_shape
    }

    fn num_parameters(&self) -> usize {
        self.gamma.len() + self.beta.len()
    }

    fn parameters(&self) -> Vec<&Tensor> {
        alloc::vec![&self.gamma, &self.beta]
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        alloc::vec![&mut self.gamma, &mut self.beta]
    }
}

// ============================================================================
// Dropout Layer
// ============================================================================

/// Dropout layer (training mode uses random dropping, inference passes through)
pub struct Dropout {
    name: String,
    rate: f32,
    training: bool,
    seed: u64,
}

impl Dropout {
    pub fn new(name: &str, rate: f32) -> Self {
        Self {
            name: String::from(name),
            rate: rate.clamp(0.0, 1.0),
            training: false, // Default to inference mode
            seed: 42,
        }
    }

    #[inline(always)]
    pub fn set_training(&mut self, training: bool) {
        self.training = training;
    }

    #[inline(always)]
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }
}

impl Layer for Dropout {
    fn forward(&self, input: &Tensor) -> Tensor {
        if !self.training || self.rate == 0.0 {
            // Inference mode: pass through
            return input.clone();
        }

        // Training mode: randomly drop units
        let mut rng = self.seed;
        let scale = 1.0 / (1.0 - self.rate);

        let output: Vec<f32> = input
            .data()
            .iter()
            .map(|&x| {
                rng = rng.wrapping_mul(0x5851f42d4c957f2d).wrapping_add(1);
                let random = (rng as f32) / (u64::MAX as f32);
                if random < self.rate { 0.0 } else { x * scale }
            })
            .collect();

        Tensor::from_data(*input.shape(), output)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        *input_shape
    }

    fn num_parameters(&self) -> usize {
        0
    }

    fn parameters(&self) -> Vec<&Tensor> {
        Vec::new()
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        Vec::new()
    }
}

// ============================================================================
// Flatten Layer
// ============================================================================

/// Flatten layer: reshape to 1D
pub struct Flatten {
    name: String,
}

impl Flatten {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
        }
    }
}

impl Layer for Flatten {
    fn forward(&self, input: &Tensor) -> Tensor {
        input.flatten()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        TensorShape::vector(input_shape.total_elements())
    }

    fn num_parameters(&self) -> usize {
        0
    }

    fn parameters(&self) -> Vec<&Tensor> {
        Vec::new()
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        Vec::new()
    }
}

// ============================================================================
// Reshape Layer
// ============================================================================

/// Reshape layer
pub struct Reshape {
    name: String,
    target_shape: TensorShape,
}

impl Reshape {
    pub fn new(name: &str, target_shape: TensorShape) -> Self {
        Self {
            name: String::from(name),
            target_shape,
        }
    }
}

impl Layer for Reshape {
    fn forward(&self, input: &Tensor) -> Tensor {
        input
            .reshape(self.target_shape)
            .unwrap_or_else(|| input.clone())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, _input_shape: &TensorShape) -> TensorShape {
        self.target_shape
    }

    fn num_parameters(&self) -> usize {
        0
    }

    fn parameters(&self) -> Vec<&Tensor> {
        Vec::new()
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        Vec::new()
    }
}

// ============================================================================
// Embedding Layer
// ============================================================================

/// Embedding layer: lookup table for discrete tokens
pub struct Embedding {
    name: String,
    num_embeddings: usize,
    embedding_dim: usize,
    weight: Tensor,
}

impl Embedding {
    pub fn new(name: &str, num_embeddings: usize, embedding_dim: usize, seed: u64) -> Self {
        let weight = Tensor::xavier(TensorShape::matrix(num_embeddings, embedding_dim), seed);

        Self {
            name: String::from(name),
            num_embeddings,
            embedding_dim,
            weight,
        }
    }

    /// Look up embeddings for given indices
    pub fn lookup(&self, indices: &[usize]) -> Tensor {
        let mut data = Vec::with_capacity(indices.len() * self.embedding_dim);

        for &idx in indices {
            let safe_idx = idx % self.num_embeddings;
            let start = safe_idx * self.embedding_dim;
            let end = start + self.embedding_dim;
            data.extend_from_slice(&self.weight.data()[start..end]);
        }

        if indices.len() == 1 {
            Tensor::from_data(TensorShape::vector(self.embedding_dim), data)
        } else {
            Tensor::from_data(TensorShape::matrix(indices.len(), self.embedding_dim), data)
        }
    }
}

impl Layer for Embedding {
    fn forward(&self, input: &Tensor) -> Tensor {
        // Interpret input as indices (first elements as usize)
        let indices: Vec<usize> = input.data().iter().map(|&v| v as usize).collect();
        self.lookup(&indices)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        let seq_len = input_shape.total_elements();
        if seq_len == 1 {
            TensorShape::vector(self.embedding_dim)
        } else {
            TensorShape::matrix(seq_len, self.embedding_dim)
        }
    }

    fn num_parameters(&self) -> usize {
        self.weight.len()
    }

    fn parameters(&self) -> Vec<&Tensor> {
        alloc::vec![&self.weight]
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        alloc::vec![&mut self.weight]
    }
}

// ============================================================================
// Conv1D Layer
// ============================================================================

/// 1D Convolution layer
pub struct Conv1D {
    name: String,
    in_channels: usize,
    out_channels: usize,
    kernel_size: usize,
    stride: usize,
    padding: usize,
    weight: Tensor,
    bias: Tensor,
    activation: Box<dyn Activation>,
}

impl Conv1D {
    pub fn new(
        name: &str,
        in_channels: usize,
        out_channels: usize,
        kernel_size: usize,
        activation_type: ActivationType,
    ) -> Self {
        let weight = Tensor::xavier(
            TensorShape::tensor3d(out_channels, in_channels, kernel_size),
            (in_channels as u64 * out_channels as u64).wrapping_mul(0xcafe),
        );
        let bias = Tensor::zeros(TensorShape::vector(out_channels));

        Self {
            name: String::from(name),
            in_channels,
            out_channels,
            kernel_size,
            stride: 1,
            padding: 0,
            weight,
            bias,
            activation: create_activation(activation_type),
        }
    }

    #[inline(always)]
    pub fn with_stride(mut self, stride: usize) -> Self {
        self.stride = stride.max(1);
        self
    }

    #[inline(always)]
    pub fn with_padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self
    }
}

impl Layer for Conv1D {
    fn forward(&self, input: &Tensor) -> Tensor {
        // Simplified 1D convolution
        let input_len = input.len();
        let padded_len = input_len + 2 * self.padding;
        let output_len = (padded_len - self.kernel_size) / self.stride + 1;

        let mut output = Vec::with_capacity(self.out_channels * output_len);

        for out_c in 0..self.out_channels {
            for o in 0..output_len {
                let start = o * self.stride;
                let mut sum = self.bias.data()[out_c];

                for k in 0..self.kernel_size {
                    let input_idx = start + k;
                    let input_val =
                        if input_idx < self.padding || input_idx >= self.padding + input_len {
                            0.0
                        } else {
                            input.data()[(input_idx - self.padding) % input_len]
                        };

                    let weight_idx = out_c * self.in_channels * self.kernel_size + k;
                    if weight_idx < self.weight.len() {
                        sum += input_val * self.weight.data()[weight_idx];
                    }
                }

                output.push(sum);
            }
        }

        let linear = Tensor::from_data(TensorShape::matrix(self.out_channels, output_len), output);

        self.activation.forward(&linear)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        let input_len = input_shape.total_elements();
        let padded_len = input_len + 2 * self.padding;
        let output_len = (padded_len - self.kernel_size) / self.stride + 1;
        TensorShape::matrix(self.out_channels, output_len)
    }

    fn num_parameters(&self) -> usize {
        self.weight.len() + self.bias.len()
    }

    fn parameters(&self) -> Vec<&Tensor> {
        alloc::vec![&self.weight, &self.bias]
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        alloc::vec![&mut self.weight, &mut self.bias]
    }
}

// ============================================================================
// MaxPool1D Layer
// ============================================================================

/// 1D Max Pooling layer
#[repr(align(64))]
pub struct MaxPool1D {
    name: String,
    kernel_size: usize,
    stride: usize,
}

impl MaxPool1D {
    pub fn new(name: &str, kernel_size: usize) -> Self {
        Self {
            name: String::from(name),
            kernel_size,
            stride: kernel_size, // Default: non-overlapping
        }
    }

    #[inline(always)]
    pub fn with_stride(mut self, stride: usize) -> Self {
        self.stride = stride.max(1);
        self
    }
}

impl Layer for MaxPool1D {
    fn forward(&self, input: &Tensor) -> Tensor {
        let input_len = input.len();
        let output_len = (input_len - self.kernel_size) / self.stride + 1;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let start = i * self.stride;
            let end = (start + self.kernel_size).min(input_len);
            let max = input.data()[start..end]
                .iter()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max);
            output.push(max);
        }

        Tensor::from_data(TensorShape::vector(output_len), output)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        let input_len = input_shape.total_elements();
        let output_len = (input_len - self.kernel_size) / self.stride + 1;
        TensorShape::vector(output_len)
    }

    fn num_parameters(&self) -> usize {
        0
    }

    fn parameters(&self) -> Vec<&Tensor> {
        Vec::new()
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        Vec::new()
    }
}

// ============================================================================
// AvgPool1D Layer
// ============================================================================

/// 1D Average Pooling layer
#[repr(align(64))]
pub struct AvgPool1D {
    name: String,
    kernel_size: usize,
    stride: usize,
}

impl AvgPool1D {
    pub fn new(name: &str, kernel_size: usize) -> Self {
        Self {
            name: String::from(name),
            kernel_size,
            stride: kernel_size,
        }
    }

    #[inline(always)]
    pub fn with_stride(mut self, stride: usize) -> Self {
        self.stride = stride.max(1);
        self
    }
}

impl Layer for AvgPool1D {
    fn forward(&self, input: &Tensor) -> Tensor {
        let input_len = input.len();
        let output_len = (input_len - self.kernel_size) / self.stride + 1;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let start = i * self.stride;
            let end = (start + self.kernel_size).min(input_len);
            let sum: f32 = input.data()[start..end].iter().sum();
            output.push(sum / (end - start) as f32);
        }

        Tensor::from_data(TensorShape::vector(output_len), output)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        let input_len = input_shape.total_elements();
        let output_len = (input_len - self.kernel_size) / self.stride + 1;
        TensorShape::vector(output_len)
    }

    fn num_parameters(&self) -> usize {
        0
    }

    fn parameters(&self) -> Vec<&Tensor> {
        Vec::new()
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        Vec::new()
    }
}

// ============================================================================
// Global Average Pooling
// ============================================================================

/// Global Average Pooling: average over all spatial dimensions
#[repr(align(64))]
pub struct GlobalAvgPool {
    name: String,
}

impl GlobalAvgPool {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
        }
    }
}

impl Layer for GlobalAvgPool {
    fn forward(&self, input: &Tensor) -> Tensor {
        Tensor::from_slice(&[input.mean()])
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, _input_shape: &TensorShape) -> TensorShape {
        TensorShape::vector(1)
    }

    fn num_parameters(&self) -> usize {
        0
    }

    fn parameters(&self) -> Vec<&Tensor> {
        Vec::new()
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        Vec::new()
    }
}

// ============================================================================
// Residual Connection
// ============================================================================

/// Residual connection wrapper
pub struct Residual {
    name: String,
    inner: Box<dyn Layer>,
}

impl Residual {
    pub fn new(name: &str, inner: Box<dyn Layer>) -> Self {
        Self {
            name: String::from(name),
            inner,
        }
    }
}

impl Layer for Residual {
    fn forward(&self, input: &Tensor) -> Tensor {
        let output = self.inner.forward(input);
        if let Some(result) = output.add(input) {
            result
        } else {
            output
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, input_shape: &TensorShape) -> TensorShape {
        self.inner.output_shape(input_shape)
    }

    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    fn parameters(&self) -> Vec<&Tensor> {
        self.inner.parameters()
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        self.inner.parameters_mut()
    }
}

// ============================================================================
// Attention Layer (Simplified Single-Head)
// ============================================================================

/// Simplified self-attention layer
pub struct SelfAttention {
    name: String,
    embed_dim: usize,
    query: Tensor,
    key: Tensor,
    value: Tensor,
    output: Tensor,
}

impl SelfAttention {
    pub fn new(name: &str, embed_dim: usize, seed: u64) -> Self {
        Self {
            name: String::from(name),
            embed_dim,
            query: Tensor::xavier(TensorShape::matrix(embed_dim, embed_dim), seed),
            key: Tensor::xavier(
                TensorShape::matrix(embed_dim, embed_dim),
                seed.wrapping_add(1),
            ),
            value: Tensor::xavier(
                TensorShape::matrix(embed_dim, embed_dim),
                seed.wrapping_add(2),
            ),
            output: Tensor::xavier(
                TensorShape::matrix(embed_dim, embed_dim),
                seed.wrapping_add(3),
            ),
        }
    }

    fn scaled_dot_product_attention(&self, q: &Tensor, k: &Tensor, v: &Tensor) -> Tensor {
        let scale = 1.0 / libm::sqrtf(self.embed_dim as f32);

        // Compute attention scores (simplified for 1D input)
        if let (Some(scores), Some(_k_t)) = (q.dot(k), k.clone().transpose()) {
            let scaled_score = scores * scale;
            let attention_weight = 1.0 / (1.0 + libm::expf(-scaled_score)); // sigmoid approx

            // Weighted sum of values
            v.mul_scalar(attention_weight)
        } else {
            v.clone()
        }
    }
}

impl Layer for SelfAttention {
    fn forward(&self, input: &Tensor) -> Tensor {
        // Project to Q, K, V
        let q = self.query.matvec(input).unwrap_or_else(|| input.clone());
        let k = self.key.matvec(input).unwrap_or_else(|| input.clone());
        let v = self.value.matvec(input).unwrap_or_else(|| input.clone());

        // Compute attention
        let attended = self.scaled_dot_product_attention(&q, &k, &v);

        // Output projection
        self.output.matvec(&attended).unwrap_or(attended)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_shape(&self, _input_shape: &TensorShape) -> TensorShape {
        TensorShape::vector(self.embed_dim)
    }

    fn num_parameters(&self) -> usize {
        self.query.len() + self.key.len() + self.value.len() + self.output.len()
    }

    fn parameters(&self) -> Vec<&Tensor> {
        alloc::vec![&self.query, &self.key, &self.value, &self.output]
    }

    fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        alloc::vec![
            &mut self.query,
            &mut self.key,
            &mut self.value,
            &mut self.output
        ]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dense_layer() {
        let layer = DenseLayer::new("dense1", 10, 5, ActivationType::ReLU);
        let input = Tensor::ones(TensorShape::vector(10));
        let output = layer.forward(&input);
        assert_eq!(output.shape().dim(0), 5);
    }

    #[test]
    fn test_layer_norm() {
        let layer = LayerNorm::new("ln1", 10);
        let input = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        let output = layer.forward(&input);
        assert_eq!(output.len(), 10);

        // Check normalization (mean ~ 0, std ~ 1)
        let mean = output.mean();
        assert!(mean.abs() < 0.1);
    }

    #[test]
    fn test_dropout() {
        let layer = Dropout::new("drop1", 0.5);
        let input = Tensor::ones(TensorShape::vector(100));
        let output = layer.forward(&input);

        // In inference mode, should pass through
        assert_eq!(output.sum(), 100.0);
    }

    #[test]
    fn test_embedding() {
        let layer = Embedding::new("embed", 100, 32, 42);
        let indices = [1, 5, 10];
        let output = layer.lookup(&indices);
        assert_eq!(output.shape().dim(0), 3);
        assert_eq!(output.shape().dim(1), 32);
    }

    #[test]
    fn test_max_pool() {
        let layer = MaxPool1D::new("pool1", 2);
        let input = Tensor::from_slice(&[1.0, 3.0, 2.0, 4.0, 5.0, 1.0]);
        let output = layer.forward(&input);
        assert_eq!(output.data(), &[3.0, 4.0, 5.0]);
    }
}
