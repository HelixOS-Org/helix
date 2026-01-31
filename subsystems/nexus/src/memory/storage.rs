//! # Memory Storage Backend
//!
//! Persistent storage backend for long-term memories.
//! Handles serialization, compression, and indexing.
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
// STORAGE TYPES
// ============================================================================

/// Stored memory record
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    /// Record ID
    pub id: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Key
    pub key: String,
    /// Value (serialized)
    pub value: Vec<u8>,
    /// Metadata
    pub metadata: RecordMetadata,
    /// Compression
    pub compression: CompressionType,
    /// Created
    pub created: Timestamp,
    /// Modified
    pub modified: Timestamp,
    /// Access count
    pub access_count: u64,
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
    Working,
    Index,
}

/// Record metadata
#[derive(Debug, Clone)]
pub struct RecordMetadata {
    /// Size (bytes)
    pub size: usize,
    /// Checksum
    pub checksum: u32,
    /// Version
    pub version: u32,
    /// Tags
    pub tags: Vec<String>,
    /// Priority
    pub priority: u8,
    /// Expiration
    pub expires: Option<Timestamp>,
}

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
    Custom(u8),
}

// ============================================================================
// INDEX
// ============================================================================

/// Storage index
#[derive(Debug, Clone)]
pub struct StorageIndex {
    /// Index entries by key
    by_key: BTreeMap<String, u64>,
    /// By type
    by_type: BTreeMap<MemoryType, Vec<u64>>,
    /// By tag
    by_tag: BTreeMap<String, Vec<u64>>,
    /// Full-text index (simplified)
    full_text: BTreeMap<String, Vec<u64>>,
}

impl StorageIndex {
    /// Create new index
    pub fn new() -> Self {
        Self {
            by_key: BTreeMap::new(),
            by_type: BTreeMap::new(),
            by_tag: BTreeMap::new(),
            full_text: BTreeMap::new(),
        }
    }

    /// Add entry
    pub fn add(&mut self, id: u64, key: &str, memory_type: MemoryType, tags: &[String]) {
        self.by_key.insert(key.into(), id);
        self.by_type.entry(memory_type).or_insert_with(Vec::new).push(id);

        for tag in tags {
            self.by_tag.entry(tag.clone()).or_insert_with(Vec::new).push(id);
        }

        // Simple word tokenization for full-text
        for word in key.split_whitespace() {
            let word = word.to_lowercase();
            self.full_text.entry(word).or_insert_with(Vec::new).push(id);
        }
    }

    /// Remove entry
    pub fn remove(&mut self, id: u64, key: &str, memory_type: MemoryType) {
        self.by_key.remove(key);

        if let Some(ids) = self.by_type.get_mut(&memory_type) {
            ids.retain(|&i| i != id);
        }

        // Remove from all tags (simplified)
        for ids in self.by_tag.values_mut() {
            ids.retain(|&i| i != id);
        }

        for ids in self.full_text.values_mut() {
            ids.retain(|&i| i != id);
        }
    }

    /// Lookup by key
    pub fn get(&self, key: &str) -> Option<u64> {
        self.by_key.get(key).copied()
    }

