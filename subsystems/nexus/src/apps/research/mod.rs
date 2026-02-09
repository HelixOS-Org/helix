// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Autonomous Research Engine — Application Understanding Research
//!
//! The application engine that researches itself. This module implements a
//! complete autonomous research engine that discovers, tests, validates, and
//! synthesizes new classification strategies, prediction features, and
//! optimization rules for the kernel's app understanding subsystem. Instead
//! of relying solely on hand-tuned heuristics, the engine formulates
//! hypotheses, runs controlled experiments, validates results, and publishes
//! its findings into a living knowledge base.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │           APPS AUTONOMOUS RESEARCH ENGINE                           │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Explorer ──▶ Hypothesis ──▶ Experiment ──▶ Validator             │
//! │      │              │              │              │                 │
//! │      ▼              ▼              ▼              ▼                 │
//! │   Genetic       Classification   A/B Test      Cross-              │
//! │   Feature       Anomaly          Welch's t     Validation          │
//! │   Search        Detection        Cohen's d     Holdout             │
//! │                                                                     │
//! │   Journal ──▶ Literature ──▶ Synthesis                             │
//! │      │              │              │                                │
//! │      ▼              ▼              ▼                                │
//! │   Publish       Novelty       Classifier                           │
//! │   Findings      Detection     Generation                           │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`explorer`] — Autonomous feature exploration via genetic search
//! - [`hypothesis`] — Hypothesis generation from classification anomalies
//! - [`experiment`] — Controlled A/B experimentation with statistical testing
//! - [`validator`] — Discovery validation: regression, cross-validation, holdout
//! - [`journal`] — Research journal and publication engine
//! - [`literature`] — Internal literature review and knowledge base
//! - [`synthesis`] — Classifier/predictor synthesis from validated discoveries

#![allow(dead_code)]

extern crate alloc;

/// Autonomous feature exploration via genetic search over classification space
pub mod explorer;

/// Hypothesis generation from observed classification anomalies and patterns
pub mod hypothesis;

/// Controlled A/B experimentation with Welch's t-test and Cohen's d
pub mod experiment;

/// Discovery validation: regression, cross-validation, holdout testing
pub mod validator;

/// Research journal recording all experiments, discoveries, and validations
pub mod journal;

/// Internal literature review and novelty detection knowledge base
pub mod literature;

/// Classifier/predictor synthesis from validated research discoveries
pub mod synthesis;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::experiment::AppsExperiment;
pub use self::explorer::AppsExplorer;
pub use self::hypothesis::AppsHypothesisEngine;
pub use self::journal::AppsJournal;
pub use self::literature::AppsLiterature;
pub use self::synthesis::AppsSynthesis;
pub use self::validator::AppsDiscoveryValidator;
