//! Memory Store Traits
//!
//! Traits for the MEMORY domain - working, episodic, semantic, and procedural memory.

#![allow(dead_code)]

use alloc::vec::Vec;

use super::component::NexusComponent;
use crate::types::{ConceptId, Duration, EpisodeId, NexusResult, ProcedureId, TimeRange};

// ============================================================================
// MEMORY STORE TRAIT
// ============================================================================

/// Base trait for memory stores
pub trait MemoryStore: NexusComponent {
    /// Item type stored
    type Item;
    /// Key type
    type Key;

    /// Store an item
    fn store(&mut self, key: Self::Key, item: Self::Item) -> NexusResult<()>;

    /// Retrieve an item
    fn retrieve(&self, key: &Self::Key) -> Option<&Self::Item>;

    /// Retrieve mutable reference
    fn retrieve_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Item>;

    /// Remove an item
    fn remove(&mut self, key: &Self::Key) -> Option<Self::Item>;

    /// Check if key exists
    fn contains(&self, key: &Self::Key) -> bool;

    /// Get count
    fn count(&self) -> usize;

    /// Clear all items
    fn clear(&mut self);

    /// Is empty?
    fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Get all keys
    fn keys(&self) -> Vec<Self::Key>;
}

// ============================================================================
// WORKING MEMORY TRAIT
// ============================================================================

/// Working memory trait (short-term, attention-focused)
pub trait WorkingMemory: MemoryStore {
    /// Set time-to-live for an item
    fn set_ttl(&mut self, key: &Self::Key, ttl: Duration) -> NexusResult<()>;

    /// Expire old items
    fn expire(&mut self) -> usize;

    /// Get current attention focus
    fn focus(&self) -> Option<&Self::Key>;

    /// Set attention focus
    fn set_focus(&mut self, key: Self::Key);

    /// Clear focus
    fn clear_focus(&mut self);

    /// Get items by access recency
    fn recent(&self, limit: usize) -> Vec<&Self::Item>;

    /// Get memory pressure (0.0 to 1.0)
    fn pressure(&self) -> f32;
}

// ============================================================================
// EPISODIC MEMORY TRAIT
// ============================================================================

/// Episodic memory trait (events and experiences)
pub trait EpisodicMemory: MemoryStore {
    /// Episode type
    type Episode;

    /// Record an episode
    fn record_episode(&mut self, episode: Self::Episode) -> EpisodeId;

    /// Retrieve episodes in time range
    fn episodes_in_range(&self, range: TimeRange) -> Vec<&Self::Episode>;

    /// Find similar episodes
    fn find_similar(&self, episode: &Self::Episode, limit: usize) -> Vec<&Self::Episode>;

    /// Get episode significance (0.0 to 1.0)
    fn significance(&self, episode: &Self::Episode) -> f32;

    /// Get most significant episodes
    fn most_significant(&self, limit: usize) -> Vec<&Self::Episode>;

    /// Consolidate episodes (merge similar, forget insignificant)
    fn consolidate(&mut self) -> ConsolidationResult;
}

/// Consolidation result
#[derive(Debug, Clone, Default)]
pub struct ConsolidationResult {
    /// Episodes merged
    pub merged: usize,
    /// Episodes forgotten
    pub forgotten: usize,
    /// Episodes retained
    pub retained: usize,
    /// Memory freed (bytes)
    pub memory_freed: usize,
}

// ============================================================================
// SEMANTIC MEMORY TRAIT
// ============================================================================

/// Semantic memory trait (facts and concepts)
pub trait SemanticMemory: MemoryStore {
    /// Concept type
    type Concept;

    /// Store a concept
    fn store_concept(&mut self, concept: Self::Concept) -> ConceptId;

    /// Find related concepts
    fn find_related(&self, concept: &Self::Concept, limit: usize) -> Vec<&Self::Concept>;

    /// Get concept by name
    fn get_by_name(&self, name: &str) -> Option<&Self::Concept>;

    /// Get relationship between concepts
    fn relationship(&self, a: ConceptId, b: ConceptId) -> Option<ConceptRelation>;

    /// Build taxonomy (hierarchy)
    fn taxonomy(&self) -> Vec<(&Self::Concept, Vec<ConceptId>)>;

    /// Search concepts
    fn search(&self, query: &str, limit: usize) -> Vec<&Self::Concept>;
}

