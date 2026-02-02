//! # AI Memory Subsystem
//!
//! The AI Memory subsystem provides persistent and efficient storage for learned
//! patterns, models, decision history, and operational knowledge.
//!
//! ## Memory Regions
//!
//! - **Working Memory**: Fast, volatile storage for current context
//! - **Short-Term Memory**: Recent events and decisions
//! - **Long-Term Memory**: Persistent patterns and models
//! - **Episodic Memory**: Historical decision outcomes
//!
//! ## Architecture
//!
//! ```text
//!                     ┌─────────────────────────────────────────────┐
//!                     │              AI Memory System               │
//!                     │                                             │
//!    Store ─────────►│  ┌─────────────────────────────────────┐    │
//!                     │  │         Memory Controller          │    │
//!    Query ─────────►│  │   - Allocation                     │    │
//!                     │  │   - Garbage Collection            │    │
//!                     │  │   - Compression                   │    │
//!                     │  └─────────────────┬───────────────────┘    │
//!                     │                    │                        │
//!                     │  ┌─────────────────┼───────────────────┐    │
//!                     │  │                 ▼                   │    │
//!                     │  │   ┌─────────────────────────────┐   │    │
//!                     │  │   │     Working Memory          │   │    │
//!                     │  │   │   (Current Context)         │   │    │
//!                     │  │   └─────────────────────────────┘   │    │
//!                     │  │                 │                   │    │
//!                     │  │                 ▼                   │    │
//!                     │  │   ┌─────────────────────────────┐   │    │
//!                     │  │   │    Short-Term Memory        │   │    │
//!                     │  │   │   (Recent Events)           │   │    │
//!                     │  │   └─────────────────────────────┘   │    │
//!                     │  │                 │                   │    │
//!                     │  │                 ▼                   │    │
//!                     │  │   ┌─────────────────────────────┐   │    │
//!                     │  │   │    Long-Term Memory         │───────────► Persist
//!                     │  │   │   (Patterns & Models)       │   │    │
//!                     │  │   └─────────────────────────────┘   │    │
//!                     │  │                                     │    │
//!                     │  └─────────────────────────────────────┘    │
//!                     │                                             │
//!                     └─────────────────────────────────────────────┘
//! ```

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use crate::core::{Confidence, DecisionId};

// =============================================================================
// Memory Entry Types
// =============================================================================

/// Unique identifier for memory entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemoryId(u64);

impl MemoryId {
    /// Create a new unique ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Get the raw value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

/// A generic memory entry
#[derive(Debug, Clone)]
pub struct MemoryEntry<T> {
    /// Unique ID
    pub id: MemoryId,
    /// The stored data
    pub data: T,
    /// Creation timestamp
    pub created_at: u64,
    /// Last access timestamp
    pub last_accessed: u64,
    /// Access count
    pub access_count: u64,
    /// Memory importance (0.0 - 1.0)
    pub importance: f32,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Time-to-live (microseconds, 0 = forever)
    pub ttl_us: u64,
}

impl<T: Clone> MemoryEntry<T> {
    /// Create a new entry
    pub fn new(data: T, importance: f32) -> Self {
        Self {
            id: MemoryId::new(),
            data,
            created_at: 0, // Would be set to actual time
            last_accessed: 0,
            access_count: 0,
            importance: importance.clamp(0.0, 1.0),
            tags: Vec::new(),
            ttl_us: 0,
        }
    }

    /// Create with TTL
    pub fn with_ttl(data: T, importance: f32, ttl_us: u64) -> Self {
        let mut entry = Self::new(data, importance);
        entry.ttl_us = ttl_us;
        entry
    }

    /// Add a tag
    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Check if expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        if self.ttl_us == 0 {
            return false;
        }
        current_time > self.created_at + self.ttl_us
    }

    /// Update access time
    pub fn touch(&mut self, current_time: u64) {
        self.last_accessed = current_time;
        self.access_count += 1;
    }

    /// Calculate memory score (for eviction decisions)
    pub fn memory_score(&self, current_time: u64) -> f32 {
        let age = (current_time.saturating_sub(self.last_accessed)) as f32;
        let recency = 1.0 / (1.0 + age / 1_000_000.0); // Decay over seconds
        let frequency = crate::math::ln_1p_f32(self.access_count as f32) / 10.0;

        self.importance * 0.5 + recency * 0.3 + frequency * 0.2
    }
}

