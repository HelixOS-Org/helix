//! # Long-Term Memory Store
//!
//! Persistent storage for long-term memories.
//! Supports various memory types and retrieval strategies.
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
// LTM TYPES
// ============================================================================

/// Long-term memory
#[derive(Debug, Clone)]
pub struct LongTermMemory {
    /// Memory ID
    pub id: u64,
    /// Memory type
    pub memory_type: LtmType,
    /// Content
    pub content: MemoryContent,
    /// Metadata
    pub metadata: MemoryMetadata,
    /// Associations
    pub associations: Vec<Association>,
    /// Access pattern
    pub access: AccessPattern,
}

/// LTM type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtmType {
    /// Episodic (events, experiences)
    Episodic,
    /// Semantic (facts, concepts)
    Semantic,
    /// Procedural (skills, procedures)
    Procedural,
    /// Autobiographical (personal history)
    Autobiographical,
}

/// Memory content
#[derive(Debug, Clone)]
pub enum MemoryContent {
    /// Text content
    Text(String),
    /// Structured data
    Structured(BTreeMap<String, String>),
    /// Embedding
    Embedding(Vec<f32>),
    /// Reference to external storage
    Reference { location: String, checksum: u64 },
    /// Procedure
    Procedure { steps: Vec<String>, parameters: BTreeMap<String, String> },
}

/// Metadata
#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    /// Created
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Last modified
    pub last_modified: Timestamp,
    /// Source
    pub source: String,
    /// Tags
    pub tags: Vec<String>,
    /// Importance
    pub importance: f64,
    /// Confidence
    pub confidence: f64,
    /// Version
    pub version: u32,
}

impl Default for MemoryMetadata {
    fn default() -> Self {
        let now = Timestamp::now();
        Self {
            created: now,
            last_accessed: now,
            last_modified: now,
            source: String::new(),
            tags: Vec::new(),
            importance: 0.5,
            confidence: 1.0,
            version: 1,
        }
    }
}

/// Association between memories
#[derive(Debug, Clone)]
pub struct Association {
    /// Target memory ID
    pub target_id: u64,
    /// Association type
    pub assoc_type: AssociationType,
    /// Strength
    pub strength: f64,
}

/// Association type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationType {
    Similar,
    Causal,
    Temporal,
    Spatial,
    Conceptual,
    Part,
    Whole,
}

/// Access pattern
#[derive(Debug, Clone, Default)]
pub struct AccessPattern {
    /// Access count
    pub access_count: u64,
    /// Recent accesses (timestamps)
    pub recent_accesses: Vec<Timestamp>,
    /// Retrieval success rate
    pub retrieval_success: f64,
}

// ============================================================================
// LTM STORE
// ============================================================================

