//! NEXUS Year 2: Activation Functions
//!
//! Non-linear activation functions for neural networks.
//! Pure Rust, no_std compatible.

#![allow(dead_code)]

use alloc::vec::Vec;

use super::tensor::Tensor;

// ============================================================================
// Activation Trait
// ============================================================================

/// Activation function trait
pub trait Activation: Send + Sync {
    /// Forward pass
    fn forward(&self, x: &Tensor) -> Tensor;

    /// Backward pass (gradient)
    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor;

    /// Name of the activation
    fn name(&self) -> &'static str;
}

// ============================================================================
// ReLU (Rectified Linear Unit)
// ============================================================================

/// ReLU: max(0, x)
#[derive(Debug, Clone, Copy)]
pub struct ReLU;

impl ReLU {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReLU {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for ReLU {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x.data().iter().map(|&v| v.max(0.0)).collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| if xi > 0.0 { gi } else { 0.0 })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "ReLU"
    }
}

// ============================================================================
// LeakyReLU
// ============================================================================

/// LeakyReLU: max(alpha*x, x)
#[derive(Debug, Clone, Copy)]
pub struct LeakyReLU {
    alpha: f32,
}

impl LeakyReLU {
    pub fn new(alpha: f32) -> Self {
        Self { alpha }
    }
}

impl Default for LeakyReLU {
    fn default() -> Self {
        Self::new(0.01)
    }
}

impl Activation for LeakyReLU {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .map(|&v| if v > 0.0 { v } else { self.alpha * v })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| if xi > 0.0 { gi } else { self.alpha * gi })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "LeakyReLU"
    }
}

// ============================================================================
// ELU (Exponential Linear Unit)
// ============================================================================

/// ELU: x if x > 0, alpha * (exp(x) - 1) otherwise
#[derive(Debug, Clone, Copy)]
pub struct ELU {
    alpha: f32,
}

impl ELU {
    pub fn new(alpha: f32) -> Self {
        Self { alpha }
    }
}

