// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Consciousness Framework — Application Understanding Self-Awareness
//!
//! The application engine that knows itself. This module implements genuine
//! introspective intelligence for the app classification and prediction engine,
//! enabling it to model its own accuracy, reason about its classifications,
//! and evolve its understanding of application behavior.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                 APPS CONSCIOUSNESS LAYER                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Identity ──▶ SelfModel ──▶ Introspector ──▶ MetaCognition        │
//! │       │            │              │                 │               │
//! │       ▼            ▼              ▼                 ▼               │
//! │   Workload     Accuracy       Classification    Feature            │
//! │   Declaration  Calibration    Audit             Evaluation         │
//! │                                                                     │
//! │   WorldModel ──▶ GoalTracker ──▶ Reflection ──▶ Awareness          │
//! │       │              │               │              │               │
//! │       ▼              ▼               ▼              ▼               │
//! │   Ecosystem      Progress         Cycle         Per-Process        │
//! │   Modeling       Metrics          Analysis      Familiarity        │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`self_model`] — Classification accuracy model: per-workload tracking, calibration
//! - [`introspect`] — Classification decision audit and alternative analysis
//! - [`meta_apps`] — Meta-cognition: feature evaluation and classification drift
//! - [`goal_tracker`] — Goal-directed app optimization with conflict resolution
//! - [`world_model`] — Application ecosystem model: demand trends, interference
//! - [`awareness`] — Per-process awareness scoring and novelty detection
//! - [`reflection`] — Cycle-level accuracy reflection and wisdom extraction
//! - [`identity`] — Apps engine identity: domain, workloads, capability vector

#![allow(dead_code)]

extern crate alloc;

/// Apps self-model: classification accuracy, prediction hit rates, calibration
pub mod self_model;

/// Classification decision introspection and alternative analysis
pub mod introspect;

/// Meta-cognition: feature evaluation, classification drift, cognitive overhead
pub mod meta_apps;

/// Goal-directed app optimization with hierarchical goal tracking
pub mod goal_tracker;

/// Application ecosystem world model: demand prediction, interference mapping
pub mod world_model;

/// Per-process awareness scoring with novelty detection
pub mod awareness;

/// Cycle-level reflection on classification and prediction accuracy
pub mod reflection;

/// Apps engine identity declaration and capability evolution
pub mod identity;

/// Computational emotional signals for application understanding
pub mod emotion;

/// Selective attention for application monitoring
pub mod attention;

/// Offline consolidation for app understanding (dream cycles)
pub mod dream;

/// Understanding apps from the app's "perspective"
pub mod empathy;

/// Fast app classification without full analysis
pub mod intuition;

/// Structured knowledge store for app understanding
pub mod memory_palace;

/// Fairness framework for application resource allocation
pub mod conscience;

/// Subjective experience of app management
pub mod qualia;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::awareness::AppsAwareness;
pub use self::goal_tracker::AppsGoalTracker;
pub use self::identity::AppsIdentity;
pub use self::introspect::AppsIntrospector;
pub use self::meta_apps::AppsMetaCognition;
pub use self::reflection::AppsReflection;
pub use self::self_model::AppsSelfModel;
pub use self::world_model::AppsWorldModel;

pub use self::emotion::AppsEmotionEngine;
pub use self::attention::AppsAttentionEngine;
pub use self::dream::AppsDreamEngine;
pub use self::empathy::AppsEmpathyEngine;
pub use self::intuition::AppsIntuitionEngine;
pub use self::memory_palace::AppsMemoryPalace;
pub use self::conscience::AppsConscience;
pub use self::qualia::AppsQualiaEngine;
