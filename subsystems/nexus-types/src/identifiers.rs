//! Typed Identifiers for NEXUS
//!
//! Type-safe identifiers for every entity in the cognitive system.
//! Each domain has its own ID types to prevent accidental mixing.

#![allow(dead_code)]

/// Macro to create type-safe IDs
#[macro_export]
macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(pub u64);

        impl $name {
            /// Create a new ID with specific value
            #[inline]
            pub const fn new(id: u64) -> Self {
                Self(id)
            }

            /// Generate a new unique ID
            #[inline]
            pub fn generate() -> Self {
                Self($crate::generate_id())
            }

            /// Get the raw value
            #[inline]
            pub const fn raw(&self) -> u64 {
                self.0
            }

            /// Null/invalid ID
            pub const NULL: Self = Self(0);

            /// Check if null
            #[inline]
            pub const fn is_null(&self) -> bool {
                self.0 == 0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::generate()
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

// ============================================================================
// CORE IDS
// ============================================================================

define_id!(NexusId, "Global NEXUS instance identifier");
define_id!(DomainId, "Cognitive domain identifier");
define_id!(ComponentId, "Component within a domain");

// ============================================================================
// PERCEPTION IDS (SENSE)
// ============================================================================

define_id!(ProbeId, "Hardware/software probe identifier");
define_id!(SignalId, "Raw signal identifier");
define_id!(EventId, "Kernel event identifier");
define_id!(StreamId, "Data stream identifier");
define_id!(SampleId, "Individual sample identifier");

// ============================================================================
// COMPREHENSION IDS (UNDERSTAND)
// ============================================================================

define_id!(PatternId, "Detected pattern identifier");
define_id!(ModelId, "Comprehension model identifier");
define_id!(FeatureId, "Extracted feature identifier");
define_id!(KnowledgeId, "Knowledge unit identifier");

// ============================================================================
// REASONING IDS (REASON)
// ============================================================================

define_id!(CausalNodeId, "Node in causal graph");
define_id!(CausalEdgeId, "Edge in causal graph");
define_id!(ConclusionId, "Reasoning conclusion identifier");
define_id!(HypothesisId, "Hypothesis identifier");
define_id!(SimulationId, "Simulation run identifier");

// ============================================================================
// DECISION IDS (DECIDE)
// ============================================================================

define_id!(PolicyId, "Policy identifier");
define_id!(IntentId, "Decision intent identifier");
define_id!(OptionId, "Decision option identifier");
define_id!(ConflictId, "Conflict identifier");

// ============================================================================
// EXECUTION IDS (ACT)
// ============================================================================

define_id!(EffectorId, "Effector identifier");
define_id!(ActionId, "Action identifier");
define_id!(TransactionId, "Transaction identifier");
define_id!(CheckpointId, "Checkpoint identifier");
define_id!(AuditId, "Audit entry identifier");

// ============================================================================
// MEMORY IDS
// ============================================================================

define_id!(MemoryId, "Memory entry identifier");
define_id!(EpisodeId, "Episode identifier");
define_id!(ConceptId, "Semantic concept identifier");
define_id!(ProcedureId, "Procedural memory identifier");

// ============================================================================
// REFLECTION IDS (REFLECT)
// ============================================================================

define_id!(InsightId, "Meta-cognitive insight identifier");
define_id!(DiagnosisId, "Self-diagnosis identifier");
define_id!(FailureId, "Cognitive failure identifier");
define_id!(SuggestionId, "Improvement suggestion identifier");
define_id!(PredictionId, "Prediction identifier");
define_id!(DecisionId, "Decision record identifier");
define_id!(IssueId, "Cognitive issue identifier");

// ============================================================================
// MESSAGE IDS (BUS)
// ============================================================================

define_id!(MessageId, "Bus message identifier");
define_id!(CorrelationId, "Correlated message chain identifier");

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generation() {
        let id1 = NexusId::generate();
        let id2 = NexusId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_id_null() {
        let id = DomainId::NULL;
        assert!(id.is_null());
        assert_eq!(id.raw(), 0);
    }

    #[test]
    fn test_id_display() {
        let id = ProbeId::new(42);
        let s = alloc::format!("{}", id);
        assert!(s.contains("42"));
    }
}
