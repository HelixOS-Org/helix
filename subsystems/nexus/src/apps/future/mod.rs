// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Future Prediction Engine — Application Behavior Long-Horizon Prediction
//!
//! The application engine that sees tomorrow. This module implements a complete
//! future prediction engine for application behavior, enabling the kernel to
//! anticipate resource needs, simulate lifecycle trajectories, and proactively
//! optimize allocation before demand materializes.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                  APPS FUTURE PREDICTION ENGINE                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Horizon ──▶ Timeline ──▶ Simulator ──▶ MonteCarlo               │
//! │      │            │            │              │                     │
//! │      ▼            ▼            ▼              ▼                     │
//! │   Multi-Scale   Lifecycle   Interaction   Workload                 │
//! │   Resource      Phase       Modeling      Distribution             │
//! │   Forecast      Projection                Sampling                 │
//! │                                                                     │
//! │   Proactive ──▶ Rehearsal ──▶ Validator                            │
//! │      │              │              │                                │
//! │      ▼              ▼              ▼                                │
//! │   Pre-scale     What-if       MAPE &                               │
//! │   Pre-classify  Dry-run       Directional                          │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`horizon`] — Multi-scale long-horizon app behavior prediction (seconds → hours)
//! - [`simulator`] — Future app behavior simulation with lifecycle and interaction models
//! - [`proactive`] — Proactive app optimization: pre-scale, pre-classify, demand spike detection
//! - [`timeline`] — App lifecycle timeline projection: startup → steady → peak → exit
//! - [`rehearsal`] — Dry-run rehearsal engine for hypothetical scenario analysis
//! - [`monte_carlo`] — Monte Carlo simulation for workload distribution futures
//! - [`validator`] — Prediction validation: MAPE, directional accuracy, bias, recalibration

#![allow(dead_code)]

extern crate alloc;

/// Multi-scale long-horizon app behavior prediction (1s, 1min, 10min, 1h)
pub mod horizon;

/// Future app behavior simulation with lifecycle modeling
pub mod simulator;

/// Proactive app optimization: pre-scale, pre-classify, demand spike detection
pub mod proactive;

/// App lifecycle timeline projection: startup → steady-state → peak → exit
pub mod timeline;

/// Dry-run rehearsal engine for hypothetical scenario analysis
pub mod rehearsal;

/// Monte Carlo simulation for workload distribution futures
pub mod monte_carlo;

/// Prediction validation: MAPE, directional accuracy, bias detection
pub mod validator;

/// Branching scenario trees for app behavior futures
pub mod scenario_tree;

/// Causal prediction of app behavior: why apps behave as they do
pub mod causal_forecast;

/// Counterfactual "what if" analysis for resource allocation decisions
pub mod counterfactual;

/// Uncertainty quantification and confidence intervals for app predictions
pub mod confidence_interval;

/// Multi-model ensemble for app behavior prediction
pub mod ensemble;

/// Multi-horizon temporal fusion for coherent app forecasts
pub mod temporal_fusion;

/// Anomaly forecasting: predict app crashes, hangs, thrashing before they happen
pub mod anomaly_forecast;

/// Pre-cognitive behavioral shift sensing below anomaly thresholds
pub mod precognition;

// Re-exports for convenience
pub use horizon::AppsHorizonPredictor;
pub use monte_carlo::AppsMonteCarlo;
pub use proactive::AppsProactive;
pub use rehearsal::AppsRehearsal;
pub use simulator::AppsSimulator;
pub use timeline::AppsTimeline;
pub use validator::AppsPredictionValidator;
pub use scenario_tree::AppsScenarioTree;
pub use causal_forecast::AppsCausalForecast;
pub use counterfactual::AppsCounterfactual;
pub use confidence_interval::AppsConfidenceInterval;
pub use ensemble::AppsEnsemble;
pub use temporal_fusion::AppsTemporalFusion;
pub use anomaly_forecast::AppsAnomalyForecast;
pub use precognition::AppsPrecognition;
