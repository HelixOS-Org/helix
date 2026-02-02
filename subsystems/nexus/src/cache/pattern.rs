//! Cache access pattern tracking and prediction.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::CacheKey;

// ============================================================================
// ACCESS PATTERN TRACKER
// ============================================================================

/// Tracks cache access patterns
pub struct AccessPatternTracker {
    /// Recent accesses
    history: Vec<CacheKey>,
    /// Max history size
    max_history: usize,
    /// Access frequency map
    frequencies: BTreeMap<CacheKey, u32>,
    /// Access sequences
    sequences: BTreeMap<(CacheKey, CacheKey), u32>,
    /// Detected pattern
    detected_pattern: Option<AccessPattern>,
}

/// Cache access pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    /// Sequential access
    Sequential,
    /// Random access
    Random,
    /// Temporal locality (same items repeatedly)
    Temporal,
    /// Strided access
    Strided { stride: i64 },
    /// Working set pattern
    WorkingSet { size: usize },
}

impl AccessPatternTracker {
    /// Create new tracker
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_history),
            max_history,
            frequencies: BTreeMap::new(),
            sequences: BTreeMap::new(),
            detected_pattern: None,
        }
    }

    /// Record access
    pub fn record(&mut self, key: CacheKey) {
        // Update sequence tracking
        if let Some(&prev) = self.history.last() {
            *self.sequences.entry((prev, key)).or_insert(0) += 1;
        }

        // Update frequency
        *self.frequencies.entry(key).or_insert(0) += 1;

        // Add to history
        self.history.push(key);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Update pattern detection
        if self.history.len() >= 100 {
            self.detect_pattern();
        }
    }

    /// Detect access pattern
    fn detect_pattern(&mut self) {
        let len = self.history.len();
        if len < 10 {
            return;
        }

        // Check for sequential
        let mut sequential_count = 0;
        for i in 1..len {
            if self.history[i] == self.history[i - 1] + 1 {
                sequential_count += 1;
            }
        }
        let seq_ratio = sequential_count as f64 / (len - 1) as f64;
        if seq_ratio > 0.7 {
            self.detected_pattern = Some(AccessPattern::Sequential);
            return;
        }

        // Check for stride
        let mut strides: BTreeMap<i64, u32> = BTreeMap::new();
        for i in 1..len {
            let stride = self.history[i] as i64 - self.history[i - 1] as i64;
            *strides.entry(stride).or_insert(0) += 1;
        }
        if let Some((&stride, &count)) = strides.iter().max_by_key(|&(_, c)| c) {
            if count as f64 / (len - 1) as f64 > 0.6 && stride != 0 && stride != 1 {
                self.detected_pattern = Some(AccessPattern::Strided { stride });
                return;
            }
        }

        // Check for temporal locality
        let unique_keys = self.frequencies.len();
        if unique_keys < len / 5 {
            self.detected_pattern = Some(AccessPattern::Temporal);
            return;
        }

        // Check for working set
        if unique_keys < len / 2 {
            self.detected_pattern = Some(AccessPattern::WorkingSet { size: unique_keys });
            return;
        }

        self.detected_pattern = Some(AccessPattern::Random);
    }

    /// Get detected pattern
    pub fn pattern(&self) -> Option<AccessPattern> {
        self.detected_pattern
    }

    /// Predict next access
    pub fn predict_next(&self) -> Option<CacheKey> {
        if self.history.is_empty() {
            return None;
        }

        let last = *self.history.last()?;

        match self.detected_pattern? {
            AccessPattern::Sequential => Some(last + 1),
            AccessPattern::Strided { stride } => Some((last as i64 + stride) as u64),
            AccessPattern::Temporal | AccessPattern::WorkingSet { .. } => {
                // Find most likely successor
                let successors: Vec<_> = self
                    .sequences
                    .iter()
                    .filter(|((prev, _), _)| *prev == last)
                    .collect();

                successors
                    .iter()
                    .max_by_key(|&(_, count)| count)
                    .map(|&((_, next), _)| *next)
            },
            AccessPattern::Random => None,
        }
    }

    /// Get prefetch suggestions
    pub fn prefetch_suggestions(&self, count: usize) -> Vec<CacheKey> {
        let mut suggestions = Vec::new();

        if let Some(next) = self.predict_next() {
            suggestions.push(next);

            match self.detected_pattern {
                Some(AccessPattern::Sequential) => {
                    for i in 1..count {
                        suggestions.push(next + i as u64);
                    }
                },
                Some(AccessPattern::Strided { stride }) => {
                    for i in 1..count {
                        suggestions.push((next as i64 + stride * i as i64) as u64);
                    }
                },
                _ => {},
            }
        }

        suggestions
    }

    /// Clear tracker
    pub fn clear(&mut self) {
        self.history.clear();
        self.frequencies.clear();
        self.sequences.clear();
        self.detected_pattern = None;
    }
}

impl Default for AccessPatternTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}
