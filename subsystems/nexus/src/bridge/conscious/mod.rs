// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Consciousness Framework — Bridge Self-Awareness
//!
//! The bridge that knows itself. This module implements genuine introspective
//! intelligence for the kernel-userland syscall bridge, enabling it to model
//! its own capabilities, reason about its decisions, and evolve its identity.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                   BRIDGE CONSCIOUSNESS LAYER                        │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   Identity ──▶ SelfModel ──▶ Introspector ──▶ MetaCognition        │
//! │       │            │              │                 │               │
//! │       ▼            ▼              ▼                 ▼               │
//! │   Capability    Limitation    Decision          Attention           │
//! │   Declaration   Assessment   Analysis          Allocation          │
//! │                                                                     │
//! │   WorldModel ──▶ GoalTracker ──▶ Reflection ──▶ Awareness          │
//! │       │              │               │              │               │
//! │       ▼              ▼               ▼              ▼               │
//! │   Environment    Progress         Insight       Consciousness      │
//! │   Prediction     Metrics          Extraction    State Machine       │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`self_model`] — Complete self-model: capabilities, limitations, confidence
//! - [`introspect`] — Decision process analysis and bias detection
//! - [`meta_bridge`] — Meta-cognition: thinking about bridge thinking
//! - [`goal_tracker`] — Goal-directed optimization with conflict resolution
//! - [`world_model`] — Environmental model of the OS world
//! - [`awareness`] — Self-awareness state machine (Dormant → Transcendent)
//! - [`reflection`] — Performance reflection and wisdom extraction
//! - [`identity`] — Bridge identity declaration and capability evolution

#![allow(dead_code)]

extern crate alloc;

/// Bridge self-model: capabilities, limitations, and confidence intervals
pub mod self_model;

/// Decision process introspection and reasoning chain analysis
pub mod introspect;

/// Meta-cognition engine: attention allocation and blind spot detection
pub mod meta_bridge;

/// Goal-directed optimization with hierarchical goal tracking
pub mod goal_tracker;

/// Environmental world model: OS state prediction and surprise detection
pub mod world_model;

/// Self-awareness state machine with continuous consciousness scoring
pub mod awareness;

/// Performance reflection engine and wisdom extraction
pub mod reflection;

/// Bridge identity declaration and capability evolution tracking
pub mod identity;

/// Computational emotional signals for bridge routing decisions
pub mod emotion;

/// Selective attention scheduling for bridge monitoring
pub mod attention;

/// Offline consolidation and dream-based pattern discovery
pub mod dream;

/// Subsystem empathy: inferring other subsystems' internal states
pub mod empathy;

/// Fast-path intuition: cached pattern matching without full analysis
pub mod intuition;

/// Structured memory organization with spaced repetition
pub mod memory_palace;

/// Ethical decision framework ensuring fairness and no starvation
pub mod conscience;

/// Subjective experience representation for bridge operations
pub mod qualia;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::awareness::BridgeAwareness;
pub use self::goal_tracker::BridgeGoalTracker;
pub use self::identity::BridgeIdentity;
pub use self::introspect::BridgeIntrospector;
pub use self::meta_bridge::BridgeMetaCognition;
pub use self::reflection::BridgeReflection;
pub use self::self_model::BridgeSelfModel;
pub use self::world_model::BridgeWorldModel;

pub use self::emotion::BridgeEmotionEngine;
pub use self::attention::BridgeAttentionEngine;
pub use self::dream::BridgeDreamEngine;
pub use self::empathy::BridgeEmpathyEngine;
pub use self::intuition::BridgeIntuitionEngine;
pub use self::memory_palace::BridgeMemoryPalace;
pub use self::conscience::BridgeConscience;
pub use self::qualia::BridgeQualiaEngine;
