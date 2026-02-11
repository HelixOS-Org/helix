// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Cooperation Future Prediction Engine
//!
//! Predicts, simulates, and validates future cooperation scenarios within
//! the kernel intelligence framework. Provides horizon forecasting, Monte Carlo
//! sampling, proactive optimization, timeline projection, negotiation rehearsal,
//! and prediction validation for cooperative subsystem interactions.

pub mod anomaly_forecast;
pub mod causal_forecast;
pub mod confidence_interval;
pub mod counterfactual;
pub mod ensemble;
pub mod horizon;
pub mod monte_carlo;
pub mod precognition;
pub mod proactive;
pub mod rehearsal;
pub mod scenario_tree;
pub mod simulator;
pub mod temporal_fusion;
pub mod timeline;
pub mod validator;

pub use anomaly_forecast::{AnomalyForecastStats, CoopAnomalyForecast};
pub use causal_forecast::{CausalForecastStats, CoopCausalForecast};
pub use confidence_interval::{ConfidenceIntervalStats, CoopConfidenceInterval};
pub use counterfactual::{CoopCounterfactual, CounterfactualStats};
pub use ensemble::{CoopEnsemble, EnsembleStats};
pub use horizon::{CoopHorizonPredictor, HorizonStats};
pub use monte_carlo::{CoopMonteCarlo, MonteCarloStats};
pub use precognition::{CoopPrecognition, PrecognitionStats};
pub use proactive::{CoopProactive, ProactiveStats};
pub use rehearsal::{CoopRehearsal, RehearsalStats};
pub use scenario_tree::{CoopScenarioTree, ScenarioTreeStats};
pub use simulator::{CoopSimulator, SimulatorStats};
pub use temporal_fusion::{CoopTemporalFusion, TemporalFusionStats};
pub use timeline::{CoopTimeline, TimelineStats};
pub use validator::{CoopPredictionValidator, ValidatorStats};
