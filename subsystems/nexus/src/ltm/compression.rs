//! # Memory Compression
//!
//! Compresses and summarizes long-term memory.
//! Reduces storage while preserving essential information.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// COMPRESSION TYPES
// ============================================================================

/// Memory item
#[derive(Debug, Clone)]
pub struct MemoryItem {
    /// Item ID
    pub id: u64,
    /// Content
    pub content: String,
    /// Type
    pub item_type: ItemType,
    /// Importance
    pub importance: f64,
    /// Size bytes
    pub size: usize,
    /// Created
    pub created: Timestamp,
    /// Compressed
    pub compressed: bool,
}

/// Item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Fact,
    Episode,
    Concept,
    Procedure,
    Relationship,
}

/// Compressed memory
#[derive(Debug, Clone)]
pub struct CompressedMemory {
    /// Compressed ID
    pub id: u64,
    /// Original IDs
    pub originals: Vec<u64>,
    /// Summary
    pub summary: String,
    /// Type
    pub item_type: ItemType,
    /// Importance
    pub importance: f64,
    /// Compression ratio
    pub ratio: f64,
    /// Created
    pub created: Timestamp,
}

/// Compression method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMethod {
    Summary,
    Merge,
    Hierarchical,
    Semantic,
    Temporal,
}

/// Compression result
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Compressed items
    pub compressed: Vec<CompressedMemory>,
    /// Items removed
    pub removed: usize,
    /// Space saved bytes
    pub space_saved: usize,
    /// Overall ratio
    pub ratio: f64,
}

/// Compression policy
#[derive(Debug, Clone)]
pub struct CompressionPolicy {
    /// Policy ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Method
    pub method: CompressionMethod,
    /// Age threshold ns
    pub age_threshold_ns: u64,
    /// Importance threshold
    pub importance_threshold: f64,
    /// Minimum items
    pub min_items: usize,
}

// ============================================================================
// MEMORY COMPRESSOR
// ============================================================================

