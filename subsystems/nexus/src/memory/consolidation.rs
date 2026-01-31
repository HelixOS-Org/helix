//! # Memory Consolidation
//!
//! Transfers memories from working memory to long-term storage.
//! Implements consolidation, compression, and organization.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CONSOLIDATION TYPES
// ============================================================================

/// Memory to consolidate
#[derive(Debug, Clone)]
pub struct ConsolidationCandidate {
    /// Source ID
    pub source_id: u64,
    /// Source type
    pub source_type: SourceType,
    /// Content
    pub content: MemoryContent,
    /// Importance
    pub importance: f64,
    /// Emotional valence (-1 to 1)
    pub valence: f64,
    /// Connections to existing memories
    pub connections: Vec<u64>,
    /// Repetition count
    pub repetitions: u32,
    /// Last accessed
    pub last_accessed: Timestamp,
}

/// Source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    WorkingMemory,
    Episodic,
    Semantic,
    Procedural,
}

/// Memory content
#[derive(Debug, Clone)]
pub enum MemoryContent {
    /// Factual knowledge
    Fact { subject: String, predicate: String, object: String },
    /// Episode
    Episode { events: Vec<String>, context: String },
    /// Skill/procedure
    Procedure { steps: Vec<String>, conditions: Vec<String> },
    /// Pattern
    Pattern { features: Vec<String>, examples: Vec<u64> },
    /// Abstract concept
    Concept { definition: String, relations: Vec<(String, u64)> },
}

/// Consolidated memory
#[derive(Debug, Clone)]
pub struct ConsolidatedMemory {
    /// Memory ID
    pub id: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Content
    pub content: MemoryContent,
    /// Strength
    pub strength: f64,
    /// Abstract level
    pub abstraction: AbstractionLevel,
    /// Linked memories
    pub links: Vec<MemoryLink>,
    /// Created
    pub created: Timestamp,
    /// Consolidation count
    pub consolidation_count: u32,
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
}

/// Abstraction level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbstractionLevel {
    Specific,
    Instance,
    Category,
    Abstract,
    Schema,
}

/// Memory link
#[derive(Debug, Clone)]
pub struct MemoryLink {
    /// Target memory
    pub target: u64,
    /// Link type
    pub link_type: LinkType,
    /// Strength
    pub strength: f64,
}

/// Link type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Temporal,      // Before/after
    Causal,        // Cause/effect
    Semantic,      // Meaning relation
    Similarity,    // Similar content
    Part,          // Part of
    Instance,      // Instance of
    Hierarchical,  // Parent/child
}

// ============================================================================
// CONSOLIDATION ENGINE
// ============================================================================

/// Memory consolidation engine
pub struct ConsolidationEngine {
    /// Pending candidates
    pending: Vec<ConsolidationCandidate>,
    /// Consolidated memories
    memories: BTreeMap<u64, ConsolidatedMemory>,
    /// Memory index by type
    by_type: BTreeMap<MemoryType, Vec<u64>>,
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
    /// Minimum importance for consolidation
    pub min_importance: f64,
    /// Minimum repetitions for consolidation
    pub min_repetitions: u32,
    /// Enable similarity merging
    pub merge_similar: bool,
    /// Similarity threshold for merging
    pub similarity_threshold: f64,
    /// Enable abstraction
    pub enable_abstraction: bool,
    /// Batch size for consolidation
    pub batch_size: usize,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            min_importance: 0.3,
            min_repetitions: 2,
            merge_similar: true,
            similarity_threshold: 0.8,
            enable_abstraction: true,
            batch_size: 10,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ConsolidationStats {
    /// Candidates processed
    pub candidates_processed: u64,
    /// Memories consolidated
    pub memories_consolidated: u64,
    /// Memories merged
    pub memories_merged: u64,
    /// Memories abstracted
    pub memories_abstracted: u64,
    /// Average strength
    pub avg_strength: f64,
}

impl ConsolidationEngine {
    /// Create new engine
    pub fn new(config: ConsolidationConfig) -> Self {
        Self {
            pending: Vec::new(),
            memories: BTreeMap::new(),
            by_type: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ConsolidationStats::default(),
        }
    }

    /// Add candidate for consolidation
    pub fn add_candidate(&mut self, candidate: ConsolidationCandidate) {
        self.pending.push(candidate);
    }

    /// Run consolidation pass
    pub fn consolidate(&mut self) -> Vec<u64> {
        let mut consolidated_ids = Vec::new();
        let batch_size = self.config.batch_size.min(self.pending.len());

        // Process batch
        for _ in 0..batch_size {
            if let Some(candidate) = self.pending.pop() {
                self.stats.candidates_processed += 1;

                // Check if eligible
                if !self.is_eligible(&candidate) {
                    continue;
                }

                // Check for similar existing memory
                if self.config.merge_similar {
                    if let Some(similar_id) = self.find_similar(&candidate) {
                        self.merge_into(similar_id, &candidate);
                        consolidated_ids.push(similar_id);
                        self.stats.memories_merged += 1;
                        continue;
                    }
                }

                // Create new consolidated memory
                let id = self.create_memory(&candidate);
                consolidated_ids.push(id);
                self.stats.memories_consolidated += 1;
            }
        }

        // Run abstraction
        if self.config.enable_abstraction {
            self.abstract_similar_memories();
        }

        self.update_avg_strength();
        consolidated_ids
    }

