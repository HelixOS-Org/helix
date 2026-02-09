//! # Consolidation Engine
//!
//! Consolidates learned knowledge for long-term retention.
//! Implements memory consolidation and integration.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CONSOLIDATION TYPES
// ============================================================================

/// Memory trace
#[derive(Debug, Clone)]
pub struct MemoryTrace {
    /// Trace ID
    pub id: u64,
    /// Content
    pub content: TraceContent,
    /// Strength
    pub strength: f64,
    /// Volatility
    pub volatility: f64,
    /// Associations
    pub associations: Vec<u64>,
    /// Rehearsals
    pub rehearsals: u32,
    /// Created
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
}

/// Trace content
#[derive(Debug, Clone)]
pub enum TraceContent {
    Fact(Fact),
    Skill(Skill),
    Episode(EpisodeSummary),
    Concept(ConceptInfo),
}

/// Fact
#[derive(Debug, Clone)]
pub struct Fact {
    /// Subject
    pub subject: String,
    /// Predicate
    pub predicate: String,
    /// Object
    pub object: String,
    /// Confidence
    pub confidence: f64,
}

/// Skill
#[derive(Debug, Clone)]
pub struct Skill {
    /// Name
    pub name: String,
    /// Proficiency
    pub proficiency: f64,
    /// Practice count
    pub practice_count: u32,
}

/// Episode summary
#[derive(Debug, Clone)]
pub struct EpisodeSummary {
    /// Title
    pub title: String,
    /// Key events
    pub key_events: Vec<String>,
    /// Emotional valence
    pub valence: f64,
}

/// Concept info
#[derive(Debug, Clone)]
pub struct ConceptInfo {
    /// Name
    pub name: String,
    /// Features
    pub features: Vec<String>,
    /// Examples
    pub examples: Vec<String>,
}

/// Consolidated memory
#[derive(Debug, Clone)]
pub struct ConsolidatedMemory {
    /// Memory ID
    pub id: u64,
    /// Source traces
    pub sources: Vec<u64>,
    /// Content
    pub content: TraceContent,
    /// Stability
    pub stability: f64,
    /// Integration level
    pub integration: f64,
    /// Created
    pub created: Timestamp,
}

/// Consolidation result
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    /// Traces consolidated
    pub traces_consolidated: usize,
    /// Memories created
    pub memories_created: usize,
    /// Traces forgotten
    pub traces_forgotten: usize,
    /// Integration score
    pub integration_score: f64,
}

// ============================================================================
// CONSOLIDATION ENGINE
// ============================================================================

/// Consolidation engine
pub struct ConsolidationEngine {
    /// Memory traces
    traces: BTreeMap<u64, MemoryTrace>,
    /// Consolidated memories
    memories: BTreeMap<u64, ConsolidatedMemory>,
    /// Association graph
    associations: BTreeMap<u64, Vec<(u64, f64)>>, // trace_id -> [(other_id, strength)]
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ConsolidationConfig,
    /// Statistics
    stats: ConsolidationStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// Decay rate
    pub decay_rate: f64,
    /// Consolidation threshold
    pub consolidation_threshold: f64,
    /// Forgetting threshold
    pub forgetting_threshold: f64,
    /// Rehearsal boost
    pub rehearsal_boost: f64,
    /// Association threshold
    pub association_threshold: f64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.01,
            consolidation_threshold: 0.7,
            forgetting_threshold: 0.1,
            rehearsal_boost: 0.2,
            association_threshold: 0.5,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ConsolidationStats {
    /// Traces created
    pub traces_created: u64,
    /// Traces consolidated
    pub traces_consolidated: u64,
    /// Traces forgotten
    pub traces_forgotten: u64,
    /// Memories created
    pub memories_created: u64,
}

impl ConsolidationEngine {
    /// Create new engine
    pub fn new(config: ConsolidationConfig) -> Self {
        Self {
            traces: BTreeMap::new(),
            memories: BTreeMap::new(),
            associations: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ConsolidationStats::default(),
        }
    }

    /// Create trace
    pub fn create_trace(&mut self, content: TraceContent, strength: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let trace = MemoryTrace {
            id,
            content,
            strength: strength.clamp(0.0, 1.0),
            volatility: 1.0,
            associations: Vec::new(),
            rehearsals: 0,
            created: now,
            last_accessed: now,
        };

        self.traces.insert(id, trace);
        self.stats.traces_created += 1;

        id
    }

    /// Add fact
    #[inline]
    pub fn add_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        confidence: f64,
    ) -> u64 {
        let fact = Fact {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence,
        };

        self.create_trace(TraceContent::Fact(fact), confidence)
    }

