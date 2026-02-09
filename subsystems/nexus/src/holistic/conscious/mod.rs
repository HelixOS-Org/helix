// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Consciousness Framework — Holistic System-Wide Consciousness
//!
//! **The CROWN of consciousness.** This module integrates every strand of
//! awareness — bridge, application, cooperative — into a single, unified
//! kernel consciousness. Where sub-modules see their own domains, *this*
//! module sees the WHOLE: the complete self-model, the full decision graph,
//! the entire goal hierarchy, and the total awareness state.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                  HOLISTIC CONSCIOUSNESS — THE CROWN                     │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │   Identity ────▶ SelfModel ────▶ Introspector ────▶ MetaCognition      │
//! │       │               │                │                  │             │
//! │       ▼               ▼                ▼                  ▼             │
//! │   "I am NEXUS"   Capability       Global Decision    Architecture      │
//! │   Purpose &      Matrix &         Coherence &        Evaluation &      │
//! │   Philosophy     Envelope         Optimality         Evolution Dir     │
//! │                                                                         │
//! │   WorldModel ────▶ GoalTracker ────▶ Reflection ────▶ Awareness        │
//! │       │                │                 │                │             │
//! │       ▼                ▼                 ▼                ▼             │
//! │   Unified OS       Root Goals        Wisdom          Consciousness     │
//! │   State &          Decomposition     Generation      State Machine     │
//! │   Surprise         & Pareto          & Growth        Dormant →         │
//! │   Detection        Optimality        Trajectory      Transcendent      │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`self_model`] — Unified kernel self-model across all subsystems
//! - [`introspect`] — System-wide decision audit and cross-module coherence
//! - [`meta_holistic`] — Highest-level meta-cognition and cognitive architecture
//! - [`goal_tracker`] — Root goal hierarchy with Pareto-optimal decomposition
//! - [`world_model`] — Complete OS world representation — the consciousness itself
//! - [`awareness`] — Consciousness state machine: Dormant → Transcendent
//! - [`reflection`] — System-wide reflection and wisdom generation
//! - [`identity`] — NEXUS kernel identity declaration and purpose

#![allow(dead_code)]

extern crate alloc;

/// Unified kernel self-model: all capabilities, limitations, performance envelope
pub mod self_model;

/// System-wide decision audit, cross-module coherence, and blind-spot scanning
pub mod introspect;

/// Highest-level meta-cognition: cognitive architecture evaluation and evolution
pub mod meta_holistic;

/// Root goal hierarchy with Pareto-optimal decomposition and synergy tracking
pub mod goal_tracker;

/// Complete OS world model — the unified representation that IS consciousness
pub mod world_model;

/// Consciousness state machine: Dormant → Awakening → Aware → Reflective → Enlightened → Transcendent
pub mod awareness;

/// System-wide reflection engine: wisdom generation and growth trajectory
pub mod reflection;

/// NEXUS kernel identity: purpose, philosophy, capability declaration
pub mod identity;

// ============================================================================
// RE-EXPORTS — Key types for external consumers
// ============================================================================

pub use self::awareness::HolisticAwareness;
pub use self::goal_tracker::HolisticGoalTracker;
pub use self::identity::HolisticIdentity;
pub use self::introspect::HolisticIntrospector;
pub use self::meta_holistic::HolisticMetaCognition;
pub use self::reflection::HolisticReflection;
pub use self::self_model::HolisticSelfModel;
pub use self::world_model::HolisticWorldModel;

// ============================================================================
// DEEP EXPANSION — Holistic Crown Consciousness Modules
// ============================================================================

/// System-wide emotional state — unified emotional landscape influencing all decisions
pub mod emotion;

/// System-wide attention allocation — cognitive resource distribution across subsystems
pub mod attention;

/// System-wide offline consolidation — cross-subsystem replay and insight generation
pub mod dream;

/// Holistic empathy — understanding the entire system's state and pain points
pub mod empathy;

/// System-wide fast pattern matching — instant recognition of system-level patterns
pub mod intuition;

/// The grand palace of all system knowledge — hierarchical long-term memory
pub mod memory_palace;

/// System-wide ethical framework — principle hierarchy and fairness enforcement
pub mod conscience;

/// Unified subjective experience — how the system "feels" right now
pub mod qualia;

pub use self::attention::HolisticAttentionEngine;
pub use self::conscience::HolisticConscience;
pub use self::dream::HolisticDreamEngine;
pub use self::emotion::HolisticEmotionEngine;
pub use self::empathy::HolisticEmpathyEngine;
pub use self::intuition::HolisticIntuitionEngine;
pub use self::memory_palace::HolisticMemoryPalace;
pub use self::qualia::HolisticQualiaEngine;
