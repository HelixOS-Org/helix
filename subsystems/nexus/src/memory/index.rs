//! # Memory Index
//!
//! Indexing structures for fast memory retrieval.
//! Supports multiple index types for different access patterns.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
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
    /// Value (memory ID)
    pub memory_id: u64,
    /// Weight
    pub weight: f64,
    /// Created
    pub created: Timestamp,
}

/// Index key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndexKey {
    /// String key
    String(String),
    /// Numeric key
    Numeric(i64),
    /// Composite key
    Composite(Vec<String>),
    /// Hash key
    Hash(u64),
}

/// Index type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    /// B-tree index
    BTree,
    /// Hash index
    Hash,
    /// Inverted index (for text)
    Inverted,
    /// Spatial index
    Spatial,
    /// Temporal index
    Temporal,
}

// ============================================================================
// B-TREE INDEX
// ============================================================================

/// B-tree index
pub struct BTreeIndex {
    /// Index ID
    id: u64,
    /// Index name
    name: String,
    /// Entries
    entries: BTreeMap<IndexKey, Vec<u64>>,
    /// Entry count
    count: usize,
}

impl BTreeIndex {
    /// Create new B-tree index
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            entries: BTreeMap::new(),
            count: 0,
        }
    }

    /// Insert entry
    pub fn insert(&mut self, key: IndexKey, memory_id: u64) {
        self.entries.entry(key)
            .or_insert_with(Vec::new)
            .push(memory_id);
        self.count += 1;
    }

    /// Get by key
    pub fn get(&self, key: &IndexKey) -> Option<&Vec<u64>> {
        self.entries.get(key)
    }

    /// Range query
    pub fn range(&self, start: &IndexKey, end: &IndexKey) -> Vec<u64> {
        self.entries.range(start..=end)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    /// Remove
    pub fn remove(&mut self, key: &IndexKey, memory_id: u64) -> bool {
        if let Some(ids) = self.entries.get_mut(key) {
            if let Some(pos) = ids.iter().position(|&id| id == memory_id) {
                ids.remove(pos);
                self.count -= 1;
                if ids.is_empty() {
                    self.entries.remove(key);
                }
                return true;
            }
        }
        false
    }

    /// Count
    pub fn len(&self) -> usize {
        self.count
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// ============================================================================
// INVERTED INDEX
// ============================================================================

/// Inverted index for text search
pub struct InvertedIndex {
    /// Index ID
    id: u64,
    /// Index name
    name: String,
    /// Term to document mapping
    term_docs: BTreeMap<String, BTreeSet<u64>>,
    /// Document frequency
    doc_freq: BTreeMap<String, usize>,
    /// Total documents
    total_docs: usize,
}

impl InvertedIndex {
    /// Create new inverted index
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            term_docs: BTreeMap::new(),
            doc_freq: BTreeMap::new(),
            total_docs: 0,
        }
    }

    /// Index document
    pub fn index_document(&mut self, memory_id: u64, text: &str) {
        let terms = self.tokenize(text);
        let unique_terms: BTreeSet<_> = terms.iter().collect();

        for term in unique_terms {
            self.term_docs.entry(term.clone())
                .or_insert_with(BTreeSet::new)
                .insert(memory_id);

            *self.doc_freq.entry(term.clone()).or_insert(0) += 1;
        }

        self.total_docs += 1;
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|s| s.to_lowercase())
            .filter(|s| s.len() >= 2)
            .collect()
    }

    /// Search
    pub fn search(&self, query: &str) -> Vec<(u64, f64)> {
        let query_terms = self.tokenize(query);

        if query_terms.is_empty() {
            return Vec::new();
        }

        // Find documents containing all terms
        let mut result_docs: Option<BTreeSet<u64>> = None;

        for term in &query_terms {
            if let Some(docs) = self.term_docs.get(term) {
                result_docs = Some(match result_docs {
                    Some(existing) => existing.intersection(docs).copied().collect(),
                    None => docs.clone(),
                });
            } else {
                return Vec::new(); // Term not found
            }
        }

        // Score documents using TF-IDF
        let docs = result_docs.unwrap_or_default();

        let mut scored: Vec<(u64, f64)> = docs.iter()
            .map(|&doc_id| {
                let score = self.compute_score(doc_id, &query_terms);
                (doc_id, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored
    }

    fn compute_score(&self, _doc_id: u64, query_terms: &[String]) -> f64 {
        let mut score = 0.0;

        for term in query_terms {
            if let Some(&df) = self.doc_freq.get(term) {
                // IDF
                let idf = (self.total_docs as f64 / df as f64).ln() + 1.0;
                score += idf;
            }
        }

        score
    }

    /// Remove document
    pub fn remove_document(&mut self, memory_id: u64) {
        for docs in self.term_docs.values_mut() {
            docs.remove(&memory_id);
        }
    }

    /// Get terms
    pub fn get_terms(&self, memory_id: u64) -> Vec<String> {
        self.term_docs.iter()
            .filter(|(_, docs)| docs.contains(&memory_id))
            .map(|(term, _)| term.clone())
            .collect()
    }
}

// ============================================================================
// TEMPORAL INDEX
// ============================================================================

/// Temporal index
pub struct TemporalIndex {
    /// Index ID
    id: u64,
    /// Index name
    name: String,
    /// Time-based entries (timestamp -> memory IDs)
    entries: BTreeMap<u64, Vec<u64>>,
}

impl TemporalIndex {
    /// Create new temporal index
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            entries: BTreeMap::new(),
        }
    }

    /// Insert
    pub fn insert(&mut self, timestamp: Timestamp, memory_id: u64) {
        self.entries.entry(timestamp.0)
            .or_insert_with(Vec::new)
            .push(memory_id);
    }

    /// Query by time range
    pub fn query_range(&self, start: Timestamp, end: Timestamp) -> Vec<u64> {
        self.entries.range(start.0..=end.0)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    /// Query before
    pub fn before(&self, timestamp: Timestamp) -> Vec<u64> {
        self.entries.range(..timestamp.0)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    /// Query after
    pub fn after(&self, timestamp: Timestamp) -> Vec<u64> {
        self.entries.range(timestamp.0..)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    /// Query most recent
    pub fn most_recent(&self, count: usize) -> Vec<u64> {
        self.entries.iter()
            .rev()
            .flat_map(|(_, ids)| ids.iter().copied())
            .take(count)
            .collect()
    }

    /// Remove
    pub fn remove(&mut self, timestamp: Timestamp, memory_id: u64) {
        if let Some(ids) = self.entries.get_mut(&timestamp.0) {
            ids.retain(|&id| id != memory_id);
            if ids.is_empty() {
                self.entries.remove(&timestamp.0);
            }
        }
    }
}

// ============================================================================
// INDEX MANAGER
// ============================================================================

/// Index manager
pub struct IndexManager {
    /// B-tree indexes
    btree_indexes: BTreeMap<String, BTreeIndex>,
    /// Inverted indexes
    inverted_indexes: BTreeMap<String, InvertedIndex>,
    /// Temporal indexes
    temporal_indexes: BTreeMap<String, TemporalIndex>,
    /// Next ID
    next_id: AtomicU64,
    /// Statistics
    stats: IndexStats,
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    /// Total indexes
    pub total_indexes: u64,
    /// Total entries
    pub total_entries: u64,
    /// Lookups
    pub lookups: u64,
    /// Inserts
    pub inserts: u64,
}

impl IndexManager {
    /// Create new index manager
    pub fn new() -> Self {
        Self {
            btree_indexes: BTreeMap::new(),
            inverted_indexes: BTreeMap::new(),
            temporal_indexes: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            stats: IndexStats::default(),
        }
    }

    /// Create B-tree index
    pub fn create_btree(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.btree_indexes.insert(name.into(), BTreeIndex::new(id, name));
        self.stats.total_indexes += 1;
        id
    }

    /// Create inverted index
    pub fn create_inverted(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.inverted_indexes.insert(name.into(), InvertedIndex::new(id, name));
        self.stats.total_indexes += 1;
        id
    }

    /// Create temporal index
    pub fn create_temporal(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.temporal_indexes.insert(name.into(), TemporalIndex::new(id, name));
        self.stats.total_indexes += 1;
        id
    }

    /// Insert into B-tree
    pub fn btree_insert(&mut self, index_name: &str, key: IndexKey, memory_id: u64) {
        if let Some(index) = self.btree_indexes.get_mut(index_name) {
            index.insert(key, memory_id);
            self.stats.inserts += 1;
            self.stats.total_entries += 1;
        }
    }

    /// Lookup in B-tree
    pub fn btree_get(&mut self, index_name: &str, key: &IndexKey) -> Vec<u64> {
        self.stats.lookups += 1;
        self.btree_indexes.get(index_name)
            .and_then(|idx| idx.get(key))
            .cloned()
            .unwrap_or_default()
    }

    /// Index text document
    pub fn index_text(&mut self, index_name: &str, memory_id: u64, text: &str) {
        if let Some(index) = self.inverted_indexes.get_mut(index_name) {
            index.index_document(memory_id, text);
            self.stats.inserts += 1;
        }
    }

    /// Search text
    pub fn search_text(&mut self, index_name: &str, query: &str) -> Vec<(u64, f64)> {
        self.stats.lookups += 1;
        self.inverted_indexes.get(index_name)
            .map(|idx| idx.search(query))
            .unwrap_or_default()
    }

    /// Insert temporal
    pub fn temporal_insert(&mut self, index_name: &str, timestamp: Timestamp, memory_id: u64) {
        if let Some(index) = self.temporal_indexes.get_mut(index_name) {
            index.insert(timestamp, memory_id);
            self.stats.inserts += 1;
            self.stats.total_entries += 1;
        }
    }

    /// Query temporal range
    pub fn temporal_range(&mut self, index_name: &str, start: Timestamp, end: Timestamp) -> Vec<u64> {
        self.stats.lookups += 1;
        self.temporal_indexes.get(index_name)
            .map(|idx| idx.query_range(start, end))
            .unwrap_or_default()
    }

    /// Get statistics
    pub fn stats(&self) -> &IndexStats {
        &self.stats
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_index() {
        let mut index = BTreeIndex::new(1, "test");

        index.insert(IndexKey::String("key1".into()), 100);
        index.insert(IndexKey::String("key1".into()), 101);
        index.insert(IndexKey::String("key2".into()), 200);

        let result = index.get(&IndexKey::String("key1".into()));
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_inverted_index() {
        let mut index = InvertedIndex::new(1, "test");

        index.index_document(1, "hello world");
        index.index_document(2, "hello there");
        index.index_document(3, "goodbye world");

        let results = index.search("hello");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_temporal_index() {
        let mut index = TemporalIndex::new(1, "test");

        index.insert(Timestamp(1000), 1);
        index.insert(Timestamp(2000), 2);
        index.insert(Timestamp(3000), 3);

        let results = index.query_range(Timestamp(1000), Timestamp(2000));
        assert_eq!(results.len(), 2);

        let recent = index.most_recent(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_index_manager() {
        let mut manager = IndexManager::new();

        manager.create_btree("names");
        manager.btree_insert("names", IndexKey::String("alice".into()), 1);
        manager.btree_insert("names", IndexKey::String("bob".into()), 2);

        let results = manager.btree_get("names", &IndexKey::String("alice".into()));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_range_query() {
        let mut index = BTreeIndex::new(1, "numbers");

        for i in 0..10 {
            index.insert(IndexKey::Numeric(i), i as u64);
        }

        let results = index.range(&IndexKey::Numeric(3), &IndexKey::Numeric(7));
        assert_eq!(results.len(), 5);
    }
}