impl Default for ELU {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Activation for ELU {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .map(|&v| {
                if v > 0.0 {
                    v
                } else {
                    self.alpha * (libm::expf(v) - 1.0)
                }
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| {
                if xi > 0.0 {
                    gi
                } else {
                    gi * self.alpha * libm::expf(xi)
                }
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "ELU"
    }
}

// ============================================================================
// SELU (Scaled ELU)
// ============================================================================

/// SELU: Scaled Exponential Linear Unit for self-normalizing networks
#[derive(Debug, Clone, Copy)]
pub struct SELU;

impl SELU {
    // Fixed constants for self-normalization
    const ALPHA: f32 = 1.6732632423543772;
    const SCALE: f32 = 1.0507009873554805;

    pub fn new() -> Self {
        Self
    }
}

impl Default for SELU {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for SELU {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .map(|&v| {
                Self::SCALE
                    * if v > 0.0 {
                        v
                    } else {
                        Self::ALPHA * (libm::expf(v) - 1.0)
                    }
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| {
                gi * Self::SCALE
                    * if xi > 0.0 {
                        1.0
                    } else {
                        Self::ALPHA * libm::expf(xi)
                    }
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "SELU"
    }
}

// ============================================================================
// Sigmoid
// ============================================================================

/// Sigmoid: 1 / (1 + exp(-x))
#[derive(Debug, Clone, Copy)]
pub struct Sigmoid;

impl Sigmoid {
    pub fn new() -> Self {
        Self
    }

    fn sigmoid(x: f32) -> f32 {
        if x >= 0.0 {
            let exp_neg_x = libm::expf(-x);
            1.0 / (1.0 + exp_neg_x)
        } else {
            let exp_x = libm::expf(x);
            exp_x / (1.0 + exp_x)
        }
    }
}

impl Default for Sigmoid {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for Sigmoid {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x.data().iter().map(|&v| Self::sigmoid(v)).collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| {
                let s = Self::sigmoid(xi);
                gi * s * (1.0 - s)
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "Sigmoid"
    }
}

// ============================================================================
// Tanh
// ============================================================================

/// Tanh: (exp(x) - exp(-x)) / (exp(x) + exp(-x))
#[derive(Debug, Clone, Copy)]
pub struct Tanh;

impl Tanh {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Tanh {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for Tanh {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x.data().iter().map(|&v| libm::tanhf(v)).collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| {
                let t = libm::tanhf(xi);
                gi * (1.0 - t * t)
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "Tanh"
    }
}

// ============================================================================
// Softmax
// ============================================================================

/// Softmax: exp(x_i) / sum(exp(x_j))
#[derive(Debug, Clone, Copy)]
pub struct Softmax;

impl Softmax {
    pub fn new() -> Self {
        Self
    }

    /// Apply softmax to a 1D slice
    pub fn apply(values: &[f32]) -> Vec<f32> {
        if values.is_empty() {
            return Vec::new();
        }

        // Numerical stability: subtract max
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let exp_values: Vec<f32> = values.iter().map(|&v| libm::expf(v - max)).collect();

        let sum: f32 = exp_values.iter().sum();

        if sum > 0.0 {
            exp_values.iter().map(|&e| e / sum).collect()
        } else {
            // Fallback to uniform
            let uniform = 1.0 / values.len() as f32;
            alloc::vec![uniform; values.len()]
        }
    }
}

impl Default for Softmax {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for Softmax {
    fn forward(&self, x: &Tensor) -> Tensor {
        // Apply softmax row-wise for 2D, or to entire vector for 1D
        let data = Self::apply(x.data());
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        // Softmax backward is complex: d_softmax = softmax * (grad - sum(grad * softmax))
        let softmax = Self::apply(x.data());
        let dot: f32 = softmax
            .iter()
            .zip(grad_output.data().iter())
            .map(|(s, g)| s * g)
            .sum();

        let data: Vec<f32> = softmax
            .iter()
            .zip(grad_output.data().iter())
            .map(|(s, g)| s * (g - dot))
            .collect();

        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "Softmax"
    }
}

// ============================================================================
// LogSoftmax
// ============================================================================

/// LogSoftmax: log(softmax(x)) with numerical stability
#[derive(Debug, Clone, Copy)]
pub struct LogSoftmax;

impl LogSoftmax {
    pub fn new() -> Self {
        Self
    }

    pub fn apply(values: &[f32]) -> Vec<f32> {
        if values.is_empty() {
            return Vec::new();
        }

        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let exp_sum: f32 = values.iter().map(|&v| libm::expf(v - max)).sum();

        let log_sum = libm::logf(exp_sum) + max;

        values.iter().map(|&v| v - log_sum).collect()
    }
}

impl Default for LogSoftmax {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for LogSoftmax {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data = Self::apply(x.data());
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let softmax = Softmax::apply(x.data());
        let grad_sum: f32 = grad_output.data().iter().sum();

        let data: Vec<f32> = softmax
            .iter()
            .zip(grad_output.data().iter())
            .map(|(s, g)| g - s * grad_sum)
            .collect();

        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "LogSoftmax"
    }
}

// ============================================================================
// GELU (Gaussian Error Linear Unit)
// ============================================================================

/// GELU: x * Φ(x) where Φ is the CDF of standard normal
/// Approximation: 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
#[derive(Debug, Clone, Copy)]
pub struct GELU;

impl GELU {
    // sqrt(2/π)
    const SQRT_2_PI: f32 = 0.7978845608028654;
    const COEF: f32 = 0.044715;

    pub fn new() -> Self {
        Self
    }

    fn gelu(x: f32) -> f32 {
        let inner = Self::SQRT_2_PI * (x + Self::COEF * x * x * x);
        0.5 * x * (1.0 + libm::tanhf(inner))
    }

    fn gelu_derivative(x: f32) -> f32 {
        let x3 = x * x * x;
        let inner = Self::SQRT_2_PI * (x + Self::COEF * x3);
        let tanh_inner = libm::tanhf(inner);
        let sech2 = 1.0 - tanh_inner * tanh_inner;
        let inner_deriv = Self::SQRT_2_PI * (1.0 + 3.0 * Self::COEF * x * x);

        0.5 * (1.0 + tanh_inner) + 0.5 * x * sech2 * inner_deriv
    }
}

impl Default for GELU {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for GELU {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x.data().iter().map(|&v| Self::gelu(v)).collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| gi * Self::gelu_derivative(xi))
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "GELU"
    }
}

// ============================================================================
// SiLU / Swish
// ============================================================================

/// SiLU (Swish): x * sigmoid(x)
#[derive(Debug, Clone, Copy)]
pub struct SiLU;

impl SiLU {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SiLU {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for SiLU {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x.data().iter().map(|&v| v * Sigmoid::sigmoid(v)).collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| {
                let s = Sigmoid::sigmoid(xi);
                gi * (s + xi * s * (1.0 - s))
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "SiLU"
    }
}

// ============================================================================
// Mish
// ============================================================================

/// Mish: x * tanh(softplus(x)) = x * tanh(ln(1 + e^x))
#[derive(Debug, Clone, Copy)]
pub struct Mish;

impl Mish {
    pub fn new() -> Self {
        Self
    }

    fn softplus(x: f32) -> f32 {
        if x > 20.0 {
            x
        } else if x < -20.0 {
            libm::expf(x)
        } else {
            libm::logf(1.0 + libm::expf(x))
        }
    }
}

impl Default for Mish {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for Mish {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .map(|&v| v * libm::tanhf(Self::softplus(v)))
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        // Derivative is complex, using numerical approximation
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| {
                let sp = Self::softplus(xi);
                let tanh_sp = libm::tanhf(sp);
                let sigmoid = Sigmoid::sigmoid(xi);
                let sech2 = 1.0 - tanh_sp * tanh_sp;

                gi * (tanh_sp + xi * sech2 * sigmoid)
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "Mish"
    }
}

// ============================================================================
// Hardswish
// ============================================================================

/// Hardswish: x * ReLU6(x + 3) / 6
#[derive(Debug, Clone, Copy)]
pub struct Hardswish;

impl Hardswish {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Hardswish {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for Hardswish {
    fn forward(&self, x: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .map(|&v| {
                if v <= -3.0 {
                    0.0
                } else if v >= 3.0 {
                    v
                } else {
                    v * (v + 3.0) / 6.0
                }
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn backward(&self, x: &Tensor, grad_output: &Tensor) -> Tensor {
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(grad_output.data().iter())
            .map(|(&xi, &gi)| {
                if xi <= -3.0 {
                    0.0
                } else if xi >= 3.0 {
                    gi
                } else {
                    gi * (2.0 * xi + 3.0) / 6.0
                }
            })
            .collect();
        Tensor::from_data(*x.shape(), data)
    }

    fn name(&self) -> &'static str {
        "Hardswish"
    }
}

// ============================================================================
// Identity
// ============================================================================

/// Identity: f(x) = x
#[derive(Debug, Clone, Copy)]
pub struct Identity;

impl Identity {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Identity {
    fn default() -> Self {
        Self::new()
    }
}

impl Activation for Identity {
    fn forward(&self, x: &Tensor) -> Tensor {
        x.clone()
    }

    fn backward(&self, _x: &Tensor, grad_output: &Tensor) -> Tensor {
        grad_output.clone()
    }

    fn name(&self) -> &'static str {
        "Identity"
    }
}

// ============================================================================
// Activation Factory
// ============================================================================

/// Type of activation function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationType {
    ReLU,
    LeakyReLU,
    ELU,
    SELU,
    Sigmoid,
    Tanh,
    Softmax,
    LogSoftmax,
    GELU,
    SiLU,
    Mish,
    Hardswish,
    Identity,
}

use alloc::boxed::Box;

/// Create activation function from type
pub fn create_activation(activation_type: ActivationType) -> Box<dyn Activation> {
    match activation_type {
        ActivationType::ReLU => Box::new(ReLU::new()),
        ActivationType::LeakyReLU => Box::new(LeakyReLU::default()),
        ActivationType::ELU => Box::new(ELU::default()),
        ActivationType::SELU => Box::new(SELU::new()),
        ActivationType::Sigmoid => Box::new(Sigmoid::new()),
        ActivationType::Tanh => Box::new(Tanh::new()),
        ActivationType::Softmax => Box::new(Softmax::new()),
        ActivationType::LogSoftmax => Box::new(LogSoftmax::new()),
        ActivationType::GELU => Box::new(GELU::new()),
        ActivationType::SiLU => Box::new(SiLU::new()),
        ActivationType::Mish => Box::new(Mish::new()),
        ActivationType::Hardswish => Box::new(Hardswish::new()),
        ActivationType::Identity => Box::new(Identity::new()),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relu() {
        let relu = ReLU::new();
        let x = Tensor::from_slice(&[-2.0, -1.0, 0.0, 1.0, 2.0]);
        let y = relu.forward(&x);
        assert_eq!(y.data(), &[0.0, 0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_sigmoid() {
        let sigmoid = Sigmoid::new();
        let x = Tensor::from_slice(&[0.0]);
        let y = sigmoid.forward(&x);
        assert!((y.data()[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_softmax() {
        let softmax = Softmax::new();
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0]);
        let y = softmax.forward(&x);
        let sum: f32 = y.data().iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_tanh() {
        let tanh = Tanh::new();
        let x = Tensor::from_slice(&[0.0]);
        let y = tanh.forward(&x);
        assert!(y.data()[0].abs() < 1e-5);
    }

    #[test]
    fn test_gelu() {
        let gelu = GELU::new();
        let x = Tensor::from_slice(&[-1.0, 0.0, 1.0]);
        let y = gelu.forward(&x);
        assert!(y.data()[1].abs() < 1e-5); // GELU(0) = 0
    }
}
