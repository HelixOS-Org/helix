//! Memory access pattern detection.

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{AccessPattern, AccessRecord};

// ============================================================================
// PATTERN DETECTOR
// ============================================================================

/// Detects memory access patterns
pub struct PatternDetector {
    /// Recent accesses for analysis
    history: VecDeque<AccessRecord>,
    /// Maximum history size
    max_history: usize,
    /// Detected strides
    strides: BTreeMap<i64, u32>,
    /// Total accesses analyzed
    total_accesses: AtomicU64,
    /// Pattern detection confidence threshold
    confidence_threshold: f64,
}

impl PatternDetector {
    /// Create new pattern detector
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_history),
            max_history,
            strides: BTreeMap::new(),
            total_accesses: AtomicU64::new(0),
            confidence_threshold: 0.7,
        }
    }

    /// Record an access
    pub fn record(&mut self, record: AccessRecord) {
        // Calculate stride from previous access
        if let Some(prev) = self.history.back() {
            let stride = record.address as i64 - prev.address as i64;
            *self.strides.entry(stride).or_insert(0) += 1;
        }

        self.history.push_back(record);
        self.total_accesses.fetch_add(1, Ordering::Relaxed);

        // Evict old entries
        if self.history.len() > self.max_history {
            self.history.pop_front();
            // Decay stride counts
            self.strides.retain(|_, count| {
                *count = (*count * 9) / 10;
                *count > 0
            });
        }
    }

    /// Detect the dominant access pattern
    pub fn detect_pattern(&self) -> (AccessPattern, f64) {
        if self.history.len() < 10 {
            return (AccessPattern::Mixed, 0.0);
        }

        // Check for sequential pattern
        let (seq_score, is_reverse) = self.check_sequential();
        if seq_score > self.confidence_threshold {
            return if is_reverse {
                (AccessPattern::ReverseSequential, seq_score)
            } else {
                (AccessPattern::Sequential, seq_score)
            };
        }

        // Check for strided pattern
        if let Some((stride, score)) = self.check_strided() {
            if score > self.confidence_threshold {
                return (AccessPattern::Strided { stride }, score);
            }
        }

        // Check for temporal locality
        let temporal_score = self.check_temporal();
        if temporal_score > self.confidence_threshold {
            return (AccessPattern::Temporal, temporal_score);
        }

        // Check for stack pattern
        let stack_score = self.check_stack();
        if stack_score > self.confidence_threshold {
            return (AccessPattern::Stack, stack_score);
        }

        // Check for pointer chasing
        let ptr_score = self.check_pointer_chasing();
        if ptr_score > self.confidence_threshold {
            return (AccessPattern::PointerChasing, ptr_score);
        }

        // Random or mixed
        let best_score = seq_score
            .max(temporal_score)
            .max(stack_score)
            .max(ptr_score);

        if best_score > 0.3 {
            (AccessPattern::Mixed, best_score)
        } else {
            (AccessPattern::Random, 1.0 - best_score)
        }
    }

    /// Check for sequential access pattern
    fn check_sequential(&self) -> (f64, bool) {
        if self.history.len() < 2 {
            return (0.0, false);
        }

        let mut forward_count = 0;
        let mut backward_count = 0;
        let cache_line = 64u64;

        for window in self.history.windows(2) {
            let diff = window[1].address.wrapping_sub(window[0].address);
            if diff <= cache_line {
                forward_count += 1;
            }
            if window[0].address.wrapping_sub(window[1].address) <= cache_line {
                backward_count += 1;
            }
        }

        let total = self.history.len() - 1;
        let forward_score = forward_count as f64 / total as f64;
        let backward_score = backward_count as f64 / total as f64;

        if forward_score >= backward_score {
            (forward_score, false)
        } else {
            (backward_score, true)
        }
    }

    /// Check for strided access pattern
    fn check_strided(&self) -> Option<(i64, f64)> {
        if self.strides.is_empty() {
            return None;
        }

        // Find most common non-zero stride
        let total: u32 = self.strides.values().sum();
        if total == 0 {
            return None;
        }

        let (stride, count) = self
            .strides
            .iter()
            .filter(|&(&s, _)| s != 0 && s.abs() > 64 && s.abs() < 4096)
            .max_by_key(|&(_, &c)| c)?;

        let score = *count as f64 / total as f64;
        Some((*stride, score))
    }

    /// Check for temporal locality
    fn check_temporal(&self) -> f64 {
        if self.history.len() < 10 {
            return 0.0;
        }

        let mut address_counts: LinearMap<u32, 64> = BTreeMap::new();
        for record in &self.history {
            // Page granularity
            let page = record.address / 4096;
            *address_counts.entry(page).or_insert(0) += 1;
        }

        // Calculate repetition ratio
        let repeated: u32 = address_counts.values().filter(|&&c| c > 1).sum();
        repeated as f64 / self.history.len() as f64
    }

    /// Check for stack-like pattern
    fn check_stack(&self) -> f64 {
        if self.history.len() < 10 {
            return 0.0;
        }

        // Stack pattern: alternating growth and shrink
        let mut stack_score = 0;
        let mut direction_changes = 0;
        let mut prev_dir: Option<bool> = None; // true = growing

        for window in self.history.windows(2) {
            let growing = window[1].address > window[0].address;

            if let Some(prev) = prev_dir {
                if prev != growing {
                    direction_changes += 1;
                }
            }
            prev_dir = Some(growing);

            // Small deltas are more stack-like
            let delta = (window[1].address as i64 - window[0].address as i64).abs();
            if delta <= 256 {
                stack_score += 1;
            }
        }

        let total = self.history.len() - 1;
        (stack_score as f64 / total as f64)
            * (direction_changes as f64 / (total / 2).max(1) as f64).min(1.0)
    }

    /// Check for pointer-chasing pattern
    fn check_pointer_chasing(&self) -> f64 {
        if self.history.len() < 10 {
            return 0.0;
        }

        // Pointer chasing: irregular strides, often multiples of pointer size
        let ptr_size = 8i64;
        let mut ptr_aligned = 0;

        for window in self.history.windows(2) {
            let delta = (window[1].address as i64 - window[0].address as i64).abs();
            if delta % ptr_size == 0 && delta > 64 {
                ptr_aligned += 1;
            }
        }

        let randomness = 1.0 - self.check_sequential().0;
        let ptr_score = ptr_aligned as f64 / (self.history.len() - 1) as f64;

        (randomness * 0.5 + ptr_score * 0.5).min(1.0)
    }

    /// Get statistics
    #[inline]
    pub fn stats(&self) -> PatternStats {
        let (pattern, confidence) = self.detect_pattern();
        PatternStats {
            total_accesses: self.total_accesses.load(Ordering::Relaxed),
            history_size: self.history.len(),
            detected_pattern: pattern,
            confidence,
            unique_strides: self.strides.len(),
        }
    }

    /// Clear history
    #[inline(always)]
    pub fn clear(&mut self) {
        self.history.clear();
        self.strides.clear();
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Pattern detection statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PatternStats {
    /// Total accesses analyzed
    pub total_accesses: u64,
    /// Current history size
    pub history_size: usize,
    /// Detected pattern
    pub detected_pattern: AccessPattern,
    /// Detection confidence
    pub confidence: f64,
    /// Number of unique strides detected
    pub unique_strides: usize,
}