// =============================================================================
// Memory Regions
// =============================================================================

/// Working memory for current context
#[derive(Debug)]
pub struct WorkingMemory {
    /// Current context variables
    context: BTreeMap<String, ContextValue>,
    /// Maximum variables
    max_entries: usize,
    /// Current timestamp
    current_time: u64,
}

/// A context value
#[derive(Debug, Clone)]
pub enum ContextValue {
    Number(f64),
    Text(String),
    List(Vec<f64>),
    Flag(bool),
}

impl WorkingMemory {
    /// Create a new working memory
    pub fn new(max_entries: usize) -> Self {
        Self {
            context: BTreeMap::new(),
            max_entries,
            current_time: 0,
        }
    }

    /// Set a value
    pub fn set(&mut self, key: &str, value: ContextValue) {
        if self.context.len() >= self.max_entries && !self.context.contains_key(key) {
            // Remove oldest or least important
            if let Some(oldest) = self.context.keys().next().cloned() {
                self.context.remove(&oldest);
            }
        }
        self.context.insert(key.to_string(), value);
    }

    /// Get a value
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.context.get(key)
    }

    /// Get numeric value
    pub fn get_number(&self, key: &str) -> Option<f64> {
        match self.context.get(key) {
            Some(ContextValue::Number(n)) => Some(*n),
            _ => None,
        }
    }

    /// Get text value
    pub fn get_text(&self, key: &str) -> Option<&str> {
        match self.context.get(key) {
            Some(ContextValue::Text(s)) => Some(s),
            _ => None,
        }
    }

    /// Get flag value
    pub fn get_flag(&self, key: &str) -> bool {
        match self.context.get(key) {
            Some(ContextValue::Flag(b)) => *b,
            _ => false,
        }
    }

    /// Remove a value
    pub fn remove(&mut self, key: &str) -> Option<ContextValue> {
        self.context.remove(key)
    }

    /// Clear all
    pub fn clear(&mut self) {
        self.context.clear();
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.context.keys()
    }

    /// Count entries
    pub fn len(&self) -> usize {
        self.context.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.context.is_empty()
    }
}

/// Short-term memory for recent events
#[derive(Debug)]
pub struct ShortTermMemory<T> {
    /// Recent entries
    entries: VecDeque<MemoryEntry<T>>,
    /// Maximum entries
    capacity: usize,
    /// Current time
    current_time: u64,
}

