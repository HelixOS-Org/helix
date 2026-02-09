// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Autonomous Research Engine — Cooperation Protocol Research
//!
//! A self-improving research engine for cooperation protocols within the
//! NEXUS kernel intelligence framework. This module discovers, tests,
//! validates, and synthesizes new negotiation strategies, fairness
//! algorithms, trust models, and resource-sharing protocols. Rather than
//! relying on statically configured cooperation rules, the engine
//! formulates hypotheses, runs controlled experiments, validates
//! discoveries against safety invariants, and publishes results into a
//! living knowledge base of proven cooperation techniques.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │        COOPERATION PROTOCOL RESEARCH ENGINE                         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Explorer ──▶ Hypothesis ──▶ Experiment ──▶ Validator             │
//! │      │              │              │              │                 │
//! │      ▼              ▼              ▼              ▼                 │
//! │   Genetic       Auction vs     Controlled     Fairness             │
//! │   Protocol      Fixed-Quota    Negotiation    Starvation           │
//! │   Search        Analysis       A/B Tests      Gaming Proof         │
//! │                                                                     │
//! │   Journal ──▶ Literature ──▶ Synthesis                             │
//! │      │              │              │                                │
//! │      ▼              ▼              ▼                                │
//! │   Publish       Known-Good    Protocol                             │
//! │   Findings      Protocols     Generation                           │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`explorer`] — Genetic exploration of cooperation protocol parameters
//! - [`hypothesis`] — Hypothesis generation for cooperation improvements
//! - [`experiment`] — Controlled experimentation on negotiation algorithms
//! - [`validator`] — Discovery validation: fairness, starvation, gaming resistance
//! - [`journal`] — Research journal archiving protocol experiments
//! - [`literature`] — Cooperation knowledge base of proven strategies
//! - [`synthesis`] — Protocol synthesis from validated research discoveries

#![allow(dead_code)]

extern crate alloc;

/// Genetic exploration of cooperation protocol parameters and strategies
pub mod explorer;

/// Hypothesis generation for cooperation protocol improvements
pub mod hypothesis;

/// Controlled experimentation on negotiation and fairness algorithms
pub mod experiment;

/// Discovery validation: fairness proofs, starvation tests, gaming resistance
pub mod validator;

/// Research journal archiving all cooperation protocol experiments
pub mod journal;

/// Cooperation knowledge base of proven protocols and fairness theorems
pub mod literature;

/// Protocol synthesis from validated cooperation research discoveries
pub mod synthesis;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::experiment::CoopExperiment;
pub use self::explorer::CoopExplorer;
pub use self::hypothesis::CoopHypothesisEngine;
pub use self::journal::CoopJournal;
pub use self::literature::CoopLiterature;
pub use self::synthesis::CoopSynthesis;
pub use self::validator::CoopDiscoveryValidator;