    fn is_eligible(&self, candidate: &ConsolidationCandidate) -> bool {
        candidate.importance >= self.config.min_importance ||
        candidate.repetitions >= self.config.min_repetitions ||
        candidate.valence.abs() > 0.5 // Emotional significance
    }

    fn find_similar(&self, candidate: &ConsolidationCandidate) -> Option<u64> {
        let memory_type = self.infer_type(candidate);

        let candidates = self.by_type.get(&memory_type)?;

        for &id in candidates {
            if let Some(memory) = self.memories.get(&id) {
                let similarity = self.compute_similarity(&candidate.content, &memory.content);
                if similarity >= self.config.similarity_threshold {
                    return Some(id);
                }
            }
        }

        None
    }

    fn infer_type(&self, candidate: &ConsolidationCandidate) -> MemoryType {
        match &candidate.content {
            MemoryContent::Episode { .. } => MemoryType::Episodic,
            MemoryContent::Procedure { .. } => MemoryType::Procedural,
            _ => MemoryType::Semantic,
        }
    }

    fn compute_similarity(&self, a: &MemoryContent, b: &MemoryContent) -> f64 {
        // Simplified similarity computation
        match (a, b) {
            (MemoryContent::Fact { subject: s1, predicate: p1, .. },
             MemoryContent::Fact { subject: s2, predicate: p2, .. }) => {
                let subject_match = if s1 == s2 { 0.5 } else { 0.0 };
                let predicate_match = if p1 == p2 { 0.5 } else { 0.0 };
                subject_match + predicate_match
            }
            (MemoryContent::Episode { context: c1, .. },
             MemoryContent::Episode { context: c2, .. }) => {
                if c1 == c2 { 0.8 } else { 0.3 }
            }
            (MemoryContent::Procedure { steps: s1, .. },
             MemoryContent::Procedure { steps: s2, .. }) => {
                let common = s1.iter().filter(|s| s2.contains(s)).count();
                common as f64 / s1.len().max(s2.len()) as f64
            }
            _ => 0.0,
        }
    }

    fn merge_into(&mut self, id: u64, candidate: &ConsolidationCandidate) {
        if let Some(memory) = self.memories.get_mut(&id) {
            // Strengthen memory
            memory.strength = (memory.strength + 0.1).min(1.0);
            memory.consolidation_count += 1;

            // Add new connections
            for &conn in &candidate.connections {
                if !memory.links.iter().any(|l| l.target == conn) {
                    memory.links.push(MemoryLink {
                        target: conn,
                        link_type: LinkType::Semantic,
                        strength: 0.5,
                    });
                }
            }
        }
    }

    fn create_memory(&mut self, candidate: &ConsolidationCandidate) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let memory_type = self.infer_type(candidate);

        let links: Vec<MemoryLink> = candidate.connections.iter()
            .map(|&target| MemoryLink {
                target,
                link_type: LinkType::Semantic,
                strength: 0.5,
            })
            .collect();

        let memory = ConsolidatedMemory {
            id,
            memory_type,
            content: candidate.content.clone(),
            strength: candidate.importance,
            abstraction: AbstractionLevel::Specific,
            links,
            created: Timestamp::now(),
            consolidation_count: 1,
        };

        self.memories.insert(id, memory);
        self.by_type.entry(memory_type).or_insert_with(Vec::new).push(id);

        id
    }

    fn abstract_similar_memories(&mut self) {
        // Find groups of similar memories
        let mut groups: Vec<Vec<u64>> = Vec::new();

        for &memory_type in &[MemoryType::Semantic, MemoryType::Episodic, MemoryType::Procedural] {
            if let Some(ids) = self.by_type.get(&memory_type) {
                let mut remaining: Vec<u64> = ids.clone();

                while !remaining.is_empty() {
                    let first = remaining.remove(0);
                    let mut group = vec![first];

                    let first_content = match self.memories.get(&first) {
                        Some(m) => m.content.clone(),
                        None => continue,
                    };

                    remaining.retain(|&id| {
                        if let Some(memory) = self.memories.get(&id) {
                            let sim = self.compute_similarity(&first_content, &memory.content);
                            if sim > 0.6 {
                                group.push(id);
                                return false;
                            }
                        }
                        true
                    });

                    if group.len() >= 3 {
                        groups.push(group);
                    }
                }
            }
        }

        // Create abstractions for groups
        for group in groups {
            self.create_abstraction(&group);
            self.stats.memories_abstracted += 1;
        }
    }