impl<T: Clone> ShortTermMemory<T> {
    /// Create new short-term memory
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
            current_time: 0,
        }
    }

    /// Add an entry
    pub fn add(&mut self, mut entry: MemoryEntry<T>) {
        entry.created_at = self.current_time;
        entry.last_accessed = self.current_time;

        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Get entry by ID
    pub fn get(&mut self, id: MemoryId) -> Option<&T> {
        self.entries.iter_mut().find(|e| e.id == id).map(|e| {
            e.touch(self.current_time);
            &e.data
        })
    }

    /// Get most recent
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &MemoryEntry<T>> {
        self.entries.iter().rev().take(count)
    }

    /// Find by predicate
    pub fn find<F>(&self, predicate: F) -> Option<&MemoryEntry<T>>
    where
        F: Fn(&T) -> bool,
    {
        self.entries.iter().find(|e| predicate(&e.data))
    }

    /// Find all by predicate
    pub fn find_all<F>(&self, predicate: F) -> Vec<&MemoryEntry<T>>
    where
        F: Fn(&T) -> bool,
    {
        self.entries.iter().filter(|e| predicate(&e.data)).collect()
    }

    /// Find by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&MemoryEntry<T>> {
        self.entries
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Update time
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Clean expired entries
    pub fn clean_expired(&mut self) {
        self.entries.retain(|e| !e.is_expired(self.current_time));
    }

    /// Count entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Long-term memory for persistent patterns
#[derive(Debug)]
pub struct LongTermMemory<T> {
    /// Stored entries indexed by ID
    entries: BTreeMap<MemoryId, MemoryEntry<T>>,
    /// Index by tag
    tag_index: BTreeMap<String, Vec<MemoryId>>,
    /// Capacity
    capacity: usize,
    /// Current time
    current_time: u64,
    /// Total bytes used (estimate)
    bytes_used: u64,
    /// Maximum bytes
    max_bytes: u64,
}

impl<T: Clone> LongTermMemory<T> {
    /// Create new long-term memory
    pub fn new(capacity: usize, max_bytes: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            tag_index: BTreeMap::new(),
            capacity,
            current_time: 0,
            bytes_used: 0,
            max_bytes,
        }
    }

    /// Store an entry
    pub fn store(
        &mut self,
        mut entry: MemoryEntry<T>,
        estimated_size: u64,
    ) -> Result<MemoryId, MemoryError> {
        // Check capacity
        if self.entries.len() >= self.capacity {
            self.evict_lowest_score()?;
        }

        // Check memory
        if self.bytes_used + estimated_size > self.max_bytes {
            while self.bytes_used + estimated_size > self.max_bytes && !self.entries.is_empty() {
                self.evict_lowest_score()?;
            }
        }

        entry.created_at = self.current_time;
        entry.last_accessed = self.current_time;
        let id = entry.id;

        // Update tag index
        for tag in &entry.tags {
            self.tag_index.entry(tag.clone()).or_default().push(id);
        }

        self.entries.insert(id, entry);
        self.bytes_used += estimated_size;

        Ok(id)
    }

    /// Retrieve an entry
    pub fn retrieve(&mut self, id: MemoryId) -> Option<&T> {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.touch(self.current_time);
            Some(&entry.data)
        } else {
            None
        }
    }

    /// Get entry metadata
    pub fn metadata(&self, id: MemoryId) -> Option<MemoryMetadata> {
        self.entries.get(&id).map(|e| MemoryMetadata {
            id: e.id,
            created_at: e.created_at,
            last_accessed: e.last_accessed,
            access_count: e.access_count,
            importance: e.importance,
            tags: e.tags.clone(),
        })
    }

    /// Find by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<MemoryId> {
        self.tag_index.get(tag).cloned().unwrap_or_default()
    }

    /// Find by importance threshold
    pub fn find_important(&self, threshold: f32) -> Vec<MemoryId> {
        self.entries
            .iter()
            .filter(|(_, e)| e.importance >= threshold)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Remove an entry
    pub fn remove(&mut self, id: MemoryId) -> Option<T> {
        if let Some(entry) = self.entries.remove(&id) {
            // Update tag index
            for tag in &entry.tags {
                if let Some(ids) = self.tag_index.get_mut(tag) {
                    ids.retain(|&i| i != id);
                }
            }
            Some(entry.data)
        } else {
            None
        }
    }

    /// Evict lowest scored entry
    fn evict_lowest_score(&mut self) -> Result<(), MemoryError> {
        let lowest = self
            .entries
            .iter()
            .min_by(|(_, a), (_, b)| {
                a.memory_score(self.current_time)
                    .partial_cmp(&b.memory_score(self.current_time))
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id);

        if let Some(id) = lowest {
            self.remove(id);
            Ok(())
        } else {
            Err(MemoryError::OutOfMemory)
        }
    }

    /// Update importance of an entry
    pub fn update_importance(&mut self, id: MemoryId, importance: f32) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.importance = importance.clamp(0.0, 1.0);
        }
    }

    /// Set current time
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Clean expired entries
    pub fn clean_expired(&mut self) {
        let expired: Vec<MemoryId> = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_expired(self.current_time))
            .map(|(id, _)| *id)
            .collect();

        for id in expired {
            self.remove(id);
        }
    }

    /// Count entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Bytes used
    pub fn bytes_used(&self) -> u64 {
        self.bytes_used
    }

    /// Clear all
    pub fn clear(&mut self) {
        self.entries.clear();
        self.tag_index.clear();
        self.bytes_used = 0;
    }
}

/// Memory metadata without the data
#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    pub id: MemoryId,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
    pub importance: f32,
    pub tags: Vec<String>,
}

/// Memory errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    OutOfMemory,
    NotFound,
    InvalidId,
    CorruptedData,
}

// =============================================================================
// AI Memory System
// =============================================================================

/// Complete AI Memory System
pub struct AiMemory {
    /// Memory budget (bytes)
    budget: u64,

    /// Working memory
    working: Mutex<WorkingMemory>,

    /// Short-term decision memory
    decisions_stm: Mutex<ShortTermMemory<DecisionRecord>>,