/// LTM store
pub struct LtmStore {
    /// Memories
    memories: BTreeMap<u64, LongTermMemory>,
    /// Type index
    type_index: BTreeMap<LtmType, Vec<u64>>,
    /// Tag index
    tag_index: BTreeMap<String, Vec<u64>>,
    /// Importance index (sorted by importance)
    importance_index: Vec<(u64, f64)>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: LtmConfig,
    /// Statistics
    stats: LtmStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct LtmConfig {
    /// Maximum memories
    pub max_memories: usize,
    /// Decay rate
    pub decay_rate: f64,
    /// Consolidation threshold
    pub consolidation_threshold: f64,
    /// Recent access window (ns)
    pub recent_access_window_ns: u64,
}

impl Default for LtmConfig {
    fn default() -> Self {
        Self {
            max_memories: 100_000,
            decay_rate: 0.01,
            consolidation_threshold: 0.7,
            recent_access_window_ns: 86_400_000_000_000, // 1 day
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct LtmStats {
    /// Total memories
    pub total_memories: u64,
    /// By type
    pub by_type: BTreeMap<String, u64>,
    /// Total accesses
    pub total_accesses: u64,
    /// Forgetting events
    pub forgetting_events: u64,
}

impl LtmStore {
    /// Create new store
    pub fn new(config: LtmConfig) -> Self {
        Self {
            memories: BTreeMap::new(),
            type_index: BTreeMap::new(),
            tag_index: BTreeMap::new(),
            importance_index: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: LtmStats::default(),
        }
    }

    /// Store memory
    pub fn store(
        &mut self,
        memory_type: LtmType,
        content: MemoryContent,
        metadata: MemoryMetadata,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let memory = LongTermMemory {
            id,
            memory_type,
            content,
            metadata: metadata.clone(),
            associations: Vec::new(),
            access: AccessPattern::default(),
        };

        // Update indexes
        self.type_index.entry(memory_type)
            .or_insert_with(Vec::new)
            .push(id);

        for tag in &metadata.tags {
            self.tag_index.entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.importance_index.push((id, metadata.importance));
        self.importance_index.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        self.memories.insert(id, memory);
        self.update_type_stats(memory_type, 1);
        self.stats.total_memories += 1;

        // Clean up if over limit
        if self.memories.len() > self.config.max_memories {
            self.forget_least_important();
        }

        id
    }

    fn update_type_stats(&mut self, memory_type: LtmType, delta: i64) {
        let key = format!("{:?}", memory_type);
        let entry = self.stats.by_type.entry(key).or_insert(0);
        if delta > 0 {
            *entry += delta as u64;
        } else {
            *entry = entry.saturating_sub((-delta) as u64);
        }
    }

    /// Retrieve memory
    pub fn retrieve(&mut self, id: u64) -> Option<&LongTermMemory> {
        if let Some(memory) = self.memories.get_mut(&id) {
            let now = Timestamp::now();
            memory.metadata.last_accessed = now;
            memory.access.access_count += 1;
            memory.access.recent_accesses.push(now);

            // Trim old accesses
            let cutoff = Timestamp(now.0.saturating_sub(self.config.recent_access_window_ns));
            memory.access.recent_accesses.retain(|t| t.0 >= cutoff.0);

            self.stats.total_accesses += 1;
        }

        self.memories.get(&id)
    }

    /// Update memory
    pub fn update(&mut self, id: u64, content: MemoryContent) -> bool {
        if let Some(memory) = self.memories.get_mut(&id) {
            memory.content = content;
            memory.metadata.last_modified = Timestamp::now();
            memory.metadata.version += 1;
            return true;
        }
        false
    }

    /// Add association
    pub fn associate(&mut self, from_id: u64, to_id: u64, assoc_type: AssociationType, strength: f64) {
        if let Some(memory) = self.memories.get_mut(&from_id) {
            // Check if association exists
            if let Some(assoc) = memory.associations.iter_mut()
                .find(|a| a.target_id == to_id && a.assoc_type == assoc_type) {
                assoc.strength = (assoc.strength + strength) / 2.0;
            } else {
                memory.associations.push(Association {
                    target_id: to_id,
                    assoc_type,
                    strength,
                });
            }
        }
    }

    /// Query by type
    pub fn by_type(&self, memory_type: LtmType) -> Vec<&LongTermMemory> {
        self.type_index.get(&memory_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.memories.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Query by tag
    pub fn by_tag(&self, tag: &str) -> Vec<&LongTermMemory> {
        self.tag_index.get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.memories.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get most important
    pub fn most_important(&self, count: usize) -> Vec<&LongTermMemory> {
        self.importance_index.iter()
            .take(count)
            .filter_map(|(id, _)| self.memories.get(id))
            .collect()
    }

    /// Get associated memories
    pub fn get_associated(&self, id: u64) -> Vec<(AssociationType, &LongTermMemory)> {
        let memory = match self.memories.get(&id) {
            Some(m) => m,
            None => return Vec::new(),
        };

        memory.associations.iter()
            .filter_map(|assoc| {
                self.memories.get(&assoc.target_id)
                    .map(|m| (assoc.assoc_type, m))
            })
            .collect()
    }

    /// Forget (remove) memory
    pub fn forget(&mut self, id: u64) -> bool {
        if let Some(memory) = self.memories.remove(&id) {
            // Update indexes
            if let Some(ids) = self.type_index.get_mut(&memory.memory_type) {
                ids.retain(|&i| i != id);
            }

            for tag in &memory.metadata.tags {
                if let Some(ids) = self.tag_index.get_mut(tag) {
                    ids.retain(|&i| i != id);
                }
            }

            self.importance_index.retain(|(i, _)| *i != id);

            self.update_type_stats(memory.memory_type, -1);
            self.stats.total_memories -= 1;
            self.stats.forgetting_events += 1;

            return true;
        }
        false
    }

    fn forget_least_important(&mut self) {
        if let Some((id, _)) = self.importance_index.pop() {
            self.forget(id);
        }
    }

    /// Decay memories (reduce importance over time)
    pub fn decay(&mut self) {
        for memory in self.memories.values_mut() {
            let age_factor = 1.0 / (memory.access.access_count as f64 + 1.0);
            memory.metadata.importance *= 1.0 - (self.config.decay_rate * age_factor);
        }

        // Re-sort importance index
        for (id, importance) in &mut self.importance_index {
            if let Some(memory) = self.memories.get(id) {
                *importance = memory.metadata.importance;
            }
        }
        self.importance_index.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    }

    /// Consolidate memories (strengthen frequently accessed)
    pub fn consolidate(&mut self) {
        for memory in self.memories.values_mut() {
            if memory.access.access_count > 10 {
                let boost = (memory.access.access_count as f64).ln() * 0.01;
                memory.metadata.importance = (memory.metadata.importance + boost).min(1.0);
            }
        }

        // Update importance index
        for (id, importance) in &mut self.importance_index {
            if let Some(memory) = self.memories.get(id) {
                *importance = memory.metadata.importance;
            }
        }
        self.importance_index.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    }

    /// Get memory
    pub fn get(&self, id: u64) -> Option<&LongTermMemory> {
        self.memories.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &LtmStats {
        &self.stats
    }
}

impl Default for LtmStore {
    fn default() -> Self {
        Self::new(LtmConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_memory() {
        let mut store = LtmStore::default();

        let id = store.store(
            LtmType::Semantic,
            MemoryContent::Text("Paris is the capital of France".into()),
            MemoryMetadata::default(),
        );

        assert!(store.get(id).is_some());
    }

    #[test]
    fn test_retrieve() {
        let mut store = LtmStore::default();

        let id = store.store(
            LtmType::Semantic,
            MemoryContent::Text("Test".into()),
            MemoryMetadata::default(),
        );

        let _ = store.retrieve(id);
        let _ = store.retrieve(id);

        let memory = store.get(id).unwrap();
        assert_eq!(memory.access.access_count, 2);
    }

    #[test]
    fn test_associations() {
        let mut store = LtmStore::default();

        let id1 = store.store(
            LtmType::Semantic,
            MemoryContent::Text("Dog".into()),
            MemoryMetadata::default(),
        );

        let id2 = store.store(
            LtmType::Semantic,
            MemoryContent::Text("Cat".into()),
            MemoryMetadata::default(),
        );

        store.associate(id1, id2, AssociationType::Similar, 0.8);

        let associated = store.get_associated(id1);
        assert_eq!(associated.len(), 1);
    }

    #[test]
    fn test_query_by_type() {
        let mut store = LtmStore::default();

        store.store(LtmType::Semantic, MemoryContent::Text("Fact".into()), MemoryMetadata::default());
        store.store(LtmType::Episodic, MemoryContent::Text("Event".into()), MemoryMetadata::default());

        let semantic = store.by_type(LtmType::Semantic);
        assert_eq!(semantic.len(), 1);
    }

    #[test]
    fn test_forget() {
        let mut store = LtmStore::default();

        let id = store.store(
            LtmType::Semantic,
            MemoryContent::Text("Temporary".into()),
            MemoryMetadata::default(),
        );

        assert!(store.forget(id));
        assert!(store.get(id).is_none());
    }

    #[test]
    fn test_importance() {
        let mut store = LtmStore::default();

        let mut high_importance = MemoryMetadata::default();
        high_importance.importance = 0.9;

        let mut low_importance = MemoryMetadata::default();
        low_importance.importance = 0.1;

        store.store(LtmType::Semantic, MemoryContent::Text("Low".into()), low_importance);
        store.store(LtmType::Semantic, MemoryContent::Text("High".into()), high_importance);

        let most_important = store.most_important(1);
        if let MemoryContent::Text(text) = &most_important[0].content {
            assert_eq!(text, "High");
        }
    }
}
