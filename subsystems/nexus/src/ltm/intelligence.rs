//! Long-Term Memory Intelligence
//!
//! This module provides the main LTM intelligence interface.

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};

use super::{
    BootId, ConsolidationResult, ConsolidationStrategy, EpisodeId, EpisodeType, EpisodicMemory,
    MemoryConsolidator, MemoryId, PatternCategory, PatternId, ProceduralMemory, ProcedureId,
    ProcedureType, SemanticMemory, TimeRange, Timestamp, WorkingMemory, WorkingMemoryContent,
};

/// LTM analysis
#[derive(Debug, Clone)]
pub struct LtmAnalysis {
    /// Total episodes
    pub total_episodes: u64,
    /// Total patterns
    pub total_patterns: u64,
    /// Reliable patterns
    pub reliable_patterns: u64,
    /// Total procedures
    pub total_procedures: u64,
    /// Working memory items
    pub working_memory_items: u64,
    /// Consolidations performed
    pub consolidations: u64,
    /// Memory health score (0-100)
    pub health_score: f32,
}

impl LtmAnalysis {
    /// Pattern reliability ratio
    #[inline]
    pub fn pattern_reliability(&self) -> f32 {
        if self.total_patterns == 0 {
            return 0.0;
        }
        self.reliable_patterns as f32 / self.total_patterns as f32
    }
}

/// Long-term memory intelligence
pub struct LongTermMemoryIntelligence {
    /// Episodic memory
    episodic: EpisodicMemory,
    /// Semantic memory
    semantic: SemanticMemory,
    /// Procedural memory
    procedural: ProceduralMemory,
    /// Working memory
    working: WorkingMemory,
    /// Consolidator
    consolidator: MemoryConsolidator,
    /// Current boot
    current_boot: BootId,
    /// Enabled
    enabled: AtomicBool,
}

impl LongTermMemoryIntelligence {
    /// Create new LTM intelligence
    pub fn new(boot_id: u64) -> Self {
        let current_boot = BootId::new(boot_id);
        Self {
            episodic: EpisodicMemory::new(current_boot),
            semantic: SemanticMemory::new(),
            procedural: ProceduralMemory::new(),
            working: WorkingMemory::new(),
            consolidator: MemoryConsolidator::new(ConsolidationStrategy::Periodic),
            current_boot,
            enabled: AtomicBool::new(true),
        }
    }

    /// Check if enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable LTM
    #[inline(always)]
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable LTM
    #[inline(always)]
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Record event to working memory
    #[inline]
    pub fn record_event(
        &mut self,
        event_type: &str,
        data: BTreeMap<String, String>,
        priority: u8,
    ) -> MemoryId {
        let content = WorkingMemoryContent::CurrentEvent {
            event_type: String::from(event_type),
            data,
        };
        self.working.store(content, priority, 60_000_000_000) // 1 minute TTL
    }

    /// Record decision
    #[inline]
    pub fn record_decision(&mut self, decision: &str, outcome: &str) -> MemoryId {
        let content = WorkingMemoryContent::RecentDecision {
            decision: String::from(decision),
            outcome: String::from(outcome),
        };
        self.working.store(content, 5, 300_000_000_000) // 5 minute TTL
    }

    /// Record pattern match
    #[inline]
    pub fn record_pattern_match(&mut self, pattern_id: PatternId, match_score: f32) -> MemoryId {
        let content = WorkingMemoryContent::ActivePattern {
            pattern_id,
            match_score,
        };
        self.working.store(content, 6, 120_000_000_000) // 2 minute TTL
    }

    /// Store context
    #[inline]
    pub fn store_context(&mut self, key: &str, value: &str) -> MemoryId {
        let content = WorkingMemoryContent::Context {
            key: String::from(key),
            value: String::from(value),
        };
        self.working.store(content, 3, 0) // No TTL
    }

    /// Get context
    #[inline(always)]
    pub fn get_context(&self, key: &str) -> Option<&str> {
        self.working.get_context(key)
    }

