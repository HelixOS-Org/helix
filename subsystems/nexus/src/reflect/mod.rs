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
pub mod calibrator;
pub mod diagnostician;
pub mod domain;
pub mod evolver;
pub mod insight;
pub mod introspector;
pub mod metrics;

// Re-exports - Metrics
// Re-exports - Calibrator
pub use calibrator::{
    CalibrationRecommendation, CalibrationReport, Calibrator, CalibratorStats, DecisionOutcome,
    DecisionRecord, PredictionOutcome, PredictionRecord,
};
// Re-exports - Diagnostician
pub use diagnostician::{
    CognitiveFailure, Diagnosis, Diagnostician, DiagnosticianStats, FailurePattern, FailureType,
    PatternType, RootCause,
};
// Re-exports - Domain
pub use domain::{ReflectConfig, ReflectDomain, ReflectError, ReflectStats};
// Re-exports - Evolver
pub use evolver::{
    AppliedImprovement, EffortLevel, Evolver, EvolverStats, ImprovementCategory,
    ImprovementSuggestion, SuggestionStatus,
};
// Re-exports - Insight
pub use insight::{Insight, InsightBatch, InsightType};
// Re-exports - Introspector
pub use introspector::{
    CognitiveHealth, CognitiveIssue, CognitiveStatus, DomainHealth, Introspector,
    IntrospectorStats, IssueType,
};
pub use metrics::{CognitiveMetrics, DomainMetrics};

pub use crate::types::{DecisionId, FailureId, InsightId, IssueId, PredictionId, SuggestionId};
