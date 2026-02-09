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

pub use horizon::{CoopHorizonPredictor, HorizonStats};
pub use monte_carlo::{CoopMonteCarlo, MonteCarloStats};
pub use proactive::{CoopProactive, ProactiveStats};
pub use rehearsal::{CoopRehearsal, RehearsalStats};
pub use simulator::{CoopSimulator, SimulatorStats};
pub use timeline::{CoopTimeline, TimelineStats};
pub use validator::{CoopPredictionValidator, ValidatorStats};
