// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Cooperation Future Prediction Engine
//!
//! Predicts, simulates, and validates future cooperation scenarios within
//! the kernel intelligence framework. Provides horizon forecasting, Monte Carlo
//! sampling, proactive optimization, timeline projection, negotiation rehearsal,
//! and prediction validation for cooperative subsystem interactions.

pub mod horizon;
pub mod monte_carlo;
pub mod proactive;
pub mod rehearsal;
pub mod simulator;
pub mod timeline;
pub mod validator;
pub mod scenario_tree;
pub mod causal_forecast;
pub mod counterfactual;
pub mod confidence_interval;
pub mod ensemble;
pub mod temporal_fusion;
pub mod anomaly_forecast;
pub mod precognition;

pub use horizon::{CoopHorizonPredictor, HorizonStats};
pub use monte_carlo::{CoopMonteCarlo, MonteCarloStats};
pub use proactive::{CoopProactive, ProactiveStats};
pub use rehearsal::{CoopRehearsal, RehearsalStats};
pub use simulator::{CoopSimulator, SimulatorStats};
pub use timeline::{CoopTimeline, TimelineStats};
pub use validator::{CoopPredictionValidator, ValidatorStats};
pub use scenario_tree::{CoopScenarioTree, ScenarioTreeStats};
pub use causal_forecast::{CoopCausalForecast, CausalForecastStats};
pub use counterfactual::{CoopCounterfactual, CounterfactualStats};
pub use confidence_interval::{CoopConfidenceInterval, ConfidenceIntervalStats};
pub use ensemble::{CoopEnsemble, EnsembleStats};
pub use temporal_fusion::{CoopTemporalFusion, TemporalFusionStats};
pub use anomaly_forecast::{CoopAnomalyForecast, AnomalyForecastStats};
pub use precognition::{CoopPrecognition, PrecognitionStats};
