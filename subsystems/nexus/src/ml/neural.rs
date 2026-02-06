//! Tiny Neural Network Implementation
//!
//! Lightweight neural networks for kernel inference.

use alloc::vec::Vec;

use super::Lcg;
use crate::math;

// ============================================================================
// ACTIVATION FUNCTIONS
// ============================================================================

/// Activation function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// ReLU
    ReLU,
    /// Sigmoid
    Sigmoid,
    /// Tanh
    Tanh,
    /// Linear (identity)
    Linear,
}

impl Activation {
    /// Apply activation
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::ReLU => x.max(0.0),
            Self::Sigmoid => super::sigmoid(x),
            Self::Tanh => math::tanh(x),
            Self::Linear => x,
        }
    }

    /// Derivative
    pub fn derivative(&self, x: f64) -> f64 {
        match self {
            Self::ReLU => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            },
            Self::Sigmoid => {
                let s = super::sigmoid(x);
                s * (1.0 - s)
            },
            Self::Tanh => {
                let t = math::tanh(x);
                1.0 - t * t
            },
            Self::Linear => 1.0,
        }
    }
}

// ============================================================================
// DENSE LAYER
// ============================================================================

/// A dense layer
pub struct DenseLayer {
    /// Weights [output_size x input_size]
    weights: Vec<Vec<f64>>,
    /// Biases [output_size]
    biases: Vec<f64>,
    /// Activation
    activation: Activation,
    /// Last input (for backprop)
    last_input: Vec<f64>,
    /// Last output (before activation)
    last_output: Vec<f64>,
}

impl DenseLayer {
    /// Create new layer
    pub fn new(input_size: usize, output_size: usize, activation: Activation) -> Self {
        let mut rng = Lcg::new(42);

        // Xavier initialization
        let scale = math::sqrt(2.0 / (input_size + output_size) as f64);

        let weights = (0..output_size)
            .map(|_| {
                (0..input_size)
                    .map(|_| (rng.next_f64() * 2.0 - 1.0) * scale)
                    .collect()
            })
            .collect();

        let biases = alloc::vec![0.0; output_size];

        Self {
            weights,
            biases,
            activation,
            last_input: Vec::new(),
            last_output: Vec::new(),
        }
    }

    /// Forward pass
    pub fn forward(&mut self, input: &[f64]) -> Vec<f64> {
        self.last_input = input.to_vec();
        self.last_output = Vec::with_capacity(self.weights.len());

        for i in 0..self.weights.len() {
            let mut sum = self.biases[i];
            for (j, &x) in input.iter().enumerate() {
                if j < self.weights[i].len() {
                    sum += self.weights[i][j] * x;
                }
            }
            self.last_output.push(sum);
        }

        self.last_output
            .iter()
            .map(|&x| self.activation.apply(x))
            .collect()
    }

    /// Backward pass (returns gradient for previous layer)
    pub fn backward(&mut self, grad_output: &[f64], learning_rate: f64) -> Vec<f64> {
        let mut grad_input = alloc::vec![0.0; self.last_input.len()];

        for (i, weight_row) in self.weights.iter_mut().enumerate() {
            let d_activation = self.activation.derivative(self.last_output[i]);
            let delta = grad_output[i] * d_activation;

            // Update biases
            self.biases[i] -= learning_rate * delta;

            // Update weights and compute gradient for input
            for (j, weight) in weight_row.iter_mut().enumerate() {
                if j < self.last_input.len() {
                    grad_input[j] += *weight * delta;
                    *weight -= learning_rate * delta * self.last_input[j];
                }
            }
        }

        grad_input
    }

    /// Output size
    pub fn output_size(&self) -> usize {
        self.weights.len()
    }
}

// ============================================================================
// TINY NEURAL NETWORK
// ============================================================================

/// Tiny neural network
pub struct TinyNN {
    /// Layers
    layers: Vec<DenseLayer>,
    /// Learning rate
    learning_rate: f64,
}

impl TinyNN {
    /// Create new network
    pub fn new(learning_rate: f64) -> Self {
        Self {
            layers: Vec::new(),
            learning_rate,
        }
    }

    /// Add layer
    pub fn add_layer(&mut self, layer: DenseLayer) {
        self.layers.push(layer);
    }

    /// Forward pass
    pub fn forward(&mut self, input: &[f64]) -> Vec<f64> {
        let mut current = input.to_vec();
        for layer in &mut self.layers {
            current = layer.forward(&current);
        }
        current
    }

    /// Train on single sample
    pub fn train(&mut self, input: &[f64], target: &[f64]) -> f64 {
        // Forward pass
        let output = self.forward(input);

        // Compute loss gradient (MSE)
        let mut grad: Vec<f64> = output
            .iter()
            .zip(target.iter())
            .map(|(o, t)| 2.0 * (o - t) / output.len() as f64)
            .collect();

        // Backward pass
        for layer in self.layers.iter_mut().rev() {
            grad = layer.backward(&grad, self.learning_rate);
        }

        // Return loss
        output
            .iter()
            .zip(target.iter())
            .map(|(o, t)| (o - t) * (o - t))
            .sum::<f64>()
            / output.len() as f64
    }

    /// Predict
    pub fn predict(&mut self, input: &[f64]) -> Vec<f64> {
        self.forward(input)
    }
}

impl Default for TinyNN {
    fn default() -> Self {
        Self::new(0.01)
    }
}
