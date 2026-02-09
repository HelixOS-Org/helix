// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Future Prediction Engine — Holistic System-Wide Future Prediction
//!
//! **The MASTER prediction engine.** This module integrates every prediction
//! capability — bridge, application, cooperative — into a single, unified
//! future-prediction framework that operates on the WHOLE system at once.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │              HOLISTIC FUTURE — THE MASTER PREDICTION ENGINE             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │   Horizon ────▶ Simulator ────▶ Timeline ────▶ MonteCarlo              │
//! │      │               │              │               │                   │
//! │      ▼               ▼              ▼               ▼                   │
//! │   Multi-scale     Full OS        Temporal        Stochastic             │
//! │   Fused            State          Branch &       Probability            │
//! │   Forecasts       Simulation     Correction     Distributions           │
//! │                                                                         │
//! │   Proactive ────▶ Rehearsal ────▶ Validator                            │
//! │      │               │               │                                  │
//! │      ▼               ▼               ▼                                  │
//! │   Anticipatory    Decision        Global                                │
//! │   Cross-domain    Impact &        Accuracy &                            │
//! │   Optimization    Risk Eval       Recalibration                         │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`horizon`] — System-wide long-horizon prediction at 1s/1m/10m/1h scales
//! - [`simulator`] — Full system simulation: CPU, memory, I/O, network, processes
//! - [`proactive`] — System-wide proactive optimization and cascade prevention
//! - [`timeline`] — Temporal projection with branching, merging, and correction
//! - [`rehearsal`] — Decision rehearsal: impact assessment, risk, counterfactuals
//! - [`monte_carlo`] — Full system Monte Carlo: failure probability, distributions
//! - [`validator`] — System-wide prediction validation and recalibration

#![allow(dead_code)]

extern crate alloc;

/// System-wide long-horizon prediction — fused forecasts at multiple time scales
pub mod horizon;

/// Full system simulation engine — models entire OS state evolution
pub mod simulator;

/// System-wide proactive optimization — anticipatory cross-domain actions
pub mod proactive;

/// Temporal projection — timeline branching, merging, and course correction
pub mod timeline;

/// Decision rehearsal — simulates impact and risk before execution
pub mod rehearsal;

/// Full system Monte Carlo — stochastic sampling of entire system futures
pub mod monte_carlo;

/// System-wide prediction validation — accuracy, systematic errors, recalibration
pub mod validator;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::horizon::{
    HolisticHorizonPredictor, HorizonDecomposition, HorizonScale, HorizonStats,
    SystemStatePrediction,
};
pub use self::monte_carlo::{
    ConfidenceBounds, ExhaustionEstimate, FailureMode, HolisticMonteCarlo, MonteCarloStats,
    PerformanceDistribution, ResourceKind,
};
pub use self::proactive::{
    ActionDomain, CascadePreventionRecord, HolisticProactive, ProactiveAction, ProactiveStats,
    Urgency,
};
pub use self::rehearsal::{
    CounterfactualResult, DecisionKind, HolisticRehearsal, ImpactAssessment, Recommendation,
    RehearsalRecommendation, RehearsalStats, RiskCategory, RiskEvaluation,
};
pub use self::simulator::{
    BottleneckPrediction, HolisticSimulator, OptimalPath, ResourceDomain, ScenarioKind,
    ScenarioResult, SimulatorStats,
};
pub use self::timeline::{
    CourseCorrection, HolisticTimeline, MergeResult, PredictedVsActual, TimelineBranch,
    TimelineStats, TimelineStatus,
};
pub use self::validator::{
    GlobalValidation, HolisticPredictionValidator, ModelSelectionSignal, PredictionDecomposition,
    RecalibrationSignal, SubsystemAccuracy, SubsystemId, SystematicError, ValidatorStats,
};
