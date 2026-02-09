//! Memory Consolidation
//!
//! This module provides working memory to long-term memory consolidation.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    EpisodeType, EpisodicMemory, ProceduralMemory, ProcedureType, SemanticMemory, TimeRange,
    Timestamp, WorkingMemory, WorkingMemoryContent,
};

/// Consolidation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsolidationStrategy {
    /// Immediate consolidation
    Immediate,
    /// Periodic consolidation
    Periodic,
    /// On-demand consolidation
    OnDemand,
    /// Threshold-based
    Threshold,
}

/// Consolidation result
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    /// Episodes consolidated
    pub episodes_consolidated: u64,
    /// Patterns extracted
    pub patterns_extracted: u64,
    /// Procedures created
    pub procedures_created: u64,
    /// Memory freed
    pub memory_freed: u64,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
}

impl ConsolidationResult {
    /// Empty result
    #[inline]
    pub fn empty() -> Self {
        Self {
            episodes_consolidated: 0,
            patterns_extracted: 0,
            procedures_created: 0,
            memory_freed: 0,
            duration_ns: 0,
        }
    }

    /// Total items processed
    #[inline(always)]
    pub fn total_items(&self) -> u64 {
        self.episodes_consolidated + self.patterns_extracted + self.procedures_created
    }
}

/// Memory consolidator
pub struct MemoryConsolidator {
    /// Strategy
    strategy: ConsolidationStrategy,
    /// Consolidation interval (nanoseconds)
    interval_ns: u64,
    /// Last consolidation time
    last_consolidation: AtomicU64,
    /// Total consolidations
    total_consolidations: AtomicU64,
    /// Working memory threshold (for threshold strategy)
    threshold_count: usize,
}

impl MemoryConsolidator {
    /// Create new consolidator
    pub fn new(strategy: ConsolidationStrategy) -> Self {
        Self {
            strategy,
            interval_ns: 3_600_000_000_000, // 1 hour default
            last_consolidation: AtomicU64::new(0),
            total_consolidations: AtomicU64::new(0),
            threshold_count: 500,
        }
    }

    /// Set interval
    #[inline(always)]
    pub fn set_interval(&mut self, interval_ns: u64) {
        self.interval_ns = interval_ns;
    }

    /// Set threshold
    #[inline(always)]
    pub fn set_threshold(&mut self, count: usize) {
        self.threshold_count = count;
    }

    /// Get strategy
    #[inline(always)]
    pub fn strategy(&self) -> ConsolidationStrategy {
        self.strategy
    }

    /// Should consolidate
    #[inline]
    pub fn should_consolidate(&self, current_time: u64) -> bool {
        match self.strategy {
            ConsolidationStrategy::Immediate => true,
            ConsolidationStrategy::Periodic => {
                let last = self.last_consolidation.load(Ordering::Relaxed);
                current_time - last >= self.interval_ns
            },
            ConsolidationStrategy::OnDemand => false,
            ConsolidationStrategy::Threshold => false, // Need more context
        }
    }

    /// Should consolidate with working memory info
    #[inline]
    pub fn should_consolidate_with_wm(&self, current_time: u64, wm_count: usize) -> bool {
        match self.strategy {
            ConsolidationStrategy::Threshold => wm_count >= self.threshold_count,
            _ => self.should_consolidate(current_time),
        }
    }

    /// Consolidate working memory to long-term
    pub fn consolidate(
        &self,
        working: &mut WorkingMemory,
        episodic: &mut EpisodicMemory,
        semantic: &mut SemanticMemory,
        procedural: &mut ProceduralMemory,
        current_time: u64,
    ) -> ConsolidationResult {
        let start = current_time;
        let mut episodes_consolidated = 0u64;
        let mut patterns_extracted = 0u64;
        let mut procedures_created = 0u64;

        // Extract episodes from working memory events
        let events: Vec<_> = working.find_events().iter().map(|i| (*i).clone()).collect();

        for item in events {
            if let WorkingMemoryContent::CurrentEvent { event_type, .. } = &item.content {
                // Create episode for significant events
                let episode_type = match event_type.as_str() {
                    "error" => EpisodeType::Anomaly,
                    "crash" => EpisodeType::Crash,
                    "recovery" => EpisodeType::Recovery,
                    "security" => EpisodeType::SecurityEvent,
                    "hardware" => EpisodeType::HardwareEvent,
                    _ => continue, // Skip non-significant events
                };

                let range = TimeRange::new(item.created, item.last_accessed);
                episodic.create_episode(episode_type, range);
                episodes_consolidated += 1;
            }
        }

        // Extract patterns from working memory
        let pattern_items: Vec<_> = working
            .find_patterns()
            .iter()
            .map(|i| (*i).clone())
            .collect();

        for item in pattern_items {
            if let WorkingMemoryContent::ActivePattern {
                pattern_id,
                match_score,
            } = &item.content
            {
                // Record observation for existing pattern
                if semantic.get_mut(*pattern_id).is_some() {
                    let ts = Timestamp::new(current_time);
                    semantic.record_observation(*pattern_id, ts, *match_score > 0.8);
                    patterns_extracted += 1;
                }
            }
        }

        // Extract procedures from successful decisions
        let decisions: Vec<_> = working
            .find_decisions()
            .iter()
            .map(|i| (*i).clone())
            .collect();

        for item in decisions {
            if let WorkingMemoryContent::RecentDecision { decision, outcome } = &item.content {
                if outcome == "success" {
                    // Create or update procedure
                    let name = alloc::format!("auto_{}", decision.replace(' ', "_"));
                    let id = procedural.create_procedure(name, ProcedureType::Routine);
                    if let Some(proc) = procedural.get_mut(id) {
                        proc.record_execution(true, 0);
                        procedures_created += 1;
                    }
                }
            }
        }

        self.last_consolidation
            .store(current_time, Ordering::Relaxed);
        self.total_consolidations.fetch_add(1, Ordering::Relaxed);

        let duration = current_time.saturating_sub(start);

        ConsolidationResult {
            episodes_consolidated,
            patterns_extracted,
            procedures_created,
            memory_freed: 0, // Would calculate actual memory freed
            duration_ns: duration,
        }
    }

    /// Total consolidations
    #[inline(always)]
    pub fn total_consolidations(&self) -> u64 {
        self.total_consolidations.load(Ordering::Relaxed)
    }

    /// Last consolidation time
    #[inline(always)]
    pub fn last_consolidation(&self) -> u64 {
        self.last_consolidation.load(Ordering::Relaxed)
    }
}

impl core::fmt::Debug for MemoryConsolidator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemoryConsolidator")
            .field("strategy", &self.strategy)
            .field("interval_ns", &self.interval_ns)
            .field("total_consolidations", &self.total_consolidations())
            .finish()
    }
}
