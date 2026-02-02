//! # Memory Index
//!
//! Implements indexing for efficient memory retrieval.
//! Supports multiple index types for fast lookup.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// INDEX TYPES
// ============================================================================

/// Index entry
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// Entry ID
    pub id: u64,
    /// Key
    pub key: IndexKey,
    /// Memory ID
    pub memory_id: u64,
    /// Score (for relevance)
    pub score: f64,
    /// Created
    pub created: Timestamp,
}

/// Index key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndexKey {
    Text(String),
    Integer(i64),
    Composite(Vec<IndexKey>),
}

/// Index definition
#[derive(Debug, Clone)]
pub struct IndexDef {
    /// Index ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub index_type: IndexType,
    /// Fields
    pub fields: Vec<String>,
    /// Unique
    pub unique: bool,
}

/// Index type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    BTree,
    Hash,
    Inverted,
    Spatial,
    Temporal,
}

/// Query
#[derive(Debug, Clone)]
pub struct IndexQuery {
    /// Index name
    pub index: String,
    /// Operation
    pub operation: QueryOp,
    /// Limit
    pub limit: Option<usize>,
}

/// Query operation
#[derive(Debug, Clone)]
pub enum QueryOp {
    Exact(IndexKey),
    Range { start: IndexKey, end: IndexKey },
    Prefix(String),
    Contains(String),
    Near { center: Vec<f64>, radius: f64 },
    TimeRange { start: Timestamp, end: Timestamp },
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Entries
    pub entries: Vec<IndexEntry>,
    /// Count
    pub count: usize,
    /// Scanned
    pub scanned: usize,
}

// ============================================================================
// INDEX MANAGER
// ============================================================================

/// Index manager
pub struct IndexManager {
    /// Index definitions
    definitions: BTreeMap<u64, IndexDef>,
    /// Name to ID mapping
    name_to_id: BTreeMap<String, u64>,
    /// BTree indexes
    btree_indexes: BTreeMap<u64, BTreeMap<IndexKey, Vec<u64>>>,
    /// Hash indexes
    hash_indexes: BTreeMap<u64, BTreeMap<u64, Vec<u64>>>,
    /// Inverted indexes
    inverted_indexes: BTreeMap<u64, BTreeMap<String, Vec<u64>>>,
    /// Entries
    entries: BTreeMap<u64, IndexEntry>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: IndexConfig,
    /// Statistics
    stats: IndexStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Maximum entries per index
    pub max_entries: usize,
    /// Enable compression
    pub compress: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            max_entries: 100000,
            compress: false,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    /// Indexes created
    pub indexes_created: u64,
    /// Entries indexed
    pub entries_indexed: u64,
    /// Queries executed
    pub queries_executed: u64,
}

impl IndexManager {
    /// Create new manager
    pub fn new(config: IndexConfig) -> Self {
        Self {
            definitions: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            btree_indexes: BTreeMap::new(),
            hash_indexes: BTreeMap::new(),
            inverted_indexes: BTreeMap::new(),
            entries: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: IndexStats::default(),
        }
    }

    /// Create index
    pub fn create_index(
        &mut self,
        name: &str,
        index_type: IndexType,
        fields: Vec<String>,
        unique: bool,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let def = IndexDef {
            id,
            name: name.into(),
            index_type,
            fields,
            unique,
        };

        self.definitions.insert(id, def);
        self.name_to_id.insert(name.into(), id);

        // Initialize storage
        match index_type {
            IndexType::BTree => {
                self.btree_indexes.insert(id, BTreeMap::new());
            }
            IndexType::Hash => {
                self.hash_indexes.insert(id, BTreeMap::new());
            }
            IndexType::Inverted => {
                self.inverted_indexes.insert(id, BTreeMap::new());
            }
            _ => {}
        }

        self.stats.indexes_created += 1;

        id
    }

