//! NEXUS Year 2: Neural Network Module
//!
//! Pure Rust, no_std compatible neural network primitives for
//! kernel-native AI inference and lightweight training.
//!
//! # Submodules
//!
//! - `tensor`: Tensor data structures and operations
//! - `layers`: Neural network layers (dense, conv, attention)
//! - `activation`: Activation functions
//! - `network`: Network composition and execution
//! - `inference`: Optimized inference engine

extern crate alloc;

pub mod activation;
pub mod inference;
pub mod layers;
pub mod network;
pub mod tensor;

// Re-export key types
pub use activation::{Activation, GELU, ReLU, Sigmoid, Softmax, Tanh};
pub use inference::{InferenceConfig, InferenceEngine, InferenceResult};
pub use layers::{Conv1D, DenseLayer, Dropout, Layer, LayerNorm, MaxPool1D};
pub use network::{NetworkBuilder, NetworkConfig, Sequential};
pub use tensor::{Tensor, TensorShape, TensorView};
