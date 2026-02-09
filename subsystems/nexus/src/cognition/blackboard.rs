//! # Cognitive Blackboard System
//!
//! Shared memory for inter-domain communication.
//! Provides a structured way for domains to share data.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// BLACKBOARD ENTRY TYPES
// ============================================================================

/// Entry in the blackboard
#[derive(Debug, Clone)]
pub struct BlackboardEntry {
    /// Unique entry ID
    pub id: u64,
    /// Entry key
    pub key: BlackboardKey,
    /// Entry value
    pub value: BlackboardValue,
    /// Source domain
    pub source: DomainId,
    /// Creation time
    pub created: Timestamp,
    /// Last update time
    pub updated: Timestamp,
    /// Time-to-live (cycles)
    pub ttl: Option<u64>,
    /// Access count
    pub access_count: u64,
    /// Visibility
    pub visibility: EntryVisibility,
}

/// Key for blackboard entries
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlackboardKey {
    /// Signal data
    Signal(u64),
    /// Pattern data
    Pattern(u64),
    /// Causal chain
    CausalChain(u64),
    /// Decision option
    Option(u64),
    /// Effect result
    Effect(u64),
    /// Memory reference
    Memory(u64),
    /// Insight
    Insight(u64),
    /// Learning
    Learning(u64),
    /// Custom key
    Custom(String),
    /// Composite key
    Composite(Vec<String>),
}

impl BlackboardKey {
    /// Get key category
    pub fn category(&self) -> &'static str {
        match self {
            Self::Signal(_) => "signal",
            Self::Pattern(_) => "pattern",
            Self::CausalChain(_) => "causal",
            Self::Option(_) => "option",
            Self::Effect(_) => "effect",
            Self::Memory(_) => "memory",
            Self::Insight(_) => "insight",
            Self::Learning(_) => "learning",
            Self::Custom(_) => "custom",
            Self::Composite(_) => "composite",
        }
    }
}

/// Value stored in blackboard
#[derive(Debug, Clone)]
pub enum BlackboardValue {
    /// No value
    None,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Unsigned integer
    Uint(u64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Bytes
    Bytes(Vec<u8>),
    /// Array of values
    Array(Vec<BlackboardValue>),
    /// Map of values
    Map(BTreeMap<String, BlackboardValue>),
    /// Signal data
    Signal(SignalEntry),
    /// Pattern data
    Pattern(PatternEntry),
    /// Causal chain
    CausalChain(CausalEntry),
    /// Decision option
    Option(OptionEntry),
    /// Effect result
    Effect(EffectEntry),
}

/// Signal entry
#[derive(Debug, Clone)]
pub struct SignalEntry {
    pub kind: u32,
    pub value: f64,
    pub unit: String,
    pub component: u64,
    pub metadata: BTreeMap<String, String>,
}

/// Pattern entry
#[derive(Debug, Clone)]
pub struct PatternEntry {
    pub pattern_type: u32,
    pub confidence: f32,
    pub occurrences: u64,
    pub components: Vec<u64>,
    pub description: String,
}

/// Causal chain entry
#[derive(Debug, Clone)]
pub struct CausalEntry {
    pub cause: u64,
    pub effect: u64,
    pub strength: f32,
    pub intermediates: Vec<u64>,
}

/// Option entry
#[derive(Debug, Clone)]
pub struct OptionEntry {
    pub action_type: u32,
    pub score: f32,
    pub confidence: f32,
    pub estimated_cost: f64,
    pub estimated_benefit: f64,
}

/// Effect entry
#[derive(Debug, Clone)]
pub struct EffectEntry {
    pub action_id: u64,
    pub success: bool,
    pub actual_cost: f64,
    pub actual_benefit: f64,
    pub side_effects: Vec<String>,
}

/// Entry visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryVisibility {
    /// Visible to all domains
    Public,
    /// Visible to specific domains
    Restricted,
    /// Private to source domain
    Private,
}

// ============================================================================
// BLACKBOARD
// ============================================================================

