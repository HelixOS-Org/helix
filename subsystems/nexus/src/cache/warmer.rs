//! Cache warming and prefetching.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::types::CacheKey;

// ============================================================================
// CACHE WARMER
// ============================================================================

/// Pre-warms cache with predicted entries
pub struct CacheWarmer {
    /// Warm candidates
    candidates: Vec<WarmCandidate>,
    /// Max candidates
    max_candidates: usize,
    /// Historical patterns
    patterns: BTreeMap<String, Vec<CacheKey>>,
}

/// Warm candidate
#[derive(Debug, Clone)]
struct WarmCandidate {
    /// Key to warm
    key: CacheKey,
    /// Priority
    priority: f64,
    /// Source (why suggested)
    source: String,
}

impl CacheWarmer {
    /// Create new warmer
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            max_candidates: 1000,
            patterns: BTreeMap::new(),
        }
    }

    /// Add warm candidate
    pub fn add_candidate(&mut self, key: CacheKey, priority: f64, source: &str) {
        self.candidates.push(WarmCandidate {
            key,
            priority,
            source: String::from(source),
        });

        // Sort by priority (descending)
        self.candidates
            .sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());

        // Limit size
        if self.candidates.len() > self.max_candidates {
            self.candidates.truncate(self.max_candidates);
        }
    }

    /// Learn pattern
    pub fn learn_pattern(&mut self, name: &str, keys: Vec<CacheKey>) {
        self.patterns.insert(String::from(name), keys);
    }

    /// Apply pattern
    pub fn apply_pattern(&mut self, name: &str) {
        if let Some(keys) = self.patterns.get(name).cloned() {
            for (i, key) in keys.iter().enumerate() {
                let priority = 1.0 - (i as f64 / keys.len() as f64);
                self.add_candidate(*key, priority, name);
            }
        }
    }

    /// Get next candidates to warm
    pub fn next_candidates(&mut self, count: usize) -> Vec<CacheKey> {
        let mut result = Vec::new();
        for _ in 0..count {
            if let Some(candidate) = self.candidates.pop() {
                result.push(candidate.key);
            } else {
                break;
            }
        }
        result
    }

    /// Has candidates
    pub fn has_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }

    /// Candidate count
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Clear candidates
    pub fn clear(&mut self) {
        self.candidates.clear();
    }
}

impl Default for CacheWarmer {
    fn default() -> Self {
        Self::new()
    }
}