/// Relationship between concepts
#[derive(Debug, Clone)]
pub struct ConceptRelation {
    /// Relation type
    pub relation_type: RelationType,
    /// Strength (0.0 to 1.0)
    pub strength: f32,
    /// Is bidirectional?
    pub bidirectional: bool,
}

/// Types of concept relations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationType {
    /// Is-a (inheritance)
    IsA,
    /// Has-a (composition)
    HasA,
    /// Part-of
    PartOf,
    /// Related to
    RelatedTo,
    /// Causes
    Causes,
    /// Opposes
    Opposes,
    /// Synonymous
    Synonym,
}

// ============================================================================
// PROCEDURAL MEMORY TRAIT
// ============================================================================

/// Procedural memory trait (skills and procedures)
pub trait ProceduralMemory: MemoryStore {
    /// Procedure type
    type Procedure;

    /// Store a procedure
    fn store_procedure(&mut self, procedure: Self::Procedure) -> ProcedureId;

    /// Find applicable procedure for context
    fn find_applicable(&self, context: &str) -> Option<&Self::Procedure>;

    /// Get procedure success rate
    fn success_rate(&self, procedure: &Self::Procedure) -> f32;

    /// Record execution result
    fn record_execution(&mut self, procedure: ProcedureId, success: bool);

    /// Get procedure execution count
    fn execution_count(&self, procedure: ProcedureId) -> u64;

    /// Rank procedures by success
    fn rank_by_success(&self) -> Vec<(ProcedureId, f32)>;

    /// Learn from experience (update procedure)
    fn learn(&mut self, procedure: ProcedureId, feedback: ExecutionFeedback);
}

/// Feedback from procedure execution
#[derive(Debug, Clone)]
pub struct ExecutionFeedback {
    /// Was successful?
    pub success: bool,
    /// Execution duration
    pub duration: Duration,
    /// Error message if failed
    pub error: Option<alloc::string::String>,
    /// Quality score (0.0 to 1.0)
    pub quality: f32,
    /// Context notes
    pub notes: Option<alloc::string::String>,
}

impl ExecutionFeedback {
    /// Create success feedback
    pub fn success(duration: Duration, quality: f32) -> Self {
        Self {
            success: true,
            duration,
            error: None,
            quality,
            notes: None,
        }
    }

    /// Create failure feedback
    pub fn failure(duration: Duration, error: impl Into<alloc::string::String>) -> Self {
        Self {
            success: false,
            duration,
            error: Some(error.into()),
            quality: 0.0,
            notes: None,
        }
    }
}

// ============================================================================
// MEMORY MANAGER TRAIT
// ============================================================================

/// Unified memory manager
pub trait MemoryManager: NexusComponent {
    /// Get working memory capacity
    fn working_capacity(&self) -> usize;

    /// Get total memory usage (bytes)
    fn total_usage(&self) -> usize;

    /// Get memory pressure (0.0 to 1.0)
    fn pressure(&self) -> f32;

    /// Trigger garbage collection
    fn gc(&mut self) -> GcResult;

    /// Get memory statistics
    fn stats(&self) -> MemoryStats;
}

/// GC result
#[derive(Debug, Clone, Default)]
pub struct GcResult {
    /// Items collected
    pub collected: usize,
    /// Memory freed (bytes)
    pub freed: usize,
    /// Duration of GC
    pub duration: Duration,
}

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Working memory items
    pub working_items: usize,
    /// Episodic memory items
    pub episodic_items: usize,
    /// Semantic memory items
    pub semantic_items: usize,
    /// Procedural memory items
    pub procedural_items: usize,
    /// Total bytes used
    pub total_bytes: usize,
    /// Cache hit rate
    pub cache_hit_rate: f32,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consolidation_result() {
        let result = ConsolidationResult {
            merged: 5,
            forgotten: 3,
            retained: 10,
            memory_freed: 1024,
        };
        assert_eq!(result.merged, 5);
        assert_eq!(result.memory_freed, 1024);
    }

    #[test]
    fn test_execution_feedback() {
        let success = ExecutionFeedback::success(Duration::from_millis(100), 0.95);
        assert!(success.success);
        assert!(success.error.is_none());

        let failure = ExecutionFeedback::failure(Duration::from_millis(50), "Timeout");
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }

    #[test]
    fn test_concept_relation() {
        let relation = ConceptRelation {
            relation_type: RelationType::IsA,
            strength: 0.9,
            bidirectional: false,
        };
        assert_eq!(relation.relation_type, RelationType::IsA);
    }
}