    /// Short-term event memory
    events_stm: Mutex<ShortTermMemory<EventRecord>>,

    /// Long-term pattern memory
    patterns_ltm: RwLock<LongTermMemory<PatternRecord>>,

    /// Long-term anomaly memory
    anomalies_ltm: RwLock<LongTermMemory<AnomalyRecord>>,

    /// Long-term model memory
    models_ltm: RwLock<LongTermMemory<ModelRecord>>,

    /// Statistics
    stats: MemoryStats,
}

/// Record of a decision
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub decision_id: DecisionId,
    pub action_type: u32,
    pub confidence: Confidence,
    pub outcome: Option<bool>,
    pub impact_score: f32,
}

/// Record of an event
#[derive(Debug, Clone)]
pub struct EventRecord {
    pub event_type: u32,
    pub severity: u8,
    pub source: String,
    pub details: String,
}

/// Record of a pattern
#[derive(Debug, Clone)]
pub struct PatternRecord {
    pub pattern_type: u32,
    pub sequence: Vec<u32>,
    pub confidence: Confidence,
    pub occurrences: u64,
    pub last_seen: u64,
}

/// Record of an anomaly
#[derive(Debug, Clone)]
pub struct AnomalyRecord {
    pub anomaly_type: u32,
    pub metric: String,
    pub expected_value: f32,
    pub observed_value: f32,
    pub deviation_sigma: f32,
}

/// Record of a model
#[derive(Debug, Clone)]
pub struct ModelRecord {
    pub model_name: String,
    pub model_type: String,
    pub version: u32,
    pub accuracy: f32,
    pub serialized_weights: Vec<u8>,
}

/// Memory statistics
struct MemoryStats {
    working_operations: AtomicU64,
    stm_writes: AtomicU64,
    stm_reads: AtomicU64,
    ltm_writes: AtomicU64,
    ltm_reads: AtomicU64,
    evictions: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            working_operations: AtomicU64::new(0),
            stm_writes: AtomicU64::new(0),
            stm_reads: AtomicU64::new(0),
            ltm_writes: AtomicU64::new(0),
            ltm_reads: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }
}

impl AiMemory {
    /// Default working memory size
    const WORKING_SIZE: usize = 100;

    /// Default STM size
    const STM_SIZE: usize = 1000;

    /// Default LTM size
    const LTM_SIZE: usize = 10000;

    /// Create a new AI Memory system
    pub fn new(budget: u64) -> Self {
        let ltm_bytes = budget / 3;

        Self {
            budget,
            working: Mutex::new(WorkingMemory::new(Self::WORKING_SIZE)),
            decisions_stm: Mutex::new(ShortTermMemory::new(Self::STM_SIZE)),
            events_stm: Mutex::new(ShortTermMemory::new(Self::STM_SIZE)),
            patterns_ltm: RwLock::new(LongTermMemory::new(Self::LTM_SIZE, ltm_bytes)),
            anomalies_ltm: RwLock::new(LongTermMemory::new(Self::LTM_SIZE / 2, ltm_bytes / 2)),
            models_ltm: RwLock::new(LongTermMemory::new(100, ltm_bytes)),
            stats: MemoryStats::default(),
        }
    }

    // =========================================================================
    // Working Memory Operations
    // =========================================================================

    /// Set a working memory value
    pub fn set_context(&self, key: &str, value: ContextValue) {
        self.working.lock().set(key, value);
        self.stats
            .working_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get a working memory value
    pub fn get_context(&self, key: &str) -> Option<ContextValue> {
        self.stats
            .working_operations
            .fetch_add(1, Ordering::Relaxed);
        self.working.lock().get(key).cloned()
    }

    /// Set numeric context
    pub fn set_number(&self, key: &str, value: f64) {
        self.set_context(key, ContextValue::Number(value));
    }

    /// Set text context
    pub fn set_text(&self, key: &str, value: &str) {
        self.set_context(key, ContextValue::Text(value.to_string()));
    }

    /// Set flag context
    pub fn set_flag(&self, key: &str, value: bool) {
        self.set_context(key, ContextValue::Flag(value));
    }

    /// Get numeric context
    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.working.lock().get_number(key)
    }

    /// Get text context
    pub fn get_text(&self, key: &str) -> Option<String> {
        self.working.lock().get_text(key).map(|s| s.to_string())
    }