/// Memory compressor
pub struct MemoryCompressor {
    /// Items
    items: BTreeMap<u64, MemoryItem>,
    /// Compressed
    compressed: BTreeMap<u64, CompressedMemory>,
    /// Policies
    policies: Vec<CompressionPolicy>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: CompressorConfig,
    /// Statistics
    stats: CompressorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct CompressorConfig {
    /// Default age ns
    pub default_age_ns: u64,
    /// Target ratio
    pub target_ratio: f64,
    /// Preserve important
    pub preserve_important: bool,
    /// Importance threshold
    pub importance_threshold: f64,
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            default_age_ns: 86_400_000_000_000, // 1 day
            target_ratio: 0.5,
            preserve_important: true,
            importance_threshold: 0.8,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct CompressorStats {
    /// Items stored
    pub items_stored: u64,
    /// Compressions performed
    pub compressions: u64,
    /// Total bytes saved
    pub bytes_saved: u64,
    /// Average ratio
    pub average_ratio: f64,
}

impl MemoryCompressor {
    /// Create new compressor
    pub fn new(config: CompressorConfig) -> Self {
        Self {
            items: BTreeMap::new(),
            compressed: BTreeMap::new(),
            policies: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: CompressorStats::default(),
        }
    }

    /// Store item
    pub fn store(&mut self, content: &str, item_type: ItemType, importance: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let item = MemoryItem {
            id,
            content: content.into(),
            item_type,
            importance: importance.clamp(0.0, 1.0),
            size: content.len(),
            created: Timestamp::now(),
            compressed: false,
        };

        self.items.insert(id, item);
        self.stats.items_stored += 1;

        id
    }

    /// Compress old items
    pub fn compress(&mut self, method: CompressionMethod) -> CompressionResult {
        let now = Timestamp::now().0;
        let age_threshold = self.config.default_age_ns;

        // Find candidates
        let candidates: Vec<_> = self
            .items
            .iter()
            .filter(|(_, item)| {
                !item.compressed
                    && now - item.created.0 > age_threshold
                    && (!self.config.preserve_important
                        || item.importance < self.config.importance_threshold)
            })
            .map(|(&id, item)| (id, item.clone()))
            .collect();

        if candidates.is_empty() {
            return CompressionResult {
                compressed: Vec::new(),
                removed: 0,
                space_saved: 0,
                ratio: 1.0,
            };
        }

        let result = match method {
            CompressionMethod::Summary => self.compress_summary(&candidates),
            CompressionMethod::Merge => self.compress_merge(&candidates),
            CompressionMethod::Hierarchical => self.compress_hierarchical(&candidates),
            CompressionMethod::Semantic => self.compress_semantic(&candidates),
            CompressionMethod::Temporal => self.compress_temporal(&candidates),
        };

        // Mark originals as compressed
        for cm in &result.compressed {
            for &orig_id in &cm.originals {
                if let Some(item) = self.items.get_mut(&orig_id) {
                    item.compressed = true;
                }
            }
        }

        // Update stats
        self.stats.compressions += 1;
        self.stats.bytes_saved += result.space_saved as u64;

        if self.stats.compressions > 0 {
            self.stats.average_ratio =
                (self.stats.average_ratio * (self.stats.compressions - 1) as f64 + result.ratio)
                    / self.stats.compressions as f64;
        }

        result
    }

    fn compress_summary(&mut self, candidates: &[(u64, MemoryItem)]) -> CompressionResult {
        let mut compressed = Vec::new();
        let mut space_saved = 0usize;

        // Group by type
        let mut by_type: BTreeMap<ItemType, Vec<(u64, &MemoryItem)>> = BTreeMap::new();

        for (id, item) in candidates {
            by_type
                .entry(item.item_type)
                .or_insert_with(Vec::new)
                .push((*id, item));
        }

        for (item_type, items) in by_type {
            if items.len() < 2 {
                continue;
            }

            let id = self.next_id.fetch_add(1, Ordering::Relaxed);

            // Create summary
            let summary = self.create_summary(&items);
            let original_size: usize = items.iter().map(|(_, i)| i.size).sum();
            let summary_size = summary.len();

            let importance = items
                .iter()
                .map(|(_, i)| i.importance)
                .fold(0.0_f64, |a, b| a.max(b));

            let cm = CompressedMemory {
                id,
                originals: items.iter().map(|(id, _)| *id).collect(),
                summary,
                item_type,
                importance,
                ratio: summary_size as f64 / original_size as f64,
                created: Timestamp::now(),
            };

            space_saved += original_size - summary_size;
            self.compressed.insert(id, cm.clone());
            compressed.push(cm);
        }

        let total_original: usize = candidates.iter().map(|(_, i)| i.size).sum();
        let ratio = if total_original > 0 {
            (total_original - space_saved) as f64 / total_original as f64
        } else {
            1.0
        };

        CompressionResult {
            compressed,
            removed: candidates.len(),
            space_saved,
            ratio,
        }
    }

    fn create_summary(&self, items: &[(u64, &MemoryItem)]) -> String {
        // Simple summary: take first words of each item
        let mut parts = Vec::new();

        for (_, item) in items.iter().take(5) {
            let words: Vec<_> = item.content.split_whitespace().take(10).collect();
            if !words.is_empty() {
                parts.push(words.join(" "));
            }
        }

        if items.len() > 5 {
            parts.push(format!("... and {} more", items.len() - 5));
        }

        parts.join("; ")
    }

    fn compress_merge(&mut self, candidates: &[(u64, MemoryItem)]) -> CompressionResult {
        // Merge similar items
        let mut compressed = Vec::new();
        let mut space_saved = 0usize;
        let mut used = Vec::new();

        for (id1, item1) in candidates {
            if used.contains(id1) {
                continue;
            }

            let mut group = vec![(*id1, item1)];

            // Find similar items
            for (id2, item2) in candidates {
                if id1 != id2 && !used.contains(id2) {
                    if self.is_similar(&item1.content, &item2.content) {
                        group.push((*id2, item2));
                        used.push(*id2);
                    }
                }
            }

            used.push(*id1);

            if group.len() > 1 {
                let cm_id = self.next_id.fetch_add(1, Ordering::Relaxed);

                let merged = self.merge_contents(&group);
                let original_size: usize = group.iter().map(|(_, i)| i.size).sum();

                let cm = CompressedMemory {
                    id: cm_id,
                    originals: group.iter().map(|(id, _)| *id).collect(),
                    summary: merged.clone(),
                    item_type: item1.item_type,
                    importance: group
                        .iter()
                        .map(|(_, i)| i.importance)
                        .fold(0.0_f64, f64::max),
                    ratio: merged.len() as f64 / original_size as f64,
                    created: Timestamp::now(),
                };

                space_saved += original_size - merged.len();
                self.compressed.insert(cm_id, cm.clone());
                compressed.push(cm);
            }
        }

        let total: usize = candidates.iter().map(|(_, i)| i.size).sum();
        let ratio = if total > 0 {
            (total - space_saved) as f64 / total as f64
        } else {
            1.0
        };

        CompressionResult {
            compressed,
            removed: used.len(),
            space_saved,
            ratio,
        }
    }

    fn is_similar(&self, a: &str, b: &str) -> bool {
        // Simple word overlap check
        let words_a: Vec<_> = a.split_whitespace().collect();
        let words_b: Vec<_> = b.split_whitespace().collect();

        let overlap = words_a.iter().filter(|w| words_b.contains(w)).count();

        let min_len = words_a.len().min(words_b.len());

        if min_len == 0 {
            return false;
        }

        overlap as f64 / min_len as f64 > 0.5
    }

    fn merge_contents(&self, items: &[(u64, &MemoryItem)]) -> String {
        // Take unique words from all items
        let mut all_words = Vec::new();

        for (_, item) in items {
            for word in item.content.split_whitespace() {
                if !all_words.contains(&word) {
                    all_words.push(word);
                }
            }
        }

        all_words.into_iter().take(50).collect::<Vec<_>>().join(" ")
    }

    fn compress_hierarchical(&mut self, candidates: &[(u64, MemoryItem)]) -> CompressionResult {
        // Create hierarchy levels
        self.compress_summary(candidates) // Simplified
    }

    fn compress_semantic(&mut self, candidates: &[(u64, MemoryItem)]) -> CompressionResult {
        // Semantic compression
        self.compress_merge(candidates) // Simplified
    }

    fn compress_temporal(&mut self, candidates: &[(u64, MemoryItem)]) -> CompressionResult {
        // Group by time periods
        self.compress_summary(candidates) // Simplified
    }

    /// Add policy
    pub fn add_policy(&mut self, policy: CompressionPolicy) {
        self.policies.push(policy);
    }

    /// Run policies
    pub fn run_policies(&mut self) -> Vec<CompressionResult> {
        let mut results = Vec::new();

        for policy in self.policies.clone() {
            let result = self.compress(policy.method);
            results.push(result);
        }

        results
    }

    /// Get item
    pub fn get(&self, id: u64) -> Option<&MemoryItem> {
        self.items.get(&id)
    }

    /// Get compressed
    pub fn get_compressed(&self, id: u64) -> Option<&CompressedMemory> {
        self.compressed.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &CompressorStats {
        &self.stats
    }

    /// Memory usage
    pub fn memory_usage(&self) -> usize {
        self.items.values().map(|i| i.size).sum::<usize>()
            + self
                .compressed
                .values()
                .map(|c| c.summary.len())
                .sum::<usize>()
    }
}

impl Default for MemoryCompressor {
    fn default() -> Self {
        Self::new(CompressorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store() {
        let mut compressor = MemoryCompressor::default();

        let id = compressor.store("test content", ItemType::Fact, 0.5);
        assert!(compressor.get(id).is_some());
    }

    #[test]
    fn test_compress_summary() {
        let mut compressor = MemoryCompressor::new(CompressorConfig {
            default_age_ns: 0, // immediate
            ..Default::default()
        });

        for i in 0..5 {
            compressor.store(&format!("item {}", i), ItemType::Fact, 0.3);
        }

        let result = compressor.compress(CompressionMethod::Summary);
        assert!(!result.compressed.is_empty());
    }

    #[test]
    fn test_compress_merge() {
        let mut compressor = MemoryCompressor::new(CompressorConfig {
            default_age_ns: 0,
            ..Default::default()
        });

        compressor.store("hello world test", ItemType::Fact, 0.3);
        compressor.store("hello world example", ItemType::Fact, 0.3);

        let result = compressor.compress(CompressionMethod::Merge);
        assert!(result.space_saved > 0 || result.compressed.len() > 0);
    }

    #[test]
    fn test_preserve_important() {
        let mut compressor = MemoryCompressor::new(CompressorConfig {
            default_age_ns: 0,
            preserve_important: true,
            importance_threshold: 0.8,
            ..Default::default()
        });

        compressor.store("important", ItemType::Fact, 0.9);
        compressor.store("not important", ItemType::Fact, 0.3);

        let result = compressor.compress(CompressionMethod::Summary);

        // Important item should not be compressed
        let items: Vec<_> = compressor
            .items
            .values()
            .filter(|i| !i.compressed)
            .collect();

        assert!(items.iter().any(|i| i.importance > 0.8));
    }
}
