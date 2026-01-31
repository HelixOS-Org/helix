//! # Working Memory
//!
//! Short-term working memory for active task processing.
//! Limited capacity buffer for current context and operations.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// WORKING MEMORY TYPES
// ============================================================================

/// Working memory item
#[derive(Debug, Clone)]
pub struct WorkingMemoryItem {
    /// Item ID
    pub id: u64,
    /// Item type
    pub item_type: ItemType,
    /// Content
    pub content: MemoryContent,
    /// Priority
    pub priority: Priority,
    /// Activation level
    pub activation: f64,
    /// Access count
    pub access_count: u32,
    /// Created
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Time to live (ns, None = no expiry)
    pub ttl_ns: Option<u64>,
}

/// Item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    /// Current goal
    Goal,
    /// Active context
    Context,
    /// Intermediate result
    Result,
    /// Pending operation
    Operation,
    /// Attention focus
    Focus,
    /// Retrieved memory
    Retrieved,
}

/// Memory content
#[derive(Debug, Clone)]
pub enum MemoryContent {
    /// Text content
    Text(String),
    /// Numeric value
    Number(f64),
    /// Structured data
    Structured(BTreeMap<String, String>),
    /// Reference to external item
    Reference(u64),
    /// Composite (multiple items)
    Composite(Vec<u64>),
}

/// Priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background,
    Low,
    Normal,
    High,
    Critical,
}

// ============================================================================
// WORKING MEMORY STORE
// ============================================================================

