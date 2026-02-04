//! Activation functions for neural network neurons.

/// Activation functions supported by neurons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationFunction {
    /// Identity function: f(x) = x
    Identity,
    /// Sigmoid: f(x) = 1 / (1 + e^(-x))
    Sigmoid,
    /// Hyperbolic tangent: f(x) = tanh(x)
    Tanh,
    /// Rectified Linear Unit: f(x) = max(0, x)
    ReLU,
    /// Leaky ReLU: f(x) = max(0.01x, x)
    LeakyReLU,
    /// Exponential Linear Unit
    ELU,
    /// Gaussian: f(x) = e^(-x^2)
    Gaussian,
    /// Sine function for HyperNEAT CPPNs
    Sine,
    /// Absolute value for CPPNs
    Abs,
    /// Step function: f(x) = x > 0 ? 1 : 0
    Step,
}

impl ActivationFunction {
    /// Apply the activation function
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::Identity => x,
            Self::Sigmoid => 1.0 / (1.0 + libm::exp(-x)),
            Self::Tanh => libm::tanh(x),
            Self::ReLU => {
                if x > 0.0 {
                    x
                } else {
                    0.0
                }
            },
            Self::LeakyReLU => {
                if x > 0.0 {
                    x
                } else {
                    0.01 * x
                }
            },
            Self::ELU => {
                if x > 0.0 {
                    x
                } else {
                    libm::exp(x) - 1.0
                }
            },
            Self::Gaussian => libm::exp(-x * x),
            Self::Sine => libm::sin(x),
            Self::Abs => libm::fabs(x),
            Self::Step => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            },
        }
    }

    /// Get a random activation function
    pub fn random(seed: u64) -> Self {
        match seed % 10 {
            0 => Self::Identity,
            1 => Self::Sigmoid,
            2 => Self::Tanh,
            3 => Self::ReLU,
            4 => Self::LeakyReLU,
            5 => Self::ELU,
            6 => Self::Gaussian,
            7 => Self::Sine,
            8 => Self::Abs,
            _ => Self::Step,
        }
    }
}