    /// Create episode directly
    #[inline(always)]
    pub fn create_episode(&mut self, episode_type: EpisodeType, start: u64, end: u64) -> EpisodeId {
        let range = TimeRange::new(Timestamp::new(start), Timestamp::new(end));
        self.episodic.create_episode(episode_type, range)
    }

    /// Create pattern
    #[inline(always)]
    pub fn create_pattern(&mut self, name: &str, category: PatternCategory) -> PatternId {
        self.semantic.create_pattern(String::from(name), category)
    }

    /// Create procedure
    #[inline(always)]
    pub fn create_procedure(&mut self, name: &str, procedure_type: ProcedureType) -> ProcedureId {
        self.procedural
            .create_procedure(String::from(name), procedure_type)
    }

    /// Tick - update time and maybe consolidate
    #[inline]
    pub fn tick(&mut self, current_time: u64) {
        if !self.is_enabled() {
            return;
        }

        self.working.update_time(current_time);

        if self.consolidator.should_consolidate(current_time) {
            self.consolidate(current_time);
        }
    }

    /// Force consolidation
    #[inline]
    pub fn consolidate(&mut self, current_time: u64) -> ConsolidationResult {
        self.consolidator.consolidate(
            &mut self.working,
            &mut self.episodic,
            &mut self.semantic,
            &mut self.procedural,
            current_time,
        )
    }

    /// Get analysis
    pub fn analyze(&self) -> LtmAnalysis {
        let reliable = self.semantic.reliable_count() as u64;
        let total_patterns = self.semantic.count() as u64;

        let health_score = if total_patterns > 0 {
            (reliable as f32 / total_patterns as f32) * 100.0
        } else {
            50.0
        };

        LtmAnalysis {
            total_episodes: self.episodic.count() as u64,
            total_patterns,
            reliable_patterns: reliable,
            total_procedures: self.procedural.count() as u64,
            working_memory_items: self.working.count() as u64,
            consolidations: self.consolidator.total_consolidations(),
            health_score,
        }
    }

    /// Get episodic memory
    #[inline(always)]
    pub fn episodic(&self) -> &EpisodicMemory {
        &self.episodic
    }

    /// Get episodic memory mutably
    #[inline(always)]
    pub fn episodic_mut(&mut self) -> &mut EpisodicMemory {
        &mut self.episodic
    }

    /// Get semantic memory
    #[inline(always)]
    pub fn semantic(&self) -> &SemanticMemory {
        &self.semantic
    }

    /// Get semantic memory mutably
    #[inline(always)]
    pub fn semantic_mut(&mut self) -> &mut SemanticMemory {
        &mut self.semantic
    }

    /// Get procedural memory
    #[inline(always)]
    pub fn procedural(&self) -> &ProceduralMemory {
        &self.procedural
    }

    /// Get procedural memory mutably
    #[inline(always)]
    pub fn procedural_mut(&mut self) -> &mut ProceduralMemory {
        &mut self.procedural
    }

    /// Get working memory
    #[inline(always)]
    pub fn working(&self) -> &WorkingMemory {
        &self.working
    }

    /// Get working memory mutably
    #[inline(always)]
    pub fn working_mut(&mut self) -> &mut WorkingMemory {
        &mut self.working
    }

    /// Get consolidator
    #[inline(always)]
    pub fn consolidator(&self) -> &MemoryConsolidator {
        &self.consolidator
    }

    /// Get current boot ID
    #[inline(always)]
    pub fn current_boot(&self) -> BootId {
        self.current_boot
    }
}

impl core::fmt::Debug for LongTermMemoryIntelligence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LongTermMemoryIntelligence")
            .field("current_boot", &self.current_boot)
            .field("episodes", &self.episodic.count())
            .field("patterns", &self.semantic.count())
            .field("procedures", &self.procedural.count())
            .field("working_items", &self.working.count())
            .finish()
    }
}