    /// Find by type
    pub fn find_by_type(&self, memory_type: MemoryType) -> &[u64] {
        self.by_type.get(&memory_type).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Find by tag
    pub fn find_by_tag(&self, tag: &str) -> &[u64] {
        self.by_tag.get(tag).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Search full-text
    pub fn search(&self, query: &str) -> Vec<u64> {
        let words: Vec<_> = query.split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();

        if words.is_empty() {
            return Vec::new();
        }

        // Find IDs matching all words
        let mut result: Option<alloc::collections::BTreeSet<u64>> = None;

        for word in words {
            if let Some(ids) = self.full_text.get(&word) {
                let set: alloc::collections::BTreeSet<u64> = ids.iter().copied().collect();
                result = Some(match result {
                    Some(existing) => existing.intersection(&set).copied().collect(),
                    None => set,
                });
            } else {
                return Vec::new(); // Word not found
            }
        }

        result.map(|s| s.into_iter().collect()).unwrap_or_default()
    }
}

impl Default for StorageIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MEMORY STORAGE
// ============================================================================

/// Memory storage backend
pub struct MemoryStorage {
    /// Records
    records: BTreeMap<u64, MemoryRecord>,
    /// Index
    index: StorageIndex,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: StorageConfig,
    /// Statistics
    stats: StorageStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Default compression
    pub default_compression: CompressionType,
    /// Enable checksums
    pub enable_checksums: bool,
    /// Maximum record size
    pub max_record_size: usize,
    /// Enable expiration
    pub enable_expiration: bool,
    /// Compact threshold (percentage of deleted)
    pub compact_threshold: f64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            default_compression: CompressionType::None,
            enable_checksums: true,
            max_record_size: 10 * 1024 * 1024, // 10 MB
            enable_expiration: true,
            compact_threshold: 0.3,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total records
    pub total_records: u64,
    /// Total size (bytes)
    pub total_size: u64,
    /// Reads
    pub reads: u64,
    /// Writes
    pub writes: u64,
    /// Deletes
    pub deletes: u64,
    /// Cache hits
    pub cache_hits: u64,
}

impl MemoryStorage {
    /// Create new storage
    pub fn new(config: StorageConfig) -> Self {
        Self {
            records: BTreeMap::new(),
            index: StorageIndex::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: StorageStats::default(),
        }
    }

    /// Store record
    pub fn store(
        &mut self,
        key: &str,
        value: Vec<u8>,
        memory_type: MemoryType,
        tags: Vec<String>,
    ) -> Result<u64, StorageError> {
        // Check size
        if value.len() > self.config.max_record_size {
            return Err(StorageError::RecordTooLarge);
        }

        // Check for existing
        if self.index.get(key).is_some() {
            return Err(StorageError::KeyExists);
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();
        let size = value.len();

        // Compute checksum
        let checksum = if self.config.enable_checksums {
            self.compute_checksum(&value)
        } else {
            0
        };

        // Compress if configured
        let (compressed_value, compression) = self.compress(value);

        let record = MemoryRecord {
            id,
            memory_type,
            key: key.into(),
            value: compressed_value,
            metadata: RecordMetadata {
                size,
                checksum,
                version: 1,
                tags: tags.clone(),
                priority: 0,
                expires: None,
            },
            compression,
            created: now,
            modified: now,
            access_count: 0,
        };

        self.index.add(id, key, memory_type, &tags);
        self.stats.total_records += 1;
        self.stats.total_size += size as u64;
        self.stats.writes += 1;

        self.records.insert(id, record);
        Ok(id)
    }

    /// Get record
    pub fn get(&mut self, key: &str) -> Result<Vec<u8>, StorageError> {
        let id = self.index.get(key).ok_or(StorageError::NotFound)?;
        self.get_by_id(id)
    }

    /// Get by ID
    pub fn get_by_id(&mut self, id: u64) -> Result<Vec<u8>, StorageError> {
        let record = self.records.get_mut(&id).ok_or(StorageError::NotFound)?;

        // Check expiration
        if self.config.enable_expiration {
            if let Some(expires) = record.metadata.expires {
                if Timestamp::now().0 > expires.0 {
                    return Err(StorageError::Expired);
                }
            }
        }

        // Decompress
        let value = self.decompress(&record.value, record.compression)?;

        // Verify checksum
        if self.config.enable_checksums {
            let checksum = self.compute_checksum(&value);
            if checksum != record.metadata.checksum {
                return Err(StorageError::ChecksumMismatch);
            }
        }

        record.access_count += 1;
        self.stats.reads += 1;

        Ok(value)
    }

    /// Update record
    pub fn update(&mut self, key: &str, value: Vec<u8>) -> Result<(), StorageError> {
        let id = self.index.get(key).ok_or(StorageError::NotFound)?;
        let record = self.records.get_mut(&id).ok_or(StorageError::NotFound)?;

        // Check size
        if value.len() > self.config.max_record_size {
            return Err(StorageError::RecordTooLarge);
        }

        let old_size = record.metadata.size;
        let new_size = value.len();

        // Compute new checksum
        let checksum = if self.config.enable_checksums {
            self.compute_checksum(&value)
        } else {
            0
        };

        let (compressed_value, compression) = self.compress(value);

        record.value = compressed_value;
        record.compression = compression;
        record.metadata.size = new_size;
        record.metadata.checksum = checksum;
        record.metadata.version += 1;
        record.modified = Timestamp::now();

        self.stats.total_size = self.stats.total_size - old_size as u64 + new_size as u64;
        self.stats.writes += 1;

        Ok(())
    }

    /// Delete record
    pub fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        let id = self.index.get(key).ok_or(StorageError::NotFound)?;
        let record = self.records.remove(&id).ok_or(StorageError::NotFound)?;

        self.index.remove(id, key, record.memory_type);
        self.stats.total_records -= 1;
        self.stats.total_size -= record.metadata.size as u64;
        self.stats.deletes += 1;

        Ok(())
    }

    /// Query by type
    pub fn query_type(&self, memory_type: MemoryType) -> Vec<&MemoryRecord> {
        self.index.find_by_type(memory_type)
            .iter()
            .filter_map(|id| self.records.get(id))
            .collect()
    }

    /// Query by tag
    pub fn query_tag(&self, tag: &str) -> Vec<&MemoryRecord> {
        self.index.find_by_tag(tag)
            .iter()
            .filter_map(|id| self.records.get(id))
            .collect()
    }

    /// Search
    pub fn search(&self, query: &str) -> Vec<&MemoryRecord> {
        self.index.search(query)
            .iter()
            .filter_map(|id| self.records.get(id))
            .collect()
    }

    /// Set expiration
    pub fn set_expiration(&mut self, key: &str, expires: Timestamp) -> Result<(), StorageError> {
        let id = self.index.get(key).ok_or(StorageError::NotFound)?;
        let record = self.records.get_mut(&id).ok_or(StorageError::NotFound)?;
        record.metadata.expires = Some(expires);
        Ok(())
    }

    /// Clean expired
    pub fn clean_expired(&mut self) -> usize {
        if !self.config.enable_expiration {
            return 0;
        }

        let now = Timestamp::now();
        let expired: Vec<(u64, String, MemoryType)> = self.records.iter()
            .filter_map(|(&id, record)| {
                if let Some(expires) = record.metadata.expires {
                    if now.0 > expires.0 {
                        return Some((id, record.key.clone(), record.memory_type));
                    }
                }
                None
            })
            .collect();

        let count = expired.len();

        for (id, key, memory_type) in expired {
            self.records.remove(&id);
            self.index.remove(id, &key, memory_type);
            self.stats.total_records -= 1;
        }

        count
    }

    fn compute_checksum(&self, data: &[u8]) -> u32 {
        // Simple checksum (in real impl, use CRC32)
        let mut sum: u32 = 0;
        for &byte in data {
            sum = sum.wrapping_add(byte as u32);
        }
        sum
    }

    fn compress(&self, data: Vec<u8>) -> (Vec<u8>, CompressionType) {
        // Simplified - no actual compression
        match self.config.default_compression {
            CompressionType::None => (data, CompressionType::None),
            _ => (data, CompressionType::None), // Would compress in real impl
        }
    }

    fn decompress(&self, data: &[u8], _compression: CompressionType) -> Result<Vec<u8>, StorageError> {
        // Simplified - no actual decompression
        Ok(data.to_vec())
    }

    /// Get statistics
    pub fn stats(&self) -> &StorageStats {
        &self.stats
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new(StorageConfig::default())
    }
}

/// Storage error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    NotFound,
    KeyExists,
    RecordTooLarge,
    ChecksumMismatch,
    Expired,
    CompressionError,
    IoError,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get() {
        let mut storage = MemoryStorage::default();

        let data = b"test data".to_vec();
        let id = storage.store("key1", data.clone(), MemoryType::Semantic, vec![]).unwrap();

        let retrieved = storage.get("key1").unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_update() {
        let mut storage = MemoryStorage::default();

        storage.store("key1", b"old".to_vec(), MemoryType::Semantic, vec![]).unwrap();
        storage.update("key1", b"new".to_vec()).unwrap();

        let retrieved = storage.get("key1").unwrap();
        assert_eq!(retrieved, b"new".to_vec());
    }

    #[test]
    fn test_delete() {
        let mut storage = MemoryStorage::default();

        storage.store("key1", b"data".to_vec(), MemoryType::Semantic, vec![]).unwrap();
        storage.delete("key1").unwrap();

        assert!(storage.get("key1").is_err());
    }

    #[test]
    fn test_query_by_type() {
        let mut storage = MemoryStorage::default();

        storage.store("s1", b"1".to_vec(), MemoryType::Semantic, vec![]).unwrap();
        storage.store("s2", b"2".to_vec(), MemoryType::Semantic, vec![]).unwrap();
        storage.store("e1", b"3".to_vec(), MemoryType::Episodic, vec![]).unwrap();

        let semantic = storage.query_type(MemoryType::Semantic);
        assert_eq!(semantic.len(), 2);
    }

    #[test]
    fn test_query_by_tag() {
        let mut storage = MemoryStorage::default();

        storage.store("k1", b"1".to_vec(), MemoryType::Semantic, vec!["tag1".into()]).unwrap();
        storage.store("k2", b"2".to_vec(), MemoryType::Semantic, vec!["tag1".into(), "tag2".into()]).unwrap();

        let tagged = storage.query_tag("tag1");
        assert_eq!(tagged.len(), 2);
    }

    #[test]
    fn test_search() {
        let mut storage = MemoryStorage::default();

        storage.store("hello world", b"1".to_vec(), MemoryType::Semantic, vec![]).unwrap();
        storage.store("hello there", b"2".to_vec(), MemoryType::Semantic, vec![]).unwrap();

        let results = storage.search("hello");
        assert_eq!(results.len(), 2);

        let results = storage.search("world");
        assert_eq!(results.len(), 1);
    }
}
