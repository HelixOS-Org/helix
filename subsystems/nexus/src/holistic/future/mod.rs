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

/// System-wide scenario tree — predicts the entire system's future as branching states
pub mod scenario_tree;

/// System-wide causal prediction — every event's cause and effect across ALL subsystems
pub mod causal_forecast;

/// System-wide counterfactual reasoning — "what if the entire system had done X?"
pub mod counterfactual;

/// System-wide confidence intervals — uncertainty quantification for every prediction
pub mod confidence_interval;

/// Master ensemble — combines ALL prediction models from ALL subsystems
pub mod ensemble;

/// System-wide multi-horizon temporal fusion — microseconds to hours, coherent view
pub mod temporal_fusion;

/// System-wide anomaly forecasting — cascading failures, performance cliffs, risk
pub mod anomaly_forecast;

/// System-wide pre-cognitive sensing — regime changes, phase transitions, paradigm shifts
pub mod precognition;

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
pub use self::scenario_tree::{
    ExpectedSystemState, HolisticScenarioTree, PruningReport, PruningStrategy, ScenarioNode,
    ScenarioTreeStats, StateTransition, SystemStateVector, TreePath,
};
pub use self::causal_forecast::{
    CausalCascade, CausalCompleteness, CausalEdge, CausalForecastStats, CausalNode,
    CrossSubsystemCausality, HolisticCausalForecast, InterventionPlan, RootCauseChain,
};
pub use self::counterfactual::{
    AlternativeTimeline, CounterfactualCascade, CounterfactualStats, DecisionQualityReport,
    GlobalRegret, HolisticCounterfactual, OptimalHistory, WhatIfQuery, WhatIfResult,
};
pub use self::confidence_interval::{
    CalibrationAudit, ConfidenceIntervalStats, GlobalUncertainty, HolisticConfidenceInterval,
    IntervalReliability, PredictionCI, UncertaintyBudget, UncertaintyPropagation,
};
pub use self::ensemble::{
    CrossSubsystemFusion, EnsembleDominance, EnsembleStats, HolisticEnsemble,
    MasterEnsembleResult, MetaAccuracy, ModelSelection,
};
pub use self::temporal_fusion::{
    FusedHorizonPrediction, FusionPanorama, HolisticTemporalFusion, LongRangeForecast,
    MicroToMacroBridge, SystemTemporalFusion, TemporalFusionStats, TemporalHorizon,
};
pub use self::anomaly_forecast::{
    AnomalyForecastStats, CascadePrediction, CliffWarning, EarlySystemWarning,
    HolisticAnomalyForecast, PreventionStrategy, SystemAnomalyForecast, SystemicRisk,
};
pub use self::precognition::{
    HolisticPrecognition, ParadigmShiftAlert, PhaseTransitionSense, PrecogAccuracyReport,
    PrecognitionStats, RegimeChangeDetection, TranscendentForesight,
};
