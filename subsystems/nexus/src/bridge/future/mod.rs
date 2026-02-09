// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Future Prediction Engine — Bridge Long-Horizon Prediction
//!
//! The bridge that sees tomorrow. This module implements a complete future
//! prediction engine for the kernel-userland syscall bridge, enabling it to
//! anticipate syscall patterns, simulate scenarios, and proactively optimize
//! resource allocation before demand materializes.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                  BRIDGE FUTURE PREDICTION ENGINE                     │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Horizon ──▶ Timeline ──▶ Simulator ──▶ MonteCarlo               │
//! │      │            │            │              │                     │
//! │      ▼            ▼            ▼              ▼                     │
//! │   Multi-Scale   Markov      Scenario      Statistical              │
//! │   TimeSeries    Chains      Branching     Sampling                 │
//! │                                                                     │
//! │   Proactive ──▶ Rehearsal ──▶ Validator                            │
//! │      │              │              │                                │
//! │      ▼              ▼              ▼                                │
//! │   Pre-alloc     Dry-run       Brier Score                          │
//! │   Pre-warm      Bottleneck    Calibration                          │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`horizon`] — Multi-scale long-horizon syscall prediction (1s → 1h)
//! - [`simulator`] — Future syscall scenario simulation with branching
//! - [`proactive`] — Proactive optimization: pre-alloc, pre-warm, pre-compute
//! - [`timeline`] — Markov-chain syscall timeline projection per process
//! - [`rehearsal`] — Dry-run rehearsal engine for bottleneck identification
//! - [`monte_carlo`] — Monte Carlo simulation for statistical future analysis
//! - [`validator`] — Prediction validation: Brier score, calibration, sharpness

#![allow(dead_code)]

extern crate alloc;

/// Multi-scale long-horizon syscall prediction (1s, 1min, 10min, 1h buckets)
pub mod horizon;

/// Future syscall scenario simulation with branching state
pub mod simulator;

/// Proactive bridge optimization: pre-allocate, pre-warm, pre-compute
pub mod proactive;

/// Markov-chain syscall timeline projection per process
pub mod timeline;

/// Dry-run rehearsal engine for bottleneck identification
pub mod rehearsal;

/// Monte Carlo simulation for statistical syscall future analysis
pub mod monte_carlo;

/// Prediction validation: Brier score, calibration curve, sharpness
pub mod validator;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::horizon::BridgeHorizonPredictor;
pub use self::monte_carlo::BridgeMonteCarlo;
pub use self::proactive::BridgeProactive;
pub use self::rehearsal::BridgeRehearsal;
pub use self::simulator::BridgeSimulator;
pub use self::timeline::BridgeTimeline;
pub use self::validator::BridgePredictionValidator;