    fn create_abstraction(&mut self, group: &[u64]) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Create abstract memory
        let abstract_memory = ConsolidatedMemory {
            id,
            memory_type: MemoryType::Semantic,
            content: MemoryContent::Concept {
                definition: "Abstracted from similar memories".into(),
                relations: group.iter().map(|&g| ("instance".into(), g)).collect(),
            },
            strength: 0.8,
            abstraction: AbstractionLevel::Category,
            links: group.iter().map(|&target| MemoryLink {
                target,
                link_type: LinkType::Instance,
                strength: 0.9,
            }).collect(),
            created: Timestamp::now(),
            consolidation_count: 1,
        };

        self.memories.insert(id, abstract_memory);
        self.by_type.entry(MemoryType::Semantic)
            .or_insert_with(Vec::new)
            .push(id);

        // Link instances back to abstraction
        for &instance_id in group {
            if let Some(memory) = self.memories.get_mut(&instance_id) {
                memory.links.push(MemoryLink {
                    target: id,
                    link_type: LinkType::Hierarchical,
                    strength: 0.9,
                });
                memory.abstraction = AbstractionLevel::Instance;
            }
        }
    }

    fn update_avg_strength(&mut self) {
        if self.memories.is_empty() {
            self.stats.avg_strength = 0.0;
        } else {
            let total: f64 = self.memories.values().map(|m| m.strength).sum();
            self.stats.avg_strength = total / self.memories.len() as f64;
        }
    }

    /// Get memory
    pub fn get_memory(&self, id: u64) -> Option<&ConsolidatedMemory> {
        self.memories.get(&id)
    }

    /// Get memories by type
    pub fn get_by_type(&self, memory_type: MemoryType) -> Vec<&ConsolidatedMemory> {
        self.by_type.get(&memory_type)
            .map(|ids| ids.iter().filter_map(|id| self.memories.get(id)).collect())
            .unwrap_or_default()
    }

    /// Decay all memories
    pub fn decay(&mut self, factor: f64) {
        for memory in self.memories.values_mut() {
            memory.strength *= factor;
        }
    }

    /// Get statistics
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
    fn test_add_and_consolidate() {
        let mut engine = ConsolidationEngine::default();

        engine.add_candidate(ConsolidationCandidate {
            source_id: 1,
            source_type: SourceType::WorkingMemory,
            content: MemoryContent::Fact {
                subject: "sky".into(),
                predicate: "is".into(),
                object: "blue".into(),
            },
            importance: 0.8,
            valence: 0.0,
            connections: Vec::new(),
            repetitions: 3,
            last_accessed: Timestamp::now(),
        });

        let ids = engine.consolidate();
        assert_eq!(ids.len(), 1);
        assert!(engine.get_memory(ids[0]).is_some());
    }

    #[test]
    fn test_merge_similar() {
        let config = ConsolidationConfig {
            min_importance: 0.0,
            min_repetitions: 0,
            merge_similar: true,
            similarity_threshold: 0.5,
            ..Default::default()
        };
        let mut engine = ConsolidationEngine::new(config);

        // Add first memory
        engine.add_candidate(ConsolidationCandidate {
            source_id: 1,
            source_type: SourceType::Semantic,
            content: MemoryContent::Fact {
                subject: "cat".into(),
                predicate: "is".into(),
                object: "animal".into(),
            },
            importance: 0.8,
            valence: 0.0,
            connections: Vec::new(),
            repetitions: 1,
            last_accessed: Timestamp::now(),
        });

        engine.consolidate();

        // Add similar memory
        engine.add_candidate(ConsolidationCandidate {
            source_id: 2,
            source_type: SourceType::Semantic,
            content: MemoryContent::Fact {
                subject: "cat".into(),
                predicate: "is".into(),
                object: "pet".into(),
            },
            importance: 0.7,
            valence: 0.0,
            connections: Vec::new(),
            repetitions: 1,
            last_accessed: Timestamp::now(),
        });

        engine.consolidate();

        // Should have merged
        assert!(engine.stats().memories_merged >= 1);
    }

    #[test]
    fn test_decay() {
        let mut engine = ConsolidationEngine::default();

        engine.add_candidate(ConsolidationCandidate {
            source_id: 1,
            source_type: SourceType::WorkingMemory,
            content: MemoryContent::Fact {
                subject: "test".into(),
                predicate: "is".into(),
                object: "test".into(),
            },
            importance: 1.0,
            valence: 0.0,
            connections: Vec::new(),
            repetitions: 5,
            last_accessed: Timestamp::now(),
        });

        let ids = engine.consolidate();
        let initial_strength = engine.get_memory(ids[0]).unwrap().strength;

        engine.decay(0.9);

        let final_strength = engine.get_memory(ids[0]).unwrap().strength;
        assert!(final_strength < initial_strength);
    }
}
