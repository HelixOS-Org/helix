//! NEXUS Reflect Domain — Meta-Cognitive Layer
//!
//! The seventh cognitive domain. REFLECT thinks about its own thinking.
//! It observes all other domains, calibrates their performance, diagnoses
//! issues in the cognitive system itself, and drives continuous improvement.
//!
//! # Philosophy
//!
//! "Penser sur sa propre pensée" — Think about one's own thinking
//!
//! REFLECT is the meta-cognitive layer that provides:
//! - Introspection into cognitive performance
//! - Calibration of confidence and predictions
//! - Diagnosis of cognitive failures
//! - Evolution of cognitive strategies
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                          REFLECT DOMAIN                                  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    FROM ALL DOMAINS                          │       │
//! │  │  (Telemetry, decisions, effects, errors, patterns)           │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    INTROSPECTOR                              │       │
//! │  │  • Monitor domain health                                     │       │
//! │  │  • Track cognitive load                                      │       │
//! │  │  • Detect anomalies in cognition                             │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    CALIBRATOR                                │       │
//! │  │  • Track prediction accuracy                                 │       │
//! │  │  • Adjust confidence thresholds                              │       │
//! │  │  • Correct systematic biases                                 │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                   DIAGNOSTICIAN                              │       │
//! │  │  • Diagnose cognitive failures                               │       │
//! │  │  • Identify bottlenecks                                      │       │
//! │  │  • Find patterns in failures                                 │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    EVOLVER                                   │       │
//! │  │  • Suggest strategy changes                                  │       │
//! │  │  • Propose parameter adjustments                             │       │
//! │  │  • Track improvement over time                               │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    OUTPUT INSIGHTS                           │       │
//! │  │  → Insight → Insight → Insight → ...                        │       │
//! │  │  To: All domains (for improvement)                           │       │
//! │  └──────────────────────────────────────────────────────────────┘       │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Structure
//!
//! - [`metrics`] - Domain and cognitive metrics
//! - [`introspector`] - Health monitoring and issue detection
//! - [`calibrator`] - Prediction and decision calibration
//! - [`diagnostician`] - Failure diagnosis and pattern detection
//! - [`evolver`] - Improvement suggestions and tracking
//! - [`insight`] - Reflection output types
//! - [`domain`] - Main orchestrator

#![allow(dead_code)]

// Submodules
pub mod metrics;
pub mod introspector;
pub mod calibrator;
pub mod diagnostician;
pub mod evolver;
pub mod insight;
pub mod domain;

// Re-exports - Metrics
pub use metrics::{
    DomainMetrics,
    CognitiveMetrics,
};

// Re-exports - Introspector
pub use introspector::{
    Introspector,
    IssueId,
    CognitiveStatus,
    IssueType,
    CognitiveIssue,
    DomainHealth,
    CognitiveHealth,
    IntrospectorStats,
};

// Re-exports - Calibrator
pub use calibrator::{
    Calibrator,
    PredictionId,
    PredictionRecord,
    PredictionOutcome,
    DecisionId,
    DecisionRecord,
    DecisionOutcome,
    CalibrationReport,
    CalibrationRecommendation,
    CalibratorStats,
};

// Re-exports - Diagnostician
pub use diagnostician::{
    Diagnostician,
    FailureId,
    FailureType,
    RootCause,
    CognitiveFailure,
    Diagnosis,
    FailurePattern,
    PatternType,
    DiagnosticianStats,
};

// Re-exports - Evolver
pub use evolver::{
    Evolver,
    SuggestionId,
    ImprovementCategory,
    EffortLevel,
    SuggestionStatus,
    ImprovementSuggestion,
    AppliedImprovement,
    EvolverStats,
};

// Re-exports - Insight
pub use insight::{
    Insight,
    InsightId,
    InsightType,
    InsightBatch,
};

// Re-exports - Domain
pub use domain::{
    ReflectDomain,
    ReflectConfig,
    ReflectStats,
    ReflectError,
};