    /// Add skill
    #[inline]
    pub fn add_skill(&mut self, name: &str, proficiency: f64) -> u64 {
        let skill = Skill {
            name: name.into(),
            proficiency,
            practice_count: 1,
        };

        self.create_trace(TraceContent::Skill(skill), proficiency)
    }

    /// Add concept
    #[inline]
    pub fn add_concept(&mut self, name: &str, features: Vec<String>) -> u64 {
        let concept = ConceptInfo {
            name: name.into(),
            features,
            examples: Vec::new(),
        };

        self.create_trace(TraceContent::Concept(concept), 0.5)
    }

    /// Associate traces
    pub fn associate(&mut self, trace1: u64, trace2: u64, strength: f64) {
        let strength = strength.clamp(0.0, 1.0);

        // Add bidirectional association
        self.associations
            .entry(trace1)
            .or_default()
            .push((trace2, strength));

        self.associations
            .entry(trace2)
            .or_default()
            .push((trace1, strength));

        // Update trace associations
        if let Some(t1) = self.traces.get_mut(&trace1) {
            if !t1.associations.contains(&trace2) {
                t1.associations.push(trace2);
            }
        }

        if let Some(t2) = self.traces.get_mut(&trace2) {
            if !t2.associations.contains(&trace1) {
                t2.associations.push(trace1);
            }
        }
    }

    /// Rehearse trace
    #[inline]
    pub fn rehearse(&mut self, trace_id: u64) {
        if let Some(trace) = self.traces.get_mut(&trace_id) {
            trace.rehearsals += 1;
            trace.strength = (trace.strength + self.config.rehearsal_boost).min(1.0);
            trace.volatility *= 0.9;
            trace.last_accessed = Timestamp::now();
        }
    }

    /// Decay traces
    pub fn decay(&mut self) {
        let threshold = self.config.forgetting_threshold;
        let decay_rate = self.config.decay_rate;
        let mut to_forget = Vec::new();

        for (id, trace) in &mut self.traces {
            // Apply decay based on volatility
            trace.strength *= 1.0 - (decay_rate * trace.volatility);

            if trace.strength < threshold {
                to_forget.push(*id);
            }
        }

        // Remove forgotten traces
        for id in to_forget {
            self.traces.remove(&id);
            self.associations.remove(&id);
            self.stats.traces_forgotten += 1;
        }
    }

    /// Consolidate
    pub fn consolidate(&mut self) -> ConsolidationResult {
        let threshold = self.config.consolidation_threshold;
        let mut to_consolidate = Vec::new();
        let mut traces_consolidated = 0;
        let mut memories_created = 0;

        // Find traces ready for consolidation
        for trace in self.traces.values() {
            if trace.strength >= threshold && trace.volatility < 0.5 {
                to_consolidate.push(trace.clone());
            }
        }

        // Group related traces
        let groups = self.group_related(&to_consolidate);

        // Create consolidated memories
        for group in groups {
            if group.is_empty() {
                continue;
            }

            let memory = self.consolidate_group(&group);
            self.memories.insert(memory.id, memory);
            memories_created += 1;
            traces_consolidated += group.len();
            self.stats.traces_consolidated += group.len() as u64;
        }

        self.stats.memories_created += memories_created as u64;

        // Apply decay after consolidation
        self.decay();

        ConsolidationResult {
            traces_consolidated,
            memories_created,
            traces_forgotten: 0, // Updated by decay
            integration_score: self.calculate_integration(),
        }
    }

    fn group_related(&self, traces: &[MemoryTrace]) -> Vec<Vec<MemoryTrace>> {
        let mut groups = Vec::new();
        let mut assigned: Vec<bool> = vec![false; traces.len()];

        for (i, trace) in traces.iter().enumerate() {
            if assigned[i] {
                continue;
            }

            let mut group = vec![trace.clone()];
            assigned[i] = true;

            // Find related traces
            for (j, other) in traces.iter().enumerate() {
                if i == j || assigned[j] {
                    continue;
                }

                if self.are_related(trace, other) {
                    group.push(other.clone());
                    assigned[j] = true;
                }
            }

            groups.push(group);
        }

        groups
    }

    fn are_related(&self, t1: &MemoryTrace, t2: &MemoryTrace) -> bool {
        // Check associations
        if t1.associations.contains(&t2.id) {
            return true;
        }

        // Check content similarity
        match (&t1.content, &t2.content) {
            (TraceContent::Fact(f1), TraceContent::Fact(f2)) => {
                f1.subject == f2.subject || f1.object == f2.object
            },
            (TraceContent::Concept(c1), TraceContent::Concept(c2)) => {
                c1.features.iter().any(|f| c2.features.contains(f))
            },
            _ => false,
        }
    }

