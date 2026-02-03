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

#![no_std]

extern crate alloc;

pub mod tensor;
pub mod layers;
pub mod activation;
pub mod network;
pub mod inference;

// Re-export key types
pub use tensor::{
    Tensor, TensorShape, TensorView,
};

pub use layers::{
    Layer, DenseLayer, LayerNorm, Dropout,
    Conv1D, MaxPool1D,
};

pub use activation::{
    Activation, ReLU, Sigmoid, Tanh, Softmax, GeLU,
};

pub use network::{
    NeuralNetwork, NetworkBuilder, Forward,
    LayerConfig, NetworkConfig,
};

pub use inference::{
    InferenceEngine, InferenceConfig, InferenceResult,
    BatchInference,
};
