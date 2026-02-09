// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Autonomous Research Engine — Bridge Self-Research
//!
//! The bridge that researches itself. This module implements a complete
//! autonomous research engine that discovers, tests, validates, and
//! synthesizes new optimization strategies for the kernel-userland bridge.
//! Instead of relying solely on hand-tuned heuristics, the bridge formulates
//! hypotheses, runs controlled experiments, validates results, and publishes
//! its findings into a living knowledge base.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │               BRIDGE AUTONOMOUS RESEARCH ENGINE                     │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Explorer ──▶ Hypothesis ──▶ Experiment ──▶ Validator             │
//! │      │              │              │              │                 │
//! │      ▼              ▼              ▼              ▼                 │
//! │   Genetic       Anomaly        A/B Test       Regression           │
//! │   Search        Detection      χ² / t-test    Reproducibility      │
//! │                                                                     │
//! │   Journal ──▶ Literature ──▶ Synthesis                             │
//! │      │              │              │                                │
//! │      ▼              ▼              ▼                                │
//! │   Publish       Novelty       Strategy                             │
//! │   Reports       Detection     Generation                           │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`explorer`] — Autonomous algorithmic exploration via genetic search
//! - [`hypothesis`] — Hypothesis generation from observed anomalies/patterns
//! - [`experiment`] — Controlled A/B experimentation with statistical testing
//! - [`validator`] — Discovery validation: regression, safety, reproducibility
//! - [`journal`] — Research journal and publication engine
//! - [`literature`] — Internal literature review and knowledge base
//! - [`synthesis`] — Strategy synthesis from validated discoveries

#![allow(dead_code)]

extern crate alloc;

/// Autonomous algorithmic exploration via genetic search over strategy params
pub mod explorer;

/// Hypothesis generation from observed bridge anomalies and patterns
pub mod hypothesis;

/// Controlled A/B experimentation with chi-squared and t-test
pub mod experiment;

/// Discovery validation: performance regression, safety, reproducibility
pub mod validator;

/// Research journal recording all experiments, discoveries, and validations
pub mod journal;

/// Internal literature review and novelty detection knowledge base
pub mod literature;

/// Strategy synthesis from validated research discoveries
pub mod synthesis;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::experiment::BridgeExperiment;
pub use self::explorer::BridgeExplorer;
pub use self::hypothesis::BridgeHypothesisEngine;
pub use self::journal::BridgeJournal;
pub use self::literature::BridgeLiterature;
pub use self::synthesis::BridgeSynthesis;
pub use self::validator::BridgeDiscoveryValidator;

/// Statistical analysis of bridge research results
pub mod analysis;

/// Proactive curiosity-driven exploration of syscall optimization space
pub mod curiosity;

/// Cross-validation of research findings via peer review
pub mod peer_review;

/// Persistent knowledge store for bridge research
pub mod knowledge_base;

/// Breakthrough detection for genuine discoveries
pub mod breakthrough;

/// Research methodology validation framework
pub mod methodology;

/// Result replication engine for reproducibility
pub mod replication;

/// Paradigm shift detection and transition management
pub mod paradigm;

pub use self::analysis::BridgeAnalysisEngine;
pub use self::breakthrough::BridgeBreakthroughDetector;
pub use self::curiosity::BridgeCuriosityEngine;
pub use self::knowledge_base::BridgeKnowledgeBase;
pub use self::methodology::BridgeMethodology;
pub use self::paradigm::BridgeParadigm;
pub use self::peer_review::BridgePeerReview;
pub use self::replication::BridgeReplication;
