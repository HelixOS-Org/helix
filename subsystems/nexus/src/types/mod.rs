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

pub mod identifiers;
pub mod temporal;
pub mod confidence;
pub mod severity;
pub mod metrics;
pub mod version;
pub mod tags;
pub mod wrappers;
pub mod envelope;
pub mod errors;

// ============================================================================
// ID GENERATOR
// ============================================================================

static GLOBAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a globally unique ID
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

pub use identifiers::{
    // Core IDs
    NexusId, DomainId, ComponentId,
    // Perception IDs (SENSE)
    ProbeId, SignalId, EventId, StreamId, SampleId,
    // Comprehension IDs (UNDERSTAND)
    PatternId, ModelId, FeatureId, KnowledgeId,
    // Reasoning IDs (REASON)
    CausalNodeId, CausalEdgeId, ConclusionId, HypothesisId, SimulationId,
    // Decision IDs (DECIDE)
    PolicyId, IntentId, OptionId, ConflictId,
    // Execution IDs (ACT)
    EffectorId, ActionId, TransactionId, CheckpointId,
    // Memory IDs
    MemoryId, EpisodeId, ConceptId, ProcedureId,
    // Reflection IDs (REFLECT)
    InsightId, DiagnosisId, FailureId, SuggestionId, PredictionId, DecisionId, IssueId,
    // Message IDs (BUS)
    MessageId, CorrelationId,
};

// ============================================================================
// RE-EXPORTS: TEMPORAL
// ============================================================================

pub use temporal::{Timestamp, Duration, TimeRange};

// ============================================================================
// RE-EXPORTS: CONFIDENCE
// ============================================================================

pub use confidence::{Confidence, ConfidenceLevel, Probability};

// ============================================================================
// RE-EXPORTS: SEVERITY
// ============================================================================

pub use severity::{Severity, SeverityClass, Priority, Urgency};

// ============================================================================
// RE-EXPORTS: METRICS
// ============================================================================

pub use metrics::{Metric, MetricValue, MetricUnit};

// ============================================================================
// RE-EXPORTS: VERSION
// ============================================================================

pub use version::{Version, VersionReq};

// ============================================================================
// RE-EXPORTS: TAGS
// ============================================================================

pub use tags::{Tag, Tags, Label, Labels};

// ============================================================================
// RE-EXPORTS: WRAPPERS
// ============================================================================

pub use wrappers::{Versioned, Timestamped, Expiring, Counted, OptionalWith};

// ============================================================================
// RE-EXPORTS: ENVELOPE
// ============================================================================

pub use envelope::{
    Signal, Knowledge, KnowledgeType,
    Conclusion, ConclusionType,
    Intent, ActionType,
    Effect, Change, ChangeType,
};

// ============================================================================
// RE-EXPORTS: ERRORS
// ============================================================================

pub use errors::{NexusError, ErrorCode, ErrorCategory, NexusResult};

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