    /// Index entry
    pub fn index(&mut self, index_name: &str, key: IndexKey, memory_id: u64, score: f64) -> Option<u64> {
        let index_id = *self.name_to_id.get(index_name)?;
        let def = self.definitions.get(&index_id)?.clone();

        let entry_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let entry = IndexEntry {
            id: entry_id,
            key: key.clone(),
            memory_id,
            score,
            created: Timestamp::now(),
        };

        // Add to appropriate index structure
        match def.index_type {
            IndexType::BTree => {
                if let Some(btree) = self.btree_indexes.get_mut(&index_id) {
                    btree.entry(key).or_insert_with(Vec::new).push(entry_id);
                }
            }
            IndexType::Hash => {
                if let Some(hash) = self.hash_indexes.get_mut(&index_id) {
                    let hash_key = self.compute_hash(&key);
                    hash.entry(hash_key).or_insert_with(Vec::new).push(entry_id);
                }
            }
            IndexType::Inverted => {
                if let Some(inverted) = self.inverted_indexes.get_mut(&index_id) {
                    if let IndexKey::Text(text) = &key {
                        // Tokenize and index
                        for token in text.split_whitespace() {
                            let normalized = token.to_lowercase();
                            inverted.entry(normalized).or_insert_with(Vec::new).push(entry_id);
                        }
                    }
                }
            }
            _ => {}
        }

        self.entries.insert(entry_id, entry);
        self.stats.entries_indexed += 1;

        Some(entry_id)
    }

    fn compute_hash(&self, key: &IndexKey) -> u64 {
        let mut hash: u64 = 0;

        match key {
            IndexKey::Text(s) => {
                for byte in s.bytes() {
                    hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
                }
            }
            IndexKey::Integer(i) => {
                hash = *i as u64;
            }
            IndexKey::Composite(keys) => {
                for k in keys {
                    hash = hash.wrapping_mul(31).wrapping_add(self.compute_hash(k));
                }
            }
        }

        hash
    }

