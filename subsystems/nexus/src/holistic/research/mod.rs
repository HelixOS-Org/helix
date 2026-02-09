// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Autonomous Research Engine — Holistic System-Wide Research & Discovery
//!
//! The MASTER research engine for the entire NEXUS kernel intelligence
//! framework. While bridge, application, and cooperation research modules
//! optimise within their domains, this holistic research engine discovers
//! cross-subsystem synergies, emergent system properties, and globally
//! coherent improvements that no isolated module can perceive.
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │          HOLISTIC SYSTEM-WIDE RESEARCH ENGINE (MASTER)               │
//! ├───────────────────────────────────────────────────────────────────────┤
//! │                                                                       │
//! │  Explorer ──▶ Hypothesis ──▶ Experiment ──▶ Validator                │
//! │     │              │              │              │                    │
//! │     ▼              ▼              ▼              ▼                    │
//! │  Multi-Obj     Cross-Module   Factorial      System-Level            │
//! │  Genetic       Span-Hypos     Interaction    Safety Proofs           │
//! │  Search        Evidence       Effect Tests   Formal Verif            │
//! │                Fusion                                                 │
//! │  Journal ──▶ Literature ──▶ Synthesis                                │
//! │     │              │              │                                   │
//! │     ▼              ▼              ▼                                   │
//! │  Cross-Ref     Unified        Staged                                 │
//! │  Impact        Knowledge      Conflict-Free                          │
//! │  Archive       Wisdom         Rollout                                │
//! │                                                                       │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`explorer`]   — System-wide algorithmic exploration with genetic search
//! - [`hypothesis`] — Cross-subsystem hypothesis generation and evidence fusion
//! - [`experiment`] — Factorial experimentation with interaction effect analysis
//! - [`validator`]  — System-level safety proofs and formal verification
//! - [`journal`]    — Cross-referenced archive of all research discoveries
//! - [`literature`] — Unified knowledge base with gap prioritisation and wisdom
//! - [`synthesis`]  — Conflict-free staged rollout of validated improvements

#![allow(dead_code)]

extern crate alloc;

/// System-wide algorithmic exploration — multi-objective genetic search
pub mod explorer;

/// Cross-subsystem hypothesis generation and evidence fusion
pub mod hypothesis;

/// Factorial experimentation with interaction effect analysis
pub mod experiment;

/// System-level discovery validation, safety proofs, and formal verification
pub mod validator;

/// Cross-referenced archive of all research discoveries across subsystems
pub mod journal;

/// Unified knowledge base with completeness scoring and wisdom extraction
pub mod literature;

/// Conflict-free staged synthesis and rollout of validated improvements
pub mod synthesis;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::experiment::HolisticExperiment;
pub use self::explorer::HolisticExplorer;
pub use self::hypothesis::HolisticHypothesisEngine;
pub use self::journal::HolisticJournal;
pub use self::literature::HolisticLiterature;
pub use self::synthesis::HolisticSynthesis;
pub use self::validator::HolisticDiscoveryValidator;
