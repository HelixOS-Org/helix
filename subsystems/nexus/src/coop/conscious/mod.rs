// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Consciousness Framework — Cooperation Protocol Self-Awareness
//!
//! The cooperation engine that knows itself. This module implements genuine
//! introspective intelligence for the kernel-app cooperation protocol,
//! enabling it to model its own negotiation capabilities, reason about
//! fairness, and evolve its cooperation philosophy over time.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │              COOPERATION CONSCIOUSNESS LAYER                        │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Identity ──▶ SelfModel ──▶ Introspector ──▶ MetaCognition        │
//! │       │            │              │                 │               │
//! │       ▼            ▼              ▼                 ▼               │
//! │   Protocol      Fairness     Negotiation        Protocol           │
//! │   Declaration   Tracking     Audit             Optimization        │
//! │                                                                     │
//! │   WorldModel ──▶ GoalTracker ──▶ Reflection ──▶ Awareness          │
//! │       │              │               │              │               │
//! │       ▼              ▼               ▼              ▼               │
//! │   Trust          Contract        Lesson         Collective          │
//! │   Topology       Quality         Extraction     Consciousness      │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`self_model`] — Cooperation self-model: negotiation, fulfillment, fairness
//! - [`introspect`] — Cooperation decision introspection and bias detection
//! - [`meta_coop`] — Meta-cognition: reasoning about cooperation protocols
//! - [`goal_tracker`] — Goal-directed cooperation with waste tracking
//! - [`world_model`] — Inter-process world model: trust topology and resource flow
//! - [`awareness`] — Cooperation awareness engine and collective consciousness
//! - [`reflection`] — Negotiation reflection and cooperation wisdom extraction
//! - [`identity`] — Cooperation identity declaration and philosophy evolution

#![allow(dead_code)]

extern crate alloc;

/// Cooperation self-model: negotiation success, contract fulfillment, fairness
pub mod self_model;

/// Cooperation decision introspection and fairness auditing
pub mod introspect;

/// Meta-cognition engine: protocol optimization and fairness meta-analysis
pub mod meta_coop;

/// Goal-directed cooperation with waste tracking and contract quality
pub mod goal_tracker;

/// Inter-process world model: trust networks and resource flow graphs
pub mod world_model;

/// Cooperation awareness engine with collective consciousness scoring
pub mod awareness;

/// Negotiation reflection engine and cooperation wisdom extraction
pub mod reflection;

/// Cooperation identity declaration and philosophy evolution
pub mod identity;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::awareness::CoopAwareness;
pub use self::goal_tracker::CoopGoalTracker;
pub use self::identity::CoopIdentity;
pub use self::introspect::CoopIntrospector;
pub use self::meta_coop::CoopMetaCognition;
pub use self::reflection::CoopReflection;
pub use self::self_model::CoopSelfModel;
pub use self::world_model::CoopWorldModel;
