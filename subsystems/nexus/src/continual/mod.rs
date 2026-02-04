//! # Continual Learning Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary lifelong learning system that enables
//! the kernel to continuously learn from experience without catastrophic forgetting.
//!
//! ## Key Features
//!
//! - **Elastic Weight Consolidation (EWC)**: Protects important weights
//! - **Progressive Neural Networks**: Lateral connections for transfer
//! - **Memory Replay**: Experience replay with prioritization
//! - **Synaptic Intelligence**: Online importance estimation
//! - **PackNet**: Network pruning for task-specific subnetworks
//! - **Meta-Continual Learning**: Learning to learn continuously
//!
//! ## Kernel Applications
//!
//! - Adapt to new workloads without forgetting old ones
//! - Transfer learning between kernel components
//! - Online adaptation to hardware changes
//! - Continuous security policy learning

#![no_std]

// Module declarations
mod ewc;
mod gem;
mod kernel;
mod manager;
mod memory;
mod packnet;
mod progressive;
mod si;
mod task;
mod types;
mod utils;

// Re-export all public types
pub use ewc::{EwcLearner, FisherInformation};
pub use gem::GemConstraint;
pub use kernel::{KernelContinualLearner, KernelLearningTask};
pub use manager::{ContinualConfig, ContinualHistory, ContinualLearningManager, ContinualSummary};
pub use memory::{BufferStats, MemoryBuffer, MemorySample, ReplayConfig};
pub use packnet::{PackNet, PruningMask};
pub use progressive::{LateralAdapter, ProgressiveColumn, ProgressiveNetwork};
pub use si::SynapticIntelligence;
pub use task::Task;
pub use types::ContinualStrategy;