/// Working memory store
pub struct WorkingMemory {
    /// Items
    items: BTreeMap<u64, WorkingMemoryItem>,
    /// Item order (for FIFO eviction)
    order: VecDeque<u64>,
    /// By type
    by_type: BTreeMap<ItemType, Vec<u64>>,
    /// Current focus
    focus_stack: Vec<u64>,
    /// Capacity
    capacity: usize,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: WorkingMemoryConfig,
    /// Statistics
    stats: WorkingMemoryStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct WorkingMemoryConfig {
    /// Maximum capacity
    pub max_capacity: usize,
    /// Activation decay rate (per access)
    pub decay_rate: f64,
    /// Minimum activation before eviction
    pub min_activation: f64,
    /// Default TTL (ns)
    pub default_ttl_ns: Option<u64>,
    /// Enable automatic eviction
    pub auto_evict: bool,
}

impl Default for WorkingMemoryConfig {
    fn default() -> Self {
        Self {
            max_capacity: 7, // Miller's law: 7 +/- 2
            decay_rate: 0.1,
            min_activation: 0.1,
            default_ttl_ns: Some(300_000_000_000), // 5 minutes
            auto_evict: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct WorkingMemoryStats {
    /// Items stored
    pub items_stored: u64,
    /// Items evicted
    pub items_evicted: u64,
    /// Items expired
    pub items_expired: u64,
    /// Access count
    pub accesses: u64,
    /// Average activation
    pub avg_activation: f64,
}

impl WorkingMemory {
    /// Create new working memory
    pub fn new(config: WorkingMemoryConfig) -> Self {
        Self {
            items: BTreeMap::new(),
            order: VecDeque::new(),
            by_type: BTreeMap::new(),
            focus_stack: Vec::new(),
            capacity: config.max_capacity,
            next_id: AtomicU64::new(1),
            config,
            stats: WorkingMemoryStats::default(),
        }
    }

    /// Store item
    pub fn store(&mut self, item_type: ItemType, content: MemoryContent, priority: Priority) -> u64 {
        // Check capacity and evict if needed
        if self.config.auto_evict {
            while self.items.len() >= self.capacity {
                self.evict_lowest();
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let item = WorkingMemoryItem {
            id,
            item_type,
            content,
            priority,
            activation: 1.0,
            access_count: 0,
            created: now,
            last_accessed: now,
            ttl_ns: self.config.default_ttl_ns,
        };

        self.items.insert(id, item);
        self.order.push_back(id);
        self.by_type.entry(item_type).or_insert_with(Vec::new).push(id);

        self.stats.items_stored += 1;
        self.update_avg_activation();

        id
    }

    /// Get item
    pub fn get(&mut self, id: u64) -> Option<&WorkingMemoryItem> {
        // Check expiry
        if let Some(item) = self.items.get(&id) {
            if self.is_expired(item) {
                self.remove(id);
                return None;
            }
        }

        // Boost activation on access
        if let Some(item) = self.items.get_mut(&id) {
            item.activation = (item.activation + 0.2).min(1.0);
            item.access_count += 1;
            item.last_accessed = Timestamp::now();
            self.stats.accesses += 1;
        }

        self.items.get(&id)
    }

    /// Get without boosting activation
    pub fn peek(&self, id: u64) -> Option<&WorkingMemoryItem> {
        self.items.get(&id).filter(|item| !self.is_expired(item))
    }

    /// Get items by type
    pub fn get_by_type(&self, item_type: ItemType) -> Vec<&WorkingMemoryItem> {
        self.by_type.get(&item_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.items.get(id))
                    .filter(|item| !self.is_expired(item))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update item content
    pub fn update(&mut self, id: u64, content: MemoryContent) -> bool {
        if let Some(item) = self.items.get_mut(&id) {
            item.content = content;
            item.activation = (item.activation + 0.3).min(1.0);
            item.last_accessed = Timestamp::now();
            true
        } else {
            false
        }
    }

    /// Set item priority
    pub fn set_priority(&mut self, id: u64, priority: Priority) {
        if let Some(item) = self.items.get_mut(&id) {
            item.priority = priority;
        }
    }

    /// Remove item
    pub fn remove(&mut self, id: u64) -> Option<WorkingMemoryItem> {
        if let Some(item) = self.items.remove(&id) {
            // Remove from order
            self.order.retain(|&x| x != id);

            // Remove from type index
            if let Some(ids) = self.by_type.get_mut(&item.item_type) {
                ids.retain(|&x| x != id);
            }

            // Remove from focus if present
            self.focus_stack.retain(|&x| x != id);

            Some(item)
        } else {
            None
        }
    }

    /// Push focus
    pub fn push_focus(&mut self, id: u64) {
        if self.items.contains_key(&id) {
            self.focus_stack.push(id);
        }
    }

    /// Pop focus
    pub fn pop_focus(&mut self) -> Option<u64> {
        self.focus_stack.pop()
    }

    /// Get current focus
    pub fn current_focus(&self) -> Option<&WorkingMemoryItem> {
        self.focus_stack.last().and_then(|&id| self.items.get(&id))
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
        self.order.clear();
        self.by_type.clear();
        self.focus_stack.clear();
    }

    /// Apply decay
    pub fn apply_decay(&mut self) {
        let expired: Vec<u64> = self.items.iter()
            .filter_map(|(&id, item)| {
                if self.is_expired(item) {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();

        for id in expired {
            self.remove(id);
            self.stats.items_expired += 1;
        }

        // Decay activation levels
        for item in self.items.values_mut() {
            item.activation = (item.activation - self.config.decay_rate).max(0.0);
        }

        // Evict items below minimum activation
        let to_evict: Vec<u64> = self.items.iter()
            .filter(|(_, item)| item.activation < self.config.min_activation)
            .map(|(&id, _)| id)
            .collect();

        for id in to_evict {
            self.remove(id);
            self.stats.items_evicted += 1;
        }

        self.update_avg_activation();
    }

    fn is_expired(&self, item: &WorkingMemoryItem) -> bool {
        if let Some(ttl) = item.ttl_ns {
            let now = Timestamp::now();
            let elapsed = now.0.saturating_sub(item.created.0);
            elapsed > ttl
        } else {
            false
        }
    }

    fn evict_lowest(&mut self) {
        // Find item with lowest priority and activation
        let lowest = self.items.iter()
            .filter(|(&id, _)| !self.focus_stack.contains(&id))
            .min_by(|(_, a), (_, b)| {
                match a.priority.cmp(&b.priority) {
                    core::cmp::Ordering::Equal => {
                        a.activation.partial_cmp(&b.activation)
                            .unwrap_or(core::cmp::Ordering::Equal)
                    }
                    other => other,
                }
            })
            .map(|(&id, _)| id);

        if let Some(id) = lowest {
            self.remove(id);
            self.stats.items_evicted += 1;
        }
    }

    fn update_avg_activation(&mut self) {
        if self.items.is_empty() {
            self.stats.avg_activation = 0.0;
        } else {
            let total: f64 = self.items.values().map(|i| i.activation).sum();
            self.stats.avg_activation = total / self.items.len() as f64;
        }
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get all items
    pub fn all_items(&self) -> impl Iterator<Item = &WorkingMemoryItem> {
        self.items.values()
    }

    /// Get statistics
    pub fn stats(&self) -> &WorkingMemoryStats {
        &self.stats
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new(WorkingMemoryConfig::default())
    }
}

// ============================================================================
// CHUNKING
// ============================================================================

/// Chunking helper for grouping items
pub struct Chunker {
    /// Maximum chunk size
    max_chunk_size: usize,
}

impl Chunker {
    /// Create new chunker
    pub fn new(max_chunk_size: usize) -> Self {
        Self { max_chunk_size }
    }

    /// Chunk items into groups
    pub fn chunk(&self, items: &[u64]) -> Vec<Vec<u64>> {
        items.chunks(self.max_chunk_size)
            .map(|c| c.to_vec())
            .collect()
    }

    /// Create composite from chunks
    pub fn create_composite(memory: &mut WorkingMemory, chunk: &[u64]) -> u64 {
        memory.store(
            ItemType::Context,
            MemoryContent::Composite(chunk.to_vec()),
            Priority::Normal,
        )
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new(4)
    }
}

// ============================================================================
// REHEARSAL
// ============================================================================

/// Rehearsal to maintain items
pub struct Rehearsal {
    /// Items being rehearsed
    items: Vec<u64>,
    /// Rehearsal interval (ns)
    interval_ns: u64,
    /// Last rehearsal
    last_rehearsal: Timestamp,
}

impl Rehearsal {
    /// Create new rehearsal
    pub fn new(interval_ns: u64) -> Self {
        Self {
            items: Vec::new(),
            interval_ns,
            last_rehearsal: Timestamp::now(),
        }
    }

    /// Add item to rehearsal
    pub fn add(&mut self, id: u64) {
        if !self.items.contains(&id) {
            self.items.push(id);
        }
    }

    /// Remove item from rehearsal
    pub fn remove(&mut self, id: u64) {
        self.items.retain(|&x| x != id);
    }

    /// Perform rehearsal
    pub fn rehearse(&mut self, memory: &mut WorkingMemory) {
        let now = Timestamp::now();
        if now.0 - self.last_rehearsal.0 < self.interval_ns {
            return;
        }

        // Access each item to boost activation
        for &id in &self.items {
            memory.get(id);
        }

        self.last_rehearsal = now;
    }

    /// Get rehearsal items
    pub fn items(&self) -> &[u64] {
        &self.items
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_storage() {
        let mut memory = WorkingMemory::default();

        let id = memory.store(
            ItemType::Context,
            MemoryContent::Text("test".into()),
            Priority::Normal,
        );

        assert!(memory.get(id).is_some());
        assert_eq!(memory.len(), 1);
    }

    #[test]
    fn test_capacity_eviction() {
        let config = WorkingMemoryConfig {
            max_capacity: 3,
            ..Default::default()
        };
        let mut memory = WorkingMemory::new(config);

        for i in 0..5 {
            memory.store(
                ItemType::Context,
                MemoryContent::Number(i as f64),
                Priority::Normal,
            );
        }

        assert!(memory.len() <= 3);
    }

    #[test]
    fn test_priority_eviction() {
        let config = WorkingMemoryConfig {
            max_capacity: 2,
            ..Default::default()
        };
        let mut memory = WorkingMemory::new(config);

        let low = memory.store(ItemType::Context, MemoryContent::Text("low".into()), Priority::Low);
        let high = memory.store(ItemType::Context, MemoryContent::Text("high".into()), Priority::High);
        memory.store(ItemType::Context, MemoryContent::Text("normal".into()), Priority::Normal);

        // Low priority should be evicted
        assert!(memory.peek(low).is_none());
        assert!(memory.peek(high).is_some());
    }

    #[test]
    fn test_focus_stack() {
        let mut memory = WorkingMemory::default();

        let id1 = memory.store(ItemType::Focus, MemoryContent::Text("1".into()), Priority::Normal);
        let id2 = memory.store(ItemType::Focus, MemoryContent::Text("2".into()), Priority::Normal);

        memory.push_focus(id1);
        memory.push_focus(id2);

        assert_eq!(memory.current_focus().map(|i| i.id), Some(id2));
        assert_eq!(memory.pop_focus(), Some(id2));
        assert_eq!(memory.current_focus().map(|i| i.id), Some(id1));
    }

    #[test]
    fn test_activation_decay() {
        let config = WorkingMemoryConfig {
            decay_rate: 0.5,
            min_activation: 0.3,
            ..Default::default()
        };
        let mut memory = WorkingMemory::new(config);

        let id = memory.store(ItemType::Context, MemoryContent::Text("test".into()), Priority::Normal);

        // Multiple decays should eventually evict
        for _ in 0..10 {
            memory.apply_decay();
        }

        assert!(memory.peek(id).is_none());
    }

    #[test]
    fn test_chunking() {
        let mut memory = WorkingMemory::default();
        let chunker = Chunker::new(3);

        let items: Vec<u64> = (0..10).map(|i| {
            memory.store(ItemType::Context, MemoryContent::Number(i as f64), Priority::Normal)
        }).collect();

        let chunks = chunker.chunk(&items);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].len(), 3);
    }
}
