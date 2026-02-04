//! # Neural Architecture Search (NAS) for Kernel Decision Making
//!
//! Automatic discovery of optimal neural network topologies for kernel-level
//! AI decisions. This revolutionary module uses evolutionary algorithms and
//! differentiable architecture search to find optimal network structures.
//!
//! ## Features
//!
//! - **DARTS (Differentiable Architecture Search)**: Gradient-based topology search
//! - **ENAS (Efficient NAS)**: Parameter sharing for fast architecture evaluation
//! - **Neural Topology Evolution**: Evolutionary search for network structures
//! - **Once-For-All Networks**: Train once, deploy many architectures
//! - **Hardware-Aware NAS**: Optimize for kernel execution constraints
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    NEURAL ARCHITECTURE SEARCH                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐    │
//! │  │                   SEARCH SPACE                                   │    │
//! │  │   Operations: [Conv, Linear, ReLU, Skip, Pool, Attention]       │    │
//! │  │   Connections: Dense, Sparse, Residual                          │    │
//! │  │   Depth: 2-32 layers                                            │    │
//! │  │   Width: 8-512 neurons                                          │    │
//! │  └─────────────────────────────────────────────────────────────────┘    │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐    │
//! │  │                   SEARCH ALGORITHM                               │    │
//! │  │                                                                  │    │
//! │  │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │    │
//! │  │   │  DARTS   │  │   ENAS   │  │ Evolution│  │    OFA   │       │    │
//! │  │   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │    │
//! │  │        └─────────────┴─────────────┴─────────────┘             │    │
//! │  │                              │                                  │    │
//! │  │                              ▼                                  │    │
//! │  │                    ┌───────────────────┐                        │    │
//! │  │                    │ Architecture Pool │                        │    │
//! │  │                    └───────────────────┘                        │    │
//! │  └─────────────────────────────────────────────────────────────────┘    │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐    │
//! │  │                   EVALUATION & SELECTION                         │    │
//! │  │   Accuracy + Latency + Memory → Pareto Optimal                  │    │
//! │  └─────────────────────────────────────────────────────────────────┘    │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

// Module declarations
mod architecture;
mod cell;
mod constraints;
mod search;
mod types;

// Re-export all public types
pub use architecture::{Architecture, ArchitectureMetrics};
pub use cell::Cell;
pub use constraints::ArchitectureConstraints;
pub use search::{
    ArchitectureEncoding, KernelNas, KernelNasTask, NasConfig, NasEngine, NasStats,
    SearchHistoryEntry, SearchSpace, Supernet,
};
pub use types::OperationType;