/// Shared blackboard for cognitive domains
pub struct CognitiveBlackboard {
    /// Entries by ID
    entries: BTreeMap<u64, BlackboardEntry>,
    /// Index by key
    key_index: BTreeMap<BlackboardKey, u64>,
    /// Index by source domain
    domain_index: BTreeMap<DomainId, Vec<u64>>,
    /// Index by category
    category_index: BTreeMap<String, Vec<u64>>,
    /// Next entry ID
    next_id: AtomicU64,
    /// Current cycle
    current_cycle: u64,
    /// Configuration
    config: BlackboardConfig,
    /// Statistics
    stats: BlackboardStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct BlackboardConfig {
    /// Maximum entries
    pub max_entries: usize,
    /// Default TTL (cycles)
    pub default_ttl: u64,
    /// Enable access tracking
    pub track_access: bool,
    /// Eviction policy
    pub eviction: EvictionPolicy,
}

impl Default for BlackboardConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            default_ttl: 1000,
            track_access: true,
            eviction: EvictionPolicy::LRU,
        }
    }
}

/// Eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// Least recently used
    LRU,
    /// Least frequently used
    LFU,
    /// First in first out
    FIFO,
    /// Oldest first
    Oldest,
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BlackboardStats {
    /// Total entries written
    pub total_writes: u64,
    /// Total entries read
    pub total_reads: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Expirations
    pub expirations: u64,
    /// Current entry count
    pub entry_count: u64,
}

impl CognitiveBlackboard {
    /// Create a new blackboard
    pub fn new(config: BlackboardConfig) -> Self {
        Self {
            entries: BTreeMap::new(),
            key_index: BTreeMap::new(),
            domain_index: BTreeMap::new(),
            category_index: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            current_cycle: 0,
            config,
            stats: BlackboardStats::default(),
        }
    }

    /// Write an entry
    pub fn write(
        &mut self,
        key: BlackboardKey,
        value: BlackboardValue,
        source: DomainId,
        ttl: Option<u64>,
        visibility: EntryVisibility,
    ) -> u64 {
        // Check if key exists
        if let Some(&id) = self.key_index.get(&key) {
            // Update existing entry
            if let Some(entry) = self.entries.get_mut(&id) {
                entry.value = value;
                entry.updated = Timestamp::now();
                entry.ttl = ttl.or(Some(self.config.default_ttl));
                self.stats.total_writes += 1;
                return id;
            }
        }

        // Evict if necessary
        if self.entries.len() >= self.config.max_entries {
            self.evict();
        }

        // Create new entry
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();
        let category = key.category().to_string();

        let entry = BlackboardEntry {
            id,
            key: key.clone(),
            value,
            source,
            created: now,
            updated: now,
            ttl: ttl.or(Some(self.config.default_ttl)),
            access_count: 0,
            visibility,
        };

        self.entries.insert(id, entry);
        self.key_index.insert(key, id);

        self.domain_index.entry(source).or_default().push(id);
        self.category_index.entry(category).or_default().push(id);

        self.stats.total_writes += 1;
        self.stats.entry_count = self.entries.len() as u64;

        id
    }

    /// Read an entry by key
    pub fn read(&mut self, key: &BlackboardKey, reader: DomainId) -> Option<&BlackboardValue> {
        self.stats.total_reads += 1;

        let id = self.key_index.get(key)?;
        let entry = self.entries.get_mut(id)?;

        // Check visibility
        match entry.visibility {
            EntryVisibility::Private if entry.source != reader => {
                self.stats.misses += 1;
                return None;
            },
            _ => {},
        }

        // Track access
        if self.config.track_access {
            entry.access_count += 1;
        }

        self.stats.hits += 1;
        Some(&entry.value)
    }

    /// Read by ID
    pub fn read_by_id(&mut self, id: u64, reader: DomainId) -> Option<&BlackboardValue> {
        self.stats.total_reads += 1;

        let entry = self.entries.get_mut(&id)?;

        match entry.visibility {
            EntryVisibility::Private if entry.source != reader => {
                self.stats.misses += 1;
                return None;
            },
            _ => {},
        }

        if self.config.track_access {
            entry.access_count += 1;
        }

        self.stats.hits += 1;
        Some(&entry.value)
    }

    /// Delete an entry
    pub fn delete(&mut self, key: &BlackboardKey) -> bool {
        if let Some(id) = self.key_index.remove(key) {
            if let Some(entry) = self.entries.remove(&id) {
                // Clean up indexes
                if let Some(ids) = self.domain_index.get_mut(&entry.source) {
                    ids.retain(|&i| i != id);
                }
                let category = key.category().to_string();
                if let Some(ids) = self.category_index.get_mut(&category) {
                    ids.retain(|&i| i != id);
                }
                self.stats.entry_count = self.entries.len() as u64;
                return true;
            }
        }
        false
    }