    /// Query index
    pub fn query(&mut self, query: &IndexQuery) -> QueryResult {
        self.stats.queries_executed += 1;

        let index_id = match self.name_to_id.get(&query.index) {
            Some(&id) => id,
            None => return QueryResult {
                entries: Vec::new(),
                count: 0,
                scanned: 0,
            },
        };

        let def = match self.definitions.get(&index_id) {
            Some(d) => d.clone(),
            None => return QueryResult {
                entries: Vec::new(),
                count: 0,
                scanned: 0,
            },
        };

        let mut entry_ids = Vec::new();
        let mut scanned = 0;

        match (&def.index_type, &query.operation) {
            (IndexType::BTree, QueryOp::Exact(key)) => {
                if let Some(btree) = self.btree_indexes.get(&index_id) {
                    if let Some(ids) = btree.get(key) {
                        entry_ids.extend(ids.iter().copied());
                    }
                    scanned = 1;
                }
            }
            (IndexType::BTree, QueryOp::Range { start, end }) => {
                if let Some(btree) = self.btree_indexes.get(&index_id) {
                    for (key, ids) in btree.range(start.clone()..=end.clone()) {
                        entry_ids.extend(ids.iter().copied());
                        scanned += 1;
                    }
                }
            }
            (IndexType::Hash, QueryOp::Exact(key)) => {
                if let Some(hash) = self.hash_indexes.get(&index_id) {
                    let hash_key = self.compute_hash(key);
                    if let Some(ids) = hash.get(&hash_key) {
                        entry_ids.extend(ids.iter().copied());
                    }
                    scanned = 1;
                }
            }
            (IndexType::Inverted, QueryOp::Contains(text)) => {
                if let Some(inverted) = self.inverted_indexes.get(&index_id) {
                    let normalized = text.to_lowercase();
                    for token in normalized.split_whitespace() {
                        if let Some(ids) = inverted.get(token) {
                            for &id in ids {
                                if !entry_ids.contains(&id) {
                                    entry_ids.push(id);
                                }
                            }
                        }
                        scanned += 1;
                    }
                }
            }
            (IndexType::Inverted, QueryOp::Prefix(prefix)) => {
                if let Some(inverted) = self.inverted_indexes.get(&index_id) {
                    let normalized = prefix.to_lowercase();
                    for (token, ids) in inverted.iter() {
                        if token.starts_with(&normalized) {
                            for &id in ids {
                                if !entry_ids.contains(&id) {
                                    entry_ids.push(id);
                                }
                            }
                        }
                        scanned += 1;
                    }
                }
            }
            _ => {}
        }

        // Collect entries
        let mut entries: Vec<_> = entry_ids.iter()
            .filter_map(|id| self.entries.get(id))
            .cloned()
            .collect();

        // Sort by score
        entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));

        // Apply limit
        if let Some(limit) = query.limit {
            entries.truncate(limit);
        }

        let count = entries.len();

        QueryResult {
            entries,
            count,
            scanned,
        }
    }

    /// Remove entry
    pub fn remove(&mut self, entry_id: u64) -> bool {
        let entry = match self.entries.remove(&entry_id) {
            Some(e) => e,
            None => return false,
        };

        // Remove from all indexes
        for (index_id, def) in &self.definitions {
            match def.index_type {
                IndexType::BTree => {
                    if let Some(btree) = self.btree_indexes.get_mut(index_id) {
                        if let Some(ids) = btree.get_mut(&entry.key) {
                            ids.retain(|&id| id != entry_id);
                        }
                    }
                }
                IndexType::Hash => {
                    if let Some(hash) = self.hash_indexes.get_mut(index_id) {
                        let hash_key = self.compute_hash(&entry.key);
                        if let Some(ids) = hash.get_mut(&hash_key) {
                            ids.retain(|&id| id != entry_id);
                        }
                    }
                }
                IndexType::Inverted => {
                    if let Some(inverted) = self.inverted_indexes.get_mut(index_id) {
                        for (_, ids) in inverted.iter_mut() {
                            ids.retain(|&id| id != entry_id);
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }

    /// Get index definition
    pub fn get_index(&self, name: &str) -> Option<&IndexDef> {
        let id = self.name_to_id.get(name)?;
        self.definitions.get(id)
    }

    /// Get entry
    pub fn get_entry(&self, id: u64) -> Option<&IndexEntry> {
        self.entries.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &IndexStats {
        &self.stats
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new(IndexConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_index() {
        let mut manager = IndexManager::default();

        let id = manager.create_index("name_idx", IndexType::BTree, vec!["name".into()], false);
        assert!(manager.get_index("name_idx").is_some());
    }

    #[test]
    fn test_btree_index() {
        let mut manager = IndexManager::default();

        manager.create_index("id_idx", IndexType::BTree, vec!["id".into()], true);

        manager.index("id_idx", IndexKey::Integer(1), 100, 1.0);
        manager.index("id_idx", IndexKey::Integer(2), 101, 1.0);
        manager.index("id_idx", IndexKey::Integer(3), 102, 1.0);

        let result = manager.query(&IndexQuery {
            index: "id_idx".into(),
            operation: QueryOp::Exact(IndexKey::Integer(2)),
            limit: None,
        });

        assert_eq!(result.count, 1);
        assert_eq!(result.entries[0].memory_id, 101);
    }

    #[test]
    fn test_btree_range() {
        let mut manager = IndexManager::default();

        manager.create_index("score_idx", IndexType::BTree, vec!["score".into()], false);

        for i in 0..10 {
            manager.index("score_idx", IndexKey::Integer(i), i as u64, 1.0);
        }

        let result = manager.query(&IndexQuery {
            index: "score_idx".into(),
            operation: QueryOp::Range {
                start: IndexKey::Integer(3),
                end: IndexKey::Integer(7),
            },
            limit: None,
        });

        assert_eq!(result.count, 5);
    }

    #[test]
    fn test_inverted_index() {
        let mut manager = IndexManager::default();

        manager.create_index("text_idx", IndexType::Inverted, vec!["content".into()], false);

        manager.index("text_idx", IndexKey::Text("hello world".into()), 1, 1.0);
        manager.index("text_idx", IndexKey::Text("hello rust".into()), 2, 1.0);
        manager.index("text_idx", IndexKey::Text("goodbye world".into()), 3, 1.0);

        let result = manager.query(&IndexQuery {
            index: "text_idx".into(),
            operation: QueryOp::Contains("hello".into()),
            limit: None,
        });

        assert_eq!(result.count, 2);
    }

    #[test]
    fn test_prefix_query() {
        let mut manager = IndexManager::default();

        manager.create_index("name_idx", IndexType::Inverted, vec!["name".into()], false);

        manager.index("name_idx", IndexKey::Text("programming".into()), 1, 1.0);
        manager.index("name_idx", IndexKey::Text("program".into()), 2, 1.0);
        manager.index("name_idx", IndexKey::Text("other".into()), 3, 1.0);

        let result = manager.query(&IndexQuery {
            index: "name_idx".into(),
            operation: QueryOp::Prefix("prog".into()),
            limit: None,
        });

        assert_eq!(result.count, 2);
    }

    #[test]
    fn test_remove() {
        let mut manager = IndexManager::default();

        manager.create_index("idx", IndexType::BTree, vec!["id".into()], false);

        let entry = manager.index("idx", IndexKey::Integer(1), 100, 1.0).unwrap();

        assert!(manager.remove(entry));
        assert!(manager.get_entry(entry).is_none());
    }
}