    /// Get flag context
    pub fn get_flag(&self, key: &str) -> bool {
        self.working.lock().get_flag(key)
    }

    // =========================================================================
    // Decision Memory Operations
    // =========================================================================

    /// Store a decision
    pub fn store_decision(&self, record: DecisionRecord, importance: f32) {
        let entry = MemoryEntry::new(record, importance).tag("decision");
        self.decisions_stm.lock().add(entry);
        self.stats.stm_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Get recent decisions
    pub fn recent_decisions(&self, count: usize) -> Vec<DecisionRecord> {
        self.stats.stm_reads.fetch_add(1, Ordering::Relaxed);
        self.decisions_stm
            .lock()
            .recent(count)
            .map(|e| e.data.clone())
            .collect()
    }

    /// Find decisions by action type
    pub fn find_decisions_by_action(&self, action_type: u32) -> Vec<DecisionRecord> {
        self.stats.stm_reads.fetch_add(1, Ordering::Relaxed);
        self.decisions_stm
            .lock()
            .find_all(|d| d.action_type == action_type)
            .into_iter()
            .map(|e| e.data.clone())
            .collect()
    }

    // =========================================================================
    // Event Memory Operations
    // =========================================================================

    /// Store an event
    pub fn store_event(&self, record: EventRecord, importance: f32) {
        let entry = MemoryEntry::new(record, importance).tag("event");
        self.events_stm.lock().add(entry);
        self.stats.stm_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Get recent events
    pub fn recent_events(&self, count: usize) -> Vec<EventRecord> {
        self.stats.stm_reads.fetch_add(1, Ordering::Relaxed);
        self.events_stm
            .lock()
            .recent(count)
            .map(|e| e.data.clone())
            .collect()
    }

    /// Find events by type
    pub fn find_events_by_type(&self, event_type: u32) -> Vec<EventRecord> {
        self.stats.stm_reads.fetch_add(1, Ordering::Relaxed);
        self.events_stm
            .lock()
            .find_all(|e| e.event_type == event_type)
            .into_iter()
            .map(|e| e.data.clone())
            .collect()
    }

    // =========================================================================
    // Pattern Memory Operations
    // =========================================================================

    /// Store a pattern
    pub fn store_pattern(
        &self,
        record: PatternRecord,
        importance: f32,
    ) -> Result<MemoryId, MemoryError> {
        let entry = MemoryEntry::new(record.clone(), importance).tag("pattern");
        let size = core::mem::size_of::<PatternRecord>() as u64 + record.sequence.len() as u64 * 4;

        let result = self.patterns_ltm.write().store(entry, size);
        self.stats.ltm_writes.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Retrieve a pattern
    pub fn retrieve_pattern(&self, id: MemoryId) -> Option<PatternRecord> {
        self.stats.ltm_reads.fetch_add(1, Ordering::Relaxed);
        self.patterns_ltm.write().retrieve(id).cloned()
    }

    /// Find patterns by type
    pub fn find_patterns_by_type(&self, pattern_type: u32) -> Vec<(MemoryId, PatternRecord)> {
        self.stats.ltm_reads.fetch_add(1, Ordering::Relaxed);
        let ltm = self.patterns_ltm.read();
        ltm.entries
            .iter()
            .filter(|(_, e)| e.data.pattern_type == pattern_type)
            .map(|(id, e)| (*id, e.data.clone()))
            .collect()
    }

    /// Find patterns by sequence prefix
    pub fn find_patterns_by_prefix(&self, prefix: &[u32]) -> Vec<(MemoryId, PatternRecord)> {
        self.stats.ltm_reads.fetch_add(1, Ordering::Relaxed);
        let ltm = self.patterns_ltm.read();
        ltm.entries
            .iter()
            .filter(|(_, e)| e.data.sequence.starts_with(prefix))
            .map(|(id, e)| (*id, e.data.clone()))
            .collect()
    }

    // =========================================================================
    // Anomaly Memory Operations
    // =========================================================================

    /// Store an anomaly
    pub fn store_anomaly(
        &self,
        record: AnomalyRecord,
        importance: f32,
    ) -> Result<MemoryId, MemoryError> {
        let entry = MemoryEntry::new(record.clone(), importance).tag("anomaly");
        let size = core::mem::size_of::<AnomalyRecord>() as u64 + record.metric.len() as u64;

        let result = self.anomalies_ltm.write().store(entry, size);
        self.stats.ltm_writes.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Find anomalies by metric
    pub fn find_anomalies_by_metric(&self, metric: &str) -> Vec<AnomalyRecord> {
        self.stats.ltm_reads.fetch_add(1, Ordering::Relaxed);
        self.anomalies_ltm
            .read()
            .entries
            .values()
            .filter(|e| e.data.metric == metric)
            .map(|e| e.data.clone())
            .collect()
    }

    // =========================================================================
    // Model Memory Operations
    // =========================================================================

    /// Store a model
    pub fn store_model(
        &self,
        record: ModelRecord,
        importance: f32,
    ) -> Result<MemoryId, MemoryError> {
        let size = core::mem::size_of::<ModelRecord>() as u64
            + record.model_name.len() as u64
            + record.model_type.len() as u64
            + record.serialized_weights.len() as u64;

        let entry = MemoryEntry::new(record, importance).tag("model");
        let result = self.models_ltm.write().store(entry, size);
        self.stats.ltm_writes.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Retrieve a model
    pub fn retrieve_model(&self, id: MemoryId) -> Option<ModelRecord> {
        self.stats.ltm_reads.fetch_add(1, Ordering::Relaxed);
        self.models_ltm.write().retrieve(id).cloned()
    }

    /// Find model by name
    pub fn find_model_by_name(&self, name: &str) -> Option<(MemoryId, ModelRecord)> {
        self.stats.ltm_reads.fetch_add(1, Ordering::Relaxed);
        self.models_ltm
            .read()
            .entries
            .iter()
            .find(|(_, e)| e.data.model_name == name)
            .map(|(id, e)| (*id, e.data.clone()))
    }

    // =========================================================================
    // Memory Management
    // =========================================================================

    /// Set current time for all memories
    pub fn set_time(&self, time: u64) {
        self.decisions_stm.lock().set_time(time);
        self.events_stm.lock().set_time(time);
        self.patterns_ltm.write().set_time(time);
        self.anomalies_ltm.write().set_time(time);
        self.models_ltm.write().set_time(time);
    }

    /// Clean expired entries
    pub fn clean_expired(&self) {
        self.decisions_stm.lock().clean_expired();
        self.events_stm.lock().clean_expired();
        self.patterns_ltm.write().clean_expired();
        self.anomalies_ltm.write().clean_expired();
        self.models_ltm.write().clean_expired();
    }

    /// Get memory usage
    pub fn memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            budget: self.budget,
            working_entries: self.working.lock().len(),
            decisions_stm_entries: self.decisions_stm.lock().len(),
            events_stm_entries: self.events_stm.lock().len(),
            patterns_ltm_entries: self.patterns_ltm.read().len(),
            patterns_ltm_bytes: self.patterns_ltm.read().bytes_used(),
            anomalies_ltm_entries: self.anomalies_ltm.read().len(),
            anomalies_ltm_bytes: self.anomalies_ltm.read().bytes_used(),
            models_ltm_entries: self.models_ltm.read().len(),
            models_ltm_bytes: self.models_ltm.read().bytes_used(),
        }
    }

    /// Get statistics
    pub fn statistics(&self) -> AiMemoryStatistics {
        let usage = self.memory_usage();
        let total_bytes =
            usage.patterns_ltm_bytes + usage.anomalies_ltm_bytes + usage.models_ltm_bytes;

        AiMemoryStatistics {
            budget: self.budget,
            bytes_used: total_bytes,
            utilization_percent: (total_bytes as f64 / self.budget as f64 * 100.0) as u8,
            working_entries: usage.working_entries,
            stm_entries: usage.decisions_stm_entries + usage.events_stm_entries,
            ltm_entries: usage.patterns_ltm_entries
                + usage.anomalies_ltm_entries
                + usage.models_ltm_entries,
            working_operations: self.stats.working_operations.load(Ordering::Relaxed),
            stm_writes: self.stats.stm_writes.load(Ordering::Relaxed),
            stm_reads: self.stats.stm_reads.load(Ordering::Relaxed),
            ltm_writes: self.stats.ltm_writes.load(Ordering::Relaxed),
            ltm_reads: self.stats.ltm_reads.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
        }
    }

    /// Clear all memories
    pub fn clear(&self) {
        self.working.lock().clear();
        self.decisions_stm.lock().clear();
        self.events_stm.lock().clear();
        self.patterns_ltm.write().clear();
        self.anomalies_ltm.write().clear();
        self.models_ltm.write().clear();
    }
}

/// Memory usage breakdown
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub budget: u64,
    pub working_entries: usize,
    pub decisions_stm_entries: usize,
    pub events_stm_entries: usize,
    pub patterns_ltm_entries: usize,
    pub patterns_ltm_bytes: u64,
    pub anomalies_ltm_entries: usize,
    pub anomalies_ltm_bytes: u64,
    pub models_ltm_entries: usize,
    pub models_ltm_bytes: u64,
}

/// Public memory statistics
#[derive(Debug, Clone)]
pub struct AiMemoryStatistics {
    pub budget: u64,
    pub bytes_used: u64,
    pub utilization_percent: u8,
    pub working_entries: usize,
    pub stm_entries: usize,
    pub ltm_entries: usize,
    pub working_operations: u64,
    pub stm_writes: u64,
    pub stm_reads: u64,
    pub ltm_writes: u64,
    pub ltm_reads: u64,
    pub evictions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn test_memory_entry() {
        let entry = MemoryEntry::new("test data", 0.8).tag("test").tag("data");

        assert_eq!(entry.importance, 0.8);
        assert_eq!(entry.tags.len(), 2);
        assert!(!entry.is_expired(1000));
    }

    #[test]
    fn test_memory_entry_ttl() {
        let entry = MemoryEntry::with_ttl("test", 0.5, 1000);
        assert!(!entry.is_expired(500));
        assert!(entry.is_expired(2000));
    }

    #[test]
    fn test_working_memory() {
        let mut wm = WorkingMemory::new(10);

        wm.set("cpu", ContextValue::Number(45.5));
        wm.set("flag", ContextValue::Flag(true));
        wm.set("name", ContextValue::Text("test".to_string()));

        assert_eq!(wm.get_number("cpu"), Some(45.5));
        assert!(wm.get_flag("flag"));
        assert_eq!(wm.get_text("name"), Some("test"));
    }

    #[test]
    fn test_short_term_memory() {
        let mut stm: ShortTermMemory<u32> = ShortTermMemory::new(3);

        for i in 0..5 {
            stm.add(MemoryEntry::new(i, 0.5));
        }

        // Should only have last 3
        assert_eq!(stm.len(), 3);

        let recent: Vec<_> = stm.recent(2).map(|e| e.data).collect();
        assert_eq!(recent, vec![4, 3]);
    }

    #[test]
    fn test_long_term_memory() {
        let mut ltm: LongTermMemory<String> = LongTermMemory::new(5, 1024);

        for i in 0..3 {
            let entry = MemoryEntry::new(format!("item_{}", i), 0.5).tag("test");
            ltm.store(entry, 10).unwrap();
        }

        assert_eq!(ltm.len(), 3);

        let by_tag = ltm.find_by_tag("test");
        assert_eq!(by_tag.len(), 3);
    }

    #[test]
    fn test_ai_memory() {
        let mem = AiMemory::new(1024 * 1024); // 1 MB

        // Working memory
        mem.set_number("cpu_usage", 45.0);
        assert_eq!(mem.get_number("cpu_usage"), Some(45.0));

        // Decision memory
        mem.store_decision(
            DecisionRecord {
                decision_id: DecisionId::new(),
                action_type: 1,
                confidence: Confidence::new(0.9),
                outcome: Some(true),
                impact_score: 0.8,
            },
            0.7,
        );

        let recent = mem.recent_decisions(10);
        assert_eq!(recent.len(), 1);

        // Pattern memory
        let id = mem
            .store_pattern(
                PatternRecord {
                    pattern_type: 0,
                    sequence: vec![1, 2, 3],
                    confidence: Confidence::new(0.8),
                    occurrences: 5,
                    last_seen: 0,
                },
                0.9,
            )
            .unwrap();

        let pattern = mem.retrieve_pattern(id);
        assert!(pattern.is_some());
    }

    #[test]
    fn test_memory_statistics() {
        let mem = AiMemory::new(1024 * 1024);

        mem.set_number("test", 1.0);
        mem.get_context("test"); // Use get_context which increments working_operations

        let stats = mem.statistics();
        assert_eq!(stats.working_operations, 2);
    }
}