    fn consolidate_group(&self, group: &[MemoryTrace]) -> ConsolidatedMemory {
        let sources: Vec<u64> = group.iter().map(|t| t.id).collect();
        let avg_strength: f64 = group.iter().map(|t| t.strength).sum::<f64>() / group.len() as f64;

        // Merge content (simplified - take first)
        let content = group
            .first()
            .map(|t| t.content.clone())
            .unwrap_or(TraceContent::Concept(ConceptInfo {
                name: "merged".into(),
                features: Vec::new(),
                examples: Vec::new(),
            }));

        ConsolidatedMemory {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            sources,
            content,
            stability: avg_strength,
            integration: 1.0 - (1.0 / group.len() as f64),
            created: Timestamp::now(),
        }
    }

    fn calculate_integration(&self) -> f64 {
        if self.memories.is_empty() {
            return 0.0;
        }

        let total: f64 = self.memories.values().map(|m| m.integration).sum();

        total / self.memories.len() as f64
    }

    /// Get trace
    #[inline(always)]
    pub fn get_trace(&self, id: u64) -> Option<&MemoryTrace> {
        self.traces.get(&id)
    }

    /// Get memory
    #[inline(always)]
    pub fn get_memory(&self, id: u64) -> Option<&ConsolidatedMemory> {
        self.memories.get(&id)
    }

    /// Get strong traces
    #[inline]
    pub fn strong_traces(&self, min_strength: f64) -> Vec<&MemoryTrace> {
        self.traces
            .values()
            .filter(|t| t.strength >= min_strength)
            .collect()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ConsolidationStats {
        &self.stats
    }
}

impl Default for ConsolidationEngine {
    fn default() -> Self {
        Self::new(ConsolidationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_trace() {
        let mut engine = ConsolidationEngine::default();

        let id = engine.add_fact("sky", "is", "blue", 0.9);
        assert!(engine.get_trace(id).is_some());
    }

    #[test]
    fn test_rehearse() {
        let mut engine = ConsolidationEngine::default();

        let id = engine.add_fact("test", "is", "true", 0.5);

        let initial = engine.get_trace(id).unwrap().strength;

        engine.rehearse(id);

        let after = engine.get_trace(id).unwrap().strength;
        assert!(after > initial);
    }

    #[test]
    fn test_associate() {
        let mut engine = ConsolidationEngine::default();

        let t1 = engine.add_fact("cat", "is", "animal", 0.8);
        let t2 = engine.add_fact("dog", "is", "animal", 0.8);

        engine.associate(t1, t2, 0.7);

        let trace1 = engine.get_trace(t1).unwrap();
        assert!(trace1.associations.contains(&t2));
    }

    #[test]
    fn test_decay() {
        let mut engine = ConsolidationEngine::new(ConsolidationConfig {
            decay_rate: 0.5,
            forgetting_threshold: 0.3,
            ..Default::default()
        });

        engine.add_fact("weak", "is", "forgotten", 0.2);

        engine.decay();

        // Weak trace should be forgotten
        assert!(engine.traces.is_empty());
    }

    #[test]
    fn test_consolidate() {
        let mut engine = ConsolidationEngine::default();

        // Create strong related traces
        let t1 = engine.add_fact("cat", "has", "fur", 0.9);
        let t2 = engine.add_fact("cat", "has", "whiskers", 0.9);

        // Make them stable
        if let Some(trace) = engine.traces.get_mut(&t1) {
            trace.volatility = 0.3;
        }
        if let Some(trace) = engine.traces.get_mut(&t2) {
            trace.volatility = 0.3;
        }

        let result = engine.consolidate();
        assert!(result.memories_created > 0);
    }

    #[test]
    fn test_add_skill() {
        let mut engine = ConsolidationEngine::default();

        let id = engine.add_skill("programming", 0.7);
        let trace = engine.get_trace(id).unwrap();

        if let TraceContent::Skill(skill) = &trace.content {
            assert_eq!(skill.name, "programming");
        }
    }

    #[test]
    fn test_strong_traces() {
        let mut engine = ConsolidationEngine::default();

        engine.add_fact("strong", "is", "remembered", 0.9);
        engine.add_fact("weak", "is", "forgotten", 0.3);

        let strong = engine.strong_traces(0.5);
        assert_eq!(strong.len(), 1);
    }
}