    /// Get entries by domain
    #[inline]
    pub fn entries_by_domain(&self, domain: DomainId) -> Vec<&BlackboardEntry> {
        self.domain_index
            .get(&domain)
            .map(|ids| ids.iter().filter_map(|id| self.entries.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get entries by category
    #[inline]
    pub fn entries_by_category(&self, category: &str) -> Vec<&BlackboardEntry> {
        self.category_index
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.entries.get(id)).collect())
            .unwrap_or_default()
    }

    /// Process tick - handle TTL
    pub fn tick(&mut self) {
        self.current_cycle += 1;

        let mut to_remove = Vec::new();

        for (id, entry) in &self.entries {
            if let Some(ttl) = entry.ttl {
                let age = self.current_cycle - entry.created.as_cycles();
                if age >= ttl {
                    to_remove.push(*id);
                }
            }
        }

        for id in to_remove {
            if let Some(entry) = self.entries.remove(&id) {
                self.key_index.remove(&entry.key);
                if let Some(ids) = self.domain_index.get_mut(&entry.source) {
                    ids.retain(|&i| i != id);
                }
                let category = entry.key.category().to_string();
                if let Some(ids) = self.category_index.get_mut(&category) {
                    ids.retain(|&i| i != id);
                }
                self.stats.expirations += 1;
            }
        }

        self.stats.entry_count = self.entries.len() as u64;
    }

    /// Evict entries
    fn evict(&mut self) {
        let to_evict = match self.config.eviction {
            EvictionPolicy::LRU => self.find_lru(),
            EvictionPolicy::LFU => self.find_lfu(),
            EvictionPolicy::FIFO => self.find_fifo(),
            EvictionPolicy::Oldest => self.find_oldest(),
        };

        if let Some(id) = to_evict {
            if let Some(entry) = self.entries.remove(&id) {
                self.key_index.remove(&entry.key);
                if let Some(ids) = self.domain_index.get_mut(&entry.source) {
                    ids.retain(|&i| i != id);
                }
                let category = entry.key.category().to_string();
                if let Some(ids) = self.category_index.get_mut(&category) {
                    ids.retain(|&i| i != id);
                }
                self.stats.evictions += 1;
            }
        }
    }

    fn find_lru(&self) -> Option<u64> {
        self.entries
            .values()
            .min_by_key(|e| e.updated.raw())
            .map(|e| e.id)
    }

    fn find_lfu(&self) -> Option<u64> {
        self.entries
            .values()
            .min_by_key(|e| e.access_count)
            .map(|e| e.id)
    }

    fn find_fifo(&self) -> Option<u64> {
        self.entries.values().min_by_key(|e| e.id).map(|e| e.id)
    }

    fn find_oldest(&self) -> Option<u64> {
        self.entries
            .values()
            .min_by_key(|e| e.created.raw())
            .map(|e| e.id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &BlackboardStats {
        &self.stats
    }

    /// Get entry count
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.key_index.clear();
        self.domain_index.clear();
        self.category_index.clear();
        self.stats.entry_count = 0;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_write_read() {
        let config = BlackboardConfig::default();
        let mut bb = CognitiveBlackboard::new(config);

        let key = BlackboardKey::Signal(1);
        let value = BlackboardValue::Float(42.0);
        let domain = DomainId::new(1);

        let id = bb.write(key.clone(), value, domain, None, EntryVisibility::Public);
        assert!(id > 0);

        let read = bb.read(&key, domain);
        assert!(read.is_some());
    }

    #[test]
    fn test_visibility() {
        let config = BlackboardConfig::default();
        let mut bb = CognitiveBlackboard::new(config);

        let key = BlackboardKey::Custom("secret".into());
        let value = BlackboardValue::String("hidden".into());
        let owner = DomainId::new(1);
        let other = DomainId::new(2);

        bb.write(key.clone(), value, owner, None, EntryVisibility::Private);

        // Owner can read
        assert!(bb.read(&key, owner).is_some());
        // Other cannot
        assert!(bb.read(&key, other).is_none());
    }

    #[test]
    fn test_eviction() {
        let mut config = BlackboardConfig::default();
        config.max_entries = 3;
        let mut bb = CognitiveBlackboard::new(config);

        let domain = DomainId::new(1);

        for i in 0..5 {
            bb.write(
                BlackboardKey::Signal(i),
                BlackboardValue::Int(i as i64),
                domain,
                None,
                EntryVisibility::Public,
            );
        }

        assert!(bb.len() <= 3);
        assert!(bb.stats().evictions > 0);
    }
}
