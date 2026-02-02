//! NEXUS Long-Term Memory Engine — COGNITION Year 2
//!
//! This module provides persistent memory capabilities, enabling NEXUS to:
//!
//! - **Remember** events and patterns across boots
//! - **Store** episodic memories of significant events
//! - **Abstract** semantic knowledge from experiences
//! - **Consolidate** working memory into long-term storage
//! - **Query** historical data efficiently
//!
//! ## Modules
//!
//! - [`types`] - Core identifiers and time structures
//! - [`episodic`] - Episode storage and retrieval
//! - [`semantic`] - Pattern and knowledge storage
//! - [`procedural`] - Learned procedure storage
//! - [`working`] - Short-term working memory
//! - [`consolidation`] - Memory consolidation engine
//! - [`intelligence`] - Main LTM intelligence interface

extern crate alloc;

pub mod consolidation;
pub mod episodic;
pub mod intelligence;
pub mod procedural;
pub mod semantic;
pub mod types;
pub mod working;

// Re-export types
// Re-export consolidation
pub use consolidation::{ConsolidationResult, ConsolidationStrategy, MemoryConsolidator};
// Re-export episodic
pub use episodic::{Episode, EpisodeEvent, EpisodeOutcome, EpisodeType, EpisodicMemory};
// Re-export intelligence
pub use intelligence::{LongTermMemoryIntelligence, LtmAnalysis};
// Re-export procedural
pub use procedural::{ProceduralMemory, Procedure, ProcedureStep, ProcedureType};
// Re-export semantic
pub use semantic::{
    ConditionOperator, PatternCategory, PatternCondition, PatternConfidence, SemanticMemory,
    SemanticPattern,
};
pub use types::{BootId, EpisodeId, MemoryId, PatternId, ProcedureId, TimeRange, Timestamp};
// Re-export working
pub use working::{WorkingMemory, WorkingMemoryContent, WorkingMemoryItem};

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use alloc::string::String;

    use super::*;

    #[test]
    fn test_timestamp() {
        let ts1 = Timestamp::new(1_000_000_000);
        let ts2 = Timestamp::new(2_000_000_000);

        assert_eq!(ts1.as_secs(), 1);
        assert_eq!(ts2.diff(&ts1), 1_000_000_000);
    }

    #[test]
    fn test_time_range() {
        let range1 = TimeRange::new(Timestamp::new(0), Timestamp::new(100));
        let range2 = TimeRange::new(Timestamp::new(50), Timestamp::new(150));

        assert!(range1.overlaps(&range2));
        assert!(range1.contains(Timestamp::new(50)));
    }

    #[test]
    fn test_episodic_memory() {
        let mut episodic = EpisodicMemory::new(BootId::new(1));

        let range = TimeRange::new(Timestamp::new(0), Timestamp::new(1000));
        let id = episodic.create_episode(EpisodeType::Anomaly, range);

        assert!(episodic.get(id).is_some());
        assert_eq!(episodic.count(), 1);
    }

    #[test]
    fn test_semantic_memory() {
        let mut semantic = SemanticMemory::new();

        let id = semantic.create_pattern(
            String::from("oom_after_video"),
            PatternCategory::ResourceUsage,
        );

        if let Some(pattern) = semantic.get_mut(id) {
            pattern.record_observation(Timestamp::new(1000), true);
            pattern.record_observation(Timestamp::new(2000), true);
        }

        assert_eq!(semantic.count(), 1);
    }

    #[test]
    fn test_procedural_memory() {
        let mut procedural = ProceduralMemory::new();

        let id = procedural.create_procedure(String::from("recover_oom"), ProcedureType::Recovery);

        if let Some(proc) = procedural.get_mut(id) {
            proc.add_step(ProcedureStep::new(1, String::from("Free caches")));
            proc.record_execution(true, 1000);
        }

        assert_eq!(procedural.count(), 1);
    }

    #[test]
    fn test_working_memory() {
        let mut working = WorkingMemory::new();

        let id = working.store(
            WorkingMemoryContent::Context {
                key: String::from("cpu_load"),
                value: String::from("75%"),
            },
            5,
            60_000_000_000,
        );

        assert!(working.peek(id).is_some());
        assert_eq!(working.count(), 1);
    }

    #[test]
    fn test_ltm_intelligence() {
        let mut ltm = LongTermMemoryIntelligence::new(1);

        let mut data = BTreeMap::new();
        data.insert(String::from("type"), String::from("error"));

        ltm.record_event("error", data, 8);
        ltm.tick(1_000_000_000);

        let analysis = ltm.analyze();
        assert!(analysis.working_memory_items >= 1);
    }
}
