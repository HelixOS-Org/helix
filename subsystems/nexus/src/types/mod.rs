//! NEXUS Shared Types — Foundational Type System
//!
//! This module defines the fundamental types shared across all cognitive domains.
//! These types form the common language of NEXUS, enabling type-safe communication
//! between domains without creating tight coupling.
//!
//! # Module Structure
//!
//! ```text
//! types/
//! ├── mod.rs          — This file (re-exports)
//! ├── identifiers.rs  — Type-safe IDs for all entities
//! ├── temporal.rs     — Timestamp, Duration, TimeRange
//! ├── confidence.rs   — Confidence and probability types
//! ├── severity.rs     — Severity and priority levels
//! ├── metrics.rs      — Metric values and units
//! ├── version.rs      — Semantic versioning
//! ├── tags.rs         — Key-value metadata
//! ├── wrappers.rs     — Generic wrapper types
//! ├── envelope.rs     — Domain message envelopes
//! └── errors.rs       — Error types
//! ```
//!
//! # Design Principles
//!
//! 1. **Opaque at Boundaries**: Each domain has its own types that only it understands
//! 2. **Shared Primitives**: IDs, timestamps, and metrics are universal
//! 3. **No Business Logic**: Types are pure data, no behavior
//! 4. **Versioned**: All persistent types carry version information

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

// ============================================================================
// SUBMODULES
// ============================================================================

pub mod confidence;
pub mod envelope;
pub mod errors;
pub mod identifiers;
pub mod metrics;
pub mod severity;
pub mod tags;
pub mod temporal;
pub mod version;
pub mod wrappers;

// ============================================================================
// ID GENERATOR
// ============================================================================

static GLOBAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a globally unique ID
#[inline(always)]
pub fn generate_id() -> u64 {
    GLOBAL_ID_COUNTER.fetch_add(1, AtomicOrdering::SeqCst)
}

/// Reset ID counter (for testing only)
#[cfg(test)]
pub fn reset_id_counter() {
    GLOBAL_ID_COUNTER.store(1, AtomicOrdering::SeqCst);
}

// ============================================================================
// RE-EXPORTS: IDENTIFIERS
// ============================================================================

// ============================================================================
// RE-EXPORTS: CONFIDENCE
// ============================================================================
pub use confidence::{Confidence, ConfidenceLevel, Probability};
// ============================================================================
// RE-EXPORTS: ENVELOPE
// ============================================================================
pub use envelope::{
    ActionParameters, ActionTarget, ActionType, Change, ChangeId, ChangeType, Conclusion,
    ConclusionType, Effect, EffectId, Intent, Knowledge, KnowledgeType, Signal,
};
// ============================================================================
// RE-EXPORTS: ERRORS
// ============================================================================
pub use errors::{ErrorCategory, ErrorCode, NexusError, NexusResult};
pub use identifiers::{
    ActionId,
    AuditId,
    CausalEdgeId,
    // Reasoning IDs (REASON)
    CausalNodeId,
    CheckpointId,
    ComponentId,
    ConceptId,
    ConclusionId,
    ConflictId,
    CorrelationId,
    DecisionId,
    DiagnosisId,
    DomainId,
    // Execution IDs (ACT)
    EffectorId,
    EpisodeId,
    EventId,
    FailureId,
    FeatureId,
    HypothesisId,
    // Reflection IDs (REFLECT)
    InsightId,
    IntentId,
    IssueId,
    KnowledgeId,
    // Memory IDs
    MemoryId,
    // Message IDs (BUS)
    MessageId,
    ModelId,
    // Core IDs
    NexusId,
    OptionId,
    // Comprehension IDs (UNDERSTAND)
    PatternId,
    // Decision IDs (DECIDE)
    PolicyId,
    PredictionId,
    // Perception IDs (SENSE)
    ProbeId,
    ProcedureId,
    SampleId,
    SignalId,
    SimulationId,
    StreamId,
    SuggestionId,
    TransactionId,
};
// ============================================================================
// RE-EXPORTS: METRICS
// ============================================================================
pub use metrics::{Metric, MetricUnit, MetricValue};
// ============================================================================
// RE-EXPORTS: SEVERITY
// ============================================================================
pub use severity::{Priority, Severity, SeverityClass, Urgency};
// ============================================================================
// RE-EXPORTS: TAGS
// ============================================================================
pub use tags::{Label, Labels, Tag, Tags};
// ============================================================================
// RE-EXPORTS: TEMPORAL
// ============================================================================
pub use temporal::{Duration, TimeRange, Timestamp};
// ============================================================================
// RE-EXPORTS: VERSION
// ============================================================================
pub use version::{Version, VersionReq};
// ============================================================================
// RE-EXPORTS: WRAPPERS
// ============================================================================
pub use wrappers::{Counted, Expiring, OptionalWith, Timestamped, Versioned};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generation() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert!(id2 > id1);
    }

    #[test]
    fn test_reexports() {
        // Verify that all types are accessible through the main module
        let _: Timestamp = Timestamp::now();
        let _: Duration = Duration::SECOND;
        let _: Confidence = Confidence::HIGH;
        let _: Severity = Severity::MEDIUM;
        let _: Priority = Priority::NORMAL;
        let _: Version = Version::INITIAL;
    }
}
